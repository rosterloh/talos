## ADDED Requirements

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

## MODIFIED Requirements

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
