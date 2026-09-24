# Node Introspection

Talos can list ROS 2 nodes through the agent.

The agent uses ROS 2 graph APIs to discover node names, namespaces, publishers,
subscribers, and services. Clients request this data with `ListNodes`.

## CLI

```bash
talos-cli list-nodes
```

## TUI

The Nodes tab shows discovered nodes on the left. Selecting a node displays its
namespace, publishers, subscribers, and services.

## Logger Levels

Talos can show and set a node's logger level through the node's
`<node>/get_logger_levels` and `<node>/set_logger_levels` services
(`rcl_interfaces/srv/GetLoggerLevels` and `SetLoggerLevels`). Nodes only serve
these when they opt in:

- rclcpp: `rclcpp::NodeOptions().enable_logger_service(true)`
- rclpy: `Node(..., enable_logger_service=True)`

Otherwise the agent reports that the logger services are not available.

Requests default to the node's own logger, whose name is the fully-qualified
node name without the leading `/` and with `/` replaced by `.` (`/ns/foo` is
`ns.foo`). Levels are `UNSET` (0, inherit the default), `DEBUG` (10), `INFO`
(20), `WARN` (30), `ERROR` (40) and `FATAL` (50).

Use `talos-cli log-level <node> [level]`, or `l` / `L` on the TUI Nodes tab.

## Limits

Node introspection reflects the ROS 2 graph state visible to the agent. Network
or ROS domain configuration issues outside Talos can affect what the agent sees.
