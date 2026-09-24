#!/usr/bin/env python3
"""Summarize LCOV reports and fail PR checks when line coverage regresses."""

from __future__ import annotations

import argparse
import os
import re
import sys
from dataclasses import dataclass
from decimal import Decimal, ROUND_HALF_UP
from fractions import Fraction
from pathlib import Path


LCOV_LINE_TOTAL_RE = re.compile(r"^LF:(\d+)$")
LCOV_LINE_HIT_RE = re.compile(r"^LH:(\d+)$")
PERCENT_PLACES = Decimal("0.01")


@dataclass(frozen=True)
class Coverage:
    covered_lines: int
    total_lines: int

    def __post_init__(self) -> None:
        if self.covered_lines < 0:
            raise ValueError("covered line count cannot be negative")
        if self.total_lines <= 0:
            raise ValueError("LCOV report did not contain coverable lines")
        if self.covered_lines > self.total_lines:
            raise ValueError("covered line count cannot exceed total line count")

    @property
    def fraction(self) -> Fraction:
        return Fraction(self.covered_lines, self.total_lines)

    @property
    def percent_label(self) -> str:
        return f"{format_percent(self.fraction)}%"


@dataclass(frozen=True)
class CoverageComparison:
    base: Coverage
    current: Coverage

    @property
    def regressed(self) -> bool:
        return self.current.fraction < self.base.fraction

    @property
    def delta(self) -> Fraction:
        return self.current.fraction - self.base.fraction

    @property
    def delta_label(self) -> str:
        return f"{format_signed_percent_points(self.delta)} pp"


def format_percent(value: Fraction) -> str:
    percent = (
        Decimal(value.numerator) * Decimal(100) / Decimal(value.denominator)
    ).quantize(PERCENT_PLACES, rounding=ROUND_HALF_UP)
    return f"{percent:.2f}"


def format_signed_percent_points(value: Fraction) -> str:
    raw_percent = Decimal(value.numerator) * Decimal(100) / Decimal(value.denominator)
    percent = raw_percent.quantize(PERCENT_PLACES, rounding=ROUND_HALF_UP)
    sign = "" if raw_percent < 0 else "+"
    return f"{sign}{percent:.2f}"


def read_lcov(path: Path) -> Coverage:
    total_lines = 0
    covered_lines = 0

    for line_number, line in enumerate(path.read_text().splitlines(), start=1):
        total_match = LCOV_LINE_TOTAL_RE.match(line)
        if total_match:
            total_lines += int(total_match.group(1))
            continue

        hit_match = LCOV_LINE_HIT_RE.match(line)
        if hit_match:
            covered_lines += int(hit_match.group(1))
            continue

        if line.startswith(("LF:", "LH:")):
            raise RuntimeError(f"invalid LCOV line count at {path}:{line_number}")

    return Coverage(covered_lines=covered_lines, total_lines=total_lines)


def compare_coverage(base: Coverage, current: Coverage) -> CoverageComparison:
    return CoverageComparison(base=base, current=current)


def coverage_markdown(title: str, coverage: Coverage) -> str:
    return "\n".join(
        [
            f"## {title}",
            "",
            "| Metric | Value |",
            "| --- | ---: |",
            f"| Line coverage | {coverage.percent_label} |",
            f"| Covered lines | {coverage.covered_lines} |",
            f"| Total lines | {coverage.total_lines} |",
            "",
        ]
    )


def comparison_markdown(comparison: CoverageComparison) -> str:
    status = "failed" if comparison.regressed else "passed"
    covered_lines_row = (
        f"| Covered lines | {comparison.base.covered_lines} | "
        f"{comparison.current.covered_lines} | |"
    )
    total_lines_row = (
        f"| Total lines | {comparison.base.total_lines} | "
        f"{comparison.current.total_lines} | |"
    )
    return "\n".join(
        [
            "## Coverage regression check",
            "",
            f"Coverage regression check {status}.",
            "",
            "| Metric | Base | Current | Delta |",
            "| --- | ---: | ---: | ---: |",
            (
                "| Line coverage | "
                f"{comparison.base.percent_label} | "
                f"{comparison.current.percent_label} | "
                f"{comparison.delta_label} |"
            ),
            covered_lines_row,
            total_lines_row,
            "",
        ]
    )


def emit_markdown(markdown: str, path: Path | None) -> None:
    if path:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(markdown)

    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with Path(summary).open("a") as handle:
            handle.write(markdown)
            handle.write("\n")


def summarize(args: argparse.Namespace) -> int:
    coverage = read_lcov(args.lcov)
    markdown = coverage_markdown("Rust coverage", coverage)
    emit_markdown(markdown, args.markdown)
    print(
        "line coverage: "
        f"{coverage.percent_label} ({coverage.covered_lines}/{coverage.total_lines})"
    )
    return 0


def compare(args: argparse.Namespace) -> int:
    comparison = compare_coverage(
        read_lcov(args.base_lcov),
        read_lcov(args.current_lcov),
    )
    markdown = comparison_markdown(comparison)
    emit_markdown(markdown, args.markdown)

    print(
        "coverage: "
        f"base {comparison.base.percent_label}, "
        f"current {comparison.current.percent_label}, "
        f"delta {comparison.delta_label}"
    )
    if comparison.regressed:
        print("coverage decreased from the base branch", file=sys.stderr)
        return 1
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    summarize_parser = subparsers.add_parser("summarize")
    summarize_parser.add_argument("lcov", type=Path)
    summarize_parser.add_argument("--markdown", type=Path)
    summarize_parser.set_defaults(func=summarize)

    compare_parser = subparsers.add_parser("compare")
    compare_parser.add_argument("base_lcov", type=Path)
    compare_parser.add_argument("current_lcov", type=Path)
    compare_parser.add_argument("--markdown", type=Path)
    compare_parser.set_defaults(func=compare)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    raise SystemExit(main())
