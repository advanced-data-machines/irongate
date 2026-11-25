"""Execute only inside the owned NIC-free guest; emit one canonical result."""

import json
import os
import platform
import subprocess
import sys
import time
import termios
import traceback
from guest_common import WORK, GATEWAY, UPSTREAM, command, guard, spawn
import scenarios
import topology
import artifacts
import portal_config


def main(run_id):
    guard(run_id)
    children, created = [], []
    report = {"schema": 2, "suite": "untagged-acceptance-v1", "scope": "isolated_packet_lab", "run_id": run_id,
              "status": "in_progress", "cases": [], "kernel": platform.release()}
    logs = []
    if (WORK/"mode").read_text().strip()in ("portal","https","radius"):
        report.update({"schema":3,"suite":"local-consent-v1","portal":True})
    if portal_config.secure():report.update({"schema":4,"suite":"local-https-v1","https":True})
    if (WORK/"mode").read_text().strip()=="radius":report.update({"schema":5,"suite":"radius-login-v1","radius":True})
    try:
        portal_config.configure(run_id)
        interfaces = json.loads(command("ip", "-j", "link").stdout)
        if {item["ifname"] for item in interfaces} != {"lo"}:
            raise RuntimeError("Demo requires -nic none and loopback-only management")
        report["management_interfaces"] = [item["ifname"] for item in interfaces]
        report["management_namespace"] = os.readlink("/proc/self/ns/net")
        report["vcpus"] = os.cpu_count()
        report["packages"] = command("dpkg-query", "-W", "-f=${Package}=${Version}\n",
                                     "iproute2", "nftables", "busybox", "ethtool", "python3").stdout.splitlines()
        report["topology"] = topology.prepare(created,portal_config.port(),portal_config.secure(),report.get("radius",False))
        for namespace, host, port in [(UPSTREAM, "192.0.2.80", "80")]:
            log = (WORK / f"service-{port}.log").open("w")
            logs.append(log)
            children.append(spawn("python3", "/opt/irongate-lab/services.py", host, port,
                                  namespace=namespace, stdout=log, stderr=log))
        log = (WORK / "supervisor.log").open("w")
        logs.append(log)
        children.append(spawn("python3", "/opt/irongate-lab/supervisor.py", run_id,
                              namespace=GATEWAY, stdout=log, stderr=log))
        if report.get("radius"):
            import radius_backend
            report["radius_packages"]=command("dpkg-query","-W","-f=${Package}=${Version}\n","freeradius","freeradius-utils").stdout.splitlines()
            report["radius_process"]=radius_backend.start(children).pid
        time.sleep(0.2)
        scenarios.run(children, report)
        report["status"] = "passed"
    except Exception as error:
        report["status"] = "failed"
        report["error"] = f"{type(error).__name__}: {error}"
        report["traceback"] = traceback.format_exc(limit=8)
        if isinstance(error, subprocess.CalledProcessError):
            report["command_error"] = {"args": error.cmd, "stdout": error.stdout, "stderr": error.stderr}
    finally:
        # Kill only processes created in this run. The daemon is a supervisor child.
        pid_file = WORK / "daemon.pid"
        if pid_file.exists():
            pid = int(pid_file.read_text())
            try:
                if os.readlink(f"/proc/{pid}/exe") == "/opt/irongate-lab/irongated":
                    os.kill(pid, 15)
            except ProcessLookupError:
                pass
            except FileNotFoundError:
                pass
        for proc in reversed(children):
            if proc.poll() is None:
                proc.terminate()
            try:
                proc.wait(timeout=3)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait()
        for log in logs:
            log.close()
        report["cleanup_errors"] = topology.cleanup(created)
        if report["cleanup_errors"]:
            report["status"] = "failed"
        report["logs"] = {p.name: p.read_text(errors="replace")[-8000:] for p in WORK.glob("*.log")}
        if report.get("radius") and (WORK/"radius/secret").exists():
            secret=(WORK/"radius/secret").read_text().strip()
            report["logs"]={name:content.replace(secret,"<redacted>").replace("lab-pass","<test-password>")for name,content in report["logs"].items()}
        if report.get("portal"):
            report["screenshots"]=artifacts.export(WORK,run_id)
        print("IG_LAB_RESULT " + json.dumps(report, separators=(",", ":")), flush=True)
        # The seed connects stdout directly to the serial TTY. Drain all image
        # chunks and the result before the shell requests shutdown; cloud-init
        # tee buffering otherwise lets shutdown logs interrupt exported chunks.
        termios.tcdrain(sys.stdout.fileno())
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1]))
