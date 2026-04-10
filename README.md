# Talos

A terminal-native tool for observing and interacting with ROS 2 systems. Talos decouples your developer workstation from the ROS 2 runtime — the agent runs on the robot, the TUI and CLI run anywhere.

```
  Developer machine                       Target device
 ┌────────────────────────┐              ┌──────────────────┐
 │                        │              │                  │
 │  talos-tui             │    UDS /     │  talos-agent     │
 │  talos-cli             │◄──QUIC──────►│     (rclrs)      │
 │                        │              │        │         │
 │  No ROS 2 required     │              │    ROS 2 Graph   │
 └────────────────────────┘              └──────────────────┘
```

## Features

- **Topic observation** — browse topics, view live message data as a collapsible tree
- **Node introspection** — list nodes with their publishers, subscribers, and services
- **Log viewer** — filterable `/rosout` stream with severity colouring
- **Joint control** — URDF-aware joint visualisation with limit gauges, position commands, and predefined poses
- **CLI** — non-interactive one-shot queries (`list-topics`, `list-nodes`, `echo`)

## Project Structure

```
talos/
├── Cargo.toml                  # Workspace root
│
├── talos-common/               # Shared library — no ROS 2 dependency
│   └── src/
│       ├── config.rs           # TOML config loading (agent, transport, poses)
│       ├── error.rs            # Error types
│       ├── protocol/
│       │   ├── codec.rs        # Length-prefixed bincode framing (tokio_util codec)
│       │   ├── messages.rs     # Request/Response enums
│       │   └── types.rs        # DynValue, TopicInfo, NodeInfo, JointInfo
│       ├── session/
│       │   ├── mod.rs          # ProtocolClient trait (transport-agnostic)
│       │   ├── uds.rs          # UDS ProtocolClient implementation
│       │   └── quic.rs         # QUIC ProtocolClient implementation (feature: quic)
│       ├── transport/
│       │   ├── mod.rs          # Transport trait (AsyncRead/AsyncWrite)
│       │   ├── uds.rs          # Unix domain socket implementation
│       │   ├── quic.rs         # QUIC endpoint setup (feature: quic)
│       │   └── certs.rs        # TLS certificate helpers (feature: quic)
│       └── urdf.rs             # URDF parsing → joint extraction
│
├── talos-agent/                # ROS 2 bridge — runs on target device
│   └── src/
│       ├── main.rs             # Startup, signal handling, task orchestration
│       ├── bridge.rs           # rclrs node, subscriptions, joint publisher
│       ├── conversions.rs      # ROS 2 msg → DynValue conversion functions
│       ├── router.rs           # Per-client subscription tracking and topic routing
│       └── server.rs           # UDS + QUIC server, request dispatch, client handlers
│
├── talos-tui/                  # Terminal UI — runs on developer machine
│   └── src/
│       ├── main.rs             # Event loop, keyboard handling
│       ├── state.rs            # AppState, response handling, filters
│       ├── client.rs           # Auto-reconnecting IPC client
│       └── ui/                 # Tab renderers (topics, nodes, log, joints)
│
└── talos-cli/                  # CLI — runs on developer machine
    └── src/
        └── main.rs             # clap subcommands (list-topics, list-nodes, echo)
```

### Dependency Graph

```
  talos-cli ────────┐
                    ▼
  talos-tui ──► talos-common ◄── talos-agent
                                      │
                                      ▼
                                    rclrs
```

Only `talos-agent` depends on ROS 2. Everything else builds standalone.

## TUI Layout

### Topics Tab

```
┌─ Talos ─────────────────────────────────────────────────────────────┐
│  [1]Topics  [2]Nodes  [3]Log  [4]Joints                ● connected │
├─────────────────────────────────────────────────────────────────────┤
│                                │                                    │
│  TOPICS                        │  DETAIL: /odom                     │
│                                │  nav_msgs/Odometry @ 20Hz          │
│    /cmd_vel           10Hz     │                                    │
│  ▶ /odom              20Hz     │  ▶ pose                            │
│    /joint_states      50Hz     │    ▶ position                      │
│    /robot_description latch    │        x: 1.2043                   │
│    /rosout            -        │        y: 0.3201                   │
│                                │        z: 0.0000                   │
│                                │    ▶ orientation                   │
│                                │        x: 0.0000                   │
│                                │        y: 0.0000                   │
│                                │        z: 0.7071                   │
│                                │        w: 0.7071                   │
│                                │  ▶ twist                           │
│                                │                                    │
├─────────────────────────────────────────────────────────────────────┤
│  ↑↓ navigate  Enter select  ←→ expand/collapse  Tab pane  q quit   │
└─────────────────────────────────────────────────────────────────────┘
```

### Nodes Tab

```
┌─────────────────────────────────────────────────────────────────────┐
│  [1]Topics  [2]Nodes  [3]Log  [4]Joints                ● connected │
├─────────────────────────────────────────────────────────────────────┤
│                                │                                    │
│  NODES                         │  NODE: /robot_state_publisher      │
│                                │  Namespace: /                      │
│  ▶ /robot_state_publisher      │                                    │
│    /joint_state_broadcaster    │  Publishers:                       │
│    /controller_manager         │    /robot_description               │
│    /move_group                 │    /tf_static                       │
│                                │                                    │
│                                │  Subscribers:                      │
│                                │    /joint_states                    │
│                                │                                    │
│                                │  Services:                         │
│                                │    /get_parameters                  │
│                                │    /describe_parameters             │
│                                │                                    │
└─────────────────────────────────────────────────────────────────────┘
```

