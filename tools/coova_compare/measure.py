"""Batch timing and separate heap instrumentation; never time Valgrind runs."""

import json
import os
import re
import statistics
import subprocess


def probe(binary, mode, data, code=53, iterations=1, prefix=()):
    command = [*prefix, str(binary), mode, str(code), f"{iterations:06}"]
    result = subprocess.run(command, input=data, capture_output=True, timeout=60,
                            env={**os.environ, "LC_ALL": "C"})
    if result.returncode:
        raise RuntimeError(f"Probe failed ({result.returncode}): {command}\n{result.stderr.decode(errors='replace')}")
    return json.loads(result.stdout), result.stderr.decode(errors="replace")


def summary(values, iterations):
    return {"iterations_per_sample": iterations, "samples_ns_per_operation": values,
            "median_ns_per_operation": statistics.median(values),
            "min_ns_per_operation": min(values), "max_ns_per_operation": max(values)}


def paired_timing(binaries, workload, iterations, samples, invoke=probe):
    values = {"c": [], "rust": []}
    for sample in range(samples):
        # Alternate order to reduce systematic warm CPU/frequency bias.
        order = ("c", "rust") if sample % 2 == 0 else ("rust", "c")
        for name in order:
            result, _ = invoke(binaries[name], workload.mode, workload.data, workload.code, iterations)
            if result["status"] not in ("encoded", "found", "missing"):
                raise AssertionError("Benchmark workload did not execute")
            values[name].append(result["elapsed_ns"] / iterations)
    return {name: summary(samples, iterations) for name, samples in values.items()}


def heap(binary, data, mode="offer", code=53):
    values = []
    for iterations in (1, 100000):
        _, stderr = probe(binary, mode, data, code, iterations=iterations,
                          prefix=("valgrind", "--tool=memcheck", "--error-exitcode=99", "--leak-check=full"))
        match = re.search(r"total heap usage: ([\d,]+) allocs, ([\d,]+) frees, ([\d,]+) bytes allocated", stderr)
        if not match:
            raise RuntimeError("Valgrind heap summary missing:\n" + stderr)
        values.append([int(value.replace(",", "")) for value in match.groups()])
    if values[0] != values[1]:
        raise AssertionError(f"Replay changed allocation/free/byte totals: {values}")
    return {"baseline_iterations": 1, "replay_iterations": 100000,
            "baseline_allocs_frees_bytes": values[0], "replay_allocs_frees_bytes": values[1],
            "additional_allocations": 0}
