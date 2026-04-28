# Topic Observation

Talos observes ROS 2 topics through configured agent subscriptions.

The agent subscribes with concrete ROS 2 message types and converts received
messages into `DynValue`. Clients receive the generic tree and can render it
without ROS 2 type definitions.

## Current Message Support

The current agent supports these message families:

- `nav_msgs/msg/Odometry`
- `geometry_msgs/msg/Twist`
- `std_msgs/msg/String`
- `sensor_msgs/msg/JointState`
- `rcl_interfaces/msg/Log`

Support for arbitrary ROS 2 message definitions is future work.

## Rates

The TUI keeps the latest value for each topic and renders on a fixed tick loop.
High-frequency topics are naturally deduplicated by display rate: the UI shows
the most recent received value at render time rather than drawing every incoming
message.

## CLI Echo

The CLI `echo` command subscribes to a single topic and prints received
`DynValue` trees until interrupted or until `--count` messages have been
printed.
