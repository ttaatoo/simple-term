# 0001-2026-02-24-repository-layout-and-boundaries

## Metadata

- Date: 2026-02-24
- Sequence: 0001
- Status: active
- Scope: architecture

## Why This Entry Exists

The workspace layout (`apps/simple-term` + `crates/simple-term`) is intentional. New contributors often try to collapse logic into the app layer or place UI concerns in the core crate. This entry defines the boundary model so changes remain coherent over time.

## System Context

Relevant directories:
- `apps/simple-term/`: binary entrypoint, GPUI integration, view orchestration
- `crates/simple-term/`: reusable terminal core, mappings, settings, PTY glue
- `docs/architecture-invariants.md`: runtime invariants and event/render constraints

Boundary invariants:
- Core crate must not depend on app crate.
- App crate may depend on core crate.
- UI event wiring lives in app crate; protocol/terminal behavior lives in core crate.
- Repository should keep only active workspace paths for this product line (`simple-term`) to avoid contributor/LLM confusion.

## Decision and Rationale

Decision:
- Keep a two-tier workspace: app shell + reusable core library.

Rationale:
- Improves testability of non-UI behavior.
- Reduces coupling between rendering code and terminal semantics.
- Preserves future options (additional frontends/tools) without refactoring the entire codebase.

Trade-offs:
- Slightly higher structure overhead than a single crate.
- Requires discipline on boundary ownership.

## Alternatives Considered

1. Single crate architecture
- Pros: fewer files, simpler onboarding at first glance
- Cons: weak separation of concerns, harder to test core logic in isolation
- Why not chosen: long-term maintenance and refactoring risk was higher

2. Multi-app workspace from day one
- Pros: explicit multi-client future support
- Cons: premature complexity for current scope
- Why not chosen: did not match current product maturity

## Safe Change Playbook

When adding behavior:
1. Decide if behavior is UI composition (`apps`) or terminal semantics (`crates`).
2. Implement core behavior in `crates/simple-term` first when possible.
3. Expose minimal interfaces to app layer; avoid leaking UI-specific types into core.
4. Add/extend tests at the layer where behavior is owned.

## Do / Avoid

Do:
- Keep protocol mappings and PTY concerns in core.
- Keep window/input event glue in app layer.
- Prefer small explicit interfaces across crate boundary.

Avoid:
- Calling UI-only APIs from core modules.
- Putting terminal protocol rules directly in view code.
- Duplicating constants/semantics between app and core.

## Typical Mistakes

- Implementing input translation in `terminal_view.rs` instead of mappings modules.
- Making core behavior depend on GPUI-specific concepts.
- Adding cross-crate shortcuts that bypass clean ownership.
- Keeping stale parallel directory trees that are not workspace members.

## Verification Strategy

Required checks:
- `cargo check --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Manual sanity:
- confirm app still runs with `cargo run -p simple-term-app`
- verify title updates, input, and scroll behavior in live terminal

Regression signals:
- new circular dependency pressure
- tests needing UI harness for logic that should be core-only

## Related Artifacts

- Related docs: `docs/architecture-invariants.md`, `AGENTS.md`
- Optional references: workspace bootstrap PR/commit history
