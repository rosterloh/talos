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
The protocol SHALL define `Request` and `Response` enums covering topic listing, node listing, topic data streaming, topic subscription, and joint control. All types SHALL derive `serde::Serialize` and `serde::Deserialize`.

#### Scenario: List topics request
- **WHEN** a client sends `Request::ListTopics`
- **THEN** the agent responds with `Response::TopicList(Vec<TopicInfo>)` containing all bridged topics

#### Scenario: List nodes request
- **WHEN** a client sends `Request::ListNodes`
- **THEN** the agent responds with `Response::NodeList(Vec<NodeInfo>)` containing discovered ROS 2 nodes

#### Scenario: Subscribe request
- **WHEN** a client sends `Request::Subscribe { topics }`
- **THEN** the agent responds with `Response::Subscribed { topics: Vec<TopicSub> }`

#### Scenario: Unsubscribe request
- **WHEN** a client sends `Request::Unsubscribe { topics }`
- **THEN** the agent responds with `Response::Unsubscribed { topics: Vec<String> }`

#### Scenario: Topic data streaming
- **WHEN** a client is subscribed to a topic and the agent receives a ROS 2 message on that topic
- **THEN** the agent sends topic data only to subscribed clients

#### Scenario: Error response
- **WHEN** a client sends a request that cannot be fulfilled
- **THEN** the agent responds with `Response::Error(String)` describing the failure

### Requirement: Subscribe request
The protocol SHALL define a `Request::Subscribe { topics: Vec<String> }` variant. When sent by a client, the agent registers the client for data delivery on the specified topics.

#### Scenario: Client subscribes to topics
- **WHEN** a client sends `Request::Subscribe { topics: ["/odom", "/joint_states"] }`
- **THEN** the agent adds those topics to the client's subscription set and responds with `Response::Subscribed`

#### Scenario: Subscribe to already-subscribed topic
- **WHEN** a client sends `Subscribe` for a topic it is already subscribed to
- **THEN** the agent treats it as a no-op for that topic and includes it in the `Subscribed` response

#### Scenario: Subscribe to unknown topic
- **WHEN** a client sends `Subscribe` for a topic the agent is not bridging
- **THEN** the agent responds with `Response::Subscribed` but the topic entry indicates it is not available, and no data stream is opened for it

### Requirement: Unsubscribe request
The protocol SHALL define a `Request::Unsubscribe { topics: Vec<String> }` variant. When sent by a client, the agent removes the client from data delivery for the specified topics.

#### Scenario: Client unsubscribes from topic
- **WHEN** a client sends `Request::Unsubscribe { topics: ["/odom"] }`
- **THEN** the agent removes `/odom` from the client's subscription set and responds with `Response::Unsubscribed`

#### Scenario: Unsubscribe from non-subscribed topic
- **WHEN** a client sends `Unsubscribe` for a topic it is not subscribed to
- **THEN** the agent treats it as a no-op and includes it in the `Unsubscribed` response

### Requirement: Subscribed response
The protocol SHALL define a `Response::Subscribed { topics: Vec<TopicSub> }` variant. `TopicSub` SHALL contain the topic name and type name for each successfully subscribed topic.

#### Scenario: Subscribed response returned
- **WHEN** the agent processes a `Subscribe` request for `/odom` (type `nav_msgs/msg/Odometry`)
- **THEN** it responds with `Subscribed { topics: [TopicSub { topic: "/odom", type_name: "nav_msgs/msg/Odometry" }] }`

### Requirement: Unsubscribed response
The protocol SHALL define a `Response::Unsubscribed { topics: Vec<String> }` variant listing the topics that were unsubscribed.

#### Scenario: Unsubscribed response returned
- **WHEN** the agent processes an `Unsubscribe` request for `/odom`
- **THEN** it responds with `Unsubscribed { topics: ["/odom"] }`

### Requirement: TopicFrame data type
The protocol SHALL define a `TopicFrame` struct containing `stamp: Timestamp` and `data: DynValue`. This type is used on QUIC unidirectional data streams where the topic identity is established by the stream header.

#### Scenario: TopicFrame on QUIC data stream
- **WHEN** the agent sends topic data on a QUIC unidirectional stream
- **THEN** each frame after the stream header is a bincode-encoded `TopicFrame { stamp, data }`

### Requirement: StreamHeader data type
The protocol SHALL define a `StreamHeader` struct containing `topic: String` and `type_name: String`. This is the first frame sent on each QUIC unidirectional data stream to bind the stream to a topic.

#### Scenario: StreamHeader sent on new data stream
- **WHEN** the agent opens a unidirectional stream for `/joint_states`
- **THEN** the first frame is `StreamHeader { topic: "/joint_states", type_name: "sensor_msgs/msg/JointState" }`

### Requirement: TopicSub data type
The protocol SHALL define a `TopicSub` struct containing `topic: String` and `type_name: String`, used in `Response::Subscribed` to describe each subscribed topic.

#### Scenario: TopicSub in response
- **WHEN** a client subscribes to `/odom`
- **THEN** the `Subscribed` response contains a `TopicSub` with the topic name and its ROS 2 message type

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
