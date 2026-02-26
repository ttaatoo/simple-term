# 0039-2026-02-26-macos-deferred-window-move-to-avoid-gpui-reentry

## Metadata

- Date: 2026-02-26
- Sequence: 0039
- Status: active
- Scope: runtime

## Why This Entry Exists

After enabling monitor-follow behavior on every show request, users observed runtime errors when toggling with `Cmd+F4`:

- `RefCell already borrowed`

The failure appeared when moving between monitors and showing an existing hidden window.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/macos.rs`
- Upstream constraints (platform, library, policy):
  - `AppShellController::show_terminal` runs inside GPUI `App::update` context
  - direct AppKit frame moves (`NSWindow::setFrameTopLeftPoint_`) can synchronously trigger window move/resize notifications
  - GPUI callbacks servicing those notifications require mutable app/view access
- Invariants already in force:
  - terminal window should appear on monitor under current mouse location when shown
  - app-shell flows must not panic or emit borrow errors during command handling

## Decision and Rationale

- Decision:
  - keep monitor-targeted placement logic
  - keep existing-window command handling out of the immediate hotkey update cycle (`App::defer`)
  - execute native frame move asynchronously on macOS main queue (`dispatch_async_f` + `_dispatch_main_q`) inside `macos::move_window_to`
  - avoid synchronous `window.activate_window()` calls from `show_terminal` update closures; rely on app-level activation (`cx.activate(true)`)
  - retain/release the native `NSWindow` pointer across deferred native callback
- Why this path was selected:
  - preserves monitor-follow behavior without forcing window recreation
  - avoids synchronous re-entry into GPUI while `App::update` is already borrowing app/window state
- Trade-offs accepted:
  - move is applied one run-loop turn later than scheduling call
  - existing-window show flow now uses a two-step sequence (immediate liveness check, deferred mutation) plus deferred native move callback
  - show flow no longer performs explicit window-level activation inside controller update closures

## Alternatives Considered

1. Keep synchronous move call and suppress errors
- Pros:
  - no structural change
- Cons:
  - borrow errors remain and can mask real runtime faults
- Why not chosen:
  - violates non-panicking app-shell invariant

2. Recreate window instead of reusing existing hidden window
- Pros:
  - avoids in-place move edge cases
- Cons:
  - destroys active terminal session/tabs and worsens UX
- Why not chosen:
  - unacceptable behavior regression

## Safe Change Playbook

When changing macOS window placement behavior:
1. Treat native frame moves as potentially re-entrant with GPUI observers.
2. Do not execute `NSWindow` frame mutations synchronously inside controller update paths.
3. If movement is triggered from a hotkey/controller command, defer the existing-window update (`App::defer`) before calling `move_window_to`.
4. If native frame mutation can emit immediate platform callbacks, queue the native call with `dispatch_async_f` on the main queue.
5. Keep placement math (`resolve_panel_placement`) independent from execution strategy (sync vs deferred).

## Do / Avoid

Do:
- keep monitor resolution based on current mouse location at show time
- isolate AppKit interop in `macos.rs`
- verify show/hide hotkey flow after any frame-move changes

Avoid:
- direct synchronous `setFrameTopLeftPoint_` calls from code running in GPUI update stacks
- performing existing-window move/pin/window-activation inside the same update cycle as the hotkey command handler
- coupling placement calculations to callback timing assumptions
- using deferred native callbacks without retaining ObjC objects for lifetime safety

## Typical Mistakes

- Calling AppKit move APIs directly during `show_terminal` update handling.
- Assuming window move callbacks are always deferred by the platform.
- Forgetting to defer the existing-window update path while only changing the new-window path.
- Queueing native callbacks without explicit retain/release of `NSWindow`.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - with 2+ monitors, move pointer across monitors and press `Cmd+F4` repeatedly
  - confirm window appears on cursor monitor and no `RefCell already borrowed` logs appear
  - verify pin/unpin still applies window level correctly after show
- Field confirmation:
  - 2026-02-26: user-verified fix in real multi-monitor flow; moving pointer to another monitor and pressing `Cmd+F4` no longer emits `RefCell already borrowed`
- Signals of regression:
  - borrow error logs reappear when toggling
  - window remains on previous monitor after toggle
  - pin/show interactions stop bringing the app to foreground consistently

## Related Artifacts

- Related docs:
  - `docs/evolution/0038-2026-02-26-macos-show-terminal-on-mouse-monitor.md`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/macos.rs`
  - `apps/simple-term/src/main.rs`
