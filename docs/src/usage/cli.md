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
