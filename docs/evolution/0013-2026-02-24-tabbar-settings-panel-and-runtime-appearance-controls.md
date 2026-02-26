# 0013-2026-02-24-tabbar-settings-panel-and-runtime-appearance-controls

## Metadata

- Date: 2026-02-24
- Sequence: 0013
- Status: superseded
- Scope: runtime, architecture, testing

## Why This Entry Exists

The terminal now includes a top-right settings surface inside the tab bar for runtime typography and app-shell mode changes. This introduces a new class of invariants: UI controls that mutate terminal geometry and persisted settings while the app is running. Cursor rendering now also depends on coordinated state across terminal core and UI (shape selection + blink redraw), so those contracts need to be explicit to avoid regressions.

This entry is superseded for panel placement by `0015-2026-02-24-settings-drawer-v1-and-live-persistence.md` (inline panel replaced by drawer) but remains relevant for runtime typography/cursor invariants.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
  - `crates/simple-term/src/terminal_settings.rs`
  - `apps/simple-term/src/macos.rs`
- Upstream constraints (platform, library, policy):
  - terminal row/column geometry depends on font metrics and viewport size
  - pointer-to-grid conversion assumes stable top tab-bar offset
  - menubar/dock policy behavior is macOS-specific (AppKit activation policy)
- Invariants already in force:
  - terminal input and rendering behavior must remain tab-scoped
  - tab bar layout must stay stable across state changes
  - persisted settings must stay valid JSON and remain load-compatible

## Decision and Rationale

- Decision:
  - Add an inline expandable settings panel in the tab bar right controls.
  - Provide runtime controls for font family cycling, font size +/- adjustments, and dock-mode toggle.
  - Add `TerminalSettings::save` to persist updates to `settings.json`.
  - Wire `TerminalSettings::default_cursor_style()` into `Term` config and render cursor geometry by shape (`block`, `beam`, `underline`, `hollow`) instead of always painting a full-cell block.
  - Drive blink visibility from UI timer ticks while honoring settings policy (`off` / `on` / terminal-controlled).
- Why this path was selected:
  - avoids introducing overlay/popup layout complexity in current GPUI structure
  - keeps control context close to tabs where users already navigate terminal sessions
  - reuses existing settings model rather than introducing a separate preferences store
- Trade-offs accepted:
  - compact inline controls are denser than a dedicated popover
  - dock mode toggle persistence is immediate, but full menubar workflow still depends on app shell lifecycle and platform behavior

## Alternatives Considered

1. Floating popover settings panel
- Pros:
  - stronger visual separation and more room for labels
- Cons:
  - higher layout/z-order complexity and regression risk
- Why not chosen:
  - unnecessary complexity for initial scope

2. Separate settings window/preferences page
- Pros:
  - scalable for many future options
- Cons:
  - slower quick-adjust workflow; breaks "top-right panel" UX
- Why not chosen:
  - mismatched with requested interaction model

## Safe Change Playbook

When modifying this area, follow these steps:
1. Keep tab bar height and terminal bounds assumptions synchronized; if top chrome height changes, update viewport/grid calculations and pointer mapping together.
2. Route runtime typography updates through shared metric recomputation and grid resync logic; do not mutate `font`/`font_size` without resizing all tabs.
3. Persist settings through `TerminalSettings::save` after user-triggered changes and keep clamping/sanitization constraints consistent with load-time rules.
4. If dock mode behavior changes, keep AppKit-specific effects behind macOS gates and preserve non-macOS compilation paths.
5. Keep cursor responsibilities split cleanly: terminal core owns cursor state (`shape`, terminal blink flag), UI owns blink cadence and repaint scheduling.

## Do / Avoid

Do:
- keep runtime appearance controls in `TerminalView` (app/UI layer)
- keep persistence and schema changes in `terminal_settings` (core config layer)
- test pure helper logic (font options, wraparound selection, mode toggling) with unit tests

Avoid:
- introducing cross-tab mutable state leaks when applying settings
- adding settings writes from background terminal event paths
- making dock-mode assumptions that require macOS APIs in non-macOS targets

## Typical Mistakes

- Updating font fields without forcing terminal grid resync, causing stale row/column geometry.
- Saving partially updated settings structs that bypass clamping/sanitization expectations.
- Showing expanded right-side controls in every tab-bar state, which can break find-panel layout behavior.
- Assuming `alacritty_terminal` will animate blinking by itself in custom frontends; it only exposes state/events, so UI must schedule redraw ticks.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
- Recommended manual checks:
  - open/close settings controls from tab-bar top-right
  - cycle font family and verify immediate glyph metric updates
  - adjust font size and verify terminal resize behavior remains stable
  - toggle mode and confirm persisted `settings.json` reflects selection
  - set cursor style to `bar` or `underline` and verify cursor is no longer full-cell thickness
  - set `blinking` to `on` and verify cursor visibility toggles periodically while idle
- Signals of regression:
  - pointer selection offset after font changes
  - terminal not resizing after font/size updates
  - settings changes not surviving app restart

## Related Artifacts

- Related docs:
  - `docs/evolution/0008-2026-02-24-macos-menubar-quick-terminal-mode.md`
  - `docs/evolution/0009-2026-02-24-terminal-tabs-and-tabbar-ui.md`
  - `docs/evolution/0010-2026-02-24-terminal-pointer-coordinate-space.md`
  - `docs/plans/2026-02-24-tabbar-settings-panel-design.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/terminal_view.rs`
  - `crates/simple-term/src/terminal_settings.rs`
