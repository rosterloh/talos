# Changelog

All notable changes to Talos are recorded here.

Future changes should be added under `[Unreleased]`. The release workflow moves
those entries into the versioned section when a release is created.

## [Unreleased]

### Changed

- Rework the native TUI as a restrained instrument console with shared semantic
  styles, full-row selection, explicit focus markers, simpler borders, bounded
  navigators, single-pane narrow layouts, contextual footers and scrollable help.
- Add selected-field navigation through nested topic payloads and arrays,
  expandable endpoint/QoS details, namespace-aware node lists and scrollable
  node details. Keep selection and viewport state across redraws and resizing.
- Prioritize log message width, add scrollable full-message snapshots and
  explicit LIVE/PAUSED controls that preserve inspected entries.
- Lead joint details with measured values, units, limits and requested targets;
  distinguish pending commands from publication acknowledgements. Parameter
  editing shows type/current value and preserves rejected input for retry.

### Fixed

- Keep DEBUG logs readable when selected, retain topic-tree expansion across
  catalog refreshes, and clamp paused log selection when old entries expire.
- Never render missing/nonfinite joint telemetry as zero or shift measurements
  between joints when a telemetry array contains an invalid element.
- Preserve parameter errors after refresh, block duplicate pending commands,
  and report unknown outcomes when a connection is lost during a request.

## [1.0.1] - 2026-09-24

### Added

- The CLI has a global `--json` flag. `list-topics`, `list-nodes`,
  `list-params` and `get-param` print a JSON array, and `echo` prints one
  `{topic, stamp, data}` object per line, with message data as plain JSON
  (byte arrays as base64, NaN/inf as `null`). `set-param` ignores the flag.

- New `talos-cli hz <topic> [--duration <s>]` prints the agent-measured rate,
  bandwidth and latency of a topic once a second, or a JSON object per second
  with `--json`.

- `/` in the TUI opens a filter prompt for the current list: topics, nodes,
  parameters, or log messages (the Log search). Matching is a case-insensitive
  substring and updates as you type; `←`/`→` move the cursor, `Ctrl-U` clears,
  `Enter` applies (empty clears the filter) and `Esc` restores the previous
  filter. The active filter is shown in the pane title (in the filter bar on
  the Log tab). The parameter value
  editor uses the same text input, so it gains cursor movement and `Ctrl-U`.
- Show and set a node's logger level through its `get_logger_levels` /
  `set_logger_levels` services, with new `GetLoggerLevel` and
  `SetLoggerLevel` agent requests. Use `talos-cli log-level <node> [level]`,
  or `l` (load) and `L` (cycle level) on the TUI Nodes tab. The target node
  must enable its logger services, e.g. rclcpp
  `NodeOptions().enable_logger_service(true)`.

- The topic detail pane in the TUI now lists each publisher and subscriber on
  the selected topic, with its reliability, durability, history and deadline,
  from a new `GetTopicEndpoints` agent request. Subscribers that can never
  match a publisher are flagged in red, e.g. a best-effort publisher with a
  reliable subscriber. Such a mismatch is the usual reason a topic shows no
  data.

- The agent now measures rate, bandwidth and header-stamp latency for every
  bridged topic, before any per-client frame dropping, and serves them through
  a new `GetTopicStats` request. The TUI polls it every second and shows the
  values in the topic list and detail pane, with a 60-second rate sparkline.
  This replaces the TUI's own Hz estimate from received frames, which read
  too low when frames were dropped and too high when message timing was
  jittery. The new requests are not understood by older agents: the agent
  and its clients must be upgraded together.

### Fixed

- TUI: the topic and node lists now refresh every 2 seconds while connected,
  so new topics and nodes appear (and new topics are subscribed as usual) and
  vanished ones are removed without a reconnect. The selection stays on the
  same topic or node. Press `r` to refresh at once.
