#!/usr/bin/env bash
# Regenerates .cargo/config.toml's [patch.crates-io] block from the
# ROS 2 message crates talos-agent actually depends on (transitively),
# using the packages colcon built into rclrs_ws/install/.
#
# Run after `pixi run build` in rclrs_ws, and whenever talos-agent/Cargo.toml
# gains or loses a ROS message crate dependency.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."
exec python3 scripts/gen_cargo_patches.py
