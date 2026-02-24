# 0003-2026-02-24-workspace-bootstrap

## Metadata

- Date: 2026-02-24
- Sequence: 0003
- Authors: ytao6122
- Status: accepted
- Scope: app, core, ci

## Summary

The repository was bootstrapped into a functional Rust workspace with separate app and core crates, enabling full CI validation and release packaging.

## Context and Problem

Release/CI infrastructure existed, but mainline code required a canonical workspace layout and concrete crate content to build and test reliably.

## Goals and Non-Goals

Goals:
- Establish workspace root manifests.
- Add app crate and reusable core crate.
- Make CI checks pass with real code and tests.

Non-Goals:
- Complete all product features.
- Finalize long-term API stability.

## Decision

Adopt two-crate layout:
- `apps/simple-term`: binary entrypoint and app integration
- `crates/simple-term`: reusable terminal core logic

This separation improves testability and future extensibility while keeping release packaging straightforward.

## Alternatives Considered

1. Single crate only
- Pros: less structure upfront
- Cons: weaker separation of concerns and reuse boundaries
- Decision: rejected

2. Multiple app crates from day one
- Pros: future-ready for many frontends
- Cons: unnecessary complexity at bootstrap stage
- Decision: rejected for now

## Implementation Details

- Added workspace manifests:
  - `Cargo.toml`
  - `Cargo.lock`
- Added application crate:
  - `apps/simple-term`
- Added core crate:
  - `crates/simple-term`
- Merged via PR to protected `main`.

## Impact

Technical impact:
- Project became buildable/testable end-to-end.

Product/user impact:
- Foundation for runnable desktop app binary.

Operational impact:
- CI and release now evaluate actual deliverable code.

## Verification

- Local verification passed: fmt/check/clippy/test.
- PR check passed before merge.

## Risks and Follow-ups

- Risk: module boundaries may require future refinement as features grow.
- Follow-up: keep architecture invariants updated in docs.

## Traceability

- Commits: `f0e0b6a46ae30d4b3c3b8edfe017d17627ba1c28`
- PRs: `#1`
- Tags: N/A
- Releases: N/A
- Workflow runs: `22334624523` (PR CI success)
- Related docs: `docs/architecture-invariants.md`

