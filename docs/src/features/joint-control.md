# Joint Control

Talos can display joint state and send joint commands when the agent is
configured for control.

## Joint Data

The Joints tab combines two sources:

- `/robot_description`, containing URDF XML.
- `/joint_states`, containing live joint positions, velocities, and efforts.

The URDF parser extracts non-fixed joints, parent and child links, joint type,
and limits. Live joint state data is merged onto those definitions.

## Display

The TUI shows each joint with current state and limit context. For joints with
limits, the detail pane displays the current position relative to the allowed
range.

If the URDF cannot be parsed, the Joints tab reports the error instead of
silently hiding the problem.

## Commands

When `[control]` is configured, clients can send:

- `SetJointPosition { joint, position }`
- `ExecutePose { name }`

The agent publishes command messages to the configured control topic. If control
is not configured, the agent returns an error for command requests.

## Pose Presets

Pose presets are configured in TOML under `[poses.<name>]`. The TUI displays
available poses and can request execution by name.
