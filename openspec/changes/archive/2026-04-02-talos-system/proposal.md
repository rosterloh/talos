## Why

There is no unified tool for observing and interacting with ROS 2 systems from a developer workstation. Existing tools (`ros2 topic echo`, `rqt`, `rviz`) require a full ROS 2 installation on the host and lack a cohesive terminal-native experience. Talos provides a lightweight Rust-based bridge that decouples the host tooling from the ROS 2 runtime, enabling topic observation, node introspection, and joint control through a TUI — without requiring ROS 2 on the developer machine.

## What Changes

- **New Rust workspace** with four crates: `talos-cli`, `talos-common`, `talos-agent`, `talos-tui`
- **IPC protocol** using length-prefixed bincode over Unix domain sockets, with a transport abstraction for future QUIC support
- **ROS 2 bridge agent** (`talos-agent`) using `rclrs` with typed subscriptions for known message types, converting ROS 2 messages to a generic `DynValue` representation
- **Terminal UI** (`talos-tui`) built with `ratatui` featuring four tabs: Topics, Nodes, Log, and Joints
- **Joint control** via URDF parsing merged with live `/joint_states`, with the ability to send position commands and execute predefined poses
- **CLI** (`talos-cli`) for non-interactive queries (list topics, echo a topic, etc.)

## Capabilities

### New Capabilities

- `ipc-protocol`: Bincode-based request/response and streaming protocol over UDS with length-prefixed framing, shared message types (`DynValue`, `TopicInfo`, `JointInfo`), and transport abstraction
- `ros2-bridge`: Agent that runs on the target device as an `rclrs` node, subscribes to configured topics, deserialises ROS 2 messages into `DynValue`, and serves them over IPC to connected clients
- `terminal-ui`: Ratatui-based interactive interface with four tabs (Topics, Nodes, Log, Joints), tree/table/log detail renderers, and client-side display throttling
- `joint-control`: URDF parsing to extract joint definitions, merging with live `/joint_states` data, limit-aware gauge visualisation, position command input, and predefined pose execution
- `cli-interface`: Non-interactive CLI for one-shot queries against the agent (list topics, list nodes, echo topic data)

### Modified Capabilities

_None — this is a greenfield project._

## Impact

- **Dependencies**: `rclrs` (ROS 2 client), `ratatui`/`crossterm` (TUI), `tokio` (async runtime), `bincode`/`serde` (serialisation), `clap` (CLI), URDF parser crate
- **Build**: Workspace requires ROS 2 environment only for `talos-agent` crate; other crates build standalone
- **Deployment**: Agent binary deployed to target device; CLI and TUI binaries run on developer host
- **ROS 2 topics**: `/odom`, `/cmd_vel`, `/robot_description`, `/joint_states`, `/rosout` as initial set, configurable via TOML
