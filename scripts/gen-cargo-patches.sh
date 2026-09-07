#!/usr/bin/env bash
# Regenerates .cargo/config.toml's [patch.crates-io] block from the
# ROS 2 message crates talos-agent actually depends on (transitively),
# using the Rust bindings shipped in the Pixi environment.
#
# Run after `pixi install`, and whenever talos-agent/Cargo.toml gains or
# loses a ROS message crate dependency.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
exec python3 scripts/gen_cargo_patches.py
