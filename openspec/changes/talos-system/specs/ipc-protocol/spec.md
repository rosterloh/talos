## ADDED Requirements

### Requirement: Length-prefixed bincode framing
The system SHALL use a 4-byte big-endian length prefix followed by a bincode-encoded payload for all IPC messages. The codec SHALL be implemented as a `tokio_util::codec::Decoder`/`Encoder` operating on `AsyncRead`/`AsyncWrite` traits.

#### Scenario: Client sends a request
- **WHEN** the client serialises a `Request` enum variant with bincode
- **THEN** the codec prepends a 4-byte big-endian length and writes the frame to the transport

#### Scenario: Agent receives a request
- **WHEN** the agent reads bytes from the transport
- **THEN** the codec reads the 4-byte length prefix, buffers until the full payload is available, and deserialises the bincode payload into a `Request`

#### Scenario: Oversized frame
- **WHEN** a frame exceeds the maximum allowed size (configurable, default 16 MiB)
- **THEN** the codec SHALL return an error and the connection SHALL be closed

### Requirement: Request/Response message types
The protocol SHALL define `Request` and `Response` enums covering topic listing, node listing, and topic data streaming. All types SHALL derive `serde::Serialize` and `serde::Deserialize`.

#### Scenario: List topics request
- **WHEN** a client sends `Request::ListTopics`
- **THEN** the agent responds with `Response::TopicList(Vec<TopicInfo>)` containing all subscribed topics

#### Scenario: List nodes request
- **WHEN** a client sends `Request::ListNodes`
- **THEN** the agent responds with `Response::NodeList(Vec<NodeInfo>)` containing discovered ROS 2 nodes

#### Scenario: Topic data streaming
- **WHEN** a client is connected and the agent receives a ROS 2 message on a subscribed topic
- **THEN** the agent sends `Response::TopicData { topic, type_name, stamp, data }` to all connected clients

#### Scenario: Error response
- **WHEN** a client sends a request that cannot be fulfilled
- **THEN** the agent responds with `Response::Error(String)` describing the failure

### Requirement: DynValue generic message representation
The protocol SHALL define a `DynValue` enum capable of representing any ROS 2 message type. It SHALL support all ROS 2 primitive types (`bool`, `i8`-`i64`, `u8`-`u64`, `f32`, `f64`, `String`), byte arrays, arrays of `DynValue`, and named struct fields preserving field order.

#### Scenario: Representing a flat message
- **WHEN** a `geometry_msgs/Twist` message is converted to `DynValue`
- **THEN** the result is a `DynValue::Struct` with `type_name: "Twist"` and fields containing nested `Struct` values for `linear` and `angular`, each with `x`, `y`, `z` as `F64` values

#### Scenario: Representing a message with arrays
- **WHEN** a `sensor_msgs/JointState` message is converted to `DynValue`
- **THEN** the `name` field is a `DynValue::Array` of `DynValue::String` values, and `position`, `velocity`, `effort` fields are `DynValue::Array` of `DynValue::F64` values

### Requirement: UDS transport
The system SHALL support Unix domain socket transport. The agent SHALL listen on a configurable socket path (default `/tmp/talos.sock`). The client SHALL connect to the same path.

#### Scenario: Agent starts and listens
- **WHEN** the agent starts
- **THEN** it creates a UDS listener at the configured socket path and accepts client connections

#### Scenario: Client connects
- **WHEN** a client connects to the UDS path
- **THEN** a framed bincode connection is established and the client can send requests and receive responses

#### Scenario: Multiple clients
- **WHEN** multiple clients connect simultaneously
- **THEN** the agent SHALL accept all connections and broadcast topic data to each

#### Scenario: Socket path already exists
- **WHEN** the agent starts and the socket path already exists from a previous run
- **THEN** the agent SHALL remove the stale socket file and create a new listener

### Requirement: Transport abstraction
The transport layer SHALL be abstracted behind an async trait so that the codec and protocol logic are transport-agnostic. The trait SHALL expose `AsyncRead`/`AsyncWrite` associated types.

#### Scenario: UDS transport implements trait
- **WHEN** the UDS transport is used
- **THEN** it implements the `Transport` trait wrapping `tokio::net::UnixStream`

#### Scenario: Future transport added
- **WHEN** a new transport (e.g., QUIC) is added
- **THEN** it can implement the same `Transport` trait without changing codec or protocol code
