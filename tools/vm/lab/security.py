"""Live ingress/egress observations, with a positive control for spoof tests."""

import ipaddress
import json
import secrets
import select
import struct
import subprocess
from guest_common import CLIENT, GATEWAY, UPSTREAM, command, spawn


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def receive(proc):
    if not select.select([proc.stdout], [], [], 3)[0]:
        raise RuntimeError("Packet observer timed out")
    line = proc.stdout.readline()
    require(bool(line), "Packet observer exited without evidence")
    return json.loads(line)


def checksum(data):
    data += bytes(len(data) % 2)
    total = sum(struct.unpack("!" + "H" * (len(data) // 2), data))
    while total >> 16:
        total = (total & 65535) + (total >> 16)
    return (~total) & 65535


def frame(marker, *, bad_mac=False, bad_ip=False, ipv6=False):
    mac = bytes.fromhex("020000000003" if bad_mac else "020000000002")
    ethernet = bytes.fromhex("020000000001") + mac
    udp = struct.pack("!HHHH", 40001, 9001, 8 + len(marker), 0) + marker
    if ipv6:
        source = ipaddress.IPv6Address("fd00:50:10::100").packed
        target = ipaddress.IPv6Address("fd00:50:2::80").packed
        pseudo = source + target + struct.pack("!I", len(udp)) + bytes([0, 0, 0, 17])
        udp = udp[:6] + struct.pack("!H", checksum(pseudo + udp) or 65535) + udp[8:]
        return ethernet + bytes.fromhex("86dd") + struct.pack("!IHBB", 0x60000000, len(udp), 17, 64) + source + target + udp
    source = bytes([10, 50, 10, 101 if bad_ip else 100])
    target = bytes([192, 0, 2, 80])
    pseudo = source + target + bytes([0, 17]) + struct.pack("!H", len(udp))
    udp = udp[:6] + struct.pack("!H", checksum(pseudo + udp) or 65535) + udp[8:]
    ip = struct.pack("!BBHHHBBH4s4s", 0x45, 0, 20 + len(udp), 0, 0x4000, 64, 17, 0, source, target)
    ip = ip[:10] + struct.pack("!H", checksum(ip)) + ip[12:]
    return ethernet + bytes.fromhex("0800") + ip + udp


def observe(children, **variant):
    marker = b"ig-acceptance-" + secrets.token_bytes(12)
    observers = []
    for namespace, interface, destination in [(GATEWAY, "down0", "020000000001"), (UPSTREAM, "eth0", "020000000102")]:
        proc = spawn("python3", "/opt/irongate-lab/capture.py", interface, destination, marker.hex(),
                     namespace=namespace, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        children.append(proc)
        observers.append(proc)
        require(receive(proc).get("ready"), "Packet observer not ready")
    for proc in observers:
        proc.stdin.write("capture\n")
        proc.stdin.flush()
        require(receive(proc).get("armed"), "Packet observer not armed")
    if variant.pop("kernel_ipv6", False):
        command("python3", "/opt/irongate-lab/inject.py", "ipv6", marker.hex(), namespace=CLIENT)
    else:
        command("python3", "/opt/irongate-lab/inject.py", "frame", frame(marker, **variant).hex(), namespace=CLIENT)
    result = [receive(proc)["received"] for proc in observers]
    for proc in observers:
        proc.stdin.close()
        require(proc.wait(timeout=3) == 0, "Packet observer failed")
    require(result[0] > 0, f"Probe did not reach gateway (ingress={result[0]}, upstream={result[1]}); drop evidence is inconclusive")
    return {"ingress_received": result[0], "upstream_received": result[1]}


def admitted(children, report, control):
    states = {"before": control("status")["allowed"]}
    report["security_authorized"] = states
    require(states["before"], "Spoof tests require live authorized state")
    probes = {"baseline": observe(children)}
    report["security_probes"] = probes
    require(probes["baseline"]["upstream_received"] > 0, "Valid raw-frame positive control failed")
    for name, variant in [("authorized_spoofed_mac_denied", {"bad_mac": True}),
                          ("authorized_spoofed_ip_denied", {"bad_ip": True}),
                          ("authorized_ipv6_userspace_denied", {"ipv6": True})]:
        probes[name] = observe(children, **variant)
        require(probes[name]["upstream_received"] == 0, f"Bypass reached upstream: {name}")
        report["cases"].append({"name": name, "passed": True})
    states["after"] = control("status")["allowed"]
    require(states["after"], "Admission expired during spoof tests; result is inconclusive")
    report["security_probes"] = probes
    report["security_authorized"] = states


def ipv6_counter():
    rules = json.loads(command("nft", "-j", "list", "ruleset", namespace=GATEWAY).stdout)
    return sum(expr.get("counter", {}).get("packets", 0) for item in rules["nftables"]
               if item.get("rule", {}).get("comment") == "iglab-ipv6-deny" for expr in item["rule"]["expr"])


def kernel_ipv6(children, report):
    # Fault injection in owned namespaces: enable real IPv6 routing with fixed
    # addresses/neighbors, so the persistent guard is the tested boundary.
    for namespace in (CLIENT, GATEWAY, UPSTREAM):
        command("sysctl", "-qw", "net.ipv6.conf.all.disable_ipv6=0", namespace=namespace)
        command("sysctl", "-qw", "net.ipv6.conf.default.disable_ipv6=0", namespace=namespace)
    for namespace, interface, address in [(CLIENT, "eth0", "fd00:50:10::100/64"),
                                         (GATEWAY, "down0", "fd00:50:10::1/64"),
                                         (GATEWAY, "up0", "fd00:50:2::1/64"), (UPSTREAM, "eth0", "fd00:50:2::80/64")]:
        command("sysctl", "-qw", f"net.ipv6.conf.{interface}.disable_ipv6=0", namespace=namespace)
        command("ip", "-6", "addr", "add", address, "dev", interface, "nodad", namespace=namespace)
    command("ip", "-6", "route", "add", "default", "via", "fd00:50:10::1", namespace=CLIENT)
    command("ip", "-6", "route", "add", "fd00:50:10::/64", "via", "fd00:50:2::1", namespace=UPSTREAM)
    command("sysctl", "-qw", "net.ipv6.conf.all.forwarding=1", namespace=GATEWAY)
    for namespace, interface, address, mac in [(CLIENT, "eth0", "fd00:50:10::1", "020000000001"),
        (GATEWAY, "down0", "fd00:50:10::100", "020000000002"),
        (GATEWAY, "up0", "fd00:50:2::80", "020000000102"), (UPSTREAM, "eth0", "fd00:50:2::1", "020000000101")]:
        mac = ":".join(mac[i:i+2] for i in range(0, 12, 2))
        command("ip", "-6", "neigh", "replace", address, "lladdr", mac, "nud", "permanent", "dev", interface, namespace=namespace)
    before = ipv6_counter()
    probe = observe(children, kernel_ipv6=True)
    after = ipv6_counter()
    report["ipv6_guard"] = {"kernel_forwarding": 1, "forward_drop_delta": after - before, **probe}
    require(probe["upstream_received"] == 0 and after > before, "Persistent IPv6 guard was not independently exercised")
    report["cases"].append({"name": "ipv6_kernel_guard_after_crash", "passed": True})
