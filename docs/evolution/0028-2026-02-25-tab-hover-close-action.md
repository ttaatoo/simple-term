# 0028-2026-02-25-tab-hover-close-action

## Metadata

- Date: 2026-02-25
- Sequence: 0028
- Status: active
- Scope: runtime, testing

## Why This Entry Exists

Tab close affordance was missing from the tab chip itself, forcing keyboard shortcuts for a common action. The UI now shows a close button on the right side of a tab item when that tab is hovered, matching expected desktop terminal/browser behavior.

This entry records interaction invariants for hover-state rendering and click-event routing.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
- Upstream constraints (platform, library, policy):
  - GPUI hover enter/leave should use `on_hover` on stateful elements.
  - Nested interactive elements can bubble pointer events unless propagation is explicitly stopped.
- Invariants already in force:
  - tab item width remains fixed for layout stability
  - last-tab close path still routes through hide-window behavior

## Decision and Rationale

- Decision:
  - add per-tab hover tracking (`hovered_tab_id`) and render tab close button only while hovered
  - keep close action inside tab chip with a compact icon button
  - call `cx.stop_propagation()` on close-button mouse down to prevent parent tab-activate handler from also firing
- Why this path was selected:
  - minimal structural change while delivering expected affordance
  - avoids introducing global hover tracking hacks or coordinate math
  - keeps close semantics aligned with existing `close_tab(...)` logic
- Trade-offs accepted:
  - close button is intentionally hidden until hover to keep chips visually compact

## Alternatives Considered

1. Always show close button on all tabs
- Pros:
  - no hover state tracking required
- Cons:
  - noisier tab UI and reduced title space
- Why not chosen:
  - worse information density for small tab widths

2. Show close button only on active tab
- Pros:
  - simpler than per-tab hover
- Cons:
  - inconsistent affordance and slower close flow for non-active tabs
- Why not chosen:
  - does not match requested interaction model

## Safe Change Playbook

When modifying this area, follow these steps:
1. Keep hover transitions centralized through a pure helper (enter/leave behavior), not ad-hoc field mutations in multiple callbacks.
2. Keep close-button event propagation stopped before calling `close_tab(...)`.
3. Keep tab title truncation + tooltip behavior when changing right-side control width.
4. Re-run tab interaction tests plus workspace checks after any event wiring updates.

## Do / Avoid

Do:
- use `on_hover` on the tab chip element to track entry/exit reliably
- keep close-button rendering conditional to hover state

Avoid:
- wiring close on the same listener path as tab activation without stopping propagation
- moving hover state updates into mouse-move coordinate logic

## Typical Mistakes

- Clicking close triggers both close and activate because parent listener still receives the event.
- Hover state sticks after tab close because stale hovered id is not cleared.

## Verification Strategy

- Required automated checks:
  - `cargo fmt --all`
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - hover each tab and confirm right-side close button appears only on hovered tab
  - click close button on active and inactive tabs and confirm only close action runs
  - verify tab strip layout stays stable with long titles
- Signals of regression:
  - close button visible permanently or never visible
  - close click also selects/switches tab unexpectedly

## Related Artifacts

- Related docs:
  - `docs/evolution/0009-2026-02-24-terminal-tabs-and-tabbar-ui.md`
  - `docs/evolution/0019-2026-02-25-tab-title-width-stability-and-tooltip-overflow.md`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/terminal_view.rs`
