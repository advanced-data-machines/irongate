"""Fresh NIC-free VM run with read-only seed artifacts and retained JSON evidence."""

import argparse
import json
import platform
import shlex
import shutil
import subprocess
import sys
from pathlib import Path
from tools.vm.image import specification
from .evidence import validate
from .acceptance import load
from .artifacts import extract
from .host import LAB, ROOT, SOURCE, boot, digest, new_run, now, overlay, qemu, result, seed, tools


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--print", action="store_true", help="Print the planned NIC-free QEMU command without changes")
    parser.add_argument("--portal",action="store_true",help="Use the guest browser image and consent walkthrough")
    args = parser.parse_args()
    if args.print:
        print(shlex.join(qemu(LAB / "runs/NEW_RUN")))
        return
    tools()
    prepared = json.loads((LAB / ("portal-prepared.json"if args.portal else"prepared.json")).read_text())
    if digest(prepared["image"]) != prepared["sha256"]:
        raise RuntimeError("Prepared backing image digest mismatch")
    metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--no-deps", "--format-version=1", "--offline"], cwd=ROOT))
    binary = Path(metadata["target_directory"]) / "x86_64-unknown-linux-musl/release/irongated"
    directory, run_id = new_run(LAB / "runs", "")
    report_path = directory / "report.json"
    report = {"schema": 2, "suite": "untagged-acceptance-v1", "status": "in_progress", "run_id": run_id, "started": now(),
              "scope": "isolated_packet_lab", "commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
              "dirty": bool(subprocess.check_output(["git", "status", "--porcelain"], cwd=ROOT)),
              "host": platform.platform(), "prepared_image_sha256": prepared["sha256"],
              "base_image": specification(), "prepared_packages": prepared["guest"]["packages"],
              "lockfile_sha256": digest(ROOT / "Cargo.lock"),
              "build": {"target": "x86_64-unknown-linux-musl", "profile": "release"},
              "resources": {"vcpus": 4, "ram_mib": 8192, "disk_gib": 32, "external_nics": 0},
              "acceptance": load(),
              "scale_qualification": {"status": "not_run", "reason": "One-client functional suite; 64-session/two-network performance and accounting are not implemented"},
              "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
              "qemu": subprocess.check_output(["qemu-system-x86_64", "--version"], text=True).splitlines()[0]}
    if args.portal:report.update({"schema":3,"suite":"local-consent-v1"})
    report_path.write_text(json.dumps(report, indent=2) + "\n")
    try:
        shutil.copy2(binary, directory / "payload/irongated")
        report["binary_sha256"] = digest(binary)
        for name in ("guest.py", "guest_common.py", "topology.py", "scenarios.py", "supervisor.py", "client_lease.py", "services.py", "probe.py", "security.py", "capture.py", "inject.py","browser.py","api_client.py","portal_scenarios.py","artifacts.py"):
            shutil.copy2(SOURCE / name, directory / "payload" / name)
        (directory / "payload/client_lease.py").chmod(0o755)
        report["harness_sha256"] = {p.name: digest(p) for p in (directory / "payload").iterdir()}
        overlay(prepared["image"], directory / "disk.qcow2")
        seed(directory, run_id, "portal"if args.portal else"test")
        print(f"Booting NIC-free packet lab; retained evidence: {directory}", flush=True)
        boot(directory)
        guest = result(directory, "IG_LAB_RESULT ")
        report["guest"] = guest
        validate(guest, run_id,portal=args.portal)
        if args.portal:extract(directory,run_id,guest.get("screenshots",{}))
        report["status"] = "passed"
    except Exception as error:
        report["status"] = "failed"
        report["error"] = f"{type(error).__name__}: {error}"
    finally:
        report["finished"] = now()
        report_path.write_text(json.dumps(report, indent=2) + "\n")
    print(f"{report['status']}: {report_path}")
    if report["status"] != "passed":
        sys.exit(1)


if __name__ == "__main__":
    main()
