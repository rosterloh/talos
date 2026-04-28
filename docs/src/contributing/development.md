# Development

## Workspace Checks

Without ROS 2:

```bash
cargo check -p talos-common -p talos-cli -p talos-tui
```

With the ROS 2/rclrs environment:

```bash
source rclrs_ws/install/setup.bash
cargo check --workspace
```

With QUIC:

```bash
cargo check --workspace --features quic
```

## Tests

```bash
cargo test --workspace
cargo test -p talos-common
cargo test -p talos-agent --test integration
cargo test -p talos-agent --test integration --features quic
```

## Rustdoc

Rustdoc is API reference and stays separate from this book:

```bash
cargo doc --workspace --no-deps
```

Use this mdBook for concepts, workflows, architecture, and contributor guidance.
Use Rustdoc for item-level API details.

## Changelog

User-facing code, behavior, documentation, CI, or configuration changes should
be recorded under `[Unreleased]` in `CHANGELOG.md`.
