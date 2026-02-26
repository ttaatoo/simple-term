# 0054-2026-02-26-cmdw-last-tab-force-hide-even-when-pinned

## Metadata

- Date: 2026-02-26
- Sequence: 0054
- Status: active
- Scope: runtime, testing

## Why This Entry Exists

`Cmd+W` on the last tab must hide the terminal window even when the window is pinned. Existing hide policy treated pinned as a blanket "do not hide" rule, so last-tab close requests were blocked.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/terminal_view.rs`
- Upstream constraints (platform, library, policy):
  - pinned mode should still prevent passive auto-hide (for example deactivation/outside click)
  - explicit user close intent (`Cmd+W` on last tab) must take precedence over pinned hold-open behavior
- Invariants already in force:
  - controller owns hide policy decisions
  - last-tab close path routes through controller instead of directly calling `cx.hide()`

## Decision and Rationale

- Decision:
  - introduce `AppCommand::ForceHideTerminal`
  - wire last-tab close callback to `ForceHideTerminal`
  - keep normal `HideTerminal` path pinned-aware; only force path bypasses pinned check
- Why this path was selected:
  - preserves existing pinned semantics for passive hide triggers
  - isolates explicit-close behavior without broad side effects
- Trade-offs accepted:
  - two hide command paths now exist and must remain clearly separated by intent

## Alternatives Considered

1. Remove pinned gating from all hide requests
- Pros:
  - simpler logic
- Cons:
  - breaks pinned behavior for deactivation/outside-click scenarios
- Why not chosen:
  - too broad; violates pinned expectations

2. Let `TerminalView` directly hide on last-tab close
- Pros:
  - no new command variant
- Cons:
  - bypasses controller policy and state synchronization
- Why not chosen:
  - weakens architecture boundary and increases drift risk

## Safe Change Playbook

When modifying this area, follow these steps:
1. Keep passive hide (`HideTerminal`) and explicit force-hide (`ForceHideTerminal`) intent-separated.
2. Route user-explicit close actions through force-hide callback from view to controller.
3. Preserve pinned checks in normal hide path unless requirement explicitly says otherwise.
4. Verify both pinned and unpinned last-tab `Cmd+W` behavior.

## Do / Avoid

Do:
- encode hide intent in command type
- keep controller as the single hide-policy owner

Avoid:
- collapsing force and non-force hide paths
- changing pinned semantics globally to solve one close-path issue

## Typical Mistakes

- Applying a global "ignore pinned on hide" fix for a targeted close-path requirement.
- Reusing deactivation callback for last-tab close without distinguishing intent.

## Verification Strategy

- Required automated checks:
  - `cargo test -p simple-term-app hide_terminal_request_is_processed_when_visible_flag_is_false`
  - `cargo test -p simple-term-app close_tab_hides_window_when_last_tab_would_be_closed`
  - `cargo check --workspace`
- Recommended manual checks:
  - set pinned, keep one tab, press `Cmd+W`, confirm window hides
  - set pinned, click outside/deactivate, confirm window does not auto-hide
- Signals of regression:
  - pinned last-tab `Cmd+W` does nothing
  - pinned window hides on passive deactivation

## Related Artifacts

- Related docs:
  - `docs/evolution/0023-2026-02-25-last-tab-close-hides-window-via-controller-path.md`
  - `docs/evolution/0053-2026-02-26-pin-shortcut-focus-scope-and-cursor-blink-default.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/terminal_view.rs`
