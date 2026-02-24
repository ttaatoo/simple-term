# Simple Term Architecture and Invariants

This document defines the core runtime structure and the behavioral invariants that should hold across refactors.

## Runtime Architecture

- `apps/simple-term/src/main.rs`
  - Loads settings, constructs window options, and boots `TerminalView`.
- `apps/simple-term/src/terminal_view.rs`
  - UI-facing layer responsible for input handling, terminal snapshotting, and painting.
  - Consumes backend events (`Wakeup`, `TitleChanged`, `Exit`) and triggers redraws/window title updates.
- `crates/simple-term/src/terminal.rs`
  - Backend wrapper around `alacritty_terminal` + PTY lifecycle.
  - Owns event-loop sender/receiver, terminal state lock, and resize/write/shutdown operations.
- `crates/simple-term/src/mappings/*`
  - Input protocol mapping (keys/mouse/colors).
- `crates/simple-term/src/terminal_hyperlinks.rs`
  - Hyperlink/path detection and extraction.

## Data and Event Flow

1. User input is captured in `TerminalView` handlers.
2. Input is translated into PTY bytes (`Terminal::write` / `write_str`) or local display operations (selection/scrollbar).
3. Backend emits terminal events through bounded channel.
4. `TerminalView` subscribes and requests repaint on wakeups/title changes.
5. Render path snapshots terminal state under lock, drops lock, then paints from immutable snapshot.

## Invariants

### Backend invariants

- `TERM` must always be set to `xterm-256color` in PTY environment.
- Backpressure policy:
  - `Wakeup` and `Bell` are droppable when channel is full.
  - `TitleChanged` and `Exit` are retained (`force_send`) when channel is full.
- `Terminal::resize` must send PTY resize and update terminal grid dimensions.

### Rendering invariants

- Terminal lock must not be held while painting.
  - `take_snapshot` acquires lock, copies render state, releases lock before draw.
- Snapshot timing tracks:
  - full snapshot time,
  - lock-hold time,
  - paint time.
- Dirty-row logic:
  - initial frame repaints all rows,
  - geometry/palette/display-offset changes repaint all rows,
  - unchanged rows are skipped,
  - cursor movement marks old and new cursor rows dirty.
- `previous_frame` cache is updated from the current snapshot after dirty-row diffing.

### Input/scroll invariants

- Typing while scrolled up forces scroll-to-bottom before sending bytes.
- Residual precise scroll movement after typing is suppressed until scroll sequence end.
- Non-precise/line scroll starts a new gesture and clears precise suppression.

### Hyperlink invariants

- URL trailing punctuation is trimmed conservatively.
- File URLs are decoded to local file paths.
- Path regex timeouts disable expensive path matching when configured to `0`.

## Observability

Set `SIMPLE_TERM_PERF=1` to enable periodic render telemetry in logs.
Logged metrics include frame count, average snapshot time, lock-hold time, paint time, and dirty-row ratio.

## Verification Targets

- Unit tests:
  - event mapping/backpressure (`crates/simple-term/src/terminal.rs`)
  - hyperlink/path matching (`crates/simple-term/src/terminal_hyperlinks.rs`)
  - scroll/input suppression and dirty rows (`apps/simple-term/src/terminal_view.rs`)
- Integration tests:
  - PTY process info lifecycle (`crates/simple-term/tests/pty_info_integration.rs`)
  - PTY terminal behavior (`crates/simple-term/tests/terminal_pty_integration.rs`)
