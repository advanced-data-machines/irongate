"""Reusable execution and failure-artifact contracts, without C tools or host reads."""

import json
from pathlib import Path
from tempfile import TemporaryDirectory
from types import SimpleNamespace
import unittest
from unittest.mock import patch

from framework import Case, Suite, Workload, execute
from measure import paired_timing
from run import run
from suites import compare_option, select


class FrameworkTests(unittest.TestCase):
    def test_selection_is_explicit_and_unknown_suite_fails(self):
        self.assertEqual([suite.name for suite in select(["dhcp-offers"])], ["dhcp-offers"])
        with self.assertRaises(ValueError):
            select(["not-a-suite"])

    def test_unexpected_value_fails_even_when_both_statuses_match(self):
        case = Case("value", "option", b"fixture", expected={"c_status": "found", "rust_status": "found", "classification": "equivalent"})
        with self.assertRaises(AssertionError):
            compare_option(case, {"status": "found", "value": "01"}, {"status": "found", "value": "01"},
                           {"status": "found", "value": "02"})

    def test_duplicate_or_empty_cases_do_not_claim_coverage(self):
        case = Case("same", "option", b"fixture")
        binaries = dict(c="c", sanitized="sanitized", rust="rust")
        for cases in ([], [case, case]):
            suite = Suite("sample", 1, "fixture", lambda: cases, lambda *args: {}, tuple)
            with self.assertRaises(AssertionError):
                execute(suite, binaries, lambda *args: ({}, ""))

    def test_timing_alternates_implementations_and_retains_raw_samples(self):
        calls = []
        def invoke(binary, *args):
            calls.append(binary)
            return {"status": "found", "elapsed_ns": 100 if binary == "c" else 200}, ""
        values = paired_timing({"c": "c", "rust": "rust"}, Workload("lookup", "option", b"x"), 10, 3, invoke)
        self.assertEqual(calls, ["c", "rust", "rust", "c", "c", "rust"])
        self.assertEqual(values["rust"]["samples_ns_per_operation"], [20, 20, 20])

    def test_failed_build_replaces_old_success_with_failed_report(self):
        with TemporaryDirectory() as directory:
            output = Path(directory)
            (output / "report.json").write_text('{"result":"passed"}')
            args = SimpleNamespace(suite=["dhcp-options"], output=output, baseline=None,
                                   coova=output, iterations=10, samples=3, checks_only=True,
                                   skip_memory=False)
            with patch("run.build", side_effect=RuntimeError("fixture build failed")):
                with self.assertRaises(RuntimeError):
                    run(args)
            report = json.loads((output / "report.json").read_text())
            self.assertEqual(report["result"], "failed")
            self.assertFalse(report["complete"])
            self.assertIn("fixture build failed", (output / "report.md").read_text())
