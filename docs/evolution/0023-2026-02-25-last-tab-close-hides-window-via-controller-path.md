# 0023-2026-02-25-last-tab-close-hides-window-via-controller-path

## Metadata

- Date: 2026-02-25
- Sequence: 0023
- Status: active
- Scope: runtime

## Why This Entry Exists

Tab close behavior previously ignored `Cmd+W` when only one tab existed (`close_tab` returned early on `tabs.len() <= 1`). That created a dead-end interaction where users could not dismiss the terminal window with the same close-tab shortcut at the last-tab boundary.

The fix requires preserving controller-managed visibility state in menubar quick-terminal mode, so this behavior contract is documented explicitly.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
  - `apps/simple-term/src/main.rs`
- Upstream constraints (platform, library, policy):
  - In quick-terminal (`menubar_only`) mode, window visibility state is tracked by `QuickTerminalController.visible`.
  - `QuickTerminalController` state is updated through `AppCommand::HideTerminal`, not by direct view-local state mutation.
  - `TerminalView` is reused across startup modes, so close behavior must work with and without a controller callback.
- Invariants already in force:
  - tab-management shortcuts stay on platform modifier (`Cmd+W`), preserving shell `Ctrl+W`.
  - closing non-last tabs must continue to select a deterministic next active tab.

## Decision and Rationale

- Decision:
  - Treat `Cmd+W` on the last tab as a hide-window request instead of a no-op.
  - Route hide through a new `TerminalView` callback field (`on_hide_terminal_requested`) when provided; fallback to `cx.hide()` when no callback exists.
  - Add a unit-level regression guard (`should_hide_window_when_closing_tab`) to preserve last-tab behavior.
- Why this path was selected:
  - preserves expected shortcut ergonomics
  - keeps quick-terminal controller state consistent by using its existing hide-command path
  - avoids closing/destroying tab state unnecessarily
- Trade-offs accepted:
  - behavior for `Cmd+W` now bifurcates by tab count (close tab vs hide window)
  - hide fallback without controller uses app-level hide semantics

## Alternatives Considered

1. Keep current behavior (no-op on last tab)
- Pros:
  - no code changes
- Cons:
  - inconsistent close behavior at tab-count boundary
  - poor UX for keyboard-driven users
- Why not chosen:
  - fails requested behavior and leaves interaction gap

2. Always call `cx.hide()` directly from `TerminalView`
- Pros:
  - simple implementation
- Cons:
  - bypasses `QuickTerminalController.visible` bookkeeping path
  - increases risk of state drift in quick-terminal mode
- Why not chosen:
  - controller-aware callback is safer for long-term maintainability

## Safe Change Playbook

When modifying tab close or window-hide behavior, follow these steps:
1. Keep tab-count decision logic centralized (`should_hide_window_when_closing_tab`) to avoid scattered edge-case checks.
2. If a mode tracks visibility in a controller, prefer callback/command routing over direct `cx.hide()` calls.
3. Preserve non-last-tab close invariants (`next_active_index_after_close`, title updates, notify behavior).
4. Add/maintain focused unit coverage for boundary behavior (0/1/2 tabs) and run workspace checks.

## Do / Avoid

Do:
- keep hide requests controller-aware when a callback exists
- keep `Cmd+W` semantics explicit in `handle_tab_keybinding` -> `close_tab`
- preserve deterministic active-tab selection for multi-tab closes

Avoid:
- adding direct app-hide calls in controller-managed paths without updating controller state
- reintroducing `tabs.len() <= 1` early-return no-op behavior
- mixing tab-close and window-close semantics across multiple call sites

## Typical Mistakes

- Treating “last tab” as an exceptional no-op instead of a hide request.
- Calling `cx.hide()` unconditionally in quick-terminal mode and forgetting controller state synchronization.
- Editing keybinding branches without keeping boundary tests aligned.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
- Recommended manual checks:
  - with multiple tabs, `Cmd+W` closes only the active tab
  - with one tab remaining, `Cmd+W` hides the terminal window instead of doing nothing
  - in menubar quick-terminal mode, toggling after last-tab hide reopens correctly (no stale visible-state behavior)
- Signals of regression:
  - `Cmd+W` on last tab does nothing
  - quick-terminal toggle gets “stuck hidden” or “stuck visible” after last-tab hide

## Related Artifacts

- Related docs:
  - `docs/evolution/0008-2026-02-24-macos-menubar-quick-terminal-mode.md`
  - `docs/evolution/0018-2026-02-25-macos-menubar-window-behavior-and-status-icon-parity.md`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/terminal_view.rs`
  - `apps/simple-term/src/main.rs`
