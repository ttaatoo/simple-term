# 0053-2026-02-26-pin-shortcut-focus-scope-and-cursor-blink-default

## Metadata

- Date: 2026-02-26
- Sequence: 0053
- Status: active
- Scope: runtime, testing

## Why This Entry Exists

Two UX regressions needed explicit guardrails:
1. `Cmd+Backquote` (pin toggle) could surface a hidden window because it was wired as a global hotkey path.
2. Cursor blinking appeared disabled by default when blinking mode was `terminal`.

These behaviors sit at the intersection of app-shell hotkey routing and terminal cursor-style defaults, so they need durable documentation.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/terminal_view.rs`
  - `crates/simple-term/src/terminal_settings.rs`
- Upstream constraints (platform, library, policy):
  - global hotkey manager receives events even when window is hidden/unfocused
  - terminal cursor blink rendering depends on both setting mode and terminal-reported cursor style
- Invariants already in force:
  - `global_hotkey` is app-shell scope
  - `pin_hotkey` is terminal-window interaction scope
  - default user experience should include visible cursor blinking unless explicitly disabled

## Decision and Rationale

- Decision:
  - register only `global_hotkey` with `GlobalHotKeyManager`
  - handle `pin_hotkey` in `TerminalView` keydown path (focused-window scope)
  - gate pin-toggle window activation behind visible-state checks
  - treat `Blinking::TerminalControlled` as blink-enabled for default cursor style seed
- Why this path was selected:
  - enforces expected shortcut scope directly in input routing layer
  - removes hidden-window popups from non-focused `Cmd+Backquote` presses
  - restores blinking behavior without forcing users to manually switch to `on`
- Trade-offs accepted:
  - pin toggle no longer works when terminal window is unfocused by design
  - `terminal` blink mode now starts from blink-on seed and depends on later terminal cursor-style updates

## Alternatives Considered

1. Keep pin hotkey global and special-case hidden state in controller
- Pros:
  - fewer keybinding changes in view layer
- Cons:
  - still violates "focus-only shortcut" contract
- Why not chosen:
  - scope mismatch is the root issue, not just hidden-state activation

2. Keep `terminal` mode default blink seed as off
- Pros:
  - no behavior change for existing implementation
- Cons:
  - default shell sessions appear non-blinking unless explicit override is sent
- Why not chosen:
  - contradicts expected cursor blink behavior

## Safe Change Playbook

When modifying this area, follow these steps:
1. Decide shortcut scope first: app-global (`main.rs`) vs focused-view (`terminal_view.rs`).
2. Keep pin-toggle activation conditional on visibility to avoid hidden-window surprise pops.
3. Validate blink mode semantics in both style-seeding (`TerminalSettings`) and render-time blink checks.
4. Add/adjust targeted tests before changing shortcut/blink behavior.

## Do / Avoid

Do:
- keep non-global shortcuts in focused keydown handlers
- compare parsed hotkey structs instead of string-only matching
- keep blink defaults and render checks aligned

Avoid:
- registering `pin_hotkey` in global manager
- calling `activate_window()` from pin toggle when terminal is hidden
- interpreting `terminal` blink mode as implicit "off"

## Typical Mistakes

- Treating all configurable shortcuts as global for convenience.
- Restoring pin-state UI without considering hidden-window activation side effects.
- Changing blink logic in render path but forgetting default cursor-style seed semantics.

## Verification Strategy

- Required automated checks:
  - `cargo test -p simple-term-app pin_toggle_activation_requires_visible_window`
  - `cargo test -p simple-term-app pin_hotkey_matches_only_configured_combination`
  - `cargo test -p simple-term terminal_controlled_blinking_defaults_to_blinking_style`
- Recommended manual checks:
  - hide window, press `Cmd+Backquote` while another app is focused, confirm no popup
  - focus terminal window, press `Cmd+Backquote`, confirm pin toggles
  - verify cursor visibly blinks in default shell session
- Signals of regression:
  - hidden window appears on `Cmd+Backquote`
  - pin toggle stops working even when terminal is focused
  - cursor remains static under default `terminal` blink mode

## Related Artifacts

- Related docs:
  - `docs/evolution/0013-2026-02-24-tabbar-settings-panel-and-runtime-appearance-controls.md`
  - `docs/evolution/0049-2026-02-26-macos-toggle-terminal-focus-restoration.md`
  - `docs/evolution/0050-2026-02-26-home-dot-simple-term-settings-source-and-bootstrap.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/terminal_view.rs`
  - `crates/simple-term/src/terminal_settings.rs`
