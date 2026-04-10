## ADDED Requirements

### Requirement: CLI connects to agent over IPC
The CLI SHALL connect to the talos agent via UDS and use the same bincode protocol as the TUI. The socket path SHALL be configurable via `--socket` flag or default to `/tmp/talos.sock`.

#### Scenario: Default connection
- **WHEN** the user runs `talos-cli list-topics` without a `--socket` flag
- **THEN** it connects to `/tmp/talos.sock`

#### Scenario: Custom socket path
- **WHEN** the user runs `talos-cli --socket /run/talos.sock list-topics`
- **THEN** it connects to `/run/talos.sock`

#### Scenario: Agent not running
- **WHEN** the CLI attempts to connect and no agent is listening
- **THEN** it prints an error message and exits with a non-zero status code

### Requirement: List topics command
The CLI SHALL provide a `list-topics` subcommand that queries the agent for all known topics and prints them in a table format.

#### Scenario: Topics available
- **WHEN** the user runs `talos-cli list-topics`
- **THEN** it prints a table with columns: topic name, message type, publisher count, subscriber count

#### Scenario: No topics
- **WHEN** the agent has no subscribed topics
- **THEN** it prints a message indicating no topics are available

### Requirement: List nodes command
The CLI SHALL provide a `list-nodes` subcommand that queries the agent for all discovered ROS 2 nodes.

#### Scenario: Nodes available
- **WHEN** the user runs `talos-cli list-nodes`
- **THEN** it prints a table with columns: node name, namespace

### Requirement: Echo topic command
The CLI SHALL provide an `echo` subcommand that prints live data from a specified topic.

#### Scenario: Echo a topic
- **WHEN** the user runs `talos-cli echo /odom`
- **THEN** it prints each received `DynValue` for `/odom` to stdout in a human-readable tree format

#### Scenario: Echo with count limit
- **WHEN** the user runs `talos-cli echo /odom --count 5`
- **THEN** it prints 5 messages and exits

#### Scenario: Topic not found
- **WHEN** the user runs `talos-cli echo /nonexistent`
- **THEN** it prints an error indicating the topic is not being bridged

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

### Requirement: CLI argument parsing with clap
The CLI SHALL use `clap` for argument parsing with derive macros. All subcommands SHALL include `--help` documentation.

#### Scenario: Help output
- **WHEN** the user runs `talos-cli --help`
- **THEN** it prints usage information listing all available subcommands
