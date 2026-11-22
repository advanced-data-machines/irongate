"""A missing or incompatible performance baseline must never pass a requested gate."""

import copy
import unittest

from baseline import compare


def report(ns=100):
    metric = {"samples_ns_per_operation": [ns, ns, ns], "iterations_per_sample": 200000,
              "median_ns_per_operation": ns}
    return {"schema_version": 2, "result": "passed_with_documented_differences",
            "measurement_context": {"cpu": "fixture"}, "suites": [{"name": "sample", "version": 1,
            "benchmarks": {"work": {"mode": "option", "code": 53, "input_sha256": "abc",
            "implementations": {"c": metric, "rust": copy.deepcopy(metric)}}}}]}


class BaselineTests(unittest.TestCase):
    def test_regression_gate_compares_each_implementation_to_itself(self):
        result = compare(report(125), report(100), 10)
        self.assertEqual(result["status"], "failed")
        self.assertEqual([row["change_percent"] for row in result["rows"]], [25, 25])
        self.assertEqual(compare(report(105), report(100), 10)["status"], "passed")

    def test_gate_without_threshold_only_observes(self):
        result = compare(report(200), report(100))
        self.assertEqual(result["status"], "observed")
        self.assertTrue(all(row["status"] == "observed" for row in result["rows"]))

    def test_environment_or_input_drift_is_incomparable_and_fails_gate(self):
        for field in ("environment", "input", "version", "iterations", "samples"):
            current = report()
            if field == "environment": current["measurement_context"] = {"cpu": "other"}
            elif field == "version": current["suites"][0]["version"] = 2
            else:
                work = current["suites"][0]["benchmarks"]["work"]
                if field == "input": work["input_sha256"] = "changed"
                elif field == "iterations": work["implementations"]["c"]["iterations_per_sample"] = 10
                else: work["implementations"]["c"]["samples_ns_per_operation"] = [100]
            self.assertEqual(compare(current, report(), 10)["status"], "failed", field)

    def test_empty_work_or_old_failed_report_cannot_pass_gate(self):
        current = report()
        current["suites"] = []
        self.assertEqual(compare(current, report(), 10)["status"], "failed")
        for change in ({"schema_version": 1}, {"result": "failed"}):
            previous = report()
            previous.update(change)
            with self.assertRaises(ValueError):
                compare(report(), previous)
