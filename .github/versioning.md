# Versioning

Merges to `main` automatically bump the Cargo workspace patch version, promote
the `[Unreleased]` changelog entries into the new version, and create a GitHub
release using those changelog entries as the release notes.

To request a larger bump, add one of these labels to the pull request before
merging:

- `version:minor`
- `version:major`

If neither label is present, the workflow uses a patch bump. The release
workflow first writes the new changelog section as `TBD`, then fills in the UTC
release date before committing the version bump.

The workflow can also be run manually from GitHub Actions with an explicit
`patch`, `minor`, or `major` input.

If branch protection blocks direct workflow pushes to `main`, allow the
`github-actions[bot]` actor to create version bump commits.

Future work should add a `CHANGELOG.md` entry under `[Unreleased]` before
merging so the generated release notes are useful.
