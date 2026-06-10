#!/usr/bin/env bash
# Build the ROS 2 Rust workspace with colcon.
#
# The parent talos cargo workspace (../.cargo/config.toml) patches crates-io
# with paths into this workspace's install/ tree. Cargo merges config from all
# ancestor directories into every package it builds here, and validates every
# [patch] entry on load. On a clean build those install paths don't exist yet,
# so cargo aborts on the first Rust package. Move the ancestor config aside for
# the duration of the build and always restore it.
set -euo pipefail

cfg="$(cd "$(dirname "$0")/../.." && pwd)/.cargo/config.toml"
bak="$cfg.colcon-bak"

if [ -f "$cfg" ]; then
  mv "$cfg" "$bak"
  trap 'mv "$bak" "$cfg"' EXIT
fi

colcon build "$@"
