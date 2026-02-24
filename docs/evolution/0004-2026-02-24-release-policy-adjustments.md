# 0004-2026-02-24-release-policy-adjustments

## Metadata

- Date: 2026-02-24
- Sequence: 0004
- Authors: taoyi
- Status: accepted
- Scope: release, governance

## Summary

Release policy and branch governance were adjusted to match current product scope (macOS-only distribution) while preserving strict default checks.

## Context and Problem

The initial release workflow built multiple platforms, but the product requirement was macOS-only support. Simultaneously, branch policy needed strict checks with an emergency administrator bypass path.

## Goals and Non-Goals

Goals:
- Restrict release artifacts to macOS.
- Keep strict branch protections and required checks.
- Allow admin bypass for urgent incidents.

Non-Goals:
- Reintroduce Linux/Windows distribution.
- Relax required status checks for normal flow.

## Decision

1. Release workflow changed to `macos-latest` only.
2. Branch protection configured with:
- required checks retained
- review requirements retained
- force push/deletion disallowed
- `isAdminEnforced=false` to permit admin bypass when needed

## Alternatives Considered

1. Keep multi-platform release and ignore unsupported assets
- Pros: future coverage
- Cons: wasted build time and user confusion
- Decision: rejected

2. Remove strict checks entirely for speed
- Pros: faster merges
- Cons: quality and governance regression
- Decision: rejected

## Implementation Details

- Updated: `.github/workflows/release.yml` to macOS-only build and packaging.
- Updated branch protection rules for `main` and `release/*`.
- Cancelled outdated multi-platform run to prevent invalid release artifacts.

## Impact

Technical impact:
- Reduced release pipeline complexity and runtime.

Product/user impact:
- Release assets now match supported platform scope.

Operational impact:
- Governance is strict by default, with explicit emergency override path.

## Verification

- Old release run cancelled: `22335167543`.
- New release run succeeded (macOS-only): `22335259583`.

## Risks and Follow-ups

- Risk: admin bypass can be overused if process discipline is weak.
- Follow-up: document criteria for using bypass and require post-incident notes.

## Traceability

- Commits: `142f9ad7148453fd18609b70ab0b3fab1be9a37d`
- PRs: `#1` (related bootstrap context)
- Tags: `v0.1.0`
- Releases: `simple-term v0.1.0`
- Workflow runs: `22335167543` (cancelled), `22335259583` (success)
- Related docs: `docs/release-strategy.md`

