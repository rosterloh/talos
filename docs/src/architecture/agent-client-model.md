# Agent And Clients

Talos uses one robot-side agent and one or more developer-side clients.

## Agent

`talos-agent` is responsible for:

- Loading `talos-agent.toml`.
- Starting configured UDS and QUIC listeners.
- Creating and spinning the ROS 2 node.
- Subscribing to configured ROS 2 topics.
- Converting supported ROS 2 messages into `DynValue`.
- Answering graph queries for topics and nodes.
- Publishing joint commands when control is configured.
- Tracking client subscriptions and routing topic data.

The agent runs the ROS 2 bridge and IPC servers as async tasks. ROS 2 callback
paths send converted topic data through a channel into the router, avoiding
blocking the callback on client I/O.

## Clients

`talos-cli` and `talos-tui` both use the `ProtocolClient` trait from
`talos-common`.

Clients send control requests such as `ListTopics`, `ListNodes`, `Subscribe`,
`Unsubscribe`, `SetJointPosition`, and `ExecutePose`. Topic data is received
separately through the session interface.

## Subscription Model

Clients do not receive all topic data automatically. They must subscribe to the
topics they want.

This matters for remote links and high-rate topics. A CLI command echoing one
topic should not pay the cost of receiving every other topic. The TUI normally
subscribes to all discovered topics because its purpose is broad observation.

## Client Lifetime

Each client connection gets a router entry with its own subscription set. When
the client disconnects, the agent deregisters it and drops the routing state for
that session.
