# 0007-2026-02-24-macos-dmg-release-packaging

## Metadata

- Date: 2026-02-24
- Sequence: 0007
- Status: active
- Scope: release, workflow

## Why This Entry Exists

For macOS users, `.dmg` is the expected installation experience. A raw binary tarball works for advanced users but is less intuitive for mainstream desktop usage.

## System Context

Before this change, release artifacts included only:
- `simple-term-<tag>-macos-<arch>.tar.gz`
- `SHA256SUMS.txt`

Release workflow already built macOS binaries in `.github/workflows/release.yml`.

## Decision and Rationale

Decision:
- Keep the existing tarball artifact.
- Add a macOS app bundle (`SimpleTerm.app`) and package it into a `.dmg`.

Rationale:
- `.dmg` provides standard macOS distribution UX.
- Tarball remains useful for scripting and low-level debugging.
- Shipping both formats supports both user personas without additional release friction.

Trade-offs:
- Workflow becomes slightly more complex.
- App is unsigned/not notarized by default; users may need manual trust steps depending on local macOS security settings.

## Alternatives Considered

1. Keep tarball-only release
- Pros: simpler workflow
- Cons: poor installer UX for desktop users
- Why not chosen: product distribution quality was insufficient

2. Build signed/notarized DMG immediately
- Pros: best end-user trust and install UX
- Cons: requires Apple signing identities, notarization credentials, and secure secret management
- Why not chosen: deferred until signing infrastructure is ready

## Safe Change Playbook

When modifying macOS packaging:
1. Ensure `target/release/simple-term` remains the canonical built executable.
2. Build/update `SimpleTerm.app` bundle metadata (`Info.plist`) carefully.
3. Package both `.tar.gz` and `.dmg` in `dist/`.
4. Keep artifact upload patterns explicit (no broad glob that uploads temp folders).
5. Verify release includes both assets and checksum file.

## Do / Avoid

Do:
- Keep `.dmg` + `.tar.gz` dual-output policy unless product policy changes.
- Clean temporary DMG staging directories before upload.
- Keep release naming stable for automation compatibility.

Avoid:
- Uploading intermediate bundle/staging directories as release assets.
- Removing tarball fallback without explicit decision.
- Assuming unsigned app behavior is identical across all macOS security policies.

## Typical Mistakes

- Creating DMG from the wrong source folder and missing `SimpleTerm.app`.
- Forgetting to include `/Applications` symlink in DMG staging area.
- Accidentally changing artifact filename conventions and breaking automation.

## Verification Strategy

Required checks:
- YAML parse validation for `.github/workflows/release.yml`.
- Successful `Release` workflow run on macOS build job.

Release validation:
- Assets include:
  - `*.dmg`
  - `*.tar.gz`
  - `SHA256SUMS.txt`
- checksum file contains both artifact entries.

## Related Artifacts

- Related docs: `README.md`, `docs/evolution/0004-2026-02-24-release-and-governance-model.md`
- Optional references: `.github/workflows/release.yml`

