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

## Limits

Node introspection reflects the ROS 2 graph state visible to the agent. Network
or ROS domain configuration issues outside Talos can affect what the agent sees.
