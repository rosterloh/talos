# Talos

Talos is a terminal-native tool for observing and interacting with ROS 2
systems. The agent runs beside ROS 2 on the robot; the CLI and TUI can run on a
developer machine without a ROS 2 installation.

![Main screen](assets/banner.png)

## What It Does

- Observes ROS 2 topics and renders live message data as generic trees.
- Lists ROS 2 nodes with graph details.
- Displays `/rosout` logs in the terminal UI.
- Shows URDF-aware joint state and can send configured joint commands.
- Connects locally over Unix domain sockets or remotely over feature-gated QUIC.

## Documentation

The canonical documentation lives in the mdBook under [`docs/`](docs/src/introduction.md).

```bash
mdbook serve docs
```

## Quick Checks

Build the shared library and workstation clients without ROS 2:

```bash
cargo check -p talos-common -p talos-cli -p talos-tui
```

Build the full workspace after sourcing the ROS 2/rclrs environment:

```bash
source rclrs_ws/install/setup.bash
cargo check --workspace
```

Enable QUIC support with:

```bash
cargo build --features quic
```

## Crates

- `talos-common`: protocol, config, session, transport, and URDF support.
- `talos-agent`: ROS 2 bridge and client server.
- `talos-cli`: scriptable command-line client.
- `talos-tui`: interactive terminal UI.

## Status

Talos is early-stage software. See the docs for current behavior, design
history, and future plans.
