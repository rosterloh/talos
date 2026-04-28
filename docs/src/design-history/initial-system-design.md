# Initial System Design

Status: Accepted

Date: 2026-04-02

## Context

Talos needed to let a developer inspect and interact with a ROS 2 system from a
terminal without installing ROS 2 on every client machine.

The core constraint was keeping ROS 2 isolated to the robot-side process while
still giving clients enough typed structure to inspect messages and send basic
commands.

## Decision

The system was split into four crates:

- `talos-common` for protocol, config, transport, and shared data types.
- `talos-agent` for ROS 2 integration.
- `talos-cli` for scriptable command-line workflows.
- `talos-tui` for interactive terminal observation and control.

Messages are converted in the agent into `DynValue`, a generic tree that
preserves enough structure for clients to render and inspect data without ROS 2
message definitions.

The initial transport was UDS with length-prefixed bincode frames. Configuration
uses TOML.

## Consequences

Only the agent depends on ROS 2. The CLI, TUI, and common library can be built
and tested separately from the ROS 2 runtime.

The agent must contain explicit conversion support for each ROS 2 message type
Talos wants to bridge. That keeps clients simple but limits dynamic coverage
until generic message support is added.

The terminal UI can focus on rendering current protocol data instead of talking
directly to ROS 2.
