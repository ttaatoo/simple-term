# Evolution Index (Knowledge Map)

This index is organized by **engineering knowledge topics**, not by commit chronology.

## 0001 Repository Layout and Dependency Boundaries

File: `0001-2026-02-24-repository-layout-and-boundaries.md`

Covers:
- why the workspace is split into `apps/simple-term` and `crates/simple-term`
- dependency direction rules
- where new code should live
- common layering mistakes

## 0002 Runtime Model and Core Invariants

File: `0002-2026-02-24-runtime-model-and-invariants.md`

Covers:
- event/data flow from UI to PTY and back
- lock/snapshot rendering constraints
- input/scroll behavior guarantees
- backpressure and event reliability rules

## 0003 Feature Development Workflow and Test Strategy

File: `0003-2026-02-24-feature-workflow-and-test-strategy.md`

Covers:
- recommended flow for adding features safely
- test placement and verification expectations
- high-risk change areas and required safeguards

## 0004 Release and Governance Operating Model

File: `0004-2026-02-24-release-and-governance-model.md`

Covers:
- SemVer and release execution model
- macOS-only artifact policy
- branch protection intent and admin bypass policy
- operational failure modes during release

## 0005 Known Pitfalls and Recovery Playbooks

File: `0005-2026-02-24-known-pitfalls-and-recovery.md`

Covers:
- repeated mistakes observed during bootstrap
- how to detect and recover quickly
- preventative guardrails for future contributors/LLMs

