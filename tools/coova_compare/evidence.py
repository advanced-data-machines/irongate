"""Provenance and report lifecycle, independent of individual comparison suites."""

from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess

from baseline import SCHEMA_VERSION
from render import markdown
from source import ROOT, REVISION


def fresh_report(selected, args):
    return {"schema_version": SCHEMA_VERSION, "result": "in_progress", "complete": False,
            "started_at_utc": datetime.now(timezone.utc).isoformat(), "source": {"revision": REVISION},
            "scope": "offline extracted-function comparisons; not full daemon readiness",
            "selected_suites": [suite.name for suite in selected], "suites": [],
            "controls": {"iterations": args.iterations, "samples": args.samples,
                         "checks_only": args.checks_only, "skip_memory": args.skip_memory}}


def provenance(report, output, manifest, binaries):
    affinity = None
    if hasattr(os, "sched_getaffinity"):
        affinity = min(os.sched_getaffinity(0))
        os.sched_setaffinity(0, {affinity})
    model = platform.processor()
    if Path("/proc/cpuinfo").exists():
        with Path("/proc/cpuinfo").open() as source:
            for line in source.read(65536).splitlines():
                if line.startswith("model name"):
                    model = line.split(":", 1)[1].strip()
                    break
    machine = {"platform": platform.platform(), "architecture": platform.machine(),
               "cpu_model": model, "cpu_affinity": affinity}
    report.update(source=manifest, machine=machine,
                  rust_git_head=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
                  binaries_sha256={name: hashlib.sha256(path.read_bytes()).hexdigest()
                                   for name, path in binaries.items()})
    paths = [*ROOT.joinpath("crates/irongate-proto").rglob("*.rs"),
             *Path(__file__).parent.glob("*.py"), *Path(__file__).parent.glob("*.c"),
             *Path(__file__).parent.glob("*.h"), ROOT / "Cargo.toml", ROOT / "Cargo.lock",
             ROOT / "crates/irongate-proto/Cargo.toml"]
    report["working_sources_sha256"] = {str(path.relative_to(ROOT)): hashlib.sha256(path.read_bytes()).hexdigest()
                                         for path in sorted(paths)}
    report["measurement_context"] = {
        "machine": machine, "c_compiler": manifest["compiler"], "rustc": manifest["rustc"],
        "c_flags": ["<generated>" if item == str(output / "generated") else item for item in manifest["c_flags"]],
        "rust_profile": manifest["rust_profile"], "fixture_profile": manifest["profile"],
        "original_c_blobs": manifest["blobs_sha256"],
        "adapters_sha256": {name: digest for name, digest in report["working_sources_sha256"].items()
                            if name.endswith(("adapter.c", "adapter.h", "coova_probe.rs", "tests/common/mod.rs"))},
    }


def write_report(output, report):
    output.mkdir(parents=True, exist_ok=True)
    # Replace previous success at run start and preserve failures explicitly.
    for name, value in (("report.json", json.dumps(report, indent=2) + "\n"),
                        ("report.md", markdown(report))):
        temporary = output / (name + ".tmp")
        temporary.write_text(value)
        temporary.replace(output / name)
