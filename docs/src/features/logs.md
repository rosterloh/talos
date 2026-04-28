# Logs

The Log view is based on `/rosout`.

To use it, configure the agent to subscribe to:

```toml
[[subscriptions]]
topic = "/rosout"
type = "rcl_interfaces/msg/Log"
```

The agent converts log messages into `DynValue`, including fields such as
severity, node name, timestamp, and message text.

## TUI Filtering

The TUI Log tab displays messages in a table and supports:

- Severity filtering.
- Node filtering.
- Text search.

The UI keeps log interaction local to the client. The agent still routes only
the `/rosout` topic data to clients that subscribed to it.
