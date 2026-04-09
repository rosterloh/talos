## 1. Protocol Messages & Types

- [ ] 1.1 Add `TopicSub`, `TopicFrame`, and `StreamHeader` structs to `talos-common/src/protocol/types.rs` with serde derives
- [ ] 1.2 Add `Subscribe`, `Unsubscribe` variants to `Request` enum and `Subscribed`, `Unsubscribed` variants to `Response` enum in `talos-common/src/protocol/messages.rs`
- [ ] 1.3 Add round-trip serialisation tests for all new message types and structs

## 2. Config Changes

- [ ] 2.1 Refactor `AgentConfig` in `talos-common/src/config.rs`: change `transport: TransportConfig` to `transport: TransportSettings` with optional `uds` and `quic` sub-sections
- [ ] 2.2 Add `QuicTransportConfig` struct with `bind_addr`, optional `cert_path`, optional `key_path`
- [ ] 2.3 Update `TransportConfig` in `talos-common/src/transport/mod.rs` to an enum (`Uds { socket_path }` / `Quic { addr, ... }`) or keep separate config types
- [ ] 2.4 Update `AgentConfig::load_or_default` and `into_transport_config` for new structure
- [ ] 2.5 Update config tests and add tests for dual-transport and QUIC-only config parsing

## 3. QUIC Transport

- [ ] 3.1 Add `quinn`, `rustls`, and `rcgen` dependencies to `talos-common/Cargo.toml` behind a `quic` feature flag
- [ ] 3.2 Implement self-signed certificate generation using `rcgen` in a new `talos-common/src/transport/certs.rs` module
- [ ] 3.3 Implement `QuicTransport` server: create quinn `Endpoint`, configure with self-signed cert or loaded cert, bind and accept connections
- [ ] 3.4 Implement `QuicTransport` client: create quinn `Endpoint` with `SkipServerVerification`, connect to remote address, configure `max_concurrent_uni_streams(64)`
- [ ] 3.5 Write integration tests for QUIC connect/accept with self-signed certs

## 4. Protocol Session Layer

- [ ] 4.1 Define `ProtocolClient` trait in new `talos-common/src/session/mod.rs` with `request()`, `subscribe()`, `unsubscribe()`, `recv_data()` methods
- [ ] 4.2 Implement `UdsProtocolClient` in `talos-common/src/session/uds.rs` — single framed connection, demux control responses from TopicData, filter by subscription set
- [ ] 4.3 Implement `QuicProtocolClient` in `talos-common/src/session/quic.rs` — control on bidi stream 0, accept server-initiated uni streams, read StreamHeader, select across data streams
- [ ] 4.4 Define `ProtocolSession` server-side struct in `talos-common/src/session/server.rs` with subscription tracking and transport-aware data push (tagged TopicData for UDS, TopicFrame on uni streams for QUIC)
- [ ] 4.5 Write unit tests for UDS protocol client demux logic (interleaved control and data)

## 5. Agent: TopicRouter & Dual Listeners

- [ ] 5.1 Implement `TopicRouter` in `talos-agent/src/router.rs` — per-client subscription tracking, route topic data to subscribers only
- [ ] 5.2 Refactor `talos-agent/src/server.rs` to use `ProtocolSession` and `TopicRouter` instead of `broadcast::channel`
- [ ] 5.3 Add QUIC accept loop in `talos-agent/src/server.rs` — spawn per-client task with `ProtocolSession` for QUIC connections
- [ ] 5.4 Update `talos-agent/src/main.rs` to spawn both UDS and QUIC listeners based on config
- [ ] 5.5 Handle `Subscribe`/`Unsubscribe` requests in `handle_request` — update client's subscription set in `TopicRouter`
- [ ] 5.6 Update `talos-agent/src/bridge.rs` to send topic data through `TopicRouter` instead of broadcast channel

## 6. TUI Client Refactor

- [ ] 6.1 Add `--remote <addr:port>` CLI flag to `talos-tui` (conflicts with `--socket`)
- [ ] 6.2 Refactor `talos-tui/src/client.rs` to use `ProtocolClient` trait instead of direct `UnixStream`
- [ ] 6.3 Add subscribe flow: after `ListTopics`, send `Subscribe` for all discovered topics
- [ ] 6.4 Update reconnect logic to re-subscribe on reconnection
- [ ] 6.5 Update connection status indicator to show transport type ("connected (uds)" / "connected (quic)")

## 7. CLI Client Refactor

- [ ] 7.1 Add `--remote <addr:port>` global CLI flag to `talos-cli` (conflicts with `--socket`)
- [ ] 7.2 Refactor `talos-cli/src/main.rs` to use `ProtocolClient` trait instead of direct `UnixStream`
- [ ] 7.3 Update `echo` command to send `Subscribe` for the target topic before listening
- [ ] 7.4 Update `list-topics` and `list-nodes` to work through `ProtocolClient`

## 8. Integration Testing

- [ ] 8.1 Add integration test: UDS client subscribes to subset of topics, verifies only subscribed data is received
- [ ] 8.2 Add integration test: QUIC client connects, subscribes, receives data on uni streams
- [ ] 8.3 Add integration test: dual-mode agent serves UDS and QUIC clients simultaneously
- [ ] 8.4 Add integration test: client unsubscribe closes QUIC data stream
