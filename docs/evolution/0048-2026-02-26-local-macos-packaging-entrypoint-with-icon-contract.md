# 0048-2026-02-26-local-macos-packaging-entrypoint-with-icon-contract

## Metadata

- Date: 2026-02-26
- Sequence: 0048
- Status: active
- Scope: release, workflow

## Why This Entry Exists

Developers were using `cargo build` output directly and then reporting missing app icon behavior. The icon contract only applies to a macOS `.app` bundle, so this entry records a local packaging entrypoint that mirrors release bundle metadata and resource copy behavior.

## System Context

- Relevant directories/modules: `scripts/package-macos-local.sh`, `Makefile`, `apps/simple-term/assets/`, `.github/workflows/release.yml`
- Upstream constraints (platform, library, policy): macOS app icon resolution requires `Contents/Resources/*.icns` plus `CFBundleIconFile` in `Info.plist`
- Invariants already in force: release packaging must keep `SimpleTerm.icns` filename and bundle copy path aligned

## Decision and Rationale

- Decision:
  - Add `scripts/package-macos-local.sh` to build a local `dist/SimpleTerm.app` and `dist/simple-term-local-preview.dmg`.
  - Add `make package-macos-local` as the one-command local entrypoint.
  - Keep local bundle `Info.plist` icon key/value aligned with release workflow (`CFBundleIconFile=SimpleTerm.icns`).
  - Treat DMG creation as best-effort for local environments where `hdiutil` may be unavailable while keeping `.app` output mandatory.
- Why this path was selected:
  - Gives contributors a deterministic local path to reproduce release-like icon behavior.
  - Avoids ad-hoc local commands that skip `.app` assembly and produce misleading icon results.
- Trade-offs accepted:
  - Packaging logic now exists in both release workflow and local script and must stay aligned.

## Alternatives Considered

1. Keep using only `cargo build` for local checks
- Pros: no additional scripts
- Cons: no `.app` bundle, so app icon behavior cannot be validated
- Why not chosen: does not satisfy desktop packaging validation needs

2. Move all packaging into one shared external tool
- Pros: single source of truth for CI and local packaging
- Cons: additional refactor and toolchain complexity not needed for current scope
- Why not chosen: small local entrypoint solves the immediate reliability gap

## Safe Change Playbook

When modifying local packaging behavior:
1. Keep `CFBundleIconFile` and copied icon filename exactly aligned with release workflow.
2. Keep icon source path stable at `apps/simple-term/assets/SimpleTerm.icns` unless workflow + docs are updated together.
3. Re-run local packaging and inspect `dist/SimpleTerm.app/Contents/Info.plist` and `dist/SimpleTerm.app/Contents/Resources/`.

## Do / Avoid

Do:
- Use `make package-macos-local` when validating icon/bundle behavior locally.
- Treat local packaging script as release-contract-adjacent code.

Avoid:
- Inferring app icon regressions from raw `target/release/simple-term` execution.
- Renaming local icon files without matching plist and copy-path updates.

## Typical Mistakes

- Running only `cargo build --release` and expecting Finder/Dock icon parity.
- Editing release workflow icon settings but forgetting to update local packaging script (or vice versa).

## Verification Strategy

- Required automated checks:
  - `cargo build --locked --release -p simple-term-app`
- Recommended manual checks:
  - `make package-macos-local`
  - `plutil -p dist/SimpleTerm.app/Contents/Info.plist`
  - verify `dist/SimpleTerm.app/Contents/Resources/SimpleTerm.icns` exists
- Signals of regression:
  - Missing `CFBundleIconFile` key in local `Info.plist`
  - Missing `.icns` in bundled resources
  - Finder/Dock showing generic executable icon for packaged `.app`

## Related Artifacts

- Related docs: `docs/evolution/0007-2026-02-24-macos-dmg-release-packaging.md`, `docs/evolution/0033-2026-02-25-macos-app-icon-pipeline-and-rounded-asset.md`
- Optional references (PRs/commits/releases): `.github/workflows/release.yml`, `scripts/package-macos-local.sh`
