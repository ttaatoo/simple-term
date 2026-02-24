# Release Strategy (simple-term)

This repository follows SemVer for `workspace.package.version` in `Cargo.toml`, uses `main` as the integration branch, and publishes desktop binaries through GitHub Releases.

## Versioning Policy

- `MAJOR` (`X.0.0`): backward-incompatible behavior or API changes.
- `MINOR` (`0.X.0`): backward-compatible features.
- `PATCH` (`0.0.X`): backward-compatible bug fixes.
- Pre-release builds use SemVer suffixes:
  - `-beta.N` for preview builds.
  - `-rc.N` for release candidates.

Tags always use a leading `v`, for example:
- `v0.3.0`
- `v0.4.0-rc.1`

## Branch Model

- `main`: always releasable integration branch.
- `release/vX.Y`: optional stabilization branches (for example, `release/v0.4`).
- `feat/*`, `fix/*`, `chore/*`: short-lived topic branches.

Rules:
- Merge to protected branches only through pull requests.
- Keep release branches limited to fix/backport work.
- After releasing from `release/vX.Y`, backport fixes to `main`.

## Release Workflow

The workflow at `.github/workflows/release.yml` supports two release paths:

1. Tag push release:
   - Push a SemVer tag (`v*`) and the workflow will build artifacts and publish a GitHub Release.
2. Manual dispatch release:
   - Run the workflow manually with:
     - `version`: SemVer without `v`, for example `0.4.0-rc.1`.
     - `ref`: branch/SHA to release from (default `main`).
   - The workflow validates:
     - SemVer format.
     - `Cargo.toml` workspace version matches the input.
     - tag does not already exist.
   - Then it creates and pushes the tag, builds artifacts, and publishes the release.

Artifacts:
- macOS: `simple-term-vX.Y.Z-macos-<arch>.tar.gz`
- Linux: `simple-term-vX.Y.Z-linux-<arch>.tar.gz`
- Windows: `simple-term-vX.Y.Z-windows-<arch>.zip`
- `SHA256SUMS.txt` attached to each release

## Recommended Cadence

- Weekly or bi-weekly `PATCH` releases for fixes.
- Monthly `MINOR` releases for features.
- Use `-rc.N` before each minor release when risk is medium/high.

## Branch Protection Settings

Apply these branch protection rules in `ttaatoo/simple-term`:

For `main` and `release/*`:
- Require a pull request before merging.
- Require at least 1 approval.
- Dismiss stale approvals on new commits.
- Require conversation resolution before merge.
- Require status checks to pass before merging:
  - `Verify (fmt, check, clippy, test)`
- Require linear history.
- Disable force pushes.
- Disable branch deletion.

Optional hardening:
- Restrict who can push to release branches.
- Require signed commits.

## Suggested Human Flow

1. Merge release PR that updates `workspace.package.version`.
2. Run `Release` workflow (`workflow_dispatch`) with:
   - `version=<same version from Cargo.toml>`
   - `ref=main` or `ref=release/vX.Y`
3. Verify GitHub Release notes and attached artifacts.
4. Announce the release and include checksum verification instructions.
