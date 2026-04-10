## ADDED Requirements

### Requirement: Four-tab layout
The TUI SHALL display four tabs: Topics, Nodes, Log, and Joints. The user SHALL switch tabs using number keys (1-4) or Tab key.

#### Scenario: Default view
- **WHEN** the TUI starts
- **THEN** it displays the Topics tab as the default view

#### Scenario: Switch tabs
- **WHEN** the user presses `2`
- **THEN** the TUI switches to the Nodes tab

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

### Requirement: Topics tab — master/detail with tree view
The Topics tab SHALL display a topic list on the left and a detail pane on the right. The detail pane renders the selected topic's latest `DynValue` as a collapsible tree.

#### Scenario: Topic list display
- **WHEN** the Topics tab is active and topic data is being received
- **THEN** the left pane shows each topic name with its current message rate in Hz

#### Scenario: Select a topic
- **WHEN** the user navigates to `/odom` with arrow keys and presses Enter
- **THEN** the right pane shows the latest `DynValue` for `/odom` as a tree

#### Scenario: Tree collapsed by default
- **WHEN** a topic's `DynValue::Struct` is displayed
- **THEN** nested structs are collapsed by default, showing only the field name and type

#### Scenario: Expand tree node
- **WHEN** the user presses the expand key on a collapsed struct field
- **THEN** the field expands to show its child fields

### Requirement: Nodes tab — node list with detail
The Nodes tab SHALL display a node list on the left and node detail on the right showing publishers, subscribers, and services for the selected node.

#### Scenario: Node list display
- **WHEN** the Nodes tab is active
- **THEN** the left pane shows all discovered ROS 2 node names

#### Scenario: Node detail
- **WHEN** a node is selected
- **THEN** the right pane shows the node's namespace, publishers, subscribers, and services

### Requirement: Log tab — full-width rosout viewer
The Log tab SHALL display `/rosout` messages in a full-width scrollable table with timestamp, severity level, and message columns.

#### Scenario: Log messages displayed
- **WHEN** the Log tab is active and `/rosout` messages are received
- **THEN** new messages appear at the top of the table with timestamp, level (coloured by severity), and message text

#### Scenario: Filter by severity
- **WHEN** the user activates the severity filter and selects "ERROR"
- **THEN** only ERROR-level messages are displayed

#### Scenario: Filter by node name
- **WHEN** the user types a node name in the filter field
- **THEN** only messages from that node are displayed

#### Scenario: Search log messages
- **WHEN** the user activates search and types a keyword
- **THEN** only messages containing that keyword are displayed

### Requirement: Display-rate throttling via tick loop
The TUI SHALL render at a fixed tick rate. Incoming topic data updates a latest-value store, and each render tick reads from this store. Messages arriving faster than the tick rate are naturally deduplicated.

#### Scenario: High-frequency topic
- **WHEN** `/joint_states` publishes at 50Hz and the TUI tick rate is 15 FPS
- **THEN** the TUI displays approximately 15 updates per second, always showing the most recent value

#### Scenario: Low-frequency topic
- **WHEN** `/robot_description` publishes once (latched)
- **THEN** the TUI displays the value and it persists across ticks until a new value arrives

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

### Requirement: Keyboard navigation
The TUI SHALL support keyboard navigation: arrow keys for list navigation, Enter to select, Tab to switch focus between panes, `q` to quit, `?` for help overlay.

#### Scenario: Quit
- **WHEN** the user presses `q`
- **THEN** the TUI exits cleanly, restoring the terminal

#### Scenario: Help overlay
- **WHEN** the user presses `?`
- **THEN** a help overlay displays showing all keybindings
