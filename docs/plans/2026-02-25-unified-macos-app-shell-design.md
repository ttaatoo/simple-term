# Unified macOS App Shell Design (No Dock Mode)

## Goal

Remove `dock_mode` entirely and unify macOS runtime behavior into one app-shell flow:

- app startup opens the main terminal window immediately
- menubar icon remains available (when enabled)
- global hotkey remains available
- menubar icon and hotkey both perform **show/hide toggle**
- window keeps normal desktop semantics (move/resize/minimize)

This is an intentional breaking change focused on long-term simplicity.

## Scope

In scope:

- remove `DockMode` from settings schema and runtime logic
- replace dual startup/controller paths with one controller
- keep outside-click auto-hide and last-tab-hide behavior controller-driven
- remove Dock Mode controls from settings drawer UI

Out of scope:

- backwards compatibility migration for old config keys
- non-macOS app-shell parity changes

## Chosen Approach

Use a single macOS controller (`AppShellController`) as the only owner of window visibility state.

Why this approach:

- removes branch drift between `regular` and `menubar_only`
- centralizes all show/hide transitions in one state machine
- preserves existing menubar and hotkey integrations without mode-specific forks

## Runtime Architecture

`main.rs` boot behavior:

1. load settings
2. on macOS, run unified shell controller
3. on non-macOS, keep existing standard window startup

`AppShellController` owns:

- `settings`
- `command_tx`/`command_rx`
- `terminal_window: Option<WindowHandle<TerminalView>>`
- `visible: bool`
- `status_item: Option<StatusItemHandle>`
- `hotkey_manager: Option<GlobalHotKeyManager>`

`bootstrap()` order:

1. apply regular activation policy
2. install status item (if `settings.button`)
3. install global hotkey
4. open initial window and set `visible = true`

## Command Flow

Commands remain:

- `ToggleTerminal`
- `HideTerminal`

Behavior:

- `ToggleTerminal`: if visible -> hide; else -> show/activate (recreate stale window if needed)
- `HideTerminal`: hide only when visible

Event sources:

- menubar icon click -> `ToggleTerminal`
- global hotkey press -> `ToggleTerminal`
- outside-click deactivation callback -> `HideTerminal`
- last-tab close callback (`TerminalView`) -> `HideTerminal`

## Window Behavior

- Use normal window (`WindowKind::Normal`) with movable/resizable/minimizable flags.
- Apply panel placement only when creating a new window (initial create or handle recovery).
- Do not force re-position/re-size on every toggle.

## Settings and UI Changes

`TerminalSettings` changes:

- remove `DockMode` enum
- remove `dock_mode` field
- keep `button`, `global_hotkey`, `auto_hide_on_outside_click`, `panel_top_inset`

Settings drawer changes:

- remove Dock Mode section and related toggle handlers/tests
- keep other controls unchanged

## Error Handling

- hotkey parse failure: fallback to `control+Backquote`
- hotkey registration failure: warn and continue without global hotkey
- status item init failure: warn and continue
- stale window handle on show: clear handle and recreate window
- window creation failure: log error and keep `visible = false`

## Verification

Automated:

- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`

Manual macOS:

- startup opens terminal window
- menubar click toggles show/hide
- global hotkey toggles show/hide
- outside click hides (when enabled)
- window position/size remain user-controlled across toggles
