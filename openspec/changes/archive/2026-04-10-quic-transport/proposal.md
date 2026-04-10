## Why

Talos currently communicates over Unix domain sockets only, meaning the TUI and CLI must run on the same machine as the agent (or share a filesystem). For robotics workflows the developer workstation and the robot are separate machines connected over WiFi or Ethernet. Adding QUIC transport enables remote observation and control without requiring filesystem sharing, while retaining UDS for zero-overhead local use.

## What Changes

- **New protocol layer** (`session` module in `talos-common`) that abstracts over transport-specific details, providing a uniform `ProtocolClient`/`ProtocolServer` interface to application code
- **Explicit topic subscription model**: clients send `Subscribe`/`Unsubscribe` requests instead of receiving all topic data automatically. The agent routes data only to subscribers, reducing wasted bandwidth on both UDS and QUIC
- **QUIC transport** (`transport::quic` module in `talos-common`) using `quinn` with insecure TLS (self-signed certs, no verification) for local-network use. Stream 0 carries control (request/response), agent-initiated unidirectional streams carry per-topic data
- **Dual-mode agent**: serves both UDS and QUIC simultaneously. Local clients connect via UDS, remote clients via QUIC
- **`TopicRouter`** replaces the broadcast channel in the agent, managing per-client subscription sets and routing topic data only to subscribers
- **Client transport selection**: `--remote <addr:port>` flag on TUI and CLI selects QUIC; default remains UDS
- **Leaner data frames**: topic data streams use `TopicFrame { stamp, data }` instead of repeating topic name and type on every message; stream identity carries that context

## Capabilities

### New Capabilities

- `quic-transport`: QUIC transport implementation using quinn, insecure TLS mode, server-initiated unidirectional streams for topic data, self-signed certificate generation
- `protocol-session`: Protocol layer that abstracts over UDS and QUIC transports, providing uniform ProtocolClient/ProtocolServer traits, subscription management, and stream multiplexing/demultiplexing

### Modified Capabilities

- `ipc-protocol`: New message types (Subscribe, Unsubscribe, Subscribed, Unsubscribed, TopicFrame, StreamHeader). TopicData still used over UDS but filtered by subscription set
- `ros2-bridge`: Agent serves both UDS and QUIC listeners. Broadcast channel replaced by TopicRouter with per-client subscription tracking
- `terminal-ui`: Client refactored to use ProtocolClient trait. Sends Subscribe after ListTopics. New `--remote` flag for QUIC connections
- `cli-interface`: Client refactored to use ProtocolClient trait. Echo command sends Subscribe for single topic. New `--remote` flag

## Impact

- **Dependencies**: New crates `quinn`, `rustls`, `rcgen` (self-signed cert generation). Existing `tokio`, `bincode`, `serde` unchanged
- **Protocol**: New Request/Response variants are additive. Existing UDS clients must adopt subscribe flow (minor change). Wire format (bincode + length prefix) unchanged for control channel
- **Config**: Agent config gains `[transport.quic]` section (bind address, optional cert/key paths). Existing `[transport]` section becomes `[transport.uds]` (**BREAKING** config format change)
- **Build**: `quinn`/`rustls` compile without ROS 2 environment, so `talos-common`, `talos-tui`, and `talos-cli` remain ROS 2-free
- **Network**: Agent listens on a configurable UDP port (default 4433) for QUIC. Firewall rules may need updating on robot
