# 0049-2026-02-26-macos-toggle-terminal-focus-restoration

## Metadata

- Date: 2026-02-26
- Sequence: 0049
- Status: active
- Scope: runtime

## Why This Entry Exists

Toggling the terminal with `Cmd+F4` could reopen a visible window that did not accept keyboard input until users clicked inside it. The behavior came from window/app activation being restored without explicitly restoring terminal view focus.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/terminal_view.rs`
- Upstream constraints (platform, library, policy):
  - keyboard input handlers are bound on focus-tracked terminal elements
  - previous fixes removed window-level activation calls inside update closures to avoid GPUI re-entry risks
- Invariants already in force:
  - `Cmd+F4` show should allow immediate typing without extra click
  - show flow must not reintroduce re-entrant window activation hazards

## Decision and Rationale

- Decision:
  - expose `TerminalView::focus_terminal(...)` and call it in both existing-window and new-window show paths
  - keep existing activation ownership split (`cx.activate(true)` vs native deferred activation) unchanged
- Why this path was selected:
  - restores input readiness at the exact layer where key routing is defined (focused terminal view)
  - avoids reintroducing native window activation calls in hotkey update closures
- Trade-offs accepted:
  - focus restore is now an explicit show-path responsibility in controller code

## Alternatives Considered

1. Rely on app activation only (`cx.activate(true)`) and no explicit focus restore
- Pros:
  - no additional focus calls
- Cons:
  - intermittent no-input state remains
- Why not chosen:
  - does not satisfy immediate-typing UX invariant

2. Re-add window-level activation (`window.activate_window`) inside show update closures
- Pros:
  - likely forces key window behavior
- Cons:
  - risks reintroducing re-entrant callback/borrow failures addressed in prior fixes
- Why not chosen:
  - violates recent safety boundary for hotkey show path

## Safe Change Playbook

When modifying this area, follow these steps:
1. Keep terminal input focus restoration explicit in show paths (existing window + newly opened window).
2. Keep native activation ordering contracts in `macos.rs` intact; do not substitute focus restoration with synchronous native activation calls.
3. Verify both same-monitor and cross-monitor reopen flows still accept typing immediately.

## Do / Avoid

Do:
- focus the terminal view (`FocusHandle`) when reopening/toggling the terminal
- keep focus restore separate from native window activation ownership logic

Avoid:
- assuming app/window activation alone implies GPUI element focus
- reintroducing `window.activate_window()` in borrowed update paths as a focus workaround

## Typical Mistakes

- Restoring visibility and app activation but forgetting to restore terminal element focus.
- Fixing focus regressions by adding native activation calls that re-open previous re-entry hazards.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
- Recommended manual checks:
  - hide terminal, press `Cmd+F4`, and type immediately without mouse click
  - repeat quick toggle cycles and verify first keypress always reaches shell
  - repeat on another monitor to ensure focus restore does not break activation-ordering fixes
- Signals of regression:
  - reopened window appears but typing does nothing until click
  - `RefCell already borrowed` logs return during rapid toggle flows

## Related Artifacts

- Related docs:
  - `docs/evolution/0039-2026-02-26-macos-deferred-window-move-to-avoid-gpui-reentry.md`
  - `docs/evolution/0047-2026-02-26-macos-cross-monitor-activation-ordering-without-reentry.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/terminal_view.rs`
