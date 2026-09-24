#!/usr/bin/env python3
"""Tests for coverage_report.py."""

from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


sys.dont_write_bytecode = True
SCRIPT = Path(__file__).with_name("coverage_report.py")
SPEC = importlib.util.spec_from_file_location("coverage_report", SCRIPT)
assert SPEC is not None
coverage_report = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = coverage_report
SPEC.loader.exec_module(coverage_report)


class CoverageReportTest(unittest.TestCase):
    def test_read_lcov_sums_line_counts(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            lcov = Path(temp_dir) / "coverage.lcov"
            lcov.write_text(
                "\n".join(
                    [
                        "TN:",
                        "SF:src/lib.rs",
                        "LF:3",
                        "LH:2",
                        "end_of_record",
                        "TN:",
                        "SF:src/main.rs",
                        "LF:2",
                        "LH:1",
                        "end_of_record",
                        "",
                    ]
                )
            )

            coverage = coverage_report.read_lcov(lcov)

        self.assertEqual(coverage.covered_lines, 3)
        self.assertEqual(coverage.total_lines, 5)
        self.assertEqual(coverage.percent_label, "60.00%")

    def test_compare_allows_equal_or_higher_coverage(self) -> None:
        comparison = coverage_report.compare_coverage(
            base=coverage_report.Coverage(covered_lines=80, total_lines=100),
            current=coverage_report.Coverage(covered_lines=81, total_lines=100),
        )

        self.assertFalse(comparison.regressed)
        self.assertEqual(comparison.delta_label, "+1.00 pp")

    def test_compare_flags_lower_coverage(self) -> None:
        comparison = coverage_report.compare_coverage(
            base=coverage_report.Coverage(covered_lines=80, total_lines=100),
            current=coverage_report.Coverage(covered_lines=79, total_lines=100),
        )

        self.assertTrue(comparison.regressed)
        self.assertEqual(comparison.delta_label, "-1.00 pp")


if __name__ == "__main__":
    unittest.main()
