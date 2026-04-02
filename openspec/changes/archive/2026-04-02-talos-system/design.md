## Context

This is a greenfield Rust workspace. The target environment is a ROS 2 robot (typically running Ubuntu with a ROS 2 Jazzy or Rolling installation) and a developer workstation that may or may not have ROS 2 installed. The agent runs co-located with ROS 2 on the target; the CLI and TUI run on the developer machine (initially the same machine, communicating over UDS).

The key constraint is that only `talos-agent` may depend on `rclrs` or any ROS 2 libraries. All other crates must build and run without a ROS 2 environment.

## Goals / Non-Goals

**Goals:**

- Establish a clean four-crate workspace with well-defined dependency boundaries
- Define a bincode-based IPC protocol that supports both request/response and streaming patterns
- Bridge a fixed set of ROS 2 topics (typed subscriptions) to a generic `DynValue` representation
- Deliver a ratatui TUI with four tabs: Topics, Nodes, Log, Joints
- Parse URDF from `/robot_description` to provide joint visualisation and control
- Keep the transport layer abstract enough to swap UDS for QUIC later

**Non-Goals:**

- Runtime discovery of arbitrary ROS 2 message types (future: runtime introspection via rosidl)
- QUIC or TCP transport (future phase — UDS only for v0.1)
- Agent-side message throttling or downsampling (client handles display rate)
- ROS 2 action client support
- Authentication or encryption on the IPC socket

## Decisions

### D1: Workspace structure — four crates with strict dependency boundaries

```
talos/
├── Cargo.toml              ← workspace root
├── talos-cli/Cargo.toml    ← depends on talos-common
├── talos-common/Cargo.toml ← no ROS 2 deps
├── talos-agent/Cargo.toml  ← depends on talos-common + rclrs
└── talos-tui/Cargo.toml    ← depends on talos-common + ratatui
```

**Rationale**: Isolating `rclrs` to `talos-agent` means the TUI and CLI can be built and tested on any machine. `talos-common` is the shared contract — protocol types, codec, transport trait, config, and URDF parsing.

**Alternatives considered**: Monolithic binary with feature flags — rejected because it would require ROS 2 to build anything; separate repos — rejected as unnecessary overhead for a cohesive system.

### D2: Bincode with length-prefixed framing over tokio `AsyncRead`/`AsyncWrite`

Wire format: `[4-byte big-endian length][bincode payload]`

Protocol messages are Rust enums (`Request`, `Response`) derived with `serde::Serialize`/`serde::Deserialize`. The codec is implemented as a `tokio_util::codec::Decoder`/`Encoder`.

**Rationale**: Bincode is compact, fast, and trivially derives from Rust types. Length-prefixed framing is simple and works identically over UDS and future TCP/QUIC streams. No schema language or codegen needed.

**Alternatives considered**: gRPC/tonic (mature streaming, but heavy codegen and proto dependency); JSON (human-readable but slower and larger); msgpack (slightly more portable but unnecessary since both ends are Rust).

### D3: Typed subscriptions with `From<T> for DynValue` conversion

For v0.1, the agent subscribes to topics using concrete `rclrs` message types and implements `From<nav_msgs::Odometry> for DynValue`, etc. Supported types:

- `nav_msgs/msg/Odometry`
- `geometry_msgs/msg/Twist`
- `std_msgs/msg/String` (for `/robot_description`)
- `sensor_msgs/msg/JointState`
- `rcl_interfaces/msg/Log` (for `/rosout`)

**Rationale**: Typed subscriptions are the simplest path to a working pipeline. The `DynValue` representation decouples the protocol from ROS 2 message definitions, so adding runtime introspection later doesn't change the wire format or client code.

**Alternatives considered**: Raw CDR byte passthrough (requires clients to have message definitions); immediate runtime introspection (complex, deferred to v0.2).

### D4: `DynValue` as the universal message representation

