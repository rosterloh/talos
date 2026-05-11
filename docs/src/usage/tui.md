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
for the topic list and subscribes to all discovered topics by default so it can
receive live data. If you manually toggle topic subscriptions, those choices are
kept in the client state and re-applied after reconnect instead of subscribing
to every topic again.

## Topics

The Topics tab shows topic names and current message rates. Selecting a topic
shows the latest `DynValue` tree for that topic. Press `s` with the topic list
focused to subscribe or unsubscribe the selected topic. The list shows the
current subscription state, including pending changes and request errors.

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
