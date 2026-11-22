"""Build an offline, feature-scoped original C probe plus a Rust example."""

import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess

from source import ROOT, prepare


def build(repository, output):
    generated = output / "generated"
    manifest = prepare(repository, generated)
    for name in ("adapter.h", "adapter.c"):
        shutil.copyfile(Path(__file__).with_name(name), generated / name)
    compiler = shlex.split(os.environ.get("CC", "cc"))
    flags = ["-std=gnu11", "-O3", "-fno-strict-aliasing", "-fno-lto",
             "-Wno-address-of-packed-member", "-I", str(generated)]
    sources = [str(generated / name) for name in ("original.c", "adapter.c")]
    c_binary = output / "coova-probe"
    subprocess.run([*compiler, *flags, *sources, "-o", str(c_binary)], check=True)
    sanitizer = output / "coova-probe-sanitized"
    sanitizer_flags = ["-std=gnu11", "-O1", "-g", "-fno-omit-frame-pointer",
                       "-fno-strict-aliasing", "-fsanitize=address,undefined",
                       "-fno-sanitize-recover=all", "-Wno-address-of-packed-member",
                       "-I", str(generated)]
    subprocess.run(["clang", *sanitizer_flags, *sources, "-o", str(sanitizer)], check=True)
    subprocess.run(["cargo", "build", "-p", "irongate-proto", "--example", "coova_probe",
                    "--release", "--offline"], check=True, cwd=ROOT)
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--format-version=1", "--no-deps", "--offline"], cwd=ROOT))
    rust_binary = Path(metadata["target_directory"]) / "release/examples/coova_probe"
    manifest.update({
        "compiler": subprocess.check_output([*compiler, "--version"], text=True).splitlines()[0],
        "c_flags": flags, "sanitizer_flags": sanitizer_flags,
        "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
        "rust_profile": "workspace release (thin LTO, codegen-units=1)",
        "profile": {"enabled": ["ENABLE_IEEE8021Q"],
                    "disabled": ["DHCPRADIUS", "DHCPOPT", "WPAD", "LOCATION", "MODULES", "IPV6", "OTHER_SENDING"],
                    "captiveportalapi_uri": None, "debug": False, "domain": "", "dns2": "0.0.0.0",
                    "mtu": 1500, "lease_seconds": 3600},
    })
    return c_binary, sanitizer, rust_binary, manifest
