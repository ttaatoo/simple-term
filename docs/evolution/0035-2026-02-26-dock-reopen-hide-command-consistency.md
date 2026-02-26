# 0035-2026-02-26-dock-reopen-hide-command-consistency

## Metadata

- Date: 2026-02-26
- Sequence: 0035
- Status: active
- Scope: runtime

## Why This Entry Exists

macOS users can reopen an already-running hidden app by clicking the Dock icon. That path made the terminal window visible without going through controller `show_terminal`, so internal `visible` state could remain stale (`false`) while the window was on screen.

When `Cmd+W` on a last tab routed into `AppCommand::HideTerminal`, `hide_terminal` would early-return on stale `visible == false`, creating a visible user regression: the window did not hide.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/terminal_view.rs`
- Upstream constraints (platform, library, policy):
  - Dock click can unhide/activate app windows outside the internal controller command path.
  - Last-tab `Cmd+W` is intentionally routed through controller command callback to keep app-shell behavior consistent.
- Invariants already in force:
  - `Cmd+W` with one tab should hide the window.
  - Hide requests must never be dropped due to stale controller visibility bookkeeping.

## Decision and Rationale

- Decision:
  - Make `hide_terminal` always process hide requests (call `cx.hide()` and set `visible = false`) without guarding on current `visible` value.
- Why this path was selected:
  - fixes Dock-reopen + `Cmd+W` regression directly at the hide gate
  - keeps last-tab hide routed through controller callback path
  - remains idempotent: repeated hide calls are safe
- Trade-offs accepted:
  - `visible` is no longer used as a precondition for `hide_terminal`; it is treated as output state after hide

## Alternatives Considered

1. Add a separate Dock-activation observer to re-sync `visible = true`
- Pros:
  - keeps strict controller-state modeling for external visibility changes
- Cons:
  - requires extra app lifecycle hooks and broader platform-specific wiring
- Why not chosen:
  - larger surface area than required for this regression

2. Bypass controller and call `cx.hide()` directly from `TerminalView`
- Pros:
  - simple view-side behavior
- Cons:
  - bypasses controller command flow and risks state drift in other paths
- Why not chosen:
  - controller-mediated hide routing is an explicit project invariant

## Safe Change Playbook

When modifying macOS app-shell visibility behavior, follow these steps:
1. Treat hide operations as idempotent and safe to execute even when controller visibility state may be stale.
2. Keep last-tab `Cmd+W` routing through controller callback/command path (`AppCommand::HideTerminal`).
3. Verify external activation flows (Dock, menubar icon, hotkey) do not break hide semantics.

## Do / Avoid

Do:
- keep hide-path logic tolerant to stale visibility bookkeeping
- prefer command routing through controller for window hide/show actions

Avoid:
- gating hide requests on `visible` alone
- introducing parallel hide paths that bypass controller state updates

## Typical Mistakes

- Assuming every “window became visible” transition passes through `show_terminal`.
- Treating controller `visible` as fully authoritative when macOS can unhide windows externally (Dock reopen).
- Reintroducing an early-return guard in `hide_terminal` that drops valid hide requests.

## Verification Strategy

- Required automated checks:
  - `cargo test --workspace`
  - `cargo check --workspace`
- Recommended manual checks:
  - hide app, reopen via Dock icon, press `Cmd+W` with one tab: window hides
  - reopen via menubar icon, press `Cmd+W` with one tab: window hides
  - with multiple tabs, `Cmd+W` still closes only active tab
- Signals of regression:
  - Dock-opened window ignores last-tab `Cmd+W`
  - toggle/hide behavior diverges between Dock-open and menubar-open flows

## Related Artifacts

- Related docs:
  - `docs/evolution/0023-2026-02-25-last-tab-close-hides-window-via-controller-path.md`
  - `docs/evolution/0025-2026-02-25-unified-macos-app-shell-without-dock-mode.md`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/main.rs`
