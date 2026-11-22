"""Suite-independent readable evidence; JSON retains complete observations."""


def markdown(report):
    lines = ["# Offline Coova/Rust comparison", "",
             f"Result: `{report['result']}`. Complete: `{report['complete']}`. Schema: {report['schema_version']}.", "",
             f"Source: `{report['source']['revision']}`. Started: {report['started_at_utc']}.", "",
             report["scope"], ""]
    if "error" in report:
        lines.extend(["## Failure", "", "```text", report["error"]["message"], "```", ""])
    for suite in report["suites"]:
        lines.extend([f"## {suite['name']} (v{suite['version']})", "", suite["description"], "",
                      "| Case | Classification / observations |", "| --- | --- |"])
        for case in suite["cases"]:
            classification = case.get("classification", ", ".join(sorted({
                item["classification"] for item in case.get("differences", [])})) or "equivalent")
            lines.append(f"| {case['name']} | {classification} |")
        lines.extend(["", "### Batch timings", "",
                      "| Workload | C median ns/op | Rust median ns/op | Cross-implementation matched? |",
                      "| --- | --- | --- | --- |"])
        if suite.get("measurement_status") == "skipped":
            lines.extend(["", "Timing and allocation measurements explicitly skipped."])
        for name, workload in suite["benchmarks"].items():
            values = workload["implementations"]
            lines.append(f"| {name} | {values['c']['median_ns_per_operation']:.1f} | "
                         f"{values['rust']['median_ns_per_operation']:.1f} | {workload['cross_implementation_comparable']} |")
            lines.extend(["", f"{name}: {workload['caveat']}"])
        lines.extend(["", "### Allocations", ""])
        for name, workload in suite["memory"].items():
            lines.append(f"{name}: {workload['status']}.")
            for implementation, values in workload.get("implementations", {}).items():
                lines.append(f"- {implementation}: baseline {values['baseline_allocs_frees_bytes']}; replay "
                             f"{values['replay_allocs_frees_bytes']}; zero additional allocations.")
        lines.append("")
    if "baseline" in report:
        baseline = report["baseline"]
        lines.extend(["## Saved baseline", "", f"Status: {baseline['status']}.", "",
                      "| Suite/workload | Implementation | Status | Change |", "| --- | --- | --- | --- |"])
        for row in baseline["rows"]:
            change = f"{row['change_percent']:+.1f}%" if "change_percent" in row else "unverified"
            lines.append(f"| {row['suite']}/{row['workload']} | {row['implementation']} | {row['status']} | {change} |")
        lines.extend(["", baseline["caveat"], ""])
    lines.extend(["Batch times are averages, not per-packet latency percentiles or wire throughput.", "",
                  "Sanitizer and Valgrind runs are separate from timing. See JSON for raw observations, "
                  "per-sample timings, source/binary hashes, environment, and known differences.", ""])
    return "\n".join(lines)