### Log Tab

```
┌─────────────────────────────────────────────────────────────────────┐
│  [1]Topics  [2]Nodes  [3]Log  [4]Joints                ● connected │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  Time     Level   Node                    Message                   │
│  ──────── ─────── ─────────────────────── ───────────────────────── │
│  12:01:03 WARN    /tf_broadcaster         TF tree not complete...   │
│  12:01:02 INFO    /controller_manager     Loaded joint_trajectory.. │
│  12:01:01 ERROR   /move_group             Failed to compute path..  │
│  12:00:58 INFO    /robot_state_publisher  Loading robot model...    │
│  12:00:55 INFO    /controller_manager     Starting controllers...   │
│                                                                     │
├─────────────────────────────────────────────────────────────────────┤
│  Filter: [severity ▼] [node: ___________] [/search ___________]    │
└─────────────────────────────────────────────────────────────────────┘
```

### Joints Tab

```
┌─────────────────────────────────────────────────────────────────────┐
│  [1]Topics  [2]Nodes  [3]Log  [4]Joints                ● connected │
├─────────────────────────────────────────────────────────────────────┤
│                                │                                    │
│  JOINTS              Pos       │  CONTROL: shoulder_pan              │
│  ──────────────── ────────     │  Type: revolute                    │
│  ▶ shoulder_pan    0.0000      │  Parent: base_link                 │
│    shoulder_lift  -1.5708      │  Child: shoulder_link              │
│    elbow           1.5708      │                                    │
│    wrist_1        -0.7854      │  Position:                         │
│    wrist_2         0.0000      │  -3.14 ├────────█─────────┤ 3.14  │
│    wrist_3         0.0000      │                 0.0000              │
│                                │                                    │
│  POSES                         │  Velocity:  0.0000                 │
│  ─────                         │  Effort:    0.0000                 │
│    home                        │                                    │
│    pick_ready                  │                                    │
│    stow                        │                                    │
│                                │                                    │
│                                │                                    │
├─────────────────────────────────────────────────────────────────────┤
│  ↑↓ navigate  e edit  x execute pose  j/o focus  Tab pane  q quit  │
└─────────────────────────────────────────────────────────────────────┘
```

## Agent Configuration

```toml
# At least one transport must be configured
[transport.uds]
socket_path = "/tmp/talos.sock"

# Optional: enable QUIC for remote access (requires --features quic)
[transport.quic]
bind_addr = "0.0.0.0:4433"
# cert_path = "/path/to/cert.der"   # optional; self-signed if omitted
# key_path  = "/path/to/key.der"

[[subscriptions]]
topic = "/odom"
type = "nav_msgs/msg/Odometry"

[[subscriptions]]
topic = "/cmd_vel"
type = "geometry_msgs/msg/Twist"

[[subscriptions]]
topic = "/robot_description"
type = "std_msgs/msg/String"

[[subscriptions]]
topic = "/joint_states"
type = "sensor_msgs/msg/JointState"

[[subscriptions]]
topic = "/rosout"
type = "rcl_interfaces/msg/Log"

[control]
method = "topic"
topic = "/joint_commands"
type = "sensor_msgs/msg/JointState"

[poses.home]
shoulder_pan = 0.0
shoulder_lift = 0.0
elbow = 0.0
wrist_1 = 0.0
wrist_2 = 0.0
wrist_3 = 0.0

[poses.pick_ready]
shoulder_pan = 0.0
shoulder_lift = -1.5708
elbow = 1.5708
wrist_1 = -0.7854
wrist_2 = 0.0
wrist_3 = 0.0
```

## CLI Usage

```sh
# List all discovered topics
talos-cli list-topics

# List all discovered nodes
talos-cli list-nodes

# Echo live data from a topic
talos-cli echo /odom

# Echo 5 messages and exit
talos-cli echo /odom --count 5

# Use a custom socket path
talos-cli --socket /run/talos.sock list-topics

# Connect to a remote agent over QUIC (requires --features quic)
talos-cli --remote 192.168.1.50:4433 list-topics
talos-cli --remote 192.168.1.50:4433 echo /odom
```

## Architecture

The IPC protocol uses **length-prefixed bincode** over Unix domain sockets or QUIC:

```
┌──────────────────────────────────────┐
│         Application Logic            │
├──────────────────────────────────────┤
│   Session (ProtocolClient trait)     │
├──────────────────────────────────────┤
│   Protocol (Request/Response enums)  │
├──────────────────────────────────────┤
│   Codec (4-byte length + bincode)    │
├──────────────────────────────────────┤
│   Transport                          │
│   ┌──────────┐  ┌────────────────┐  │
│   │   UDS    │  │      QUIC      │  │
│   └──────────┘  └────────────────┘  │
└──────────────────────────────────────┘
```

Clients send explicit `Subscribe` requests after connecting. The agent's `TopicRouter` delivers data only for subscribed topics — no broadcast flooding.

ROS 2 messages are deserialised by the agent into a generic `DynValue` tree, so clients never need ROS 2 message definitions or libraries.

## Building

```sh
# Build everything except talos-agent (no ROS 2 required)
cargo build -p talos-common -p talos-cli -p talos-tui

# Build talos-agent (requires ROS 2 environment with rclrs)
source rclrs_ws/install/setup.bash
cargo build -p talos-agent

# Enable QUIC transport (feature-gated across all crates)
cargo build --features quic
```

## License

MIT
