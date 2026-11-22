"""Provision a separate lab image using temporary QEMU userspace NAT only."""

import json
from tools.vm.image import fetch, specification
from .host import LAB, boot, digest, new_run, now, overlay, result, seed, tools


def main():
    tools()
    manifest = LAB / "prepared.json"
    if manifest.exists():
        data = json.loads(manifest.read_text())
        if digest(data["image"]) != data["sha256"]:
            raise RuntimeError("Prepared image changed; refusing cached provenance")
        print(f"Verified existing lab image: {data['image']}")
        return
    directory, run_id = new_run(LAB, "prepare-")
    overlay(fetch(), directory / "disk.qcow2")
    script = '''#!/bin/bash
set -euo pipefail
test "$(cat /sys/class/dmi/id/sys_vendor)" = QEMU
export DEBIAN_FRONTEND=noninteractive
apt-get -o Acquire::Retries=1 -o Acquire::http::Timeout=20 update
apt-get -y --no-install-recommends install iproute2 nftables busybox ethtool python3
python3 - <<'PY'
import json, pathlib, subprocess
packages=subprocess.check_output(['dpkg-query','-W','-f=${Package}=${Version}\\n','iproute2','nftables','busybox','ethtool','python3'],text=True).splitlines()
print('IG_PROVISION_RESULT '+json.dumps({'status':'prepared','run_id':pathlib.Path('/run/irongate-packet-lab/id').read_text().strip(),'packages':packages}),flush=True)
PY
'''
    (directory / "payload/provision.sh").write_text(script)
    seed(directory, run_id, "prepare")
    print(f"Preparing guest packages with temporary userspace NAT; log: {directory / 'serial.log'}", flush=True)
    boot(directory, prepare=True)
    outcome = result(directory, "IG_PROVISION_RESULT ")
    if outcome.get("status") != "prepared" or outcome.get("run_id") != run_id:
        raise RuntimeError("Guest preparation did not complete")
    image = directory / "disk.qcow2"
    image.chmod(0o444)
    manifest.write_text(json.dumps({"schema": 1, "created": now(), "image": str(image),
                                    "sha256": digest(image), "base_image": specification(), "guest": outcome}, indent=2) + "\n")
    print("Prepared immutable package image. Demonstration boots use -nic none and a fresh overlay.")


if __name__ == "__main__":
    main()
