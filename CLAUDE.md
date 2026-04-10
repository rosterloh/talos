# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Talos is a terminal-native tool for observing and interacting with ROS 2 systems. It decouples the developer workstation from the ROS 2 runtime by bridging ROS 2 topics through a custom IPC protocol over Unix domain sockets or QUIC.

## Build Commands

```bash
# ROS 2 environment is required for talos-agent (rclrs bindings)
source rclrs_ws/install/setup.bash

# Build / check everything
cargo build
cargo check --workspace

# Build without ROS 2 (cli, tui, common only)
cargo check -p talos-common -p talos-cli -p talos-tui

# Enable QUIC transport (feature-gated across all crates)
cargo build --features quic

# Tests
cargo test --workspace                        # all tests
cargo test -p talos-common                    # protocol, config, URDF tests
cargo test -p talos-agent --test integration  # UDS integration tests
cargo test -p talos-agent --test integration --features quic  # + QUIC tests

# ROS 2 workspace (Pixi-managed, only needed once or after rclrs changes)
cd rclrs_ws && pixi run build
```

## Architecture

```
talos-cli ──┐
            ▼
talos-tui ──► talos-common ◄── talos-agent ──► rclrs / ROS 2
```

**talos-common** — Shared library with no ROS 2 dependency. Contains:
- `protocol/` — Request/Response enums, `DynValue` tree (transport-agnostic ROS 2 message representation), length-prefixed bincode codec (4-byte BE + bincode)
- `session/` — `ProtocolClient` trait with `UdsProtocolClient` and `QuicProtocolClient` (feature `quic`) implementations
- `transport/` — `TransportServer` trait, UDS and QUIC endpoint setup, self-signed cert generation
- `config.rs` — TOML config: `[transport.uds]`, `[transport.quic]`, `[[subscriptions]]`, `[control]`, `[poses.*]`
- `urdf.rs` — URDF XML parsing for joint extraction

**talos-agent** — Runs on the robot. ROS 2 bridge + IPC server.
- `bridge.rs` — rclrs node that subscribes to ROS 2 topics, converts messages to `DynValue`, sends through an mpsc channel to the router (lock-free callback path)
- `router.rs` — `TopicRouter` manages per-client subscription sets and routes `TopicData` to subscribed clients only
- `server.rs` — UDS and QUIC accept loops, per-client handler tasks, request dispatch

**talos-tui** — Developer workstation terminal UI (ratatui). Auto-reconnects, subscribes to all topics, tabs for Topics/Nodes/Log/Joints.

**talos-cli** — Developer workstation CLI. Subcommands: `list-topics`, `list-nodes`, `echo <topic>`.

## Key Design Patterns

- **ROS 2 isolation**: Only `talos-agent` depends on rclrs. Client tools work without any ROS 2 installation.
- **`DynValue` tree**: ROS 2 messages are converted to a generic `DynValue` enum in the agent, so clients need no message type definitions.
- **Explicit subscriptions**: Clients send `Subscribe { topics }` after connecting; the `TopicRouter` only delivers data for subscribed topics.
- **Feature-gated QUIC**: All crates accept a `quic` Cargo feature. Without it, only UDS transport is compiled.

## Config Format (TOML)

The agent reads `talos-agent.toml` (or path via `--config`). Key sections:
- `[transport.uds]` / `[transport.quic]` — at least one required
- `[[subscriptions]]` — topic name + ROS 2 message type
- `[control]` — optional joint command publishing
- `[poses.<name>]` — named joint position presets

## OpenSpec Workflow

The `openspec/` directory tracks architectural specs and change proposals. Use the `/opsx:propose`, `/opsx:apply`, `/opsx:archive`, and `/opsx:explore` skills to manage the spec-driven workflow.
