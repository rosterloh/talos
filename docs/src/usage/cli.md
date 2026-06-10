# CLI

The CLI binary is named `talos-cli`.

## List Topics

```bash
talos-cli list-topics
```

This asks the agent for the current topic list and prints topic name, type,
publisher count, and subscriber count.

## List Nodes

```bash
talos-cli list-nodes
```

This asks the agent for discovered ROS 2 nodes and prints their names and
namespaces.

## Echo Topic Data

```bash
talos-cli echo /joint_states --count 5
```

`echo` subscribes to the requested topic before waiting for data. A count of
zero means unlimited output.

## Parameters

View and set ROS 2 parameters on any node in the graph. Pass the
fully-qualified node name (e.g. `/talos_agent`).

List a node's parameters with their current values:

```bash
talos-cli list-params /talos_agent
```

Get specific parameter values:

```bash
talos-cli get-param /talos_agent use_sim_time rate
```

Set a parameter (the value type is inferred — `true`/`false` for bool, integers,
floats, `[1, 2, 3]` for arrays, otherwise a string):

```bash
talos-cli set-param /talos_agent rate 50.0
```

`set-param` exits non-zero if the node rejects the change and prints the reason.
The agent reaches each node through the standard `rcl_interfaces` parameter
services, so the target node must be running.

## Socket Selection

The default transport is UDS:

```bash
talos-cli --socket /tmp/talos.sock list-topics
```

With the `quic` feature enabled, `--remote` selects QUIC:

```bash
talos-cli --remote 192.168.1.50:4433 list-topics
```

`--socket` and `--remote` are mutually exclusive. If the binary was compiled
without QUIC support, `--remote` returns an error.
