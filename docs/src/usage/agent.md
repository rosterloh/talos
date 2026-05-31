# Agent

`talos-agent` runs on the machine that has ROS 2 access.

```bash
talos-agent --config talos-agent.toml
```

The `--config` argument is optional. Without it, the agent checks for
`talos-agent.toml` in the current directory. If the file is not present, the
agent uses defaults: UDS enabled at `/tmp/talos.sock`, QUIC disabled, no topic
subscriptions, and no joint control.

## Startup Behavior

On startup, the agent:

1. Loads configuration.
2. Starts configured transport listeners.
3. Creates the ROS 2 bridge node.
4. Subscribes to configured topics with supported message types.
5. Waits for clients to connect and subscribe.

If neither UDS nor QUIC is configured, the agent logs an error and exits without
serving clients.

## ROS 2 Environment

The agent requires the ROS 2 Lyrical environment, managed by Pixi. It provides
the ROS 2 runtime and the pre-generated Rust message bindings; `rclrs` itself is
pulled from crates.io. Enter the environment before building or running:

```bash
pixi install
pixi shell        # or prefix commands with `pixi run`
```

## Logging

The agent uses `tracing`. Set `RUST_LOG` to adjust log verbosity:

```bash
RUST_LOG=talos_agent=debug talos-agent --config talos-agent.toml
```
