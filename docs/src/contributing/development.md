# Development

## Branching And Releases

`main` is the stable release branch. Use `dev` for ongoing integration work.
Create feature branches from `dev` and open pull requests back into `dev`.

When `dev` is ready to release, open a pull request from `dev` to `main`.
Merging that pull request runs the version bump workflow, promotes
`CHANGELOG.md` entries from `[Unreleased]`, and creates the GitHub release. Add
`version:minor` or `version:major` to the `dev` -> `main` pull request when the
release should be larger than a patch bump.

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
