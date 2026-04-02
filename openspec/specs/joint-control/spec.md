## ADDED Requirements

### Requirement: URDF parsing from /robot_description
The system SHALL parse the URDF XML received from the `/robot_description` topic to extract joint definitions including name, type, parent link, child link, and limits (lower, upper, effort, velocity).

#### Scenario: URDF received
- **WHEN** a `/robot_description` message containing valid URDF XML is received
- **THEN** the system parses it and extracts all non-fixed joints with their properties

#### Scenario: Invalid URDF
- **WHEN** the `/robot_description` payload is not valid URDF XML
- **THEN** the system logs a warning and the Joints tab displays an error message

#### Scenario: URDF updates
- **WHEN** a new `/robot_description` message is received with different content
- **THEN** the joint definitions are re-parsed and the Joints tab updates accordingly

### Requirement: Joint state merging
The Joints tab SHALL merge URDF joint definitions with live `/joint_states` data to display each joint's current position, velocity, and effort alongside its defined limits.

#### Scenario: Joint data merged
- **WHEN** URDF defines joint `shoulder_pan` with limits [-3.14, 3.14] and `/joint_states` reports position 0.5
- **THEN** the Joints tab shows `shoulder_pan` at position 0.5 with limit context

#### Scenario: Joint in URDF but not in joint_states
- **WHEN** URDF defines a joint that does not appear in `/joint_states`
- **THEN** the joint is listed with position, velocity, and effort shown as unknown/N/A

### Requirement: Limit-aware gauge visualisation
Each joint in the Joints tab detail pane SHALL display a horizontal gauge bar showing the current position relative to the joint's limits as defined in the URDF.

#### Scenario: Revolute joint gauge
- **WHEN** a revolute joint with limits [-3.14, 3.14] has position 0.0
- **THEN** the gauge shows a marker at the centre of the bar

#### Scenario: Joint at limit
- **WHEN** a joint's position equals its upper limit
- **THEN** the gauge shows the marker at the right end of the bar

### Requirement: Joint position command input
The Joints tab SHALL allow the user to input a target position for the selected joint. The command is sent to the agent which publishes it to the configured control topic.

#### Scenario: User sets joint position
- **WHEN** the user selects `shoulder_pan`, presses the edit key, enters `1.57`, and confirms
- **THEN** the TUI sends `Request::SetJointPosition { joint: "shoulder_pan", position: 1.57 }` to the agent

#### Scenario: Position outside limits
- **WHEN** the user enters a position that exceeds the URDF-defined limits
- **THEN** the TUI SHALL display a warning and clamp the value to the nearest limit before sending

### Requirement: Predefined pose execution
The Joints tab SHALL display a list of predefined poses (from agent config) and allow the user to execute them.

#### Scenario: Pose list displayed
- **WHEN** the Joints tab is active
- **THEN** the left pane shows available poses below the joint list

#### Scenario: Execute pose
- **WHEN** the user selects the `home` pose and presses the execute key
- **THEN** the TUI sends `Request::ExecutePose { name: "home" }` to the agent

### Requirement: Joint detail panel
The Joints tab detail pane SHALL show the selected joint's type, parent/child links, current position/velocity/effort, limits, and the gauge visualisation.

#### Scenario: Joint selected
- **WHEN** the user selects `elbow` in the joint list
- **THEN** the detail pane shows: joint type (revolute/prismatic), parent link, child link, position gauge, velocity, effort, and limit values
