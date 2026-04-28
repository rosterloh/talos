# Versioning

`main` is the stable release branch. Day-to-day development lands on `dev`,
and feature branches should be opened against `dev`.

Merging `dev` into `main` automatically bumps the Cargo workspace patch
version, promotes the `[Unreleased]` changelog entries into the new version,
and creates a GitHub release using those changelog entries as the release
notes. Other pushes to `main` do not bump the version unless the workflow is
run manually.

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
merging to `dev` so the generated release notes are useful when `dev` is
promoted to `main`.
