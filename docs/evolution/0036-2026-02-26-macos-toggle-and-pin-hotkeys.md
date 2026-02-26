# 0036-2026-02-26-macos-toggle-and-pin-hotkeys

## Metadata

- Date: 2026-02-26
- Sequence: 0036
- Status: active
- Scope: runtime

## Why This Entry Exists

The macOS app shell had one global shortcut (`global_hotkey`) for show/hide behavior. Users requested two explicit shortcuts:

- one shortcut to toggle terminal visibility
- one shortcut to pin/unpin the terminal so it remains always visible

This is not obvious from file diffs because the behavior spans settings schema defaults, global hotkey registration, app-shell command routing, and native window-level controls.

## System Context

- Relevant directories/modules:
  - `crates/simple-term/src/terminal_settings.rs`
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/macos.rs`
  - `apps/simple-term/src/terminal_view.rs`
- Upstream constraints (platform, library, policy):
  - `global-hotkey` parsing must degrade gracefully on invalid strings.
  - macOS window-level pinning must be applied through AppKit window handles.
  - hide/show authority remains in `AppShellController`.
- Invariants already in force:
  - app-shell behavior is controller-driven (`AppCommand`), not view-driven.
  - hotkey registration failure must warn and continue without crashing startup.

## Decision and Rationale

- Decision:
  - Keep `global_hotkey` as the show/hide toggle and set its default to `command+F4`.
  - Add `pin_hotkey` (default `command+Backquote`) to toggle controller pin state.
  - While pinned, hide requests are ignored and window level is set to floating (always-on-top).
  - Preserve compatibility for user wording `cmd+r5`, and remap legacy `command+F5` configs, to the current default toggle (`command+F4`).
- Why this path was selected:
  - minimal schema expansion (`pin_hotkey`) with backward-compatible defaults
  - preserves existing command bus (`AppCommand`) and app-shell control ownership
  - pin behavior is explicit and reversible without introducing new window objects
- Trade-offs accepted:
  - pinned mode intentionally blocks hide requests until unpinned
  - pinning uses macOS-specific window-level value and remains a macOS-only behavior path

## Alternatives Considered

1. Reuse `global_hotkey` for both behaviors by context
- Pros:
  - no new setting field
- Cons:
  - ambiguous UX; no deterministic pin shortcut
- Why not chosen:
  - user request explicitly asks for separate shortcuts

2. Implement pin purely as “disable outside-click auto-hide” without native window level
- Pros:
  - fewer native calls
- Cons:
  - does not deliver “always show” semantics when other windows overlap
- Why not chosen:
  - misses expected pinned-window behavior

## Safe Change Playbook

When modifying macOS shortcut or pin behavior, follow these steps:
1. Keep shortcut parsing + fallback centralized in `AppShellController`; never spread parse fallbacks across unrelated modules.
2. Preserve controller command ownership (`ToggleTerminal`, `TogglePinned`, `HideTerminal`).
3. Apply pin level in both window-create and window-reuse paths; pin state must survive reopen/toggle paths.
4. Keep hide gate explicit (`should_process_hide_terminal_request`) and include pin-state checks there.
5. If adding new hotkeys, guard against duplicate IDs and degrade with warnings instead of aborting startup.

## Do / Avoid

Do:
- keep hotkey defaults and sanitization aligned between settings defaults and runtime fallbacks
- keep pin behavior idempotent (reapplying level must be safe)
- keep terminal window visibility/pin transitions routed through controller methods

Avoid:
- direct view-side window-level manipulation for app-shell behavior
- introducing hide paths that bypass `AppShellController`
- assuming all config hotkey strings are parseable without fallback

## Typical Mistakes

- Registering multiple global shortcuts without handling conflicts (duplicate key combinations).
- Applying pinned level only on first window creation and forgetting existing-window update path.
- Dropping hide requests based only on stale visibility flags without considering pin-state intent.

## Verification Strategy

- Required automated checks:
  - `cargo fmt --all`
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - press `Cmd+F4` to show/hide terminal
  - press `Cmd+\`` to pin, click outside app, verify terminal stays visible
  - while pinned, try hide paths (outside-click, last-tab close) and confirm they do not hide
  - unpin with `Cmd+\`` and confirm hide behavior returns
- Signals of regression:
  - pin toggle does not keep window visible
  - window loses always-on-top behavior after reopen
  - show/hide shortcut registration fails on valid defaults

## Related Artifacts

- Related docs:
  - `docs/evolution/0025-2026-02-25-unified-macos-app-shell-without-dock-mode.md`
  - `docs/evolution/0035-2026-02-26-dock-reopen-hide-command-consistency.md`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/macos.rs`
  - `crates/simple-term/src/terminal_settings.rs`
