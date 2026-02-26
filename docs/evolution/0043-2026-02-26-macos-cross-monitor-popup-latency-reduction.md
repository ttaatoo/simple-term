# 0043-2026-02-26-macos-cross-monitor-popup-latency-reduction

## Metadata

- Date: 2026-02-26
- Sequence: 0043
- Status: active
- Scope: runtime

## Why This Entry Exists

Users observed a visible delay when quickly moving the mouse to another monitor and toggling the terminal. Existing-window show flow used an extra deferred app update before scheduling native frame restore, adding unnecessary latency.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/macos.rs`
- Upstream constraints (platform, library, policy):
  - native frame application remains deferred on main queue for GPUI re-entry safety
  - app-shell commands are processed while controller state is mutably borrowed
- Invariants already in force:
  - show on current mouse monitor
  - existing window reuse must preserve session state

## Decision and Rationale

- Decision:
  - remove the extra `cx.defer` layer in existing-window show path
  - apply monitor frame + pin level directly in `window_handle.update`
  - call `cx.activate(true)` only after scheduling that frame update for existing-window reuse
- Why this path was selected:
  - reduces cross-monitor popup latency while preserving deferred native frame safety in `macos.rs`
- Trade-offs accepted:
  - existing-window show path now performs update work inline in command handling path

## Alternatives Considered

1. Keep current logic and only optimize monitor detection
- Pros:
  - minimal code movement
- Cons:
  - does not address defer-induced latency
- Why not chosen:
  - user-visible delay remained

2. Make native frame application synchronous again
- Pros:
  - potentially lowest latency
- Cons:
  - reintroduces GPUI re-entry borrow risk
- Why not chosen:
  - safety regression

## Safe Change Playbook

When modifying this area, follow these steps:
1. Keep one deferred boundary for native frame mutation (inside `macos.rs`).
2. Avoid adding extra app-level defers in existing-window show path unless required by new borrow constraints.
3. Verify quick cross-monitor toggle behavior manually.

## Do / Avoid

Do:
- apply existing-window placement/pinning in one `window_handle.update`
- activate app after frame scheduling for hidden-window reopen

Avoid:
- chaining multiple deferred hops for reopen path
- restoring synchronous AppKit frame mutation in GPUI update context

## Typical Mistakes

- Assuming all defer layers are needed after native move is already deferred.
- Activating app before scheduling existing-window placement, causing perceived delay on target monitor.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - move pointer quickly across monitors and trigger show/hide repeatedly
  - confirm no noticeable delay and no `RefCell already borrowed` logs
- Signals of regression:
  - popup appears with lag when switching monitors quickly
  - borrow/re-entry errors return in logs

## Related Artifacts

- Related docs:
  - `docs/evolution/0039-2026-02-26-macos-deferred-window-move-to-avoid-gpui-reentry.md`
  - `docs/evolution/0042-2026-02-26-macos-existing-window-frame-restore.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/main.rs`