- QUIC clients no longer lose a topic's stream for the rest of the session when
  a single message is between 8 and 16 MiB. The data-stream decoder now uses
  the protocol's 16 MiB frame limit instead of the 8 MiB codec default.
- A message over the 16 MiB frame limit is now logged and dropped by the agent.
  Before, it disconnected UDS clients (which reconnected into the same frame)
  or closed that topic's QUIC stream.
- The agent now queues at most 1024 undelivered frames per client and drops
  new frames beyond that. A slow client can no longer grow agent memory
  without bound.
- `talos-agent` now exits with an error when no transport is configured.
- TUI: fixed a crash when truncating long string values that contain
  multi-byte UTF-8 characters.
- TUI: the terminal is restored if the TUI panics, and errors from the app
  loop now give a non-zero exit code.
- TUI: topic, node, parameter, joint, pose and log lists now scroll to keep
  the selection visible.
- TUI: a rejected parameter set is no longer hidden by the refresh that
  follows it, and agent errors for parameter loads and sets are shown instead
  of leaving "loading…" on screen.
- TUI: the status bar and help overlay now show the real Joints key bindings
  (`e` edit, `x` execute pose, `j`/`o` switch list), and no longer list the
  Log bindings that don't exist (`n`, `/`).
- `talos echo` now exits with an error when the agent does not confirm the
  topic, instead of waiting forever.
- `talos` no longer panics when its output pipe closes early (e.g.
  `talos echo /x | head`). It now exits quietly, like other Unix tools.
- The agent writes each QUIC topic stream from its own task. A stream stalled
  by flow control now drops only its own frames and no longer blocks control
  requests or other topics for that client.
- The agent now refuses to start on a UDS socket that another agent is still
  serving. Before, it deleted that agent's socket and took it over. A stale
  socket file is still replaced.
- TUI: a topic that stops publishing now shows 0 Hz instead of keeping its
  last rate forever.
- TUI: the log selection stays on the same entry as new `/rosout` messages
  arrive, and is clamped when the severity filter shortens the list.
- TUI: joint and pose commands now show their outcome on the Joints tab.
  This includes agent errors and the "clamped to limit" note, which was never
  displayed before.
- The version bump workflow now also moves the `[workspace]` version in
  `pixi.toml`, which had been left at 0.1.5; it is set to 1.0.0 to match the
  release.

### Changed

- The README now links to the published book at
  <https://rosterloh.github.io/talos/>. The roadmap no longer lists
  publishing to GitHub Pages as future work, since the Docs workflow already
  deploys from `main`.
- Protect `dev` from deletion with a repository ruleset, so the automatic
  head-branch cleanup no longer deletes it when a `dev` -> `main` release pull
  request merges.

## [1.0.0] - 2026-09-24

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
- Add a CI workflow that enforces `cargo fmt` and `cargo clippy -D warnings`,
  and runs the test suites. Formatting and the non-ROS crates are checked on a
  plain stable toolchain; `talos-agent` is built, linted and tested inside the
  Pixi ROS 2 Lyrical environment.
- Add `talos-agent/tests/bridge_live.rs`, which publishes on a real ROS 2 topic
  and asserts the bridge forwards it. The existing tests drive the IPC protocol
  with synthetic responses, so they cannot detect a bridge that never delivers
  any message.

### Changed

- Update dependencies: `toml` 0.8 → 1, `rcgen` 0.13 → 0.14 (renamed
  `CertifiedKey` field in cert generation), `ratatui` 0.29 → 0.30,
  `crossterm` 0.28 → 0.29, plus semver-compatible lockfile refreshes.
- Migrate the protocol codec from `bincode` 1 to 2 using the `legacy`
  configuration, keeping the wire format byte-identical so mixed-version
  agents and clients still interoperate. Serialization now goes through
  shared `protocol::codec::{to_vec, from_slice}` helpers. (crates.io lists
  a `bincode` 3.0.0, but it is an empty placeholder release; 2.x is the
  current real release line.)
