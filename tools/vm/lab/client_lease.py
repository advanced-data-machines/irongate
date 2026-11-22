#!/usr/bin/python3
"""BusyBox DHCP hook: exact owned client/address only, never host DNS changes."""

import json
import os
from pathlib import Path
import subprocess
import sys
from guest_common import guard

WORK = Path("/run/irongate-packet-lab")
guard((WORK / "id").read_text().strip())
if os.readlink("/proc/self/ns/net") == os.readlink("/proc/1/ns/net"):
    raise SystemExit("Refusing DHCP hook in management/host namespace")
if sys.argv[1] in ("bound", "renew"):
    if os.environ.get("interface") != "eth0" or os.environ.get("ip") != "10.50.10.100":
        raise SystemExit("Unexpected lab DHCP address/interface")
    if os.environ.get("router", "").strip() != "10.50.10.1":
        raise SystemExit("Unexpected lab DHCP router")
    subprocess.run(["ip", "addr", "replace", "10.50.10.100/24", "dev", "eth0"], check=True)
    subprocess.run(["ip", "route", "replace", "default", "via", "10.50.10.1", "dev", "eth0"], check=True)
    (WORK / "client-lease.json").write_text(json.dumps({
        "address": os.environ["ip"], "router": os.environ["router"], "server": os.environ.get("serverid"),
        "lease_seconds": os.environ.get("lease"), "namespace": os.readlink("/proc/self/ns/net"),
    }))
