# TUI

`talos-tui` is the interactive terminal client.

```bash
talos-tui
```

By default it connects over UDS at `/tmp/talos.sock`. With the `quic` feature:

```bash
talos-tui --remote 192.168.1.50:4433
```

## Views

The TUI has four tabs:

- Topics
- Nodes
- Log
- Joints

Use number keys `1` through `4` to switch tabs. `Tab` switches focus between
panes. `q` quits.

## Connection Behavior

The TUI reconnects when the agent connection is lost. After connecting, it asks
for the topic list and subscribes to discovered topics so it can receive live
data.

## Topics

The Topics tab shows topic names and current message rates. Selecting a topic
shows the latest `DynValue` tree for that topic.

## Nodes

The Nodes tab lists ROS 2 nodes and shows publishers, subscribers, and services
for the selected node.

## Logs

The Log tab displays `/rosout` entries with timestamp, severity, node, and
message fields. It supports severity, node, and text filtering.

## Joints

The Joints tab combines URDF joint definitions with live `/joint_states` data.
It can display limits, current position, velocity, effort, and configured poses.
When control is configured, it can send joint position and pose commands to the
agent.
