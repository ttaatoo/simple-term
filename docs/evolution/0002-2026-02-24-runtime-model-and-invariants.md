# 0002-2026-02-24-runtime-model-and-invariants

## Metadata

- Date: 2026-02-24
- Sequence: 0002
- Status: active
- Scope: runtime

## Why This Entry Exists

Most regressions in terminal apps come from violating invisible runtime assumptions (locks, event ordering, scroll/input interactions). This entry makes those assumptions explicit for safe maintenance.

## System Context

Primary modules:
- `apps/simple-term/src/terminal_view.rs`
- `crates/simple-term/src/terminal.rs`
- `crates/simple-term/src/mappings/*`
- `crates/simple-term/src/terminal_hyperlinks.rs`

Runtime flow:
1. UI captures input events.
2. Input is mapped to terminal protocol bytes or local UI actions.
3. Core sends backend events through bounded channels.
4. View layer consumes events and schedules repaint/title updates.
5. Rendering uses snapshots to avoid holding locks during paint.

## Decision and Rationale

Decision:
- Preserve strict snapshot-and-paint separation and explicit event backpressure policy.

Rationale:
- Prevent UI stutter and deadlock risk under heavy terminal output.
- Ensure critical events (title, exit) are retained even under channel pressure.
- Keep input/scroll interaction predictable.

Trade-offs:
- More explicit event handling branches.
- Additional complexity in dirty-row and suppression logic.

## Alternatives Considered

1. Keep lock during full paint
- Pros: simpler implementation
- Cons: major contention and potential responsiveness issues
- Why not chosen: unacceptable runtime behavior risk

2. Treat all events as equally droppable under pressure
- Pros: easier queue policy
- Cons: can lose critical terminal state transitions
- Why not chosen: correctness risk

## Safe Change Playbook

1. For rendering changes, audit lock scope first.
2. For event changes, classify event criticality (droppable vs retained).
3. For input/scroll changes, check suppression and alt-screen behavior.
4. Update unit tests around dirty rows, event mapping, and scroll interactions.

## Do / Avoid

Do:
- Keep paint path lock-free using snapshots.
- Preserve explicit backpressure semantics.
- Add narrow regression tests for every behavior fix.

Avoid:
- Expanding lock duration in render loop.
- Conflating precise scroll events with line-scroll gestures.
- Changing event priorities without test updates.

## Typical Mistakes

- Hidden lock extension caused by accessing terminal state during paint.
- Dropping title/exit updates when queue gets full.
- Breaking scroll suppression after input while scrolled up.

## Verification Strategy

Required checks:
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`

Targeted test focus:
- `terminal_view` tests for dirty rows and scroll logic
- `terminal.rs` tests for channel/backpressure behavior
- hyperlink/path detection tests for parser changes

Manual checks:
- high-output command responsiveness
- mouse scroll with and without modifiers
- title update and process exit behavior

## Related Artifacts

- Related docs: `docs/architecture-invariants.md`
- Optional references: integration tests in `crates/simple-term/tests/`

