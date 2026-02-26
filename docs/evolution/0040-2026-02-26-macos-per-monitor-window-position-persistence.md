# 0040-2026-02-26-macos-per-monitor-window-position-persistence

## Metadata

- Date: 2026-02-26
- Sequence: 0040
- Status: active
- Scope: runtime

## Why This Entry Exists

Users can move the terminal window manually after it appears. Without persistence, the next show event recenters the window and drops the user’s preferred location. This entry records the monitor-aware persistence contract and where it is enforced.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/macos.rs`
  - `crates/simple-term/src/terminal_settings.rs`
- Upstream constraints (platform, library, policy):
  - macOS monitor geometry and visible frame must be read through AppKit (`NSScreen`, `NSWindow`).
  - Show/hide lifecycle remains controller-owned (`AppShellController`).
- Invariants already in force:
  - show flow still targets the monitor under current mouse location
  - placement remains clamped to visible monitor bounds

## Decision and Rationale

- Decision:
  - persist the last moved window origin per monitor key in `settings.json`
  - capture window monitor+origin when hiding
  - on show, prefer persisted origin for the target monitor; fallback to centered top inset if none exists
- Why this path was selected:
  - preserves user intent without changing open/close lifecycle or tab/session behavior
  - keeps monitor-specific logic in macOS glue, not terminal rendering code
- Trade-offs accepted:
  - monitor identity uses screen-frame keying; if display layout changes drastically, a saved key may not match
  - persistence currently stores origin (x/y) only, not per-monitor size variants

## Alternatives Considered

1. Persist one global window position
- Pros:
  - simpler schema
- Cons:
  - wrong behavior with multi-monitor setups
- Why not chosen:
  - requirement is explicitly monitor-specific

2. Persist live on every move callback
- Pros:
  - always up to date
- Cons:
  - high-frequency disk writes and tighter callback coupling
- Why not chosen:
  - saving on hide is sufficient for next-show behavior and lower risk

## Safe Change Playbook

When modifying this area, follow these steps:
1. Keep monitor key generation and placement clamping centralized in `macos.rs`.
2. Update both capture path (`hide_terminal`) and apply path (`show_terminal`) together.
3. Sanitize persisted monitor positions in `TerminalSettings::sanitize` for malformed values.
4. Verify show/hide behavior on at least two monitors.

## Do / Avoid

Do:
- keep per-monitor positions in settings as plain serializable data
- clamp persisted origins to current visible bounds before applying

Avoid:
- writing settings on every bounds notification
- bypassing controller-owned show/hide state when persisting window position

## Typical Mistakes

- Saving absolute global coordinates without monitor discrimination.
- Applying persisted coordinates without clamping to visible frame.
- Forgetting to persist after updating `monitor_window_positions`.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - move terminal on monitor A, hide/show, verify same spot
  - move terminal on monitor B, hide/show, verify independent remembered spot
  - unplug/rearrange displays and confirm fallback/clamping keeps window visible
- Signals of regression:
  - window recenters every show despite prior move
  - window restores off-screen or behind menubar/dock

## Related Artifacts

- Related docs:
  - `docs/evolution/0038-2026-02-26-macos-show-terminal-on-mouse-monitor.md`
  - `docs/evolution/0039-2026-02-26-macos-deferred-window-move-to-avoid-gpui-reentry.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/macos.rs`
  - `crates/simple-term/src/terminal_settings.rs`
