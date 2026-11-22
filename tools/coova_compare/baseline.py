"""Compare each implementation with its own saved run; no language speed ranking."""

SCHEMA_VERSION = 2


def compare(current, previous, max_regression=None):
    if previous.get("schema_version") != SCHEMA_VERSION:
        raise ValueError("Baseline must use comparison report schema 2.")
    if previous.get("result") != "passed_with_documented_differences":
        raise ValueError("Baseline must be a completed successful report.")
    compatible = current["measurement_context"] == previous["measurement_context"]
    rows = []
    previous_suites = {suite["name"]: suite for suite in previous["suites"]}
    for suite in current["suites"]:
        old_suite = previous_suites.get(suite["name"], {})
        for name, workload in suite["benchmarks"].items():
            old = old_suite.get("benchmarks", {}).get(name, {})
            same_work = compatible and old_suite.get("version") == suite["version"] and all(
                old.get(key) == workload[key] for key in ("mode", "code", "input_sha256"))
            for implementation, metrics in workload["implementations"].items():
                reference = old.get("implementations", {}).get(implementation, {})
                enough = len(metrics["samples_ns_per_operation"]) >= 3 and len(reference.get("samples_ns_per_operation", [])) >= 3
                same_count = reference.get("iterations_per_sample") == metrics["iterations_per_sample"]
                row = {"suite": suite["name"], "workload": name, "implementation": implementation,
                       "status": "incomparable", "reason": "Environment, suite version, workload or sample controls differ."}
                old_ns = reference.get("median_ns_per_operation", 0)
                if same_work and enough and same_count and old_ns > 0:
                    change = 100 * (metrics["median_ns_per_operation"] / old_ns - 1)
                    row.update(status="observed", reason="", change_percent=change,
                               baseline_ns=old_ns, current_ns=metrics["median_ns_per_operation"])
                    if max_regression is not None:
                        row["status"] = "regressed" if change > max_regression else "within_threshold"
                rows.append(row)
    failed = max_regression is not None and (not rows or any(
        row["status"] != "within_threshold" for row in rows))
    return {"status": "failed" if failed else "passed" if max_regression is not None else "observed", "max_regression_percent": max_regression,
            "context_compatible": compatible, "rows": rows,
            "caveat": "Same-implementation batch medians; machine load/frequency noise remains. No statistical significance claim."}
