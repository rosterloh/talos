## ADDED Requirements

### Requirement: ROS 2 node lifecycle
The agent SHALL create an `rclrs` node on startup and spin it within a tokio async runtime. The node SHALL be properly shut down when the agent process exits.

#### Scenario: Agent starts
- **WHEN** the agent binary is launched
- **THEN** it initialises an `rclrs` context, creates a node named `talos_agent`, and begins spinning

#### Scenario: Agent shuts down
- **WHEN** the agent receives SIGINT or SIGTERM
- **THEN** it gracefully shuts down the `rclrs` node and closes all client connections

### Requirement: Typed topic subscriptions from configuration
The agent SHALL subscribe to topics defined in the TOML configuration file using typed `rclrs` subscriptions. Each subscription specifies a topic name and ROS 2 message type.

#### Scenario: Subscribe to configured topics
- **WHEN** the agent starts with a config containing `/odom` as `nav_msgs/msg/Odometry`
- **THEN** it creates a typed subscription for `nav_msgs::msg::Odometry` on `/odom`

#### Scenario: Unknown message type in config
- **WHEN** the config specifies a message type not supported by the agent's compiled-in types
- **THEN** the agent SHALL log a warning and skip that subscription

### Requirement: Supported message types for v0.1
The agent SHALL support typed subscriptions and `DynValue` conversion for: `nav_msgs/msg/Odometry`, `geometry_msgs/msg/Twist`, `std_msgs/msg/String`, `sensor_msgs/msg/JointState`, `rcl_interfaces/msg/Log`.

#### Scenario: Odometry message received
- **WHEN** the agent receives an `Odometry` message on a subscribed topic
- **THEN** it converts the message to `DynValue::Struct` and broadcasts `Response::TopicData` to all connected clients

#### Scenario: Log message received
- **WHEN** the agent receives a `Log` message on `/rosout`
- **THEN** it converts it to `DynValue::Struct` including severity level, node name, timestamp, and message text

#### Scenario: JointState message received
- **WHEN** the agent receives a `JointState` message
- **THEN** it converts the parallel arrays (name, position, velocity, effort) into a `DynValue::Struct` with `DynValue::Array` fields

### Requirement: Node and topic discovery
The agent SHALL use `rclrs` graph APIs to discover nodes and topics on the ROS 2 graph and respond to `ListTopics` and `ListNodes` requests.

#### Scenario: Client requests topic list
- **WHEN** a client sends `Request::ListTopics`
- **THEN** the agent queries the ROS 2 graph for all topics and returns `Response::TopicList` with topic names, types, and publisher/subscriber counts

#### Scenario: Client requests node list
- **WHEN** a client sends `Request::ListNodes`
- **THEN** the agent queries the ROS 2 graph for all nodes and returns `Response::NodeList` with node names, namespaces, and their publisher/subscriber/service lists

### Requirement: TOML-based agent configuration
The agent SHALL read its configuration from a TOML file specifying: socket path, subscriptions (topic + type), control settings, and predefined poses.

#### Scenario: Default config path
- **WHEN** the agent is started without a `--config` flag
- **THEN** it looks for `talos-agent.toml` in the current directory

#### Scenario: Custom config path
- **WHEN** the agent is started with `--config /path/to/config.toml`
- **THEN** it reads configuration from that path

#### Scenario: Missing config
- **WHEN** no config file is found
- **THEN** the agent SHALL use sensible defaults (socket at `/tmp/talos.sock`, no subscriptions)

### Requirement: Joint command publishing
The agent SHALL publish joint commands to a configurable ROS 2 topic when instructed by a client. The publish topic, message type, and mapping are defined in config.

#### Scenario: Client sends joint position command
- **WHEN** a client sends `Request::SetJointPosition { joint, position }`
- **THEN** the agent publishes a `JointState` message to the configured control topic with the specified joint set to the requested position

#### Scenario: Client executes a predefined pose
- **WHEN** a client sends `Request::ExecutePose { name: "home" }`
- **THEN** the agent looks up the pose in config, constructs a `JointState` with all joint positions from the pose, and publishes it

#### Scenario: Control not configured
- **WHEN** a client sends a control command but no `[control]` section exists in config
- **THEN** the agent responds with `Response::Error` indicating control is not configured
