# TUI

`talos-tui` is the interactive terminal client.

```bash
talos-tui
```

By default it connects over UDS at `/tmp/talos.sock`. With the `quic` feature:

```bash
talos-tui --remote 192.168.1.50:4433
```

## Views

The TUI has five tabs:

- Topics
- Nodes
- Log
- Joints
- Params

Use number keys `1` through `5` to switch tabs. `Tab` or `Shift-Tab` switches
focus between panes; `Esc` returns to the list. `r` refreshes the topic and node
lists immediately. `q` or `Ctrl-C` quits outside an editor or modal.

The navigator is bounded to 28–42 columns. Below 90 columns, only the focused
pane is shown: `Enter` opens a topic or node, `Tab` switches panes, and `Esc`
returns. Selection and viewport offsets survive pane changes and resizing;
offsets clamp when content or the viewport changes. Short terminals show only
the selected joints/poses list; `j` and `o` switch between them.

The footer shows actions for the current context. Press `?` for contextual
help, scroll it with arrows or `PageUp`/`PageDown`, and close it with `Esc` or
`?`. Other keys do not dismiss help. Node details, joint details, endpoint/QoS
information and expanded log messages also scroll with arrows or page keys.

Ordinary data uses the terminal's foreground and background. Cyan marks focus;
green denotes success, amber pending operations or warnings, and red errors.
Selected rows have a contrasting background and a marker: `>` for keyboard
focus, `·` for an inactive pane's retained selection. Focused pane headings also
carry `>`, so focus does not rely on color. Set a nonempty `NO_COLOR` to remove
colors while retaining markers, bold and reverse selection.

## Filtering

Press `/` to filter the list on the current tab: topic names on Topics, node
names (including namespace) on Nodes, parameter names on Params, and message
text on Log. Matching is a case-insensitive substring and the list updates as
you type. While the prompt is open every key goes to it: `←`/`→` move the
cursor, `Backspace` deletes, `Ctrl-U` clears, `Enter` applies and `Esc`
restores the previous filter. Applying an empty prompt clears the filter.

The active filter is shown in the pane title (on the Log tab, in the filter
bar), and the selection moves to stay inside the filtered list.

## Connection Behavior

The TUI reconnects when the agent connection is lost. After connecting, it asks
for the topic list and subscribes to all discovered topics by default so it can
receive live data. While connected, it re-fetches the topic and node lists
every 2 seconds, so nodes and topics that appear or go away show up without a
reconnect; the selection stays on the same item by name. If you manually toggle
topic subscriptions, those choices are kept in the client state and re-applied
after reconnect instead of subscribing to every topic again. Topics that disappear from the latest agent topic list are
removed from the Topics pane and reconnect request until the agent advertises
them again, which also drops their cached sample/count history.

## Topics

The Topics tab shows topic names and current message rates. Selecting a topic
shows the latest `DynValue` tree for that topic. The subscription, topic name,
and rate occupy separate columns; unavailable/stale rates show `—`.
Press `s` anywhere in the
Topics tab to subscribe or unsubscribe the selected topic. The list shows the
current subscription state, including pending changes and request errors.

Open the detail pane with `Enter` or `Tab`. `Up`/`Down` selects visible payload
fields. `Right` expands the selected structure or array, `Left` collapses it
(or selects its parent when already collapsed), and `Enter` toggles it.
Nested structures and individual array elements are navigable. Selection is
tracked by field path across refreshed samples, and expansion survives catalog
refreshes. A vanished field falls back to a visible ancestor or the first row.

The payload takes priority over metadata. Press `i` to open the scrollable
endpoint/QoS view, then `i` again to return to the payload. Subscription status
and errors remain in the summary. Rate history appears when there is room.

Before you customize anything, newly discovered topics inherit the default
behavior and are subscribed automatically. After you manually subscribe or
unsubscribe any topic, the TUI switches to using your explicit desired set:
newly discovered topics stay unsubscribed until you opt into them with `s`.

If a manual subscribe or unsubscribe request fails, the TUI keeps your desired
choice in client state, shows the error on that topic, and retries the desired
state after reconnect.

## Nodes

The Nodes tab lists ROS 2 nodes and shows publishers, subscribers, and services
for the selected node. Fully qualified names distinguish nodes with identical
names in different namespaces. Detail headings include category counts; open
the pane and scroll to reach long publisher, subscriber, and service lists.

Press `l` to load the selected node's logger level into the detail pane and
`L` to set it to the next level (DEBUG, INFO, WARN, ERROR, FATAL, then back to
DEBUG). See [Logger Levels](../features/node-introspection.md#logger-levels).

## Logs

The Log tab displays `/rosout` entries with timestamp, severity, node, and
message fields. Message width takes priority: below 80 columns the timestamp
column is hidden, and below 55 columns the node column is hidden. The expanded
view always includes timestamp, severity and node.

Entries arrive newest first in `LIVE` mode. Navigating with `Up`/`Down` pauses
following and preserves the selected entry as new messages arrive. `Enter`
opens a wrapped, scrollable snapshot of the full message; `Enter` or `Esc`
closes it. `Space` pauses or resumes at the newest entry, with an explicit
`LIVE`/`PAUSED` indicator. `f` cycles severity and `/` searches message text.
The ring buffer retains 10,000 entries: an evicted selection clamps to the
oldest remaining match, but an open message snapshot survives eviction.

## Joints

The Joints tab combines URDF joint definitions with live `/joint_states` data.
It can display limits, current position, velocity, effort, and configured poses.
When control is configured, it can send joint position and pose commands to the
agent.

Operational details lead with measured position, requested target, command
status and limits; parent/child links follow. Revolute/continuous joints use
radians, rad/s and N·m; prismatic joints use meters, m/s and N. Missing or
nonfinite measurements show `No telemetry` and no position gauge. Requested
targets never replace measured values.

Use `e` to edit a joint target, `Enter` to send it, and `Esc` to cancel. Inputs
must be finite numbers and joint limits clamp commands with a visible note.
`o` selects poses; `x` asks for confirmation before executing one. Pending
commands block another joint/pose command until the reply. `published` means
the agent acknowledged publication, not that physical motion completed. Errors
and an unknown outcome after connection loss remain visible.

## Params

The Params tab lists the nodes discovered in the graph in the left pane. Select
a node and press `Enter` to load its parameters into the right pane, where each
parameter is shown with its current value and type.

Switch focus to the parameter list with `Tab`, select a parameter, and press
`e` to edit it. Loading a node also opens the parameter pane. Name, type and
current value use width-aware columns. The editor keeps the parameter's type
and current value visible beside the entered value. `Left`/`Right` moves the
cursor and `Ctrl-U` clears the input.
Long input scrolls horizontally to keep the cursor visible. `PageUp`/`PageDown`
scroll long reply/error messages, including while the editor is open.

Press `Enter` to apply; the existing parameter type is preserved, and input
that would change it is rejected locally. Pending requests block duplicate
submission. A successful reply closes the editor; a rejection or transport
error preserves the entered value and explanation for correction. The list
refresh does not hide the set result. `Esc` closes the editor but cannot cancel
an already submitted request; reopening the same failed edit retains its value.
