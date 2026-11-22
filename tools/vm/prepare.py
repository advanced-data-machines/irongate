"""Prepare a private disposable disk and offline NoCloud seed; never boots."""

import json
import os
from pathlib import Path
import secrets
import shutil
import subprocess
import tempfile

from image import ARTIFACTS, fetch, specification


def prepare():
    for tool in ("qemu-img", "cloud-localds"):
        if not shutil.which(tool):
            raise SystemExit(f"Missing {tool}. Install qemu-utils and cloud-image-utils first; see docs/qemu-first-boot.md.")
    run = ARTIFACTS / "learn"
    if run.exists():
        raise SystemExit(f"Already prepared: {run}. Boot it with just vm-boot; existing disks are never reset implicitly.")
    image = fetch()
    info = json.loads(subprocess.check_output(["qemu-img", "info", "-f", "qcow2", "--output=json", str(image)]))
    if info.get("backing-filename") or info.get("format") != "qcow2":
        raise SystemExit("Expected a standalone qcow2 base; refusing an unexpected backing chain.")
    os.umask(0o077)
    with tempfile.TemporaryDirectory(prefix="prepare-", dir=ARTIFACTS) as temporary:
        directory = Path(temporary)
        password = secrets.token_urlsafe(18)
        user_data = {
            "hostname": "irongate-learn", "manage_etc_hosts": True,
            "users": [{"name": "lab", "shell": "/bin/bash", "groups": ["sudo"],
                       "lock_passwd": False, "plain_text_passwd": password,
                       "sudo": ["ALL=(ALL) NOPASSWD:ALL"]}],
            "ssh_pwauth": False, "disable_root": True, "package_update": False, "package_upgrade": False,
            # The base may wait for a NIC on its very first boot. There are no
            # NICs in lesson one; prevent that wait on subsequent guest boots.
            "runcmd": [["systemctl", "mask", "--now", "systemd-networkd-wait-online.service"],
                       ["systemctl", "reset-failed", "systemd-networkd-wait-online.service"]],
        }
        (directory / "user-data").write_text("#cloud-config\n" + json.dumps(user_data, indent=2) + "\n")
        (directory / "meta-data").write_text(json.dumps({"instance-id": "irongate-learn-" + secrets.token_hex(8),
                                                        "local-hostname": "irongate-learn"}) + "\n")
        (directory / "network-config").write_text("version: 2\nethernets: {}\n")
        (directory / "console-login.txt").write_text(f"Guest console user: lab\nGuest console password: {password}\n")
        subprocess.run(["cloud-localds", "--network-config", str(directory / "network-config"),
                        str(directory / "seed.img"), str(directory / "user-data"), str(directory / "meta-data")], check=True)
        subprocess.run(["qemu-img", "create", "-f", "qcow2", "-F", "qcow2", "-b", str(image),
                        str(directory / "disk.qcow2"), "32G"], check=True)
        (directory / "image-provenance.json").write_text(json.dumps(specification(), indent=2) + "\n")
        directory.rename(run)
    print(f"Prepared {run}. Base unchanged; guest writes go into disk.qcow2.")
    print("Console credentials are private in target/vm/learn/console-login.txt (not printed here).")
    print("Next: just vm-command to inspect the command; just vm-boot to start it.")


if __name__ == "__main__":
    prepare()
