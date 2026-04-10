## ADDED Requirements

### Requirement: ProtocolClient trait
The system SHALL define a `ProtocolClient` trait that provides a transport-agnostic interface for client applications. The trait SHALL expose methods for sending control requests, subscribing to topics, and receiving topic data.

#### Scenario: Send a control request
- **WHEN** application code calls `protocol_client.request(Request::ListTopics)`
- **THEN** the request is sent over the underlying transport and the corresponding `Response` is returned

#### Scenario: Subscribe to topics
- **WHEN** application code calls `protocol_client.subscribe(&["/odom".into()])`
- **THEN** the client sends a `Subscribe` request and returns the `Vec<TopicSub>` from the response

#### Scenario: Receive topic data
- **WHEN** application code calls `protocol_client.recv_data()`
- **THEN** it receives the next `TopicFrame` from any subscribed topic, tagged with the topic name

### Requirement: UDS protocol client implementation
The system SHALL provide a `UdsProtocolClient` that implements `ProtocolClient` over a single UDS connection. Control requests and data frames are multiplexed on the same framed stream. The implementation SHALL demultiplex incoming frames, routing control responses to `request()` callers and data frames to `recv_data()` callers.

#### Scenario: Interleaved control and data
- **WHEN** the client has active subscriptions and sends a `ListTopics` request
- **THEN** `request()` returns the `TopicList` response while `recv_data()` continues to yield `TopicData` frames, despite sharing one connection

#### Scenario: Data filtering
- **WHEN** the agent sends `TopicData` for a topic the client has not subscribed to
- **THEN** the `UdsProtocolClient` drops the frame silently

### Requirement: QUIC protocol client implementation
The system SHALL provide a `QuicProtocolClient` that implements `ProtocolClient` over a QUIC connection. Control messages use bidirectional stream 0. Topic data arrives on server-initiated unidirectional streams.

#### Scenario: Control on stream 0
- **WHEN** application code calls `request(Request::SetJointPosition { .. })`
- **THEN** the request is sent on the bidirectional control stream and the response is read from the same stream

#### Scenario: Data from uni streams
- **WHEN** the agent opens a unidirectional stream for `/odom`
- **THEN** `recv_data()` accepts the stream, reads the `StreamHeader`, and yields subsequent `TopicFrame` messages tagged with topic `/odom`

#### Scenario: Select across streams
- **WHEN** multiple data streams are active
- **THEN** `recv_data()` uses `select!` across all open unidirectional streams and returns whichever fires first

### Requirement: ProtocolServer trait
The system SHALL define a `ProtocolServer` trait for the agent side, providing methods to accept client sessions, handle control requests, and push topic data to subscribers.

#### Scenario: Accept a client session
- **WHEN** a UDS or QUIC client connects
- **THEN** `protocol_server.accept()` returns a `ProtocolSession` handle for that client

#### Scenario: Push data to subscriber
- **WHEN** the agent calls `session.send_data(topic, frame)` for a subscribed topic
- **THEN** the frame is delivered to the client via the appropriate transport mechanism

### Requirement: TopicRouter
The agent SHALL maintain a `TopicRouter` that tracks per-client subscription sets and routes incoming topic data only to clients subscribed to that topic.

#### Scenario: Route to subscribers only
- **WHEN** client A is subscribed to `/odom` and client B is subscribed to `/joint_states`
- **THEN** an `/odom` message is sent only to client A, and a `/joint_states` message is sent only to client B

#### Scenario: Client subscribes to additional topic
- **WHEN** client A sends `Subscribe { topics: ["/rosout"] }`
- **THEN** client A's subscription set is updated and subsequent `/rosout` data is routed to client A

#### Scenario: Client disconnects
- **WHEN** a client's connection drops
- **THEN** the `TopicRouter` removes that client's subscription state and cleans up resources

#### Scenario: No subscribers for topic
- **WHEN** a topic data message arrives and no clients are subscribed to that topic
- **THEN** the message is discarded with no work done
