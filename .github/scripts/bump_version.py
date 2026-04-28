#!/usr/bin/env python3
"""Bump the Cargo workspace package version and matching lockfile entries."""

from __future__ import annotations

import argparse
import os
import re
from pathlib import Path


VERSION_RE = re.compile(
    r'(?ms)(^\[workspace\.package\]\n.*?^version\s*=\s*")'
    r"(\d+)\.(\d+)\.(\d+)"
    r'(")'
)
PACKAGE_BLOCK_RE = re.compile(r"(?ms)^\[\[package\]\]\n.*?(?=^\[\[package\]\]|\Z)")
PACKAGE_NAME_RE = re.compile(r'(?m)^name\s*=\s*"([^"]+)"')
PACKAGE_VERSION_RE = re.compile(r'(?m)^version\s*=\s*"\d+\.\d+\.\d+"')
WORKSPACE_MEMBERS_RE = re.compile(r"(?ms)^\[workspace\]\n.*?^members\s*=\s*\[(.*?)\]")
QUOTED_RE = re.compile(r'"([^"]+)"')


def bump_version(version: str, kind: str) -> str:
    major, minor, patch = (int(part) for part in version.split("."))

    if kind == "major":
        return f"{major + 1}.0.0"
    if kind == "minor":
        return f"{major}.{minor + 1}.0"
    if kind == "patch":
        return f"{major}.{minor}.{patch + 1}"

    raise ValueError(f"unsupported bump kind: {kind}")


def workspace_members(root: Path, cargo_toml: str) -> set[str]:
    match = WORKSPACE_MEMBERS_RE.search(cargo_toml)
    if not match:
        raise RuntimeError("could not find [workspace] members in Cargo.toml")

    names: set[str] = set()
    for member in QUOTED_RE.findall(match.group(1)):
        member_toml = root / member / "Cargo.toml"
        text = member_toml.read_text()
        name_match = re.search(r'(?ms)^\[package\]\n.*?^name\s*=\s*"([^"]+)"', text)
        if not name_match:
            raise RuntimeError(f"could not find package name in {member_toml}")
        names.add(name_match.group(1))

    return names


def update_lockfile(lockfile: Path, package_names: set[str], version: str) -> None:
    if not lockfile.exists():
        return

    lock_text = lockfile.read_text()

    def replace_block(match: re.Match[str]) -> str:
        block = match.group(0)
        name_match = PACKAGE_NAME_RE.search(block)
        if not name_match or name_match.group(1) not in package_names:
            return block
        return PACKAGE_VERSION_RE.sub(f'version = "{version}"', block, count=1)

    lockfile.write_text(PACKAGE_BLOCK_RE.sub(replace_block, lock_text))


def write_github_output(version: str) -> None:
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with Path(output).open("a") as handle:
            handle.write(f"version={version}\n")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("kind", choices=("patch", "minor", "major"))
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    root = args.root
    cargo_path = root / "Cargo.toml"
    cargo_text = cargo_path.read_text()
    match = VERSION_RE.search(cargo_text)
    if not match:
        raise RuntimeError("could not find [workspace.package] version in Cargo.toml")

    current_version = ".".join(match.group(i) for i in range(2, 5))
    new_version = bump_version(current_version, args.kind)

    print(f"{current_version} -> {new_version}")
    write_github_output(new_version)

    if args.dry_run:
        return

    package_names = workspace_members(root, cargo_text)
    new_cargo_text = VERSION_RE.sub(
        rf"\g<1>{new_version}\g<5>",
        cargo_text,
        count=1,
    )
    cargo_path.write_text(new_cargo_text)
    update_lockfile(root / "Cargo.lock", package_names, new_version)


if __name__ == "__main__":
    main()
