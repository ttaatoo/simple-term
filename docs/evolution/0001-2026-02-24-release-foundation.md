# 0001-2026-02-24-release-foundation

## Metadata

- Date: 2026-02-24
- Sequence: 0001
- Authors: taoyi
- Status: accepted
- Scope: release, ci, docs

## Summary

A release foundation was introduced so the repository can publish tagged, reproducible artifacts via GitHub Actions using SemVer conventions.

## Context and Problem

Before this change, the repository had verification CI but no formal release pipeline. This created operational risk:
- No standardized tag-to-release path
- No release artifact packaging contract
- No automated release-note generation and checksum publication

## Goals and Non-Goals

Goals:
- Create a release workflow triggered by tag and by manual dispatch.
- Enforce version consistency between workflow input and workspace manifest.
- Publish release assets with checksums.

Non-Goals:
- Modify runtime terminal behavior.
- Introduce distribution installers (PKG/MSI/DEB) at this stage.

## Decision

The team added `.github/workflows/release.yml` with two entry modes:
1. Tag push (`v*`)
2. Manual dispatch (`version`, `ref`)

The workflow validates SemVer, validates `Cargo.toml` workspace version, creates/pushes an annotated tag (manual path), builds artifacts, and publishes a GitHub Release with generated notes.

## Alternatives Considered

1. Manual ad hoc release commands only
- Pros: no workflow complexity
- Cons: high human error risk, weak auditability
- Decision: rejected

2. Release only on tag push (no manual dispatch)
- Pros: simpler workflow
- Cons: poorer operator UX for controlled release initiation
- Decision: rejected

## Implementation Details

- Added: `.github/workflows/release.yml`
- Added: `docs/release-strategy.md`
- Validation points:
  - SemVer input regex
  - workspace version match
  - duplicate tag prevention

## Impact

Technical impact:
- Release flow became deterministic and repeatable.

Product/user impact:
- No user-facing runtime change.

Operational impact:
- Release publication moved from manual to CI-governed automation.

## Verification

- Local `cargo check --workspace` passed.
- Workflow YAML parse check passed.

## Risks and Follow-ups

- Risk: cross-platform build complexity can increase maintenance cost.
- Follow-up: policy later narrowed to macOS-only (see entry 0004).

## Traceability

- Commits: `3e61c5b4817dc193bf262020f2668391101646da`
- PRs: N/A (initial commit phase)
- Tags: N/A
- Releases: N/A
- Workflow runs: N/A
- Related docs: `docs/release-strategy.md`

