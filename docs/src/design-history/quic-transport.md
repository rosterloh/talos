# QUIC Transport

Status: Accepted

Date: 2026-04-10

## Context

UDS works well for local workflows but does not handle the common robotics
setup where the robot and developer workstation are different machines.

Talos needed a remote transport that could support observation and control
across a local network while preserving UDS for low-overhead local use.

## Decision

QUIC was added as a feature-gated transport using `quinn`.

The protocol gained an explicit subscription model. Clients subscribe to topics
before receiving topic data, and the agent routes each topic only to subscribed
clients.

The session layer moved above raw transport details. Application code talks to
`ProtocolClient`; UDS and QUIC implement the same client behavior with different
stream layouts.

QUIC uses:

- A client-opened bidirectional stream for control requests and responses.
- Server-opened unidirectional streams for subscribed topic data.
- A `StreamHeader` once per topic stream.
- Repeated `TopicFrame` values after the header.

## Consequences

Remote clients can use `--remote <addr:port>` when built with the `quic`
feature. UDS remains the default path for local clients.

Per-topic streams give QUIC independent flow control for different topics and
avoid repeating topic metadata on every data frame.

The initial QUIC security model is intentionally local-network only: the agent
can generate a self-signed certificate and clients skip certificate
verification. Authentication and verified TLS remain future work.

The agent configuration changed to separate `[transport.uds]` and
`[transport.quic]` sections.
