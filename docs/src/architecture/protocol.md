# Protocol

The Talos protocol is defined in `talos-common`.

The `protocol` module owns schema and framing types: request/response enums,
topic data types, `DynValue`, stream headers, and the length-prefixed bincode
codec. The application-facing client API lives in `session`, while socket and
stream setup lives in `transport`.

Control messages use a four-byte big-endian length prefix followed by a
bincode-encoded payload. The codec is implemented around Tokio async I/O and is
used by both UDS and QUIC control paths.

## Requests

Clients send `Request` values:

- `ListTopics`
- `ListNodes`
- `ListPoses`
- `Subscribe { topics }`
- `Unsubscribe { topics }`
- `SetJointPosition { joint, position }`
- `ExecutePose { name }`
- `ListParameters { node }`
- `GetParameters { node, names }`
- `SetParameter { node, name, value }`
- `GetTopicStats`

## Responses

The agent replies with `Response` values:

- `TopicList`
- `NodeList`
- `PoseList`
- `Subscribed`
- `Unsubscribed`
- `TopicData`
- `Parameters { node, parameters }`
- `ParameterSet { node, name, successful, reason }`
- `Ok`
- `Error`
- `TopicStats` (list of `{ topic, rate_hz, bandwidth_bps, latency_ms }`)

UDS carries control responses and topic data on the same framed connection.
Because that connection carries data for multiple topics, UDS topic frames keep
the topic name and type in each `TopicData` response.

QUIC uses a bidirectional stream for control and server-initiated
unidirectional streams for topic data. A topic stream starts with a
`StreamHeader` containing the topic and type name, then carries `TopicFrame`
values with timestamp and data.

## Compatibility

bincode encodes each enum variant by its index, so new `Request` and
`Response` variants are only ever appended. A test in `protocol/tests.rs`
enforces the existing indices.

When an agent receives a request frame it can't decode, typically a variant
added by a newer client, it replies with `Error` and keeps the connection
open. Agents up to v1.0.0 instead close the connection. To cope with that,
the TUI sends a single `GetTopicStats` after connecting. If the connection
closes on it, the TUI stops sending `GetTopicStats` to that agent until it is
restarted.

## DynValue

`DynValue` is the generic representation clients receive for ROS 2 message
payloads. It can represent:

- Booleans.
- Signed and unsigned integer primitives.
- `f32` and `f64`.
- Strings.
- Byte arrays.
- Arrays of other `DynValue` values.
- Structs with an ordered field list.

The agent handles typed ROS 2 subscriptions and conversions. Clients render or
print the resulting `DynValue` tree without knowing the original ROS 2 message
type at compile time.

## Frame Limits

Oversized frames are rejected by the codec. The default maximum frame size is
intended to protect the agent and clients from unbounded buffering.
