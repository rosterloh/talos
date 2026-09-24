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

## Topic Rate

```bash
talos-cli hz /scan --duration 10
```

`hz` polls the agent's topic stats once a second and prints the rate,
bandwidth and header-stamp latency (`-` for unstamped types) of one topic:

```text
rate: 10.00 Hz  bandwidth: 11.3 KB/s  latency: 4.2 ms
```

The numbers are measured by the agent (see
[Topic Observation](../features/topic-observation.md#rates)), so the topic must
be in the agent's `[[subscriptions]]`. Without `--duration` it runs until
interrupted.

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

## JSON Output

The global `--json` flag switches to machine-readable output for piping into
tools such as `jq`:

| Command | Output |
|---|---|
| `list-topics` | one array of `{name, type_name, publisher_count, subscriber_count}` |
| `list-nodes` | one array of `{name, namespace, publishers, subscribers, services}` |
| `list-params`, `get-param` | one array of `{name, type, value}` |
| `echo` | one `{topic, stamp: {sec, nanosec}, data}` object per line per message |
| `hz` | one `{topic, rate_hz, bandwidth_bps, latency_ms}` object per line per second |

`set-param` ignores `--json`; errors still go to stderr with a non-zero exit.

```bash
talos-cli --json echo /joint_states --count 1 | jq '.data.position'
```

Message data and parameter values are converted to plain JSON rather than
Talos' internal tagged form:

- messages become objects with fields in message order (the type name is
  dropped), and arrays and sequences become arrays;
- `uint8[]`/`byte[]` fields and byte-array parameters become base64 strings;
- NaN and infinite floats become `null`, as JSON has no representation for
  them, and so do unset parameters and a missing `latency_ms`.

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
