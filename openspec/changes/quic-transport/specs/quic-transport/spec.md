## ADDED Requirements

### Requirement: QUIC endpoint using quinn
The agent SHALL create a QUIC server endpoint using the `quinn` crate, binding to the address specified in `[transport.quic]` configuration. The endpoint SHALL accept incoming QUIC connections from clients.

#### Scenario: Agent starts with QUIC enabled
- **WHEN** the agent config contains a `[transport.quic]` section with `bind_addr = "0.0.0.0:4433"`
- **THEN** the agent binds a QUIC endpoint on UDP port 4433 and accepts connections

#### Scenario: Agent starts without QUIC config
- **WHEN** the agent config does not contain a `[transport.quic]` section
- **THEN** the agent does not create a QUIC endpoint and serves only UDS

### Requirement: Self-signed certificate generation
The agent SHALL generate a self-signed TLS certificate at startup using `rcgen` if no certificate files are configured or found. The generated certificate SHALL be used to configure the QUIC endpoint's TLS identity.

#### Scenario: No certificate configured
- **WHEN** the agent starts with `[transport.quic]` but no `cert_path`/`key_path`
- **THEN** the agent generates a self-signed certificate in memory and uses it for the QUIC endpoint

#### Scenario: Certificate files configured
- **WHEN** `cert_path` and `key_path` are specified and the files exist
- **THEN** the agent loads the certificate and key from disk

#### Scenario: Certificate files missing
- **WHEN** `cert_path` and `key_path` are specified but the files do not exist
- **THEN** the agent SHALL log an error and exit

### Requirement: Insecure client TLS verification
The QUIC client SHALL use a custom `ServerCertVerifier` that accepts any server certificate without verification. This enables connections to agents using self-signed certificates on trusted local networks.

#### Scenario: Client connects to agent with self-signed cert
- **WHEN** a client connects via QUIC to an agent using a self-signed certificate
- **THEN** the TLS handshake succeeds and the connection is established

### Requirement: Client QUIC endpoint
The client (TUI or CLI) SHALL create a QUIC client endpoint and connect to the agent at the address specified by the `--remote` flag. The client SHALL configure `max_concurrent_uni_streams` to at least 64 to permit server-initiated data streams.

#### Scenario: Client connects to remote agent
- **WHEN** the user runs `talos-tui --remote 192.168.1.50:4433`
- **THEN** the TUI creates a QUIC connection to `192.168.1.50:4433`

#### Scenario: Connection failure
- **WHEN** the client cannot reach the specified QUIC address
- **THEN** the client SHALL display an error and retry with backoff (TUI) or exit with non-zero status (CLI)

### Requirement: Bidirectional control stream
Upon QUIC connection establishment, the client SHALL open a bidirectional stream (stream 0) for control messages. All `Request`/`Response` pairs SHALL be exchanged on this stream using the existing length-prefixed bincode framing.

#### Scenario: Client opens control stream
- **WHEN** a QUIC connection is established
- **THEN** the client opens a bidirectional stream and sends control requests on it

#### Scenario: Control request/response
- **WHEN** the client sends `Request::ListTopics` on the control stream
- **THEN** the agent responds with `Response::TopicList` on the same stream

### Requirement: Server-initiated unidirectional data streams
When a client subscribes to a topic, the agent SHALL open a unidirectional stream to the client for that topic. The first frame on the stream SHALL be a `StreamHeader { topic, type_name }`. Subsequent frames SHALL be `TopicFrame { stamp, data }`.

#### Scenario: Topic data stream opened
- **WHEN** the agent receives `Subscribe { topics: ["/odom"] }` from a QUIC client
- **THEN** the agent opens a unidirectional stream, sends `StreamHeader { topic: "/odom", type_name: "nav_msgs/msg/Odometry" }`, and begins sending `TopicFrame` messages

#### Scenario: Multiple topic streams
- **WHEN** a client subscribes to `/odom` and `/joint_states`
- **THEN** the agent opens two separate unidirectional streams, one per topic

#### Scenario: Client receives data stream
- **WHEN** the agent opens a unidirectional stream to the client
- **THEN** the client accepts the stream via `connection.accept_uni()`, reads the `StreamHeader`, and processes subsequent `TopicFrame` messages

### Requirement: Data stream closure on unsubscribe
When a client unsubscribes from a topic, the agent SHALL close (finish) the corresponding unidirectional stream.

#### Scenario: Client unsubscribes
- **WHEN** the client sends `Unsubscribe { topics: ["/odom"] }` on the control stream
- **THEN** the agent closes the unidirectional stream for `/odom`

#### Scenario: Client disconnects
- **WHEN** the QUIC connection is closed or lost
- **THEN** the agent closes all data streams and removes the client's subscription state
