# Source Map

This page maps common Talos changes to the first source files to inspect.

## Shared Boundaries

`talos-common` is shared by the agent, CLI, and TUI. It does not depend on
ROS 2.

| Area | Owns | Start here |
| --- | --- | --- |
| `protocol` | Wire schema, `DynValue`, request/response enums, topic frame types, and length-prefixed bincode framing. | `talos-common/src/protocol/` |
| `session` | Application-facing client API used by `talos-cli` and `talos-tui`. | `talos-common/src/session/mod.rs` |
| `transport` | Endpoint setup, listeners, raw connections, UDS plumbing, QUIC plumbing, and certificates. | `talos-common/src/transport/` |
| `config` | Shared TOML configuration structs and parsing. | `talos-common/src/config.rs` |
| `urdf` | Shared URDF joint extraction helpers. | `talos-common/src/urdf.rs` |

The intended dependency direction is:

```text
CLI/TUI application code -> session::ProtocolClient -> protocol + transport
agent server code        -> protocol + transport + router/bridge
ROS 2 conversions        -> talos-agent only
```

If a client feature needs to talk to the agent, add or use a protocol request
through `ProtocolClient`. Do not make CLI or TUI code open transport endpoints
directly unless the change is explicitly about transport plumbing.

## Common Tasks

### Add A Request Or Response

Start in `talos-common/src/protocol/messages.rs`. Add the schema variant there,
then update protocol tests in `talos-common/src/protocol/tests.rs` if framing or
serialization coverage should change.

After the shared schema exists, wire behavior through the agent request dispatch
in `talos-agent/src/server.rs`, and call it from clients through the
`ProtocolClient::request` API in `talos-common/src/session/mod.rs`.

Update CLI or TUI code only after the protocol and agent behavior are defined.

### Add A ROS Message Conversion

Start in `talos-agent/src/conversions.rs`. ROS 2 message typing and conversion
belong in the agent because `talos-common`, `talos-cli`, and `talos-tui` must
remain free of ROS 2 dependencies.

Then update subscription setup in `talos-agent/src/bridge.rs` so the configured
message type is subscribed and converted into `DynValue`.

If the new message type should be configurable, update
`docs/src/configuration/agent-config.md` and the config examples that mention
supported message types.

### Change TUI Topic Behavior

Start in `talos-tui/src/state.rs` for subscription intent, topic catalog state,
and reconnect behavior. Use `talos-tui/src/client.rs` for session command
wiring, and `talos-tui/src/ui/topics_tab.rs` for rendering and input behavior
specific to the Topics tab.

The TUI should continue to communicate through `ProtocolClient`; topic behavior
changes should not bypass the session layer to talk to transports directly.

### Change Transport Framing

Start in `talos-common/src/protocol/codec.rs` for the length-prefixed bincode
frame format and frame limits.

Then inspect each transport path that uses the codec:

- `talos-common/src/session/uds.rs`
- `talos-common/src/transport/uds.rs`
- `talos-common/src/session/quic.rs`
- `talos-common/src/transport/quic.rs`

Keep schema changes in `protocol/messages.rs` or `protocol/types.rs`; keep
endpoint/listener changes in `transport`.

### Change Config

Start in `talos-common/src/config.rs`. Add or change the shared config model
there, then update `talos-common/src/config_tests.rs` for TOML parsing behavior.

Agent behavior that consumes config usually belongs in `talos-agent/src/main.rs`
or `talos-agent/src/bridge.rs`. User-facing config documentation belongs in
`docs/src/configuration/agent-config.md`.

## When In Doubt

Use the narrowest layer that owns the concept:

- Schema or frame encoding belongs in `protocol`.
- Client application calls belong in `session`.
- Socket, stream, listener, certificate, or endpoint setup belongs in
  `transport`.
- ROS 2 type knowledge belongs in `talos-agent`.
- TUI presentation and interaction state belongs in `talos-tui`.
