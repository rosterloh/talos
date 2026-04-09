## MODIFIED Requirements

### Requirement: Typed topic subscriptions from configuration
The agent SHALL subscribe to topics defined in the TOML configuration file using typed `rclrs` subscriptions. Each subscription specifies a topic name and ROS 2 message type.

#### Scenario: Subscribe to configured topics
- **WHEN** the agent starts with a config containing `/odom` as `nav_msgs/msg/Odometry`
- **THEN** it creates a typed subscription for `nav_msgs::msg::Odometry` on `/odom`

#### Scenario: Unknown message type in config
- **WHEN** the config specifies a message type not supported by the agent's compiled-in types
- **THEN** the agent SHALL log a warning and skip that subscription

### Requirement: TOML-based agent configuration
The agent SHALL read its configuration from a TOML file specifying: UDS transport settings, optional QUIC transport settings, subscriptions (topic + type), control settings, and predefined poses.

#### Scenario: Default config path
- **WHEN** the agent is started without a `--config` flag
- **THEN** it looks for `talos-agent.toml` in the current directory

#### Scenario: Custom config path
- **WHEN** the agent is started with `--config /path/to/config.toml`
- **THEN** it reads configuration from that path

#### Scenario: Missing config
- **WHEN** no config file is found
- **THEN** the agent SHALL use sensible defaults (UDS socket at `/tmp/talos.sock`, no QUIC, no subscriptions)

#### Scenario: UDS-only config
- **WHEN** the config contains only `[transport.uds]`
- **THEN** the agent serves UDS only

#### Scenario: Dual transport config
- **WHEN** the config contains both `[transport.uds]` and `[transport.quic]`
- **THEN** the agent serves both UDS and QUIC simultaneously

## ADDED Requirements

### Requirement: Dual transport listeners
The agent SHALL support running both UDS and QUIC listeners simultaneously. Each listener spawns independent per-client tasks using the protocol session abstraction.

#### Scenario: Both transports active
- **WHEN** the agent starts with both `[transport.uds]` and `[transport.quic]` configured
- **THEN** it accepts UDS connections on the socket path and QUIC connections on the bind address concurrently

#### Scenario: UDS client and QUIC client connected
- **WHEN** a local UDS client and a remote QUIC client are connected simultaneously
- **THEN** each client receives topic data independently based on its own subscription set

### Requirement: TopicRouter replaces broadcast channel
The agent SHALL use a `TopicRouter` to route incoming ROS 2 topic data to connected clients based on their subscription sets, replacing the current `broadcast::channel` that sends all data to all clients.

#### Scenario: Data routed to subscriber
- **WHEN** client A is subscribed to `/odom` and a new `/odom` message arrives from ROS 2
- **THEN** the `TopicRouter` sends the data to client A only

#### Scenario: Data not routed to non-subscriber
- **WHEN** client B is not subscribed to `/odom` and a new `/odom` message arrives
- **THEN** client B does not receive the message

#### Scenario: Multiple subscribers
- **WHEN** clients A and B are both subscribed to `/joint_states`
- **THEN** both clients receive the `/joint_states` data

### Requirement: QUIC self-signed certificate generation
When QUIC is enabled and no certificate files are configured, the agent SHALL generate a self-signed certificate at startup using `rcgen` and use it for the QUIC endpoint.

#### Scenario: Certificate auto-generated
- **WHEN** `[transport.quic]` is present without `cert_path`/`key_path`
- **THEN** the agent generates a self-signed certificate and logs that it is using an auto-generated certificate
