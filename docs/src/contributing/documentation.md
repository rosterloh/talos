# Documentation

The `docs/` mdBook is the canonical documentation source.

## Local Preview

```bash
mdbook serve docs
```

By default, mdBook serves at `http://localhost:3000` and rebuilds when files
change.

## Build

```bash
mdbook build docs
```

The generated site is written to `docs/book/`.

## Style

Write current behavior as prose. Avoid preserving formal requirement language
unless it makes the behavior clearer.

Use design history pages for context on accepted decisions. Do not duplicate
current behavior there.

Keep future plans in the `future/` section so readers can distinguish planned
work from implemented behavior.

When documenting Rust APIs, link or name the relevant type, trait, or module,
but keep full API reference in Rustdoc.
