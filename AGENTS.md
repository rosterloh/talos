# AGENTS.md

This file provides guidance to coding agents when working with code in this repository.

## Project Overview

Talos is a terminal-native tool for observing and interacting with ROS 2 systems. It decouples the developer workstation from the ROS 2 runtime by bridging ROS 2 topics through a custom IPC protocol over Unix domain sockets or QUIC.

## Build Commands

```bash
# ROS 2 Lyrical environment (Pixi) is required for talos-agent. It provides the
# ROS 2 runtime + pre-generated Rust message bindings, which .cargo/config.toml
# patches in; rclrs is pinned to an upstream git rev in Cargo.toml.
pixi install                 # one time: resolve the robostack-lyrical env
pixi shell                   # enter the env (or prefix commands with `pixi run`)

# Build / check everything (inside the pixi shell, or via `pixi run <task>`)
cargo build                  # or: pixi run build
cargo check --workspace

# Build without ROS 2 (cli, tui, common only) — no pixi env needed
cargo check -p talos-common -p talos-cli -p talos-tui

# Enable QUIC transport (feature-gated across all crates)
cargo build --features quic  # or: pixi run build-quic

# Tests
cargo test --workspace                        # all tests (or: pixi run test)
cargo test -p talos-common                    # protocol, config, URDF tests
cargo test -p talos-agent --test integration  # UDS integration tests
cargo test -p talos-agent --test integration --features quic  # + QUIC tests
```

> **Note:** `rclrs` is pinned to a git revision because ROS 2 Lyrical support
> ([ros2-rust/ros2_rust#658](https://github.com/ros2-rust/ros2_rust/pull/658))
> is merged upstream but unreleased. Drop the `[patch.crates-io.rclrs]` block in
> `Cargo.toml` once a release with the Lyrical bindings ships.
>
> **Not yet runtime-ready:** the agent compiles and its unit tests pass, but any
> message with a primitive sequence (`sensor_msgs/JointState`, the `rcl_interfaces`
> parameter services) is read at the wrong offset and segfaults — the Lyrical
> `Sequence<T>` ABI mismatch in
> [ros2-rust/ros2_rust#659](https://github.com/ros2-rust/ros2_rust/issues/659).
> `rosidl_runtime_rs` 0.7.0 carries the fix, but nothing consumes it yet: `rclrs`
> and the robostack-generated message crates both require `^0.6`, and the
> generator-side change (`ros2-rust/rosidl_rust#38`) is still open. Reproduced on
> both `osx-arm64` and `linux-aarch64` via
> `cargo test -p talos-agent --test integration`.

## Default Change Workflow

- Keep `main` stable and release-oriented. Do not land day-to-day development directly on `main` unless the user explicitly asks for release maintenance or a hotfix.
- Make repository changes on feature branches from `dev`, then open pull requests back into `dev`.
- Promote `dev` to `main` with a pull request when preparing a release; merging `dev` into `main` runs the automated version bump and release workflow.
- After implementing and validating changes, commit them, push the branch, and open a pull request to the appropriate base branch.
- Keep unrelated local files out of the commit; do not stage generated output such as `docs/book/`.

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

## Documentation

The `docs/` mdBook is the canonical project documentation. Keep current behavior, architecture, usage, configuration, design history, and future plans there. `README.md` should stay short and point readers to the book.

Build and preview docs with:

```bash
mdbook build docs
mdbook serve docs
```

## Changelog Discipline

- Always describe code, behavior, documentation, CI, or configuration changes in `CHANGELOG.md` under `[Unreleased]`.
- Keep changelog entries concise and user-facing. Use sections such as `Added`, `Changed`, `Fixed`, `Removed`, or `Security`.
- Do not edit dated release sections manually unless you are doing release maintenance.
