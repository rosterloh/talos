#!/usr/bin/env python3
"""Promote changelog entries into a release section and date releases."""

from __future__ import annotations

import argparse
import re
from pathlib import Path


HEADING_RE = re.compile(r"(?m)^## .*$")
UNRELEASED_RE = re.compile(r"^## \[?Unreleased\]?\s*$", re.IGNORECASE)


def find_heading(text: str, pattern: re.Pattern[str]) -> re.Match[str]:
    for match in HEADING_RE.finditer(text):
        if pattern.match(match.group(0)):
            return match
    raise RuntimeError(f"could not find heading matching {pattern.pattern}")


def next_heading_start(text: str, start: int) -> int:
    match = HEADING_RE.search(text, start)
    return match.start() if match else len(text)


def normalize_notes(notes: str) -> str:
    notes = notes.strip()
    return notes if notes else "- No notable changes recorded."


def prepare_release(changelog: Path, version: str, notes_file: Path | None) -> None:
    text = changelog.read_text()
    unreleased = find_heading(text, UNRELEASED_RE)
    body_start = unreleased.end()
    body_end = next_heading_start(text, body_start)
    notes = normalize_notes(text[body_start:body_end])

    release_heading = f"## [{version}] - TBD"
    if re.search(rf"(?m)^## \[{re.escape(version)}\] - ", text):
        raise RuntimeError(f"CHANGELOG.md already contains a {version} release section")

    replacement = f"\n\n{release_heading}\n\n{notes}\n\n"
    text = text[:body_start] + replacement + text[body_end:]
    changelog.write_text(text)

    if notes_file:
        notes_file.write_text(notes + "\n")


def finalize_release(changelog: Path, version: str, date: str) -> None:
    text = changelog.read_text()
    old = f"## [{version}] - TBD"
    new = f"## [{version}] - {date}"
    if old not in text:
        raise RuntimeError(f"could not find {old!r} in CHANGELOG.md")
    changelog.write_text(text.replace(old, new, 1))


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])

    subparsers = parser.add_subparsers(dest="command", required=True)

    prepare = subparsers.add_parser("prepare")
    prepare.add_argument("version")
    prepare.add_argument("--notes-file", type=Path)

    finalize = subparsers.add_parser("finalize")
    finalize.add_argument("version")
    finalize.add_argument("--date", required=True)

    args = parser.parse_args()
    changelog = args.root / "CHANGELOG.md"

    if args.command == "prepare":
        prepare_release(changelog, args.version, args.notes_file)
    elif args.command == "finalize":
        finalize_release(changelog, args.version, args.date)
    else:
        raise RuntimeError(f"unsupported command: {args.command}")


if __name__ == "__main__":
    main()
