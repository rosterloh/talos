# Changelog

All notable changes to Talos are recorded here.

Future changes should be added under `[Unreleased]`. The release workflow moves
those entries into the versioned section when a release is created.

## [Unreleased]

### Added

- Add `sensor_msgs/msg/LaserScan` subscription support with full field conversion (`header`, `angle_min/max/increment`, `time_increment`, `scan_time`, `range_min/max`, `ranges`, `intensities`).
- Add `sensor_msgs/msg/Imu` subscription support with full field conversion (`header`, `orientation` + covariance, `angular_velocity` + covariance, `linear_acceleration` + covariance).
- Add `geometry_msgs/msg/PoseStamped` subscription support (`header`, `pose`).
- Add per-subscription `qos` config field accepting `"default"` (Reliable, Volatile, KeepLast — rclrs default depth) or `"sensor_data"` (BestEffort, Volatile, KeepLast 5). Omitting `qos` preserves existing behavior.

### Changed

- Cache mdBook tooling in the Docs workflow to reduce CI time.
- Update the release workflow so feature development targets `dev` and version bumps run when `dev` is promoted to `main`.

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
