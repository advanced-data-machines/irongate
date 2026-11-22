"""Own three fresh guest namespaces; never alter the management namespace routes."""

import json
from guest_common import CLIENT, GATEWAY, UPSTREAM, command

NAMES = [CLIENT, GATEWAY, UPSTREAM]


def prepare(created):
    existing = {entry["name"] for entry in json.loads(command("ip", "-j", "netns").stdout.strip() or "[]")}
    if existing.intersection(NAMES):
        raise RuntimeError("Owned namespace names already exist; refusing to reuse/delete them")
    for name in NAMES:
        command("ip", "netns", "add", name)
        created.append(name)
        command("ip", "link", "set", "lo", "up", namespace=name)
        command("sysctl", "-qw", "net.ipv6.conf.all.disable_ipv6=1", namespace=name)
        command("sysctl", "-qw", "net.ipv6.conf.default.disable_ipv6=1", namespace=name)
    for left, right, left_ns, right_ns in [("igc0", "igd0", CLIENT, GATEWAY), ("igw0", "igu0", GATEWAY, UPSTREAM)]:
        command("ip", "link", "add", left, "netns", left_ns, "type", "veth", "peer", "name", right, "netns", right_ns)
    for name, old, new, mac in [
        (CLIENT, "igc0", "eth0", "02:00:00:00:00:02"),
        (GATEWAY, "igd0", "down0", "02:00:00:00:00:01"),
        (GATEWAY, "igw0", "up0", "02:00:00:00:01:01"),
        (UPSTREAM, "igu0", "eth0", "02:00:00:00:01:02"),
    ]:
        command("ip", "link", "set", old, "name", new, namespace=name)
        command("ip", "link", "set", new, "address", mac, "mtu", "1500", "up", namespace=name)
        command("ethtool", "-K", new, "tx", "off", "tso", "off", "gso", "off", "gro", "off", namespace=name)
    for name, address, device in [(GATEWAY, "10.50.10.1/24", "down0"),
                                   (GATEWAY, "192.0.2.1/24", "up0"), (UPSTREAM, "192.0.2.80/24", "eth0")]:
        command("ip", "addr", "add", address, "dev", device, namespace=name)
    command("ip", "route", "add", "10.50.10.0/24", "via", "192.0.2.1", namespace=UPSTREAM)
    command("sysctl", "-qw", "net.ipv4.ip_forward=0", namespace=GATEWAY)
    for scope in ("all", "down0", "up0"):
        command("sysctl", "-qw", f"net.ipv4.conf.{scope}.arp_ignore=8", namespace=GATEWAY)
    rules = '''table inet ig_lab {
      chain input { type filter hook input priority 0; policy drop;
        iifname "down0" ip saddr 10.50.10.100 tcp dport 8080 accept; }
      chain forward { type filter hook forward priority 0; policy drop; counter drop; }
      chain output { type filter hook output priority 0; policy drop;
        oifname "down0" ip daddr 10.50.10.100 tcp sport 8080 accept; }
    }'''
    command("nft", "-f", "-", namespace=GATEWAY, input=rules)
    return {
        "guard": json.loads(command("nft", "-j", "list", "ruleset", namespace=GATEWAY).stdout),
        "ip_forward": command("sysctl", "-n", "net.ipv4.ip_forward", namespace=GATEWAY).stdout.strip(),
        "namespaces": {name: command("readlink", "/proc/self/ns/net", namespace=name).stdout.strip() for name in NAMES},
    }


def cleanup(created):
    errors = []
    for name in reversed(created):
        try:
            result = command("ip", "netns", "delete", name, check=False)
            if result.returncode:
                errors.append({"namespace": name, "error": result.stderr})
        except Exception as error:
            errors.append({"namespace": name, "error": str(error)})
    return errors
