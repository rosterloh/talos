# Getting Started

Talos is a Cargo workspace with four crates:

- `talos-common`: protocol, config, transport, session, and URDF support.
- `talos-agent`: ROS 2 bridge process that runs on the robot.
- `talos-cli`: command-line client.
- `talos-tui`: terminal UI client.

## Build Without ROS 2

The client crates and shared library do not require a ROS 2 environment:

```bash
cargo check -p talos-common -p talos-cli -p talos-tui
```

## Build Everything

The agent depends on `rclrs`, so source the ROS 2/rclrs workspace first:

```bash
source rclrs_ws/install/setup.bash
cargo check --workspace
```

## Enable QUIC

QUIC support is feature-gated:

```bash
cargo build --features quic
```

Without the `quic` feature, Talos builds UDS support only.

## Run The Agent

With no `--config` argument, the agent looks for `talos-agent.toml` in the
current directory. If the file is absent, it starts with default UDS transport
and no configured topic subscriptions.

```bash
talos-agent --config talos-agent.toml
```

## Run A Client

For local UDS access:

```bash
talos-cli list-topics
talos-cli echo /joint_states --count 5
talos-tui
```

For remote QUIC access, build with `--features quic` and pass `--remote`:

```bash
talos-cli --remote 192.168.1.50:4433 list-topics
talos-tui --remote 192.168.1.50:4433
```
