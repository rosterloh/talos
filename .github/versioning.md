# Versioning

Merges to `main` automatically bump the Cargo workspace patch version.

To request a larger bump, add one of these labels to the pull request before
merging:

- `version:minor`
- `version:major`

If neither label is present, the workflow uses a patch bump.

The workflow can also be run manually from GitHub Actions with an explicit
`patch`, `minor`, or `major` input.

If branch protection blocks direct workflow pushes to `main`, allow the
`github-actions[bot]` actor to create version bump commits.
