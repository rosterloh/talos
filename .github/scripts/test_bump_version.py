#!/usr/bin/env python3
"""Tests for bump_version.py."""

from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


sys.dont_write_bytecode = True
SCRIPT = Path(__file__).with_name("bump_version.py")


class BumpVersionTest(unittest.TestCase):
    def test_bump_moves_cargo_and_pixi_versions_together(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            (root / "Cargo.toml").write_text(
                '[workspace]\nmembers = ["app"]\n\n[workspace.package]\nversion = "1.0.0"\n'
            )
            (root / "app").mkdir()
            (root / "app" / "Cargo.toml").write_text('[package]\nname = "app"\n')
            (root / "Cargo.lock").write_text(
                '[[package]]\nname = "app"\nversion = "1.0.0"\n\n'
                '[[package]]\nname = "dep"\nversion = "1.0.0"\n'
            )
            (root / "pixi.toml").write_text(
                '[workspace]\nname = "app"\nversion = "1.0.0"\n\n'
                '[dependencies]\nfoo = { version = "1.0.0", build = "x*" }\n'
            )

            subprocess.run(
                [sys.executable, str(SCRIPT), "minor", "--root", str(root)],
                check=True,
                capture_output=True,
            )

            self.assertIn('version = "1.1.0"', (root / "Cargo.toml").read_text())
            lock = (root / "Cargo.lock").read_text()
            self.assertIn('name = "app"\nversion = "1.1.0"', lock)
            self.assertIn('name = "dep"\nversion = "1.0.0"', lock)
            pixi = (root / "pixi.toml").read_text()
            self.assertIn('[workspace]\nname = "app"\nversion = "1.1.0"', pixi)
            self.assertIn('foo = { version = "1.0.0", build = "x*" }', pixi)


if __name__ == "__main__":
    unittest.main()
