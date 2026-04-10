## ADDED Requirements

### Requirement: QUIC remote connection flag
The CLI SHALL accept a `--remote <addr:port>` global flag. When provided, the CLI SHALL connect to the agent via QUIC instead of UDS.

#### Scenario: Remote flag provided
- **WHEN** the user runs `talos-cli --remote 192.168.1.50:4433 list-topics`
- **THEN** the CLI connects via QUIC at the specified address

#### Scenario: No remote flag
- **WHEN** the user runs `talos-cli list-topics` without `--remote`
- **THEN** the CLI connects via UDS to the default socket path

#### Scenario: Both socket and remote provided
- **WHEN** the user provides both `--socket` and `--remote`
- **THEN** the CLI SHALL report a conflict error and exit with non-zero status

### Requirement: CLI uses ProtocolClient trait
The CLI SHALL use the `ProtocolClient` trait for all communication with the agent. The concrete implementation (UDS or QUIC) is selected based on CLI flags.

#### Scenario: UDS protocol client used
- **WHEN** the CLI runs without `--remote`
- **THEN** it creates a `UdsProtocolClient`

#### Scenario: QUIC protocol client used
- **WHEN** the CLI runs with `--remote`
- **THEN** it creates a `QuicProtocolClient`

### Requirement: Echo command uses explicit subscribe
The `echo` subcommand SHALL send a `Subscribe` request for the specified topic before listening for data, instead of relying on receiving all broadcast data.

#### Scenario: Echo subscribes to single topic
- **WHEN** the user runs `talos-cli echo /odom`
- **THEN** the CLI sends `Subscribe { topics: ["/odom"] }` and only receives data for `/odom`

#### Scenario: Echo with count and subscribe
- **WHEN** the user runs `talos-cli echo /odom --count 5`
- **THEN** the CLI subscribes to `/odom`, prints 5 messages, and exits
