"""Guest-only safeguards and namespace command helpers (standard library)."""

import os
from pathlib import Path
import subprocess

WORK = Path("/run/irongate-packet-lab")
CLIENT, GATEWAY, UPSTREAM = "iglab-client", "iglab-gateway", "iglab-upstream"


def guard(run_id):
    if (os.geteuid() != 0 or len(run_id) != 32
            or any(c not in "0123456789abcdef" for c in run_id)
            or Path("/sys/class/dmi/id/sys_vendor").read_text().strip() != "QEMU"
            or (WORK / "id").read_text().strip() != run_id):
        raise RuntimeError("Requires the owned disposable QEMU guest/run identity")


def namespace_guard():
    guard((WORK / "id").read_text().strip())
    if os.readlink("/proc/self/ns/net") == os.readlink("/proc/1/ns/net"):
        raise RuntimeError("Refusing fixture network I/O in the management/host namespace")


def command(*args, namespace=None, timeout=15, check=True, **kwargs):
    argv = (["ip", "netns", "exec", namespace] if namespace else []) + list(args)
    return subprocess.run(argv, check=check, capture_output=True, text=True,
                          timeout=timeout, **kwargs)


def spawn(*args, namespace, **kwargs):
    return subprocess.Popen(["ip", "netns", "exec", namespace, *args], **kwargs)
