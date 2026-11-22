"""Run comparison suites with separate correctness, timing and heap evidence."""

import argparse
import json
import math
from pathlib import Path

from baseline import compare
from benchmarks import measure
from build import build
from evidence import fresh_report, provenance, write_report
from framework import execute
from source import ROOT
from suites import SUITES, select


def arguments():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--coova", type=Path, default=ROOT.parent / "coova-chilli")
    parser.add_argument("--output", type=Path, default=ROOT / "target/coova-compare")
    parser.add_argument("--iterations", type=int, default=200000)
    parser.add_argument("--samples", type=int, default=5)
    parser.add_argument("--suite", action="append", choices=[suite.name for suite in SUITES])
    parser.add_argument("--list-suites", action="store_true")
    parser.add_argument("--checks-only", action="store_true", help="Correctness and sanitizers; skip timing and heap runs")
    parser.add_argument("--skip-memory", action="store_true", help="Explicitly omit Valgrind")
    parser.add_argument("--baseline", type=Path, help="Saved schema-2 successful report for same-implementation timings")
    parser.add_argument("--max-regression-percent", type=float, help="Optional baseline gate; no threshold is assumed")
    args = parser.parse_args()
    if not 1 <= args.iterations <= 100000000 or not 1 <= args.samples <= 25:
        parser.error("iterations must be 1..100000000 and samples 1..25")
    if args.baseline and (args.checks_only or args.baseline.resolve() == args.output.resolve() / "report.json"):
        parser.error("baseline needs timing and a saved file separate from output/report.json")
    if args.max_regression_percent is not None:
        if not args.baseline or not math.isfinite(args.max_regression_percent) or args.max_regression_percent < 0 or args.samples < 3:
            parser.error("a regression gate needs --baseline, a finite nonnegative percent, and at least 3 samples")
    return args


def run(args):
    selected = select(args.suite)
    output = args.output.resolve()
    previous = json.loads(args.baseline.read_text()) if args.baseline else None
    report = fresh_report(selected, args)
    write_report(output, report)
    try:
        c, sanitized, rust, manifest = build(args.coova.resolve(), output)
        binaries = {"c": c, "sanitized": sanitized, "rust": rust}
        provenance(report, output, manifest, binaries)
        for suite in selected:
            report["active_suite"] = suite.name
            result = execute(suite, binaries)
            report["suites"].append(result)
            measure(suite, result, binaries, args.iterations, args.samples, args.checks_only, args.skip_memory)
        report.pop("active_suite", None)
        report.update(result="passed_with_documented_differences", complete=True)
        if previous is not None:
            report["baseline"] = compare(report, previous, args.max_regression_percent)
            if report["baseline"]["status"] == "failed":
                report["result"] = "regression_gate_failed"
        write_report(output, report)
    except Exception as error:
        report.update(result="failed", complete=False, error={"type": type(error).__name__, "message": str(error)})
        write_report(output, report)
        raise
    return report


def main():
    args = arguments()
    if args.list_suites:
        for suite in SUITES:
            print(f"{suite.name} (v{suite.version}): {suite.description}")
        return
    report = run(args)
    count = sum(len(suite["cases"]) for suite in report["suites"])
    print(f"{report['result']}: {len(report['suites'])} suites, {count} cases; ASan/UBSan clean.")
    print(f"Reports: {args.output.resolve() / 'report.json'} and report.md")
    if report["result"] != "passed_with_documented_differences":
        raise SystemExit(1)


if __name__ == "__main__":
    main()
