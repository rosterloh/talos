# Agent Config

The agent reads TOML configuration from `talos-agent.toml` or the path passed to
`--config`.

## Minimal UDS Config

```toml
[transport.uds]
socket_path = "/tmp/talos.sock"
```

## QUIC Config

QUIC requires building with the `quic` feature:

```toml
[transport.quic]
bind_addr = "0.0.0.0:4433"
# cert_path = "/path/to/cert.der"
# key_path = "/path/to/key.der"
```

If `cert_path` and `key_path` are omitted, the agent generates a self-signed
certificate at startup.

## Dual Transport Config

UDS and QUIC can be enabled together:

```toml
[transport.uds]
socket_path = "/tmp/talos.sock"

[transport.quic]
bind_addr = "0.0.0.0:4433"
```

At least one transport must be configured for the agent to serve clients.

## Topic Subscriptions

The agent subscribes to configured topics with compiled-in ROS 2 message types.
An optional `qos` field selects the QoS profile; when omitted it defaults to
`"default"` (Reliable, Volatile, KeepLast — depth inherited from rclrs default).

```toml
[[subscriptions]]
topic = "/odom"
type = "nav_msgs/msg/Odometry"

[[subscriptions]]
topic = "/cmd_vel"
type = "geometry_msgs/msg/Twist"

[[subscriptions]]
topic = "/robot_description"
type = "std_msgs/msg/String"

[[subscriptions]]
topic = "/joint_states"
type = "sensor_msgs/msg/JointState"

[[subscriptions]]
topic = "/rosout"
type = "rcl_interfaces/msg/Log"

# Sensor topics: use sensor_data QoS (BestEffort, Volatile, KeepLast 5)
[[subscriptions]]
topic = "/scan"
type = "sensor_msgs/msg/LaserScan"
qos = "sensor_data"

[[subscriptions]]
topic = "/imu/data"
type = "sensor_msgs/msg/Imu"
qos = "sensor_data"

[[subscriptions]]
topic = "/pose"
type = "geometry_msgs/msg/PoseStamped"

# Camera frames: the agent forwards already-encoded image bytes verbatim
# ("dumb relay") without decoding them. Publish a CompressedImage topic, e.g.
# via image_transport's republish node, and use sensor_data QoS.
[[subscriptions]]
topic = "/camera/image_raw/compressed"
type = "sensor_msgs/msg/CompressedImage"
qos = "sensor_data"
```

Unknown message types are skipped by the agent.

> **Camera streams (prototype).** `sensor_msgs/msg/CompressedImage` is forwarded
> as `{ header, format, data }` where `data` is the opaque compressed payload
> (JPEG/PNG/etc.) — the agent never decodes it. This works over the existing
> reliable topic pipeline, which is fine for compressed frames within the 16 MiB
> frame cap. Raw `sensor_msgs/msg/Image` and a moq-style per-frame-group stream
> with stale-frame dropping are intentionally **not** part of this prototype; see
> the roadmap.

## Joint Control

Joint control is optional:

```toml
[control]
method = "topic"
topic = "/joint_commands"
type = "sensor_msgs/msg/JointState"
```

Named poses are stored under `poses`:

```toml
[poses.home]
shoulder_pan = 0.0
shoulder_lift = -1.57
elbow = 1.57
```

If control is not configured, joint command requests return an error.
