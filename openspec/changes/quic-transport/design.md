## Context

Talos v0.1 uses Unix domain sockets as the sole transport between the agent (running on a robot with ROS 2) and clients (TUI/CLI on a developer workstation). The transport layer in `talos-common` already defines `Transport`, `TransportClient`, and `TransportServer` traits, but only UDS implements them. The TUI and CLI bypass the traits entirely, using `UnixStream` directly.

The agent broadcasts all topic data to every connected client via a `tokio::broadcast` channel — clients have no way to filter what they receive. The protocol is a flat `Request`/`Response` enum pair serialised with length-prefixed bincode.

Adding QUIC transport requires: a new transport implementation, a protocol layer that abstracts stream topology differences between UDS (single multiplexed connection) and QUIC (multiple independent streams), an explicit subscription model, and changes to the agent's routing logic.

## Goals / Non-Goals

**Goals:**

- Remote observation and control of a ROS 2 robot from a developer workstation over WiFi/Ethernet
- QUIC transport using quinn with server-initiated unidirectional streams per subscribed topic
- Dual-mode agent serving both UDS and QUIC simultaneously
- Explicit subscribe/unsubscribe protocol for both transports (uniform model)
- Protocol layer that shields application code from transport-specific stream management
- Per-topic flow control and back-pressure over QUIC (no head-of-line blocking between topics)
- Insecure TLS mode for local-network use (self-signed certs, no verification)

**Non-Goals:**

- Authenticated/encrypted connections for untrusted networks (future work)
- Connection migration or 0-RTT reconnection (QUIC supports it, but not a v1 goal)
- Generic/dynamic ROS 2 message type support (orthogonal feature)
- ROS 2 service or action proxying over QUIC
- Backwards compatibility with v0.1 config format (clean break to `[transport.uds]`/`[transport.quic]`)

## Decisions

### 1. Protocol layer above transport (not unified transport trait)

**Choice:** Introduce a `session` module with `ProtocolClient`/`ProtocolServer` traits that sit above the raw transport, rather than trying to make the existing `Transport` trait accommodate QUIC streams.

**Alternatives considered:**
- *Extend Transport trait with stream methods*: Would require UDS to stub out `open_uni`/`accept_uni` methods it can never use. Leaky abstraction.
- *Separate code paths with no shared abstraction*: Duplicates all application-level protocol logic. Harder to test.

**Rationale:** The transport layer stays thin (raw bytes in/out). The protocol layer owns framing, subscription state, and the mapping between logical channels (control, data) and physical streams. UDS multiplexes on one connection; QUIC maps to separate streams. Application code sees neither.

### 2. Stream-per-topic with explicit subscribe (hybrid model)

**Choice:** Bidirectional stream 0 for control (request/response). Client sends `Subscribe { topics }` on stream 0. Agent opens one unidirectional stream per subscribed topic, sends a `StreamHeader` as the first frame, then continuous `TopicFrame` messages.

**Alternatives considered:**
- *Two streams (control + broadcast)*: Simpler but still mixes all topic data — a high-frequency topic starves others.
- *No subscribe, broadcast everything*: Current model. Wastes bandwidth, especially over a network link.

**Rationale:** Stream-per-topic gives independent QUIC flow control per topic, eliminates head-of-line blocking, and enables selective subscription. The subscribe model also benefits UDS — CLI echoing one topic no longer receives all topic data.

### 3. Uniform subscribe protocol for both transports

**Choice:** Both UDS and QUIC clients must explicitly subscribe to topics. No auto-subscribe for UDS.

**Alternatives considered:**
- *UDS auto-subscribes to all topics*: Preserves v0.1 behavior but creates two protocol flows to test and maintain.

**Rationale:** One protocol, tested once. The TUI change is trivial — send `Subscribe` after `ListTopics`. Simplifies the agent's routing logic.

### 4. TopicRouter replaces broadcast channel

**Choice:** A `TopicRouter` struct in the agent that maintains a `HashMap<ClientId, HashSet<String>>` of subscriptions and routes incoming ROS 2 data only to subscribers.

**Alternatives considered:**
- *Keep broadcast channel, filter client-side*: Still wastes serialisation and send costs for unwanted topics. Broadcast lag affects all topics.
- *Per-topic broadcast channels*: More channels to manage, complex lifecycle when clients subscribe/unsubscribe.

**Rationale:** Centralised routing is simpler to reason about and efficient — only subscribed clients pay the cost of serialisation and framing for a given topic.

### 5. quinn for QUIC, insecure mode via custom ServerCertVerifier

**Choice:** Use `quinn` (v0.11.x) as the QUIC implementation. Agent generates a self-signed certificate at startup using `rcgen`. Clients use a `SkipServerVerification` implementation of `rustls::client::danger::ServerCertVerifier` that accepts any certificate.

**Alternatives considered:**
- *s2n-quic*: AWS-maintained, but less community adoption in Rust ecosystem and less flexible TLS configuration.
- *Pre-shared keys*: Simpler but quinn doesn't expose PSK APIs directly, would require custom rustls config.

**Rationale:** quinn is the mature, tokio-native QUIC library. The insecure verifier is a well-documented pattern from quinn's own examples. Upgrade path to real TLS is straightforward — swap the verifier.

### 6. Lean data frames on QUIC streams

**Choice:** QUIC data streams use `TopicFrame { stamp: Timestamp, data: DynValue }` without topic name or type. A `StreamHeader { topic: String, type_name: String }` is sent once as the first frame on each new data stream, binding identity to the stream.

Over UDS, `Response::TopicData { topic, type_name, stamp, data }` continues to be used (topic name needed because all data shares one connection), but filtered by the agent to only send subscribed topics.

**Rationale:** Avoids repeating topic metadata on every message over QUIC. At 50Hz on a topic, that's meaningful bandwidth savings. UDS keeps the existing tagged format because there's no stream identity to carry the context.

### 7. Config format change

**Choice:** Agent config changes from `[transport]` with `socket_path` to:

```toml
[transport.uds]
socket_path = "/tmp/talos.sock"

[transport.quic]
bind_addr = "0.0.0.0:4433"
```

Both sections are optional. If only `[transport.uds]` is present, the agent serves UDS only (backwards-compatible behavior with format change). If both are present, the agent serves both.

Client-side transport selection is via CLI flags: `--socket` (UDS, default) or `--remote <addr:port>` (QUIC).

## Risks / Trade-offs

**[QUIC overhead for local use]** QUIC adds TLS handshake and UDP framing overhead compared to UDS. → Mitigation: UDS remains the default for local clients. QUIC is opt-in via `--remote`.

**[Stream lifecycle complexity]** Agent must track per-client stream handles and clean up on disconnect or unsubscribe. → Mitigation: Each client session is a self-contained task; dropping the task drops all streams. quinn handles QUIC-level cleanup.

**[Config breaking change]** `[transport]` → `[transport.uds]` breaks existing config files. → Mitigation: Clear error message on parse failure pointing to the new format. This is v0.1 → v0.2, pre-release software.

**[Insecure by default]** No certificate verification means any device on the local network can connect. → Mitigation: Acceptable for the stated use case (trusted local network). Document the security posture. Future work adds real TLS.

**[quinn compile time]** quinn + rustls adds significant compile time. → Mitigation: Only `talos-common` gains the dependency. Feature-gate QUIC support behind a `quic` cargo feature so builds without it remain fast.

**[Client max_concurrent_uni_streams]** The QUIC client must configure `max_concurrent_uni_streams` high enough for all subscribed topics; otherwise the server cannot open data streams. → Mitigation: Set a generous default (e.g., 64). Document the limit.
