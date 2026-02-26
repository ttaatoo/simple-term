# 0033-2026-02-25-macos-app-icon-pipeline-and-rounded-asset

## Metadata

- Date: 2026-02-25
- Sequence: 0033
- Status: active
- Scope: release, workflow

## Why This Entry Exists

The release workflow produced a macOS `.app` bundle without an explicit app icon, so Finder and Dock could fall back to a generic executable icon. This entry records the icon asset contract and packaging path so future changes do not silently drop app branding.

## System Context

- Relevant directories/modules: `apps/simple-term/assets/`, `.github/workflows/release.yml`
- Upstream constraints (platform, library, policy): macOS app bundles resolve icons from `Contents/Resources/*.icns` with `CFBundleIconFile` in `Info.plist`
- Invariants already in force: release workflow must keep `.dmg` + `.tar.gz` outputs and stable `SimpleTerm.app` bundle assembly

## Decision and Rationale

- Decision:
  - Store canonical icon artifact at `apps/simple-term/assets/SimpleTerm.icns`.
  - Keep a rounded 1024 PNG source snapshot at `apps/simple-term/assets/SimpleTerm-icon-1024-rounded.png`.
  - During bundle creation, copy `SimpleTerm.icns` into `Contents/Resources/` and set `CFBundleIconFile=SimpleTerm.icns`.
- Why this path was selected:
  - Makes icon inclusion deterministic and independent of ad-hoc local packaging state.
  - Keeps release workflow self-contained with no runtime icon generation dependency.
- Trade-offs accepted:
  - Adds binary assets to git history.
  - Manual regeneration is required when icon design changes.

## Alternatives Considered

1. Generate `.icns` dynamically in CI from a PNG source
- Pros: smaller repo footprint and source-only icon editing
- Cons: extra workflow complexity and tool-chain assumptions on runner image
- Why not chosen: static `.icns` provides simpler, more predictable release behavior

2. Keep app icon unspecified in `Info.plist`
- Pros: no extra files or workflow changes
- Cons: poor product polish; icon can regress to generic app glyph
- Why not chosen: fails desktop UX expectations for macOS releases

## Safe Change Playbook

When modifying app icon assets or packaging:
1. Update `apps/simple-term/assets/SimpleTerm.icns` (and refresh `SimpleTerm-icon-1024-rounded.png` if visual source changes).
2. Keep `CFBundleIconFile` in `.github/workflows/release.yml` aligned with the resource filename.
3. Ensure workflow copies the `.icns` into `dist/SimpleTerm.app/Contents/Resources/`.
4. Build and inspect release artifacts to confirm Finder/Dock icon resolution.

## Do / Avoid

Do:
- Keep icon filename stable unless workflow and plist are updated together.
- Preserve transparent rounded corners in icon exports for modern macOS appearance.
- Treat icon assets as release-critical bundle resources.

Avoid:
- Referencing a bundle icon name not present in `Contents/Resources/`.
- Renaming `SimpleTerm.icns` without updating `CFBundleIconFile`.
- Relying on local Finder cache as sole validation signal.

## Typical Mistakes

- Copying icon to the wrong bundle subdirectory (not `Contents/Resources`).
- Setting `CFBundleIconFile` to a mismatched filename.
- Replacing only PNG source without regenerating `.icns`.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - CI release workflow YAML remains valid
- Recommended manual checks:
  - Run release packaging path and inspect `dist/SimpleTerm.app/Contents/Resources/SimpleTerm.icns`.
  - Open packaged app/DMG on macOS and confirm icon in Finder + Dock.
- Signals of regression:
  - Generic executable icon shown in Finder/Dock.
  - Missing `SimpleTerm.icns` in bundled resources.

## Related Artifacts

- Related docs: `docs/evolution/0007-2026-02-24-macos-dmg-release-packaging.md`
- Optional references (PRs/commits/releases): `.github/workflows/release.yml`
