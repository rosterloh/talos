#!/usr/bin/env python3
"""Prepare a checkout for Cargo commands that do not need ROS crates."""

from __future__ import annotations

import argparse
from pathlib import Path


AGENT_MEMBER = '    "talos-agent",\n'


def prepare(root: Path) -> None:
    cargo_toml = root / "Cargo.toml"
    text = cargo_toml.read_text()
    if AGENT_MEMBER not in text:
        raise RuntimeError("talos-agent workspace member not found")
    cargo_toml.write_text(text.replace(AGENT_MEMBER, "", 1))

    cargo_config = root / ".cargo" / "config.toml"
    if cargo_config.exists():
        cargo_config.rename(cargo_config.with_name("config.toml.ros"))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=Path, default=Path.cwd())
    args = parser.parse_args()

    prepare(args.root)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
