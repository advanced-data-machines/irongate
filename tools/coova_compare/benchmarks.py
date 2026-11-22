"""Workload metadata makes unmatched cross-language results explicit."""

import hashlib

from framework import require
from measure import heap, paired_timing


def measure(suite, result, binaries, iterations, samples, checks_only=False, skip_memory=False):
    result["measurement_status"] = "skipped" if checks_only else "measured"
    if checks_only:
        return
    for workload in suite.workloads():
        require(workload.name not in result["benchmarks"], "Duplicate benchmark workload")
        result["benchmarks"][workload.name] = {
            "mode": workload.mode, "code": workload.code,
            "input_sha256": hashlib.sha256(workload.data).hexdigest(),
            "cross_implementation_comparable": workload.cross_implementation_comparable,
            "caveat": workload.caveat,
            "implementations": paired_timing(binaries, workload, iterations, samples),
        }
    for workload in suite.memory_workloads():
        result["memory"][workload.name] = {"status": "skipped"}
        if not skip_memory:
            result["memory"][workload.name] = {"status": "measured", "implementations": {
                name: heap(binaries[name], workload.data, workload.mode, workload.code) for name in ("c", "rust")}}
