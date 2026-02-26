# 0038-2026-02-26-macos-show-terminal-on-mouse-monitor

## Metadata

- Date: 2026-02-26
- Sequence: 0038
- Status: active
- Scope: runtime

## Why This Entry Exists

Quick-terminal placement already used the active mouse monitor when creating a new window, but not when reusing an existing hidden window. Users switching monitors could trigger show/hide and get the terminal on a different display than the cursor.

This entry documents the placement invariant so future controller changes do not regress monitor targeting.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/macos.rs`
- Upstream constraints (platform, library, policy):
  - monitor selection is macOS-specific (`NSEvent::mouseLocation` + `NSScreen`)
  - `AppShellController` owns show/hide state and window lifecycle
  - `WindowHandle::update` is the safe path for mutating an existing GPUI window
- Invariants already in force:
  - quick-terminal show requests must activate the app
  - monitor targeting must be derived from current mouse location at show time

## Decision and Rationale

- Decision:
  - resolve panel placement at the beginning of every `show_terminal` call
  - when reusing an existing window handle, call `macos::move_window_to` before finishing show flow
  - keep new-window path behavior unchanged (still uses the same placement object)
- Why this path was selected:
  - minimal, controller-local change with no settings schema updates
  - keeps monitor targeting consistent between first-open and subsequent re-show flows
- Trade-offs accepted:
  - showing the terminal may reposition a previously moved window to the current monitor center
  - no additional persistence of per-monitor last position in this iteration

## Alternatives Considered

1. Only move new windows
- Pros:
  - no behavioral change from existing reuse flow
- Cons:
  - does not satisfy cursor-monitor targeting after the first show
- Why not chosen:
  - fails the explicit monitor-following requirement

2. Destroy and recreate window on every show
- Pros:
  - always guarantees fresh placement
- Cons:
  - risks tab/session churn and extra lifecycle complexity
- Why not chosen:
  - unnecessary disruption; reuse path already exists and only needed repositioning

## Safe Change Playbook

When modifying monitor-aware show behavior, follow these steps:
1. Keep placement resolution in the controller show path (`show_terminal`), not in view rendering code.
2. Resolve placement before branching between existing-window and new-window paths.
3. Apply `move_window_to` inside `WindowHandle::update` without adding synchronous window-level activation calls.
4. Keep `resolve_panel_placement` and `move_window_to` as the only AppKit placement touchpoints.

## Do / Avoid

Do:
- keep monitor targeting tied to current pointer location
- preserve app activation semantics after repositioning
- keep reuse and create paths aligned on the same placement source

Avoid:
- adding independent monitor math in multiple files
- moving window placement logic into `TerminalView`
- assuming first-open placement also covers subsequent toggle events

## Typical Mistakes

- Repositioning only on window creation and forgetting the existing-window reuse branch.
- Running window-level activation in the same update closure as monitor repositioning.
- Duplicating monitor lookup logic outside `macos.rs`.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
- Recommended manual checks:
  - attach at least two monitors
  - move cursor to monitor A and trigger show/hide shortcut; confirm terminal appears on A
  - move cursor to monitor B and trigger show/hide shortcut; confirm terminal appears on B
  - repeat while pinned and unpinned to ensure focus and pin level still apply
- Signals of regression:
  - terminal reappears on previous monitor after toggling
  - activation occurs without monitor move
  - pin behavior changes while monitor targeting is applied

## Related Artifacts

- Related docs:
  - `docs/evolution/0008-2026-02-24-macos-menubar-quick-terminal-mode.md`
  - `docs/evolution/0018-2026-02-25-macos-menubar-window-behavior-and-status-icon-parity.md`
  - `docs/evolution/0036-2026-02-26-macos-toggle-and-pin-hotkeys.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/macos.rs`
