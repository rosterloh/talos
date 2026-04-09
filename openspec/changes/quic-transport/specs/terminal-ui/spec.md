## MODIFIED Requirements

### Requirement: Connection status indicator
The TUI SHALL display a connection status indicator in the top-right corner showing whether the agent is connected and which transport is in use.

#### Scenario: Connected via UDS
- **WHEN** the TUI has an active UDS connection to the agent
- **THEN** it displays a green "connected (uds)" indicator

#### Scenario: Connected via QUIC
- **WHEN** the TUI has an active QUIC connection to the agent
- **THEN** it displays a green "connected (quic)" indicator

#### Scenario: Disconnected
- **WHEN** the IPC connection is lost
- **THEN** it displays a red "disconnected" indicator

## ADDED Requirements

### Requirement: QUIC remote connection flag
The TUI SHALL accept a `--remote <addr:port>` CLI flag. When provided, the TUI SHALL connect to the agent via QUIC instead of UDS.

#### Scenario: Remote flag provided
- **WHEN** the user runs `talos-tui --remote 192.168.1.50:4433`
- **THEN** the TUI connects to the agent via QUIC at the specified address

#### Scenario: No remote flag
- **WHEN** the user runs `talos-tui` without `--remote`
- **THEN** the TUI connects via UDS to the default socket path

#### Scenario: Both socket and remote provided
- **WHEN** the user provides both `--socket` and `--remote`
- **THEN** the TUI SHALL report a conflict error and exit

### Requirement: TUI uses ProtocolClient trait
The TUI client SHALL use the `ProtocolClient` trait for all communication with the agent. The concrete implementation (UDS or QUIC) is selected based on CLI flags.

#### Scenario: UDS protocol client used
- **WHEN** the TUI starts without `--remote`
- **THEN** it creates a `UdsProtocolClient` and uses it for all agent communication

#### Scenario: QUIC protocol client used
- **WHEN** the TUI starts with `--remote 192.168.1.50:4433`
- **THEN** it creates a `QuicProtocolClient` and uses it for all agent communication

### Requirement: Explicit topic subscription on connect
After connecting to the agent, the TUI SHALL send `ListTopics` to discover available topics, then send `Subscribe` for all discovered topics to begin receiving data.

#### Scenario: Subscribe after connect
- **WHEN** the TUI connects and receives the topic list
- **THEN** it sends `Subscribe { topics: [all discovered topics] }` and begins receiving topic data

#### Scenario: Reconnect re-subscribes
- **WHEN** the TUI reconnects after a connection drop
- **THEN** it re-sends `ListTopics` followed by `Subscribe` to restore its subscriptions

### Requirement: Auto-reconnect for QUIC
The TUI SHALL auto-reconnect with backoff when a QUIC connection is lost, using the same reconnection strategy as UDS.

#### Scenario: QUIC connection lost
- **WHEN** the QUIC connection drops
- **THEN** the TUI retries connection with a 2-second delay, updating the status indicator to "disconnected" until reconnected
