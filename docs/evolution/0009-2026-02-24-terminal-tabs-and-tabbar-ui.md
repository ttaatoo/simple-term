# 0009-2026-02-24-terminal-tabs-and-tabbar-ui

## Metadata

- Date: 2026-02-24
- Sequence: 0009
- Status: active
- Scope: runtime, architecture, testing

## Why This Entry Exists

`simple-term` previously rendered exactly one terminal session per window. The new requirement adds a `atom_one_dark`-style top tab bar and true multi-tab session management. This introduces new invariants around session isolation, active-tab rendering, and input routing that are not visible from commit-level file diffs.

## System Context

Relevant modules:
- `apps/simple-term/src/terminal_view.rs`
- `apps/simple-term/src/main.rs`
- `crates/simple-term/src/terminal.rs`

Upstream constraints:
- Mouse coordinates in GPUI input events are window-relative.
- Terminal content is rendered in a canvas snapshot pipeline that depends on stable row cache semantics.
- PTY lifecycle is owned by `Terminal` and should remain drop-driven (shutdown on drop).

Invariants already in force:
- PTY rendering/input mapping behavior must stay consistent with existing single-tab behavior.
- Keyboard escape mapping and mouse reporting semantics must not regress.
- Window resize must keep terminal grid dimensions synchronized with visible content area.

## Decision and Rationale

Decision:
- Keep a single `TerminalView`, but make it manage multiple `Terminal` sessions (`tabs`) with one active tab.
- Add a top tab bar in GPUI with Warp-like integrated titlebar treatment: dark 40px bar, left-aligned clickable tab items, reserved left traffic-light drag region, and compact `+` / dropdown-style controls.
- Align tab/chrome visual tokens with `atom_one_dark` theme values (`#101010` bar background, `#a855f7` active accent, subdued white-alpha controls) while keeping the Warp-like structural layout.
- Keep tab-item usability states explicit (hover contrast and hidden scrollbar for overflowed tab lists) so multi-tab navigation remains discoverable.
- Add multi-tab shortcuts (`Cmd+T`, `Cmd+W`, `Cmd+[`, `Cmd+]`, `Ctrl+Tab`, `Ctrl+Shift+Tab`, `Cmd+1..9`) at the view layer before terminal key passthrough.

Why this path was selected:
- Reuses the existing terminal rendering and input logic with minimal behavioral risk.
- Preserves backend boundaries (`crates/simple-term`) and keeps tab orchestration in app/UI layer.
- Keeps each tab as a real independent PTY session instead of reusing a single process.

Trade-offs accepted:
- Row-cache/selection transient state is reset on tab switch to avoid cross-tab contamination.
- Tab bar uses bounded tab widths (`min`/`max`) to avoid stretch-induced visual centering while still truncating long titles safely.

## Alternatives Considered

1. Open one OS window per terminal tab
- Pros: low in-view complexity
- Cons: does not satisfy in-window tab bar requirement; poorer UX parity with atom_one_dark
- Why not chosen: mismatched product requirement

2. Keep one `Terminal` process and emulate tabs with shell multiplexing
- Pros: simpler app state
- Cons: fake tabs, no true session isolation, shell-dependent behavior
- Why not chosen: not reliable and not equivalent to multi-session tabs

## Safe Change Playbook

When modifying multi-tab terminal behavior:
1. Treat tab metadata (`id`, `number`, `title`) and PTY session (`Terminal`) as a unit; never split lifecycle ownership.
2. Keep input routing (`mouse`, `scroll`, `keyboard`) bound only to the active tab.
3. If tab bar height/layout changes, update grid sizing math and terminal bounds origin together.
4. On tab switching logic changes, preserve wraparound semantics for relative navigation and stable active-tab fallback after close.
5. Keep terminal event pollers tab-scoped and avoid global title updates that ignore active tab.

## Do / Avoid

Do:
- Keep tab/session management in `apps/simple-term/src/terminal_view.rs`.
- Add pure helper tests for tab numbering, active-index selection, and title sanitization.
- Validate resize behavior with tab bar offset applied.

Avoid:
- Moving tab logic into `crates/simple-term` (core crate should remain tab-agnostic).
- Sharing mutable selection/scroll transient state across tabs without explicit reset rules.
- Re-introducing full-window grid sizing that ignores the tab bar height.
- Letting tab items use stretch growth (`flex: 1` equivalent), which breaks left-anchored layout and causes centered-looking tab labels.

## Typical Mistakes

- Forgetting to subtract tab bar height when computing terminal line count.
- Updating window title from inactive tab events.
- Closing active tab without deterministic neighbor selection.
- Letting click handlers leak to terminal input area and causing accidental shell input.
- Replacing right-side tab controls without preserving a compact width budget, causing tab strip compression and unstable alignment.

## Verification Strategy

Required automated checks:
- `cargo check --workspace`
- `cargo test --workspace`

Recommended manual checks:
- create multiple tabs, run independent commands, and switch tabs repeatedly
- close active and inactive tabs and verify fallback selection
- verify keyboard shortcuts for create/close/switch/numbered-tab selection
- verify right-click/open-link and selection-copy still work on active tab
- verify resize keeps terminal rows/columns stable below tab bar

Signals of regression:
- mouse selection offset by tab bar height
- tab switching displays stale or cross-tab row cache artifacts
- window title oscillates or shows inactive tab title

## Related Artifacts

- Related docs: `docs/architecture-invariants.md`
- Optional references: `apps/simple-term/src/terminal_view.rs`
