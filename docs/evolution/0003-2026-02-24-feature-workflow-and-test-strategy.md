# 0003-2026-02-24-feature-workflow-and-test-strategy

## Metadata

- Date: 2026-02-24
- Sequence: 0003
- Status: active
- Scope: workflow, testing

## Why This Entry Exists

The project needs a repeatable development pattern so future contributors and LLM agents can add features without violating architecture or destabilizing runtime behavior.

## System Context

Quality gates currently used:
- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

High-risk areas:
- `terminal_view.rs` input/render path
- `terminal.rs` PTY/event lifecycle
- mappings modules (`keys`, `mouse`, `colors`)

## Decision and Rationale

Decision:
- Use a behavior-first flow with fast local verification before pushing.

Rationale:
- The codebase has a sensitive runtime surface where small changes can create non-obvious regressions.
- Strong local verification reduces CI churn and flaky review loops.

Trade-offs:
- Slightly slower local cycle compared to pushing early.
- Requires discipline in test updates and scope control.

## Alternatives Considered

1. CI-only validation (minimal local checks)
- Pros: faster local iteration
- Cons: more remote failures and slower feedback loops
- Why not chosen: low signal-to-noise during feature development

2. Heavy end-to-end only strategy
- Pros: realistic behavior coverage
- Cons: expensive and slower for small regressions
- Why not chosen: poor iteration speed for day-to-day work

## Safe Change Playbook

1. Define behavior contract before editing code.
2. Identify ownership layer (`apps` vs `crates`) and keep boundaries clean.
3. Add/update nearest tests first (unit/integration based on ownership).
4. Implement minimal change set.
5. Run full workspace checks locally before push.
6. Add/update evolution entry when decision/invariant changes.

## Do / Avoid

Do:
- Keep changes small and scoped to one concern.
- Add regression tests for every bug fix.
- Prefer explicit failure modes over hidden fallbacks.

Avoid:
- Bundling refactor + behavior change + infra change in one patch.
- Skipping clippy/test because a feature "looks fine" manually.
- Adding logic where no invariant is documented.

## Typical Mistakes

- Editing both app and core without clear ownership rationale.
- Changing input semantics without mapping-layer tests.
- Passing local run but failing CI due to lint/test mismatches.

## Verification Strategy

Required local checks before push:
- `cargo fmt --all -- --check`
- `cargo check --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Manual smoke (for UI-visible behavior):
- typing while scrolled up
- mouse + wheel interactions
- title update/exit events
- hyperlink hover/open behavior

## Related Artifacts

- Related docs: `AGENTS.md`, `docs/architecture-invariants.md`, `docs/troubleshooting.md`
- Optional references: CI workflow definition in `.github/workflows/ci.yml`

