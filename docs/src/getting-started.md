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

The agent depends on `rclrs`. Install the Pixi environment — it provides the ROS
2 Lyrical runtime and the pre-generated Rust message bindings (`rclrs` itself
comes from crates.io; `.cargo/config.toml` patches the message crates to the
environment). Then build inside it:

```bash
pixi install                 # one time
pixi run check               # or: pixi shell, then cargo check --workspace
```

> **Note:** Building the agent needs an `rclrs` release with ROS 2 Lyrical
> support, which is pending upstream PR
> [ros2-rust/ros2_rust#640](https://github.com/ros2-rust/ros2_rust/pull/640).
> Until it lands on crates.io, the agent build fails with
> `Unsupported ROS distribution`. The client crates build without ROS 2.

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
