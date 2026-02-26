# 0042-2026-02-26-macos-existing-window-frame-restore

## Metadata

- Date: 2026-02-26
- Sequence: 0042
- Status: active
- Scope: runtime

## Why This Entry Exists

Per-monitor size persistence existed in settings, but existing hidden windows only restored position on show. Size remained from the previously used monitor, causing cross-monitor size bleed.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/macos.rs`
- Upstream constraints (platform, library, policy):
  - existing-window show path reuses the same `NSWindow` instance
  - frame updates must stay on main queue to avoid GPUI re-entry issues
- Invariants already in force:
  - show on current mouse monitor
  - per-monitor placement (position + size) is persisted in settings

## Decision and Rationale

- Decision:
  - change macOS apply step from top-left-only move to full-frame apply (x/y/width/height)
  - keep deferred main-queue frame mutation strategy
- Why this path was selected:
  - directly fixes mismatch between persisted size and applied size in existing-window reuse path
  - preserves prior re-entry safety constraints from deferred native callbacks
- Trade-offs accepted:
  - show path now always applies full frame, even when only position changed

## Alternatives Considered

1. Resize only when monitor key changes
- Pros:
  - fewer frame updates
- Cons:
  - more state branching and edge cases around stale monitor detection
- Why not chosen:
  - unnecessary complexity for little benefit

2. Recreate window on monitor switch
- Pros:
  - guaranteed fresh bounds
- Cons:
  - destroys active terminal session state
- Why not chosen:
  - behavior regression

## Safe Change Playbook

When modifying this area, follow these steps:
1. Ensure existing-window show path applies full frame, not only top-left.
2. Keep AppKit frame mutation deferred on main queue.
3. Verify size restoration across at least two monitors with different saved sizes.

## Do / Avoid

Do:
- treat per-monitor placement as a full frame contract
- keep native frame application isolated in `macos.rs`

Avoid:
- restoring only position in reuse paths
- synchronous frame changes inside GPUI update callbacks

## Typical Mistakes

- Assuming `setFrameTopLeftPoint_` is sufficient after introducing size persistence.
- Restoring size only for new window creation but not existing-window reuse.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - save different sizes on monitor A and B, toggle show on each monitor, confirm correct size restores independently
- Signals of regression:
  - monitor A size appears on monitor B after toggle

## Related Artifacts

- Related docs:
  - `docs/evolution/0041-2026-02-26-macos-per-monitor-window-size-persistence.md`
  - `docs/evolution/0039-2026-02-26-macos-deferred-window-move-to-avoid-gpui-reentry.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/macos.rs`
