# 0046-2026-02-26-macos-hidden-window-frame-apply-ordering

## Metadata

- Date: 2026-02-26
- Sequence: 0046
- Status: superseded
- Scope: runtime

## Why This Entry Exists

Reopening the terminal on a different monitor could briefly render on the old monitor before jumping to the target monitor. This was caused by activation occurring before deferred native frame mutation completed.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/macos.rs`
  - `apps/simple-term/src/main.rs`
- Upstream constraints (platform, library, policy):
  - app activation (`cx.activate(true)`) can make the existing hidden `NSWindow` visible immediately
  - existing frame updates were deferred via `dispatch_async_f` for GPUI re-entry safety
- Invariants already in force:
  - show-on-cursor-monitor behavior
  - no synchronous re-entry hazards for visible-window frame changes

## Decision and Rationale

- Decision:
  - apply frame synchronously when the native `NSWindow` is hidden (`isVisible == false`) using `setFrame:display:NO`
  - keep deferred main-queue frame mutation for visible windows
- Why this path was selected:
  - removes the stale-frame visibility race in hidden-window reopen path
  - preserves deferred-safety boundary for paths that can trigger live callbacks
- Trade-offs accepted:
  - hidden-window path now has dual execution strategy (sync for hidden, async for visible)

## Alternatives Considered

1. Keep fully deferred mutation and make window transparent until move applies
- Pros:
  - avoids synchronous frame mutation
- Cons:
  - masks symptom, adds visual/state complexity
- Why not chosen:
  - not a root-cause fix

2. Restore fully synchronous frame mutation for all windows
- Pros:
  - simplest ordering
- Cons:
  - risks reintroducing GPUI borrow/re-entry failures on visible windows
- Why not chosen:
  - safety regression risk

## Safe Change Playbook

When modifying this area, follow these steps:
1. Treat hidden-window and visible-window frame mutation as separate risk classes.
2. Keep hidden-window frame apply before activation visibility transitions.
3. Keep deferred main-queue mutation for visible windows unless re-entry impact is revalidated.

## Do / Avoid

Do:
- check `isVisible` before choosing frame-apply strategy
- use `display:NO` for hidden-window synchronous frame apply

Avoid:
- delaying hidden-window frame application until after activation
- introducing visual masking tricks (alpha fades) as primary correctness fixes

## Typical Mistakes

- Assuming one deferred strategy fits both hidden and visible windows.
- Fixing monitor flash with opacity hacks instead of correcting frame/activation ordering.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - with 2+ monitors, hide terminal, move pointer to another monitor, press `Cmd+F4`
  - verify no old-monitor flash before appearing on target monitor
- Signals of regression:
  - terminal appears on old monitor then jumps
  - `RefCell already borrowed` or move/re-entry errors while toggling

## Related Artifacts

- Related docs:
  - `docs/evolution/0039-2026-02-26-macos-deferred-window-move-to-avoid-gpui-reentry.md`
  - `docs/evolution/0043-2026-02-26-macos-cross-monitor-popup-latency-reduction.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/macos.rs`

Superseded by:
- `docs/evolution/0047-2026-02-26-macos-cross-monitor-activation-ordering-without-reentry.md`
