# Menubar Quick Terminal Design

## Context

`simple-term` currently starts as a standard single-window GPUI desktop app. The goal is to turn it into a quick-terminal experience on macOS:

- trigger by global shortcut (`Ctrl+\`` by default)
- trigger by menubar icon click
- show terminal near the top of the current mouse screen, centered
- click outside terminal auto-hides it
- default regular desktop app behavior, with opt-in menubar-only mode

## Scope

This design intentionally keeps terminal core behavior untouched (`crates/simple-term`) and implements menubar/shortcut/window-shell behavior in `apps/simple-term` only.

In scope:

- macOS menubar status item integration
- global hotkey registration and event handling
- app-level toggle state machine (`Hidden`/`Visible`)
- top-centered panel placement on mouse screen
- auto-hide on window deactivation
- dock-mode configuration switch (`accessory` vs `regular`)

Out of scope:

- full preferences UI
- runtime hotkey recorder
- non-macOS menubar behavior parity

## Architecture

### App Shell Controller

Add an app-level controller in `apps/simple-term/src/main.rs` that owns:

- loaded `TerminalSettings`
- optional terminal window handle
- current visibility flag
- command queue receiver (`toggle`, `hide`, `quit`)

The controller is the single place that can:

- open terminal window on first demand
- reposition and activate window on show
- hide app/window on hide commands

### macOS Integration Module

Add `apps/simple-term/src/macos.rs` (macOS-only) containing:

- status item creation (`NSStatusItem`) and click callback bridge
- activation policy helper (`Accessory` / `Regular`)
- placement resolver from mouse screen (`NSEvent.mouseLocation + NSScreen`)
- window move helper via raw AppKit handle

This keeps Cocoa/Objective-C code out of the terminal rendering module.

### Global Shortcut

Use `global-hotkey` crate with:

- default `Ctrl+Backquote`
- optional string override from settings (`global_hotkey`)
- event bridge sending `toggle` command into app queue

## State and Flow

### Toggle flow

1. menubar click or hotkey emits `Toggle`
2. if hidden: resolve placement -> ensure window exists -> move/resize -> activate app/window
3. if visible: hide app

### Outside-click auto-hide

`TerminalView` observes window activation state. When window becomes inactive, it calls an app-provided callback that emits `Hide` command.

This reuses GPUI window activation events and avoids extra event tap complexity.

## Configuration

Extend `TerminalSettings`:

- `dock_mode`: `"menubar_only" | "regular"` (default `regular`)
- `global_hotkey`: string (default `"control+Backquote"`)
- `panel_top_inset`: number (default small positive value)

Existing `default_width/default_height` remain panel size source.

## Progress Snapshot (2026-02-24)

Completed:

- macOS menubar module scaffold (`status item`, placement resolver, window move helper)
- global hotkey wiring scaffold
- popup quick-terminal controller scaffold (`toggle/hide` flow)
- outside-click auto-hide hook via window deactivation observer
- settings schema extension for `dock_mode`, `global_hotkey`, `panel_top_inset`

Deferred for next menubar-focused iteration:

- keep default startup in `regular` mode for ongoing non-menubar feature development
- dedicated preferences UI for runtime mode/hotkey changes
- deeper manual UX polish pass (animation/timing/toggle edge-cases)

## Error Handling

- hotkey parse/register failure: log warning and continue with menubar click path
- status item init failure: log warning and continue with hotkey path
- stale window handle: reopen window lazily

## Verification

Automated:

- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`

Manual smoke:

- hotkey toggles show/hide
- menubar click toggles show/hide
- outside click hides panel
- multi-display follows mouse screen
- dock mode switch from config (`menubar_only` vs `regular`)