- Upgrade the ROS 2 environment from Kilted to Lyrical Luth and stop building
  `rclrs` from source. `rosidl_runtime_rs` now comes from crates.io, and the
  `robostack-lyrical` conda packages provide the ROS 2 runtime plus
  pre-generated Rust message bindings. `talos-agent` takes its message types
  from `ros-env`, which compiles those bindings against `rosidl_runtime_rs` 0.7
  and so picks up the Lyrical `Sequence<T>` ABI fix
  ([ros2-rust/ros2_rust#659](https://github.com/ros2-rust/ros2_rust/issues/659)).
  `rclrs` moves to 0.8 (the first crates.io release with Lyrical support) and
  `ros-env` to 0.3.
- Move the Pixi manifest to the repository root (from `rclrs_ws/`) and slim it to
  `ros2-ros-base` + the Rust toolchain. Removed the `rclrs_ws` colcon /
  vcstool source-build workspace and its `setup.bash` step; building now only
  needs `pixi install` then `cargo build`.
- Add `osx-arm64` to the Pixi platforms, so the agent can be built and checked
  on Apple Silicon workstations alongside `linux-64` / `linux-aarch64`.
- Follow RoboStack's package rename from `ros-lyrical-*` to `ros2-*`
  ([RoboStack/vinca#104](https://github.com/RoboStack/vinca/pull/104)): depend
  on `ros2-ros-base` and pin `ros2-distro-mutex` to a `lyrical` build, so the
  distro stays explicit once the legacy aliases are dropped. Refresh
  `pixi.lock` (`ros2-rclcpp` 32.0.3, `ros2-sensor-msgs` 5.9.3,
  `ros2-rosidl-generator-rs` 0.5.0).
- Bump the Pixi Rust toolchain from 1.93 to 1.98.
- Bump the Pixi `compilers` from 1.11 to 2.0 (clang 21 on macOS, gcc 15 on
  Linux). Compilers 2.0 no longer exports `CC`/`CFLAGS`/`LDFLAGS`; on macOS its
  clang config files add the environment's `lib` directory as an rpath, so the
  `RUSTFLAGS` rpath workaround for `osx-arm64` is removed.
- Refresh `Cargo.lock` with semver-compatible updates (e.g. `tokio` 1.53,
  `quinn` 0.11.12, `rustls` 0.23.45, `clap` 4.6.7).
- Run the CI `talos-agent` job on a native aarch64 runner (`ubuntu-26.04-arm`)
  as well as x86-64 (`ubuntu-26.04`), covering the `linux-aarch64` Pixi platform.

> Known issue on Lyrical: the `DynamicMessage` fallback in `rclrs` still panics
> on primitive or string sequence fields (e.g. `sensor_msgs/PointCloud2`).
> Topics with a compiled-in converter and the parameter services are unaffected.
> Fix proposed upstream in
> [ros2-rust/ros2_rust#714](https://github.com/ros2-rust/ros2_rust/pull/714); the
> affected `talos-agent` test is ignored until it ships.

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
- Fix the agent bridge delivering no topic data at all after the `rclrs` 0.8
  upgrade. Two independent causes: subscription handles returned by
  `create_subscription` were discarded, and under `rclrs` 0.8 that handle owns
  the subscription's place in the executor wait set, so every subscription was
  torn down immediately (the ROS graph reported zero subscribers); and
  `executor.spin()` was called directly inside a `tokio::spawn`ed task, so ROS 2
  callbacks ran on a Tokio worker thread that never yields and the
  bridge-to-router forwarder they woke was parked in that worker's
  non-stealable LIFO slot and never polled. Subscriptions are now held for the
  lifetime of the spin, which runs under `tokio::task::block_in_place`.
- Clear the accumulated `clippy` and `rustfmt` backlog across all four crates
  (collapsible `if let` chains, `Default` field reassignment, `approx_constant`
  in test fixtures, a `loop`/`match` that reads better as `while let`, and a
  test module that preceded production items).

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
