# Topic Observation

Talos observes ROS 2 topics through configured agent subscriptions.

The agent subscribes with concrete ROS 2 message types and converts received
messages into `DynValue`. Clients receive the generic tree and can render it
without ROS 2 type definitions.

## Current Message Support

The current agent supports these message families:

- `nav_msgs/msg/Odometry`
- `geometry_msgs/msg/Twist`
- `geometry_msgs/msg/PoseStamped`
- `std_msgs/msg/String`
- `sensor_msgs/msg/JointState`
- `sensor_msgs/msg/LaserScan`
- `sensor_msgs/msg/Imu`
- `rcl_interfaces/msg/Log`

Support for arbitrary ROS 2 message definitions is future work.

## QoS Profiles

Each subscription can specify a `qos` field in the config. Two profiles are
available:

| Profile       | Reliability | Durability | History      | Typical use        |
|---------------|-------------|------------|--------------|--------------------|
| `default`     | Reliable    | Volatile   | KeepLast (rclrs default) | Control, odometry  |
| `sensor_data` | BestEffort  | Volatile   | KeepLast(5)              | Laser, IMU, camera |

When `qos` is omitted the `default` profile is used, preserving existing
behavior.

High-rate sensor topics (`sensor_msgs/msg/LaserScan`, `sensor_msgs/msg/Imu`)
should generally use `sensor_data` QoS. A reliable publisher (e.g. a nav stack
node) is compatible with a best-effort subscriber, so `sensor_data` is safe to
use even when the publisher uses reliable QoS.

## Endpoint QoS

When a topic delivers no data, the cause is usually a QoS mismatch. The topic
detail pane lists every publisher and subscription on the selected topic
(including the agent's own) with its QoS, from the agent's
`GetTopicEndpoints` query. A subscriber that can't match a publisher gets a
red warning. These pairs are flagged, following the ROS 2 compatibility rules:

- a best-effort publisher with a reliable subscriber;
- a volatile publisher with a transient-local subscriber;
- a publisher deadline longer than the subscriber's.

A frequent case is a sensor publishing best-effort while the agent's
subscription uses the `default` (reliable) profile. Setting
`qos = "sensor_data"` on that subscription fixes it.

## Rates

The agent measures every bridged topic, whether or not any client is
subscribed. Once a second it closes a window of message and byte counts and
smooths the result with an exponential moving average. `GetTopicStats` returns:

- **Rate**: messages per second.
- **Bandwidth**: bytes per second, estimated from the decoded message size
  (roughly the CDR payload without length prefixes or padding).
- **Latency**: agent receive time minus `header.stamp`, for stamped types.
  This uses the agent's wall clock, so it has no meaning under simulated time.

These numbers come from the agent rather than from frames a client receives.
A client that falls behind has frames dropped by the agent, but the stats stay
accurate. The TUI polls the stats every second and shows them in the topic list
and detail pane, with a 60-second rate sparkline.
`talos-cli hz <topic>` prints them from the command line.

The TUI keeps the latest value for each topic and renders on a fixed tick loop.
High-frequency topics are naturally deduplicated by display rate: the UI shows
the most recent received value at render time rather than drawing every incoming
message.

## CLI Echo

The CLI `echo` command subscribes to a single topic and prints received
`DynValue` trees until interrupted or until `--count` messages have been
printed.
