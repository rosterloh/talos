# Changelog

All notable changes to Talos are recorded here.

Future changes should be added under `[Unreleased]`. The release workflow moves
those entries into the versioned section when a release is created.

## [Unreleased]

### Added

- Add per-topic subscribe and unsubscribe controls in the TUI Topics tab, with subscription choices preserved across reconnects.
- Document how customized topic subscriptions handle newly discovered topics and retry failed manual toggles after reconnect.

### Changed

- Cache mdBook tooling in the Docs workflow to reduce CI time.
- Update the release workflow so feature development targets `dev` and version bumps run when `dev` is promoted to `main`.
- Keep TUI topic ordering stable while subscription acknowledgements and refreshed topic lists arrive mid-session.
- Stop reconnect requests from retrying topics that disappeared from the latest agent topic list, and document that those topics drop out of the pane until re-advertised.
- Treat a fresh `TopicList` as a reconnect catalog rather than proof of active subscriptions, which avoids false subscribed badges before subscribe acknowledgements land.
- Let `s` toggle the selected topic from either Topics pane and make pending subscription badges easier to distinguish without relying on color.

### Fixed

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
