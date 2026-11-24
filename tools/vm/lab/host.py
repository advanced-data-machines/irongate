"""Small QEMU/seed helpers; host code never configures network interfaces."""

from datetime import datetime, timezone
import base64
import hashlib
import json
import os
from pathlib import Path
import secrets
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[3]
LAB = ROOT / "target/vm/packet-lab"
SOURCE = Path(__file__).parent


def now():
    return datetime.now(timezone.utc).isoformat()


def digest(path):
    with Path(path).open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def tools():
    for name in ("qemu-system-x86_64", "qemu-img", "cloud-localds"):
        if not shutil.which(name):
            raise RuntimeError(f"Missing {name}")
    if not os.access("/dev/kvm", os.R_OK | os.W_OK):
        raise RuntimeError("KVM is not accessible; no emulation fallback selected")


def overlay(backing, disk):
    subprocess.run(["qemu-img", "create", "-q", "-f", "qcow2", "-F", "qcow2", "-b", str(backing), str(disk), "32G"], check=True)


def seed(directory, run_id, mode):
    startup = "bash /opt/irongate-lab/provision.sh" if mode == "prepare" else f"python3 /opt/irongate-lab/guest.py {run_id} > /dev/ttyS1 2>&1"
    # Only guest cloud-init executes this shell text; all values are fixed or hex.
    script = ("#!/bin/bash\nset -euo pipefail\n"
              "test \"$(cat /sys/class/dmi/id/sys_vendor)\" = QEMU\n"
              "trap 'systemctl poweroff' EXIT\n"
              "systemctl mask --now systemd-networkd-wait-online.service || true\n"
              "chmod 700 /run/irongate-packet-lab\n"
              "stty -F /dev/ttyS1 raw -echo clocal -crtscts 115200\n" + startup + "\n")
    data = {"hostname": "irongate-packet-lab", "manage_etc_hosts": True,
            "ssh_pwauth": False, "disable_root": True, "package_update": False, "package_upgrade": False,
            "write_files": [
                {"path": "/run/irongate-packet-lab/id", "content": run_id + "\n", "permissions": "0600"},
                {"path":"/run/irongate-packet-lab/mode","content":mode+"\n"if mode in ("portal","https")else"packet\n","permissions":"0600"},
                {"path": "/run/irongate-packet-lab/start.sh", "content": script, "permissions": "0700"}],
            "runcmd": [["bash", "/run/irongate-packet-lab/start.sh"]]}
    for artifact in sorted((directory / "payload").iterdir()):
        if not artifact.is_file() or artifact.stat().st_size > 8 * 1024 * 1024:
            raise RuntimeError("Unexpected/oversized seed artifact")
        data["write_files"].append({"path": "/opt/irongate-lab/" + artifact.name,
                                    "encoding": "b64", "permissions": "0755",
                                    "content": base64.b64encode(artifact.read_bytes()).decode("ascii")})
    (directory / "user-data").write_text("#cloud-config\n" + json.dumps(data) + "\n")
    (directory / "meta-data").write_text(json.dumps({"instance-id": "iglab-" + run_id, "local-hostname": "irongate-packet-lab"}))
    network = "version: 2\nethernets: {}\n" if mode != "prepare" else (
        "version: 2\nethernets:\n  management:\n    match:\n      macaddress: '52:54:00:12:34:56'\n    dhcp4: true\n    dhcp6: false\n")
    (directory / "network-config").write_text(network)
    subprocess.run(["cloud-localds", "--network-config", str(directory / "network-config"),
                    str(directory / "seed.img"), str(directory / "user-data"), str(directory / "meta-data")], check=True)


def qemu(directory, prepare=False):
    command = ["qemu-system-x86_64", "-name", "irongate-packet-lab", "-machine", "q35,accel=kvm",
               "-cpu", "host", "-smp", "4", "-m", "8192", "-display", "none", "-serial", "stdio",
               "-serial", f"file:{directory / 'evidence.log'}",
               "-monitor", "none", "-no-reboot", "-drive", f"file={directory / 'disk.qcow2'},format=qcow2,if=virtio",
               "-drive", f"file={directory / 'seed.img'},format=raw,if=virtio,readonly=on"]
    if prepare:
        command += ["-netdev", "user,id=prepnet", "-device", "virtio-net-pci,netdev=prepnet,mac=52:54:00:12:34:56"]
    else:
        command += ["-nic", "none"]
    return command


def boot(directory, prepare=False):
    command = qemu(directory, prepare)
    (directory / "command.json").write_text(json.dumps(command, indent=2) + "\n")
    with (directory / "serial.log").open("w") as log:
        subprocess.run(command, stdin=subprocess.DEVNULL, stdout=log, stderr=subprocess.STDOUT,
                       timeout=600 if prepare else 300, check=True)


def result(directory, marker):
    path = directory / ("evidence.log" if marker == "IG_LAB_RESULT " and (directory / "evidence.log").exists() else "serial.log")
    for line in path.read_text(errors="replace").splitlines():
        if marker in line:
            return json.loads(line.split(marker, 1)[1])
    raise RuntimeError(f"No {marker.strip()} result; see {path}")


def new_run(parent, prefix):
    os.umask(0o077)
    parent.mkdir(parents=True, exist_ok=True)
    run_id = secrets.token_hex(16)
    directory = parent / (prefix + run_id)
    directory.mkdir()
    (directory / "payload").mkdir()
    return directory, run_id
