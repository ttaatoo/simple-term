# Local Pin Hotkey and Tab-Bar Pin Indicator Design

## Goal

Fix pin behavior and visibility semantics on macOS:

- `Cmd+\`` must no longer be a global shortcut.
- `Cmd+\`` should only work when the terminal window is focused.
- pin/unpin should only change pinned state (not show/hide).
- add a clickable tab-bar indicator showing pinned (`📌`) vs unpinned (`○`) state.
- add settings-panel recording for `pin_hotkey`.

## Scope

In scope:

- macOS app-shell hotkey registration behavior in `apps/simple-term/src/main.rs`
- pin-state command routing and view synchronization
- window-local pin-hotkey handling in `apps/simple-term/src/terminal_view.rs`
- tab-bar pin status indicator and click-to-toggle interaction
- settings panel pin-hotkey recording UI and persistence

Out of scope:

- redesigning existing show/hide global hotkey behavior
- non-macOS pin UX parity
- menubar-specific pin controls

## Current Problem

`pin_hotkey` is currently registered as an OS global hotkey, so `Cmd+\`` can trigger outside terminal focus and feels like a global action. It also risks mixing semantics between pinning and visibility transitions.

## Chosen Approach

Keep `AppShellController` as the only source of truth for `pinned` and keep all app-shell state transitions controller-driven.

Implementation direction:

1. register only `global_hotkey` at OS-global layer.
2. handle `pin_hotkey` as window-local keybinding inside `TerminalView`.
3. when view requests pin toggle (hotkey or tab-bar click), route to controller via callback/command.
4. controller toggles `pinned`, applies native window level, and pushes current pin state back to view.

Why this path:

- preserves existing architectural invariant (controller owns app-shell state).
- removes unintended global behavior for pinning.
- keeps indicator state consistent with actual native window level.

## Architecture and Data Flow

### 1) Global hotkeys (`main.rs`)

- `install_global_hotkeys()` should register only `global_hotkey`.
- keep parsing fallback behavior for `global_hotkey`.
- stop registering `pin_hotkey` with `GlobalHotKeyManager`.

### 2) Pin requests from view (`main.rs` + `terminal_view.rs`)

- pass a new callback into `TerminalView`:
  - `on_toggle_pin_requested: Arc<dyn Fn() + Send + Sync>`
- callback sends `AppCommand::TogglePinned`.

### 3) Controller applies pin and syncs UI (`main.rs`)

- `toggle_terminal_pin()` remains the only place that mutates `self.pinned`.
- after toggling, apply `macos::set_window_pinned(window, pinned)` as today.
- push current `pinned` to the view via `window_handle.update(...)` so indicator state always reflects source-of-truth.

### 4) Local pin hotkey (`terminal_view.rs`)

- add pin-hotkey matching helper based on existing hotkey tokenization/validation path.
- in `on_key_down`, before terminal-input mapping, check whether pressed keystroke matches `settings.pin_hotkey`.
- when matched: trigger `on_toggle_pin_requested` and consume event.
- no visibility toggle behavior on this path.

### 5) Tab-bar indicator (`terminal_view.rs`)

- add right-side control next to settings button:
  - shows `📌` when pinned
  - shows `○` when unpinned
- click action triggers `on_toggle_pin_requested`.
- style aligned with existing compact tab-bar controls.

### 6) Pin-hotkey recording in settings (`terminal_view.rs`)

- mirror existing `Show/Hide Shortcut` recorder with `Pin/Unpin Shortcut`.
- persist to `settings.pin_hotkey`.
- call existing `on_hotkeys_updated(global_hotkey, pin_hotkey)` so config is saved and runtime state updates stay centralized.
- reject updates where `pin_hotkey == global_hotkey` to avoid ambiguous shortcut semantics.

## Error Handling and Fallback

- invalid pin-hotkey capture: do not apply new value; keep prior setting.
- `pin_hotkey` empty in config: keep existing default fallback (`command+Backquote`).
- view sync failure due stale handle: controller remains authoritative; state re-applied next valid window update.
- hide requests remain gated by `!pinned` to preserve existing pinned visibility invariant.

## Verification Plan

Automated:

- `cargo check --workspace`
- `cargo test --workspace`

Manual macOS:

- `Cmd+F4` still toggles window globally.
- `Cmd+\`` does nothing when terminal is not focused.
- with terminal focused, `Cmd+\`` toggles only pin state.
- clicking tab-bar indicator toggles pin and updates icon immediately.
- pinned state blocks outside-click hide and last-tab hide.
- settings panel records `pin_hotkey`, persists after restart, and rejects conflict with show/hide hotkey.

## Risks

- adding view/controller callback wiring can introduce stale-state mismatch if not synchronized after every toggle.
- key-routing order mistakes may accidentally send pin hotkey through terminal input path.

Mitigation:

- keep pin mutation/controller sync in a single function (`toggle_terminal_pin`).
- add targeted tests for key matching, UI state transitions, and conflict rejection.
