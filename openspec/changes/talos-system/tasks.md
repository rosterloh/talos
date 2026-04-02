## 1. Workspace Setup

- [x] 1.1 Convert root `Cargo.toml` to workspace with members: `talos-cli`, `talos-common`, `talos-agent`, `talos-tui`
- [x] 1.2 Scaffold crate directories with `Cargo.toml` and `src/lib.rs` or `src/main.rs` for each member
- [x] 1.3 Add shared workspace dependencies: `tokio`, `serde`, `bincode`, `tracing`, `tracing-subscriber`

## 2. talos-common: Protocol Types

- [x] 2.1 Define `DynValue` enum with all ROS 2 primitive types, `Array`, `Bytes`, and `Struct` variants; derive `Serialize`/`Deserialize`
- [x] 2.2 Define `TopicInfo`, `NodeInfo`, `Timestamp`, and `JointInfo` structs
- [x] 2.3 Define `Request` enum (`ListTopics`, `ListNodes`, `SetJointPosition`, `ExecutePose`)
- [x] 2.4 Define `Response` enum (`TopicList`, `NodeList`, `TopicData`, `PoseList`, `Error`)
- [x] 2.5 Write unit tests for bincode round-trip serialisation of all protocol types

## 3. talos-common: Codec and Transport

- [x] 3.1 Implement length-prefixed frame codec using `tokio_util::codec::Decoder`/`Encoder` with configurable max frame size
- [x] 3.2 Define `Transport` async trait with `AsyncRead`/`AsyncWrite` associated types
- [x] 3.3 Implement UDS transport: server (listen + accept) and client (connect) using `tokio::net::UnixStream`/`UnixListener`
- [x] 3.4 Write integration tests: client connects to server, sends request, receives response over UDS

## 4. talos-common: Config and URDF

- [x] 4.1 Define config structs (`AgentConfig`, `TransportConfig`, `SubscriptionConfig`, `ControlConfig`, `PoseConfig`) with serde TOML deserialization
- [x] 4.2 Implement config loading with default values and `--config` path override
- [x] 4.3 Add `urdf-rs` dependency and implement joint extraction from URDF XML string (name, type, limits, parent/child links)
- [x] 4.4 Write tests for config loading and URDF joint extraction

## 5. talos-agent: Core

- [ ] 5.1 Set up `rclrs` node creation and async spinning within tokio runtime
- [ ] 5.2 Implement graceful shutdown on SIGINT/SIGTERM (shut down rclrs node, close client connections)
- [ ] 5.3 Implement UDS server: accept connections, register clients, broadcast responses to all connected clients
- [ ] 5.4 Implement request handler: dispatch `ListTopics`/`ListNodes` using rclrs graph APIs
- [ ] 5.5 Load agent config from TOML on startup, parse CLI args with clap (`--config` flag)

## 6. talos-agent: ROS 2 Subscriptions

- [ ] 6.1 Implement `From<nav_msgs::msg::Odometry> for DynValue`
- [ ] 6.2 Implement `From<geometry_msgs::msg::Twist> for DynValue`
- [ ] 6.3 Implement `From<std_msgs::msg::String> for DynValue`
- [ ] 6.4 Implement `From<sensor_msgs::msg::JointState> for DynValue`
- [ ] 6.5 Implement `From<rcl_interfaces::msg::Log> for DynValue`
- [ ] 6.6 Implement subscription creation from config: match type string to typed subscription factory, skip unknown types with warning
- [ ] 6.7 Wire subscription callbacks to broadcast `Response::TopicData` to connected clients

## 7. talos-agent: Joint Control

- [ ] 7.1 Implement joint command publisher: create `JointState` publisher on configured control topic
- [ ] 7.2 Handle `Request::SetJointPosition` — construct and publish `JointState` for single joint
- [ ] 7.3 Handle `Request::ExecutePose` — look up pose in config, construct and publish full `JointState`
- [ ] 7.4 Return `Response::Error` when control is not configured

## 8. talos-tui: App Skeleton

- [ ] 8.1 Set up ratatui app with crossterm backend, tick-based event loop, and clean terminal restore on exit
- [ ] 8.2 Implement tab bar rendering (Topics, Nodes, Log, Joints) with number key and Tab switching
- [ ] 8.3 Implement connection status indicator (top-right corner)
- [ ] 8.4 Implement IPC client: connect to agent, spawn reader task that updates shared state (`Arc<Mutex<AppState>>`)
- [ ] 8.5 Implement keyboard navigation framework: arrow keys, Enter, Tab between panes, `q` quit, `?` help overlay

## 9. talos-tui: Topics Tab

- [ ] 9.1 Implement topic list widget: display topic names with live Hz counter
- [ ] 9.2 Implement `DynValue` tree renderer: collapsed by default, expand/collapse with keys
- [ ] 9.3 Wire topic selection to detail pane: show latest `DynValue` for selected topic
- [ ] 9.4 Implement Hz calculation from message timestamps in the latest-value store

## 10. talos-tui: Nodes Tab

- [ ] 10.1 Implement node list widget from `Response::NodeList` data
- [ ] 10.2 Implement node detail widget: namespace, publishers, subscribers, services

## 11. talos-tui: Log Tab

- [ ] 11.1 Implement ring buffer for `/rosout` messages (configurable max size)
- [ ] 11.2 Implement log table widget: timestamp, severity (coloured), node name, message columns
- [ ] 11.3 Implement severity filter (dropdown or toggle)
- [ ] 11.4 Implement node name filter (text input)
- [ ] 11.5 Implement keyword search filter

## 12. talos-tui: Joints Tab

- [ ] 12.1 Implement joint list widget showing joint names and current positions from merged URDF + `/joint_states`
- [ ] 12.2 Implement limit-aware gauge bar widget for joint position visualisation
- [ ] 12.3 Implement joint detail pane: type, parent/child links, position gauge, velocity, effort
- [ ] 12.4 Implement position input: edit key, numeric entry, limit clamping with warning
- [ ] 12.5 Implement pose list widget below joint list
- [ ] 12.6 Implement pose execution: select pose, confirm, send `Request::ExecutePose`

## 13. talos-cli

- [ ] 13.1 Set up clap derive CLI with subcommands: `list-topics`, `list-nodes`, `echo`
- [ ] 13.2 Implement `list-topics`: connect, send `Request::ListTopics`, print table, disconnect
- [ ] 13.3 Implement `list-nodes`: connect, send `Request::ListNodes`, print table, disconnect
- [ ] 13.4 Implement `echo <topic>`: connect, receive `TopicData` stream, print `DynValue` tree to stdout, support `--count` flag
- [ ] 13.5 Implement `--socket` global flag for custom socket path
