# 0047-2026-02-26-macos-cross-monitor-activation-ordering-without-reentry

## Metadata

- Date: 2026-02-26
- Sequence: 0047
- Status: active
- Scope: runtime

## Why This Entry Exists

Cross-monitor reopen still emitted `RefCell already borrowed` when `Cmd+F4` was pressed after moving the pointer to another monitor. A prior fix removed monitor flash but reintroduced synchronous native frame mutation inside controller-owned update flow.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/macos.rs`
- Upstream constraints (platform, library, policy):
  - `show_terminal` executes while controller state is mutably borrowed
  - synchronous AppKit frame changes can trigger observer callbacks that re-enter GPUI update paths
  - hidden-window cross-monitor reopen must avoid showing stale frame on old monitor
- Invariants already in force:
  - window should appear on pointer monitor when shown
  - no re-entrant borrow failures during hotkey toggle

## Decision and Rationale

- Decision:
  - keep all frame mutation deferred to main-queue callback (`dispatch_async_f`)
  - when a hidden window needs cross-monitor move, defer activation into the same native callback and execute it only after `setFrame`
  - return activation ownership from `macos::move_window_to` so controller skips `cx.activate(true)` when native callback will activate
- Why this path was selected:
  - removes stale-frame flash by ordering `frame -> unhide/orderFront -> activate`
  - avoids synchronous `setFrame` during borrowed controller/update scope
- Trade-offs accepted:
  - activation path is now split by move result (`app`-handled vs `native`-handled)

## Alternatives Considered

1. Keep synchronous hidden-window `setFrame:display:NO`
- Pros:
  - simple ordering
- Cons:
  - can still trigger callback/re-entry while controller borrow is active
- Why not chosen:
  - reproduced `RefCell already borrowed`

2. Keep app-level `cx.activate(true)` before deferred move
- Pros:
  - simple existing control flow
- Cons:
  - old-monitor flash race remains
- Why not chosen:
  - user-visible correctness failure

## Safe Change Playbook

When modifying this area, follow these steps:
1. Do not run native frame mutation synchronously from controller update paths.
2. If activation depends on frame relocation, perform activation in the same deferred native callback after frame apply.
3. Keep controller-level activation conditional on move result to avoid double-activation races.

## Do / Avoid

Do:
- treat activation ownership as explicit state (`ActivationHandledByApp` vs `ActivationDeferredToNative`)
- keep hidden-window reveal ordered after frame apply

Avoid:
- synchronous `setFrame` from borrowed app/controller update stacks
- unconditional `cx.activate(true)` when native callback is responsible for activation

## Typical Mistakes

- Optimizing monitor-flash timing by reintroducing synchronous native frame APIs.
- Deferring move but not activation, which preserves stale-frame flash race.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - hide terminal, move pointer between monitors, press `Cmd+F4` repeatedly
  - verify no old-monitor flash and no `RefCell already borrowed` logs
  - verify same-monitor reopen remains responsive
- Signals of regression:
  - `RefCell already borrowed` on hotkey reopen
  - old-monitor flash before final monitor placement

## Related Artifacts

- Related docs:
  - `docs/evolution/0039-2026-02-26-macos-deferred-window-move-to-avoid-gpui-reentry.md`
  - `docs/evolution/0046-2026-02-26-macos-hidden-window-frame-apply-ordering.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/macos.rs`
