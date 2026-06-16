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

ws="$(cd "$(dirname "$0")/.." && pwd)"

# Guard against overlay/underlay version skew: a source interface package built
# at a newer revision than the robostack conda underlay produces introspection
# libraries that reference symbols the underlay's generator libraries lack,
# which fails at dlopen (e.g. the DynamicMessage fallback). Fail loudly here
# instead. The .repos file pins each source package to the underlay's version;
# this catches drift if that pin is bypassed or the underlay is bumped.
if [ -n "${CONDA_PREFIX:-}" ]; then
  skew=0
  while IFS= read -r pxml; do
    name=$(grep -m1 '<name>' "$pxml" | sed -E 's/.*<name>(.*)<.*/\1/')
    sver=$(grep -m1 '<version>' "$pxml" | sed -E 's/.*<version>(.*)<.*/\1/')
    uxml="$CONDA_PREFIX/share/$name/package.xml"
    [ -f "$uxml" ] || continue  # not provided by the underlay; nothing to match
    uver=$(grep -m1 '<version>' "$uxml" | sed -E 's/.*<version>(.*)<.*/\1/')
    if [ "$sver" != "$uver" ]; then
      echo "VERSION SKEW: $name source=$sver underlay=$uver" >&2
      skew=1
    fi
  done < <(find "$ws/src" -name package.xml)
  if [ "$skew" -ne 0 ]; then
    echo "Source packages disagree with the conda underlay; pin them in the" >&2
    echo "matching *.repos file (version = underlay package.xml <version>)." >&2
    exit 1
  fi
else
  echo "warning: CONDA_PREFIX unset; skipping overlay/underlay version check" >&2
fi

cfg="$(cd "$(dirname "$0")/../.." && pwd)/.cargo/config.toml"
bak="$cfg.colcon-bak"

if [ -f "$cfg" ]; then
  mv "$cfg" "$bak"
  trap 'mv "$bak" "$cfg"' EXIT
fi

colcon build "$@"