```rust
enum DynValue {
    Bool(bool),
    I8(i8), I16(i16), I32(i32), I64(i64),
    U8(u8), U16(u16), U32(u32), U64(u64),
    F32(f32), F64(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<DynValue>),
    Struct { type_name: String, fields: Vec<(String, DynValue)> },
}
```

**Rationale**: Covers all ROS 2 primitive and composite types. Using `Vec<(String, DynValue)>` for struct fields preserves field order (important for display) and field names (important for tree rendering). Bincode serialises this efficiently.

### D5: Transport abstraction via async trait

```rust
trait Transport: Send + Sync {
    type Reader: AsyncRead + Unpin + Send;
    type Writer: AsyncWrite + Unpin + Send;
    async fn connect(config: &TransportConfig) -> Result<(Self::Reader, Self::Writer)>;
}
```

UDS implementation wraps `tokio::net::UnixStream`. Future QUIC implementation wraps `quiche` streams. The codec layer operates on `AsyncRead`/`AsyncWrite`, so it's transport-agnostic.

### D6: TUI architecture — four tabs with adaptive detail renderers

| Tab | Left pane | Right pane |
|-----|-----------|------------|
| Topics | Topic list with live Hz | Detail: tree view (collapsed by default, expand with keys) |
| Nodes | Node list | Node info: publishers, subscribers, services |
| Log | Full-width `/rosout` log table | Filterable by severity, node, keyword |
| Joints | Joint list + poses list | Joint detail: gauge bar (limit-aware), position control |

Rendering approach:
- Main loop runs at a fixed tick rate (~15-30 FPS)
- Incoming `TopicData` messages update a `HashMap<String, TopicData>` — latest value wins
- Render reads from the map each tick — this is the throttle mechanism
- `/rosout` messages append to a ring buffer; the Log tab renders the visible window

### D7: Joint control via configurable ROS 2 publishing

The agent reads URDF from `/robot_description` and extracts joint definitions. The TUI merges these with live `/joint_states` to show limit-aware gauges.

For control, the agent publishes to a configurable topic (default: topic-based publishing of `sensor_msgs/JointState`). Predefined poses are defined in the agent TOML config.

### D8: Configuration via TOML

```toml
[transport]
socket_path = "/tmp/talos.sock"

[[subscriptions]]
topic = "/odom"
type = "nav_msgs/msg/Odometry"

[control]
method = "topic"
topic = "/joint_commands"
type = "sensor_msgs/msg/JointState"

[poses.home]
shoulder_pan = 0.0
shoulder_lift = 0.0
```

**Rationale**: TOML is the Rust ecosystem standard for config. Subscriptions and control mapping are configurable so the agent adapts to different robots without recompilation.

## Risks / Trade-offs

- **Typed subscriptions limit flexibility** → Mitigated by designing `DynValue` as the wire type now; adding runtime introspection later doesn't change the protocol or client code.

- **URDF parsing adds complexity to `talos-common`** → Mitigated by using an existing URDF parser crate (`urdf-rs`). Only joint extraction is needed, not full kinematic modelling.

- **`rclrs` maturity** → `rclrs` is less mature than `rclcpp`/`rclpy`. Mitigated by keeping the `rclrs` surface area minimal (node creation, typed subscriptions, topic publishing) and isolating it in `talos-agent`.

- **Single-client UDS** → v0.1 supports multiple clients connecting to the same socket, but the agent broadcasts all data to all clients. No per-client filtering. Acceptable for local development; may need refinement for resource-constrained targets.

- **Joint control safety** → Sending raw joint commands could damage hardware. Mitigated by requiring explicit config to enable control, and relying on the robot's own joint limit enforcement in its controller stack.

## Open Questions

- Should `talos-common` own the URDF parsing, or should it live in a separate `talos-urdf` crate?
- What is the reconnection strategy when the agent restarts? Auto-reconnect with backoff in the client?
- Should the TUI support multiple simultaneous agent connections (multiple robots)?
