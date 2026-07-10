# Changelog

All notable changes to Talos are recorded here.

Future changes should be added under `[Unreleased]`. The release workflow moves
those entries into the versioned section when a release is created.

## [Unreleased]

### Added

- Runtime dynamic-message fallback in `talos-agent`: topics whose message type
  has no compiled-in converter are now subscribed via `rclrs`'s
  `DynamicMessage` introspection support and rendered into the existing
  `DynValue` tree, instead of being skipped. Scalars, nested messages, fixed
  arrays, unbounded and bounded sequences (including sequences of nested
  messages, e.g. `PoseArray.poses`), and arrays of nested messages are covered;
  only wide/long-double/wstring scalars currently fall back to a debug
  representation (tracked as follow-up). The static registry remains the fast
  path for known types, so existing behavior is unchanged.
- Add a Coverage workflow that publishes LCOV artifacts for non-ROS crates and
  fails pull requests when line coverage drops against the base branch.

### Changed

- Update dependencies: `toml` 0.8 → 1, `rcgen` 0.13 → 0.14 (renamed
  `CertifiedKey` field in cert generation), `ratatui` 0.29 → 0.30,
  `crossterm` 0.28 → 0.29, plus semver-compatible lockfile refreshes.
  `bincode` stays on 1.x; migrating the protocol codec to bincode 3 is
  deferred to a dedicated change.

## [0.2.0] - 2026-06-10

### Added

- Add a source map and rustdoc notes clarifying `talos-common` protocol,
  session, transport, config, and URDF ownership boundaries.
- Add per-topic subscribe and unsubscribe controls in the TUI Topics tab, with subscription choices preserved across reconnects.
- Document how customized topic subscriptions handle newly discovered topics and retry failed manual toggles after reconnect.
- Add `sensor_msgs/msg/LaserScan` subscription support with full field conversion (`header`, `angle_min/max/increment`, `time_increment`, `scan_time`, `range_min/max`, `ranges`, `intensities`).
- Add `sensor_msgs/msg/Imu` subscription support with full field conversion (`header`, `orientation` + covariance, `angular_velocity` + covariance, `linear_acceleration` + covariance).
- Add `geometry_msgs/msg/PoseStamped` subscription support (`header`, `pose`).
- Add per-subscription `qos` config field accepting `"default"` (Reliable, Volatile, KeepLast — rclrs default depth) or `"sensor_data"` (BestEffort, Volatile, KeepLast 5). Omitting `qos` preserves existing behavior.
- View and set ROS 2 parameters on any node via the standard `rcl_interfaces` parameter services. The agent gains `ListParameters`, `GetParameters`, and `SetParameter` protocol requests.
- CLI commands `list-params`, `get-param`, and `set-param`.
- A TUI Params tab to browse a node's parameters and edit values in place.

### Changed

- Split CLI command handling, parameter protocol types, and TUI parameter input handling into focused modules.
- Centralize talos-agent supported ROS message type subscription wiring in a registry for easier message support additions.
- Split the agent server implementation into focused UDS, QUIC, request, graph, and control modules without changing the public server API.
- Cache mdBook tooling in the Docs workflow to reduce CI time.
- Split TUI app state topic, log, and joint behavior into focused state modules without changing UI behavior.
- Update the release workflow so feature development targets `dev` and version bumps run when `dev` is promoted to `main`.
- Keep TUI topic ordering stable while subscription acknowledgements and refreshed topic lists arrive mid-session.
- Stop reconnect requests from retrying topics that disappeared from the latest agent topic list, and document that those topics drop out of the pane until re-advertised.
- Treat a fresh `TopicList` as a reconnect catalog rather than proof of active subscriptions, which avoids false subscribed badges before subscribe acknowledgements land.
- Let `s` toggle the selected topic from either Topics pane and make pending subscription badges easier to distinguish without relying on color.
- Split TUI input handling from terminal setup so key behavior can be tested independently of the terminal lifecycle.

### Fixed

- Preserve existing parameter types when editing TUI parameter values that look like another type.
- Return CLI errors for failed `list-params` and `get-param` responses.
- Show fully-qualified node names in the TUI Params tab so namespaced nodes are distinguishable.
- Fix clean `pixi run build` of `rclrs_ws` failing on the first Rust package: the parent talos cargo workspace leaked into the nested build via `.cargo/config.toml` `[patch.crates-io]` paths and the root `Cargo.toml` workspace. The build now isolates the ancestor cargo config, and `rclrs_ws` is excluded from the talos workspace.
- Clear stale TUI topic subscription errors when later topic data confirms a desired subscription is healthy again.
- Clear stale unsubscribe errors after reconnect when the desired state is already unsubscribed.
- Ignore stale TUI subscribe or unsubscribe acknowledgements after desired topic intent changes.
- Roll back optimistic TUI topic toggles if the client command channel has already stopped.

## [0.1.5] - 2026-04-28

### Added

- Add Mermaid rendering support for mdBook diagrams.

### Changed

- Convert the introduction and architecture overview diagrams to Mermaid flowcharts.

## [0.1.4] - 2026-04-28

- No notable changes recorded.

## [0.1.3] - 2026-04-28

### Added

- Add canonical mdBook documentation under `docs/` with current behavior, design history, and future plans.
- Add a GitHub Actions workflow to build and publish the mdBook to GitHub Pages.
- Add Dependabot version updates for Cargo dependencies and GitHub Actions.

### Changed

- Shorten `README.md` to a project overview that links to the mdBook.
- Replace legacy spec workflow guidance with mdBook documentation guidance.
- Document the default branch-and-pull-request workflow for repository changes.

### Removed

- Remove the legacy spec source tree after migrating its useful content into the mdBook.

## [0.1.2] - 2026-04-28

- No notable changes recorded.

## [0.1.1] - 2026-04-28

### Added

- Add an automated version bump and GitHub release workflow with changelog-backed release notes.
- Consolidate repository agent guidance in `AGENTS.md` and keep `CLAUDE.md` as a symlink for compatibility.

## [0.1.0] - TBD

### Added

- Terminal-native ROS 2 observation and interaction architecture with a robot-side agent and developer-side CLI/TUI clients.
- Shared `talos-common` protocol, config, transport, session, and URDF parsing code without a ROS 2 runtime dependency.
- Length-prefixed bincode IPC protocol with typed requests and transport-agnostic dynamic message data through `DynValue`.
- Unix domain socket transport for local agent/client communication.
- Feature-gated QUIC transport for remote agent/client communication, including self-signed certificate support.
- ROS 2 bridge subscriptions for configured topics using supported message types: odometry, twist, string, joint state, and ROS log messages.
- Per-client topic subscription routing so clients only receive requested topic data.
- CLI commands for listing topics, listing nodes, and echoing live topic data.
- Ratatui terminal UI with Topics, Nodes, Log, and Joints tabs.
- URDF-aware joint display with joint state updates, limits, command publishing, and configured pose execution support.
