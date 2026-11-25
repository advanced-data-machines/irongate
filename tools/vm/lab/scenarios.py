"""Independent real client probes and root-only test consent controls."""

import json
import os
import select
import signal
import socket
import subprocess
import time
from guest_common import CLIENT, GATEWAY, UPSTREAM, WORK, command, spawn
import security
import portal_config


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def control(verb):
    path = WORK / "probe-control.sock"
    with socket.socket(socket.AF_UNIX, socket.SOCK_DGRAM) as sock:
        sock.bind(str(path))
        try:
            sock.settimeout(2)
            sock.connect(str(WORK / "control.sock"))
            sock.send(verb.encode())
            result = json.loads(sock.recv(8192))
            require(result.get("ok"), f"Control failed: {result}")
            return result
        finally:
            path.unlink()


class Probe:
    def __init__(self, children):
        self.proc = spawn("python3", "/opt/irongate-lab/probe.py", namespace=CLIENT,
                          stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        children.append(self.proc)

    def request(self, target="192.0.2.80:80"):
        self.proc.stdin.write(target + "\n")
        self.proc.stdin.flush()
        if not select.select([self.proc.stdout], [], [], 4)[0]:
            raise RuntimeError("Client probe produced no result")
        line = self.proc.stdout.readline()
        require(bool(line), "Client probe exited without evidence")
        return json.loads(line)["reachable"]

    def close(self):
        self.proc.stdin.close()
        self.proc.wait(timeout=4)


def fresh(children, target="192.0.2.80:80"):
    probe = Probe(children)
    result = probe.request(target)
    probe.close()
    return result


def run(children, report):
    cases = report["cases"]
    for _ in range(100):
        if (WORK / "control.sock").exists():
            break
        time.sleep(0.05)
    require((WORK / "control.sock").exists(), "Daemon did not become ready")
    dhcp = command("busybox", "udhcpc", "-f", "-n", "-q", "-i", "eth0", "-t", "5", "-T", "2",
                   "-s", "/opt/irongate-lab/client_lease.py", namespace=CLIENT, timeout=20)
    report["dhcp_client_log"] = dhcp.stdout + dhcp.stderr
    report["lease"] = json.loads((WORK / "client-lease.json").read_text())
    require(report["lease"]["address"] == "10.50.10.100", "Wrong DHCP address")
    status = control("status")
    require(status["phase"] == "Leased" and not status["allowed"], "Lease unexpectedly authorized traffic")
    cases.append({"name": "real_busybox_dhcp", "passed": True})
    require(fresh(children, f"10.50.10.1:{portal_config.port()}"), "Native local service unreachable before admission")
    require(control("status")["counters"]["arp"] > 0, "No userspace ARP reply observed")
    cases.append({"name": "userspace_arp_and_native_local_tcp", "passed": True})
    require(not fresh(children), "Upstream reachable without consent")
    require(control("status")["counters"]["denied"] > 0, "No userspace policy drop recorded")
    cases.append({"name": "unauthorized_transit_denied", "passed": True})
    if report.get("accounting"):
        import accounting_scenarios
        accounting_scenarios.run(children,report)
    elif report.get("radius"):
        import radius_scenarios
        radius_scenarios.run(children,report)
    elif report.get("portal"):
        import portal_scenarios
        portal_scenarios.run(children,report)
    if report.get("https"):
        import tls_scenarios
        tls_scenarios.run(children,report)
    require(control("consent")["allowed"], "Policy application did not authorize")
    require(fresh(children), "Fresh admitted upstream TCP failed")
    live = Probe(children)
    require(live.request(), "Admitted persistent TCP failed")
    dns = command("python3", "-c", "import socket; s=socket.socket(socket.AF_INET,socket.SOCK_DGRAM); s.settimeout(1); "
                  "s.sendto(bytes.fromhex('123401000001000000000000066f726967696e04746573740000010001'),('192.0.2.80',53)); "
                  "r=s.recv(512); assert r[:2]==bytes.fromhex('1234') and r[-4:]==bytes([192,0,2,80]); print('DNS fixture verified')",
                  namespace=CLIENT)
    report["dns_probe"] = dns.stdout.strip()
    cases.append({"name": "authorized_bidirectional_tcp_and_dns", "passed": True})
    security.admitted(children, report, control)
    require(not control("logout")["allowed"], "Logout left access authorized")
    require(not live.request(), "Existing TCP survived logout")
    live.close()
    require(not fresh(children), "Fresh TCP survived logout")
    cases.append({"name": "logout_blocks_existing_and_new_tcp", "passed": True})
    control("consent")
    expiring = Probe(children)
    require(expiring.request(), "Renewed admission did not reach origin")
    time.sleep(6)
    require(not control("status")["allowed"], "Expired access still authorized")
    require(not expiring.request(), "Existing TCP survived expiry")
    expiring.close()
    require(not fresh(children), "Fresh TCP survived expiry")
    cases.append({"name": "expiry_blocks_existing_and_new_tcp", "passed": True})
    control("consent")
    dying = Probe(children)
    require(dying.request(), "Crash baseline TCP failed")
    report["before_crash"] = control("status")
    # Fix neighbor evidence before crash, so failure cannot just be absent ARP.
    for ns, address, mac, device in [
        (CLIENT, "10.50.10.1", "02:00:00:00:00:01", "eth0"),
        (GATEWAY, "10.50.10.100", "02:00:00:00:00:02", "down0"),
        (GATEWAY, "192.0.2.80", "02:00:00:00:01:02", "up0"),
        (UPSTREAM, "192.0.2.1", "02:00:00:00:01:01", "eth0"),
    ]:
        command("ip", "neigh", "replace", address, "lladdr", mac, "nud", "permanent", "dev", device, namespace=ns)
    pid = int((WORK / "daemon.pid").read_text())
    require(os.readlink(f"/proc/{pid}/exe") == "/opt/irongate-lab/irongated", "Refusing to kill an unexpected executable")
    os.kill(pid, signal.SIGKILL)
    time.sleep(0.1)
    # Fault injection: even if kernel routing is enabled, the persistent guard
    # must prevent bypass. This affects only the owned gateway namespace.
    command("sysctl", "-qw", "net.ipv4.ip_forward=1", namespace=GATEWAY)
    require(not dying.request(), "Existing TCP survived daemon crash")
    dying.close()
    require(not fresh(children), "Kernel forwarding bypassed the persistent guard")
    guard = json.loads(command("nft", "-j", "list", "ruleset", namespace=GATEWAY).stdout)
    drops = sum(expr.get("counter", {}).get("packets", 0) for item in guard["nftables"]
                if item.get("rule", {}).get("chain") == "forward" for expr in item["rule"]["expr"])
    require(drops > 0, "Crash guard was not independently exercised by kernel transit")
    report["crash_guard"] = {"kernel_ip_forward": 1, "forward_drop_packets": drops, "ruleset": guard}
    cases.append({"name": "crash_guard_with_kernel_routing_and_fixed_neighbors", "passed": True})
    security.kernel_ipv6(children, report)
