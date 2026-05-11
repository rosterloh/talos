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

## Rates

The TUI keeps the latest value for each topic and renders on a fixed tick loop.
High-frequency topics are naturally deduplicated by display rate: the UI shows
the most recent received value at render time rather than drawing every incoming
message.

## CLI Echo

The CLI `echo` command subscribes to a single topic and prints received
`DynValue` trees until interrupted or until `--count` messages have been
printed.
