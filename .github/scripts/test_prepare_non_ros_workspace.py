#!/usr/bin/env python3
"""Tests for prepare_non_ros_workspace.py."""

from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


sys.dont_write_bytecode = True
SCRIPT = Path(__file__).with_name("prepare_non_ros_workspace.py")
SPEC = importlib.util.spec_from_file_location("prepare_non_ros_workspace", SCRIPT)
assert SPEC is not None
prepare_non_ros_workspace = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
sys.modules[SPEC.name] = prepare_non_ros_workspace
SPEC.loader.exec_module(prepare_non_ros_workspace)


class PrepareNonRosWorkspaceTest(unittest.TestCase):
    def test_prepare_removes_agent_member_and_moves_cargo_config(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            cargo_dir = root / ".cargo"
            cargo_dir.mkdir()
            (cargo_dir / "config.toml").write_text("[patch.crates-io]\n")
            cargo_toml = root / "Cargo.toml"
            cargo_toml.write_text(
                "\n".join(
                    [
                        "[workspace]",
                        'members = [',
                        '    "talos-cli",',
                        '    "talos-common",',
                        '    "talos-agent",',
                        '    "talos-tui",',
                        "]",
                        "",
                    ]
                )
            )

            prepare_non_ros_workspace.prepare(root)

            self.assertNotIn('"talos-agent"', cargo_toml.read_text())
            self.assertFalse((cargo_dir / "config.toml").exists())
            self.assertEqual(
                (cargo_dir / "config.toml.ros").read_text(),
                "[patch.crates-io]\n",
            )


if __name__ == "__main__":
    unittest.main()
