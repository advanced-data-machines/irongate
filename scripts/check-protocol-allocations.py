"""Compare Valgrind heap totals with identical setup and zero/100000 replays.

Requires a release replay example and system Valgrind.
No allocator wrappers, unsafe Rust, networking or third-party Python packages.
"""

import os
from pathlib import Path
import re
import shutil
import subprocess
import sys


def totals(binary, iterations):
    result = subprocess.run(
        ["valgrind", "--tool=memcheck", "--error-exitcode=99", "--leak-check=full",
         str(binary), iterations],
        capture_output=True, text=True, env={**os.environ, "LC_ALL": "C"},
    )
    if result.returncode:
        sys.exit(result.stderr or f"Replay failed: {result.returncode}")
    match = re.search(
        r"total heap usage: ([\d,]+) allocs, ([\d,]+) frees, ([\d,]+) bytes allocated",
        result.stderr,
    )
    if not match:
        sys.exit("Could not read Valgrind heap summary:\n" + result.stderr)
    return tuple(int(value.replace(",", "")) for value in match.groups())


def main():
    if not shutil.which("valgrind"):
        sys.exit("Valgrind is required; this optional check was not run.")
    # just invokes this with Cargo's actual target directory, including overrides.
    if len(sys.argv) != 2:
        sys.exit("Usage: check-protocol-allocations.py PATH_TO_REPLAY_BINARY")
    binary = Path(sys.argv[1]).resolve()
    baseline = totals(binary, "000000")
    replay = totals(binary, "100000")
    print(f"Setup totals (allocs, frees, bytes): {baseline}")
    print(f"100000 replays (allocs, frees, bytes): {replay}")
    if baseline != replay:
        sys.exit("FAIL: replay changed heap allocation totals.")
    print(f"PASS: zero additional heap allocations across 100000 {binary.name} replays.")


if __name__ == "__main__":
    main()
