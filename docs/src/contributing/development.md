# Development

## Branching And Releases

`main` is the stable release branch. Use `dev` for ongoing integration work.
Create feature branches from `dev` and open pull requests back into `dev`.

When `dev` is ready to release, open a pull request from `dev` to `main`.
Merging that pull request runs the version bump workflow, promotes
`CHANGELOG.md` entries from `[Unreleased]`, and creates the GitHub release. Add
`version:minor` or `version:major` to the `dev` -> `main` pull request when the
release should be larger than a patch bump.

The bump moves the version in `Cargo.toml`, `Cargo.lock` and `pixi.toml` together.
Merge the release pull request with a merge commit rather than a squash, so `dev`
and `main` do not diverge. The repository deletes merged head branches
automatically; a `Keep dev` repository ruleset blocks deleting `dev`, so it
survives the release merge.

## Workspace Checks

Without ROS 2:

```bash
cargo check -p talos-common -p talos-cli -p talos-tui
```

With the ROS 2 Lyrical environment (provided by Pixi):

```bash
pixi install
pixi run check          # or: pixi shell, then cargo check --workspace
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
cargo test -p talos-agent --test bridge_live
```

`bridge_live` is the only test that exercises a real ROS 2 topic: it publishes
on one and asserts the bridge forwards the message to the router. The
`integration` tests drive the IPC protocol with synthetic responses, so they
stay green even if the bridge delivers nothing. Run it inside the Pixi
environment.

## Lints

CI enforces both of these, so run them before pushing:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --features quic -- -D warnings
```

The `talos-agent` job builds and tests inside the Pixi ROS 2 environment on both
x86-64 and aarch64 Linux runners.

## Coverage

The Coverage workflow reports LCOV output for the non-ROS crates
(`talos-common`, `talos-cli`, and `talos-tui`) with the `quic` feature enabled.
Pull requests compare that coverage against the base branch and fail if line
coverage decreases.

Install `cargo-llvm-cov` before running the local coverage command:

```bash
cargo install cargo-llvm-cov --locked

cargo llvm-cov -p talos-common -p talos-cli -p talos-tui \
  --features quic \
  --lcov \
  --output-path coverage/lcov.info

python3 .github/scripts/coverage_report.py summarize coverage/lcov.info
```

On machines without the Pixi environment, temporarily remove `talos-agent` from
the workspace members before running non-ROS coverage; the CI workflow does this
because these packages do not need ROS 2.

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
