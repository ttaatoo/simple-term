# 0017-2026-02-25-settings-popup-overlay-window

## Metadata

- Date: 2026-02-25
- Sequence: 0017
- Status: active
- Scope: runtime, testing

## Why This Entry Exists

Settings previously rendered as an in-flow right drawer under the terminal content row. That layout made the settings surface feel attached to the terminal pane instead of acting like an independent popup window, and it tightly coupled settings visibility with terminal horizontal layout.

The code diff alone does not preserve the layout invariant needed for future changes: settings should open as an overlay popup without resizing terminal content.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
- Upstream constraints (platform, library, policy):
  - GPUI overlays must be absolutely positioned relative to a `relative()` root when they should not participate in flex layout.
  - Scrollable content inside the settings panel still requires explicit non-zero `scrollbar_width(...)`.
- Invariants already in force:
  - settings content can exceed viewport and must remain scrollable
  - plain `Esc` must close the settings panel
  - opening settings must not alter terminal surface width

## Decision and Rationale

- Decision:
  - Move settings rendering out of `content_row` flex flow.
  - Keep terminal content full-width and add a modal-style absolute overlay (`settings-popup-overlay`) at the root.
  - Render settings as a centered popup card (`settings-popup`) with rounded border styling and backdrop occlusion.
  - Preserve existing scroll-handle-driven scrollbar behavior inside the popup body.
- Why this path was selected:
  - meets user expectation of a popup window
  - decouples terminal layout from settings visibility
  - keeps existing settings control wiring intact (low refactor risk)
- Trade-offs accepted:
  - popup is modal-like and blocks background interactions while open
  - this is an in-window popup, not a separate native OS window

## Alternatives Considered

1. Keep the right drawer and only tweak visuals
- Pros:
  - minimal code churn
- Cons:
  - still participates in layout; does not satisfy popup-window behavior
- Why not chosen:
  - fails the requested interaction model

2. Spawn a second native window for settings
- Pros:
  - true standalone window semantics
- Cons:
  - much higher complexity (window lifecycle, focus, sync, platform behavior)
- Why not chosen:
  - disproportionate scope for current UX request

## Safe Change Playbook

When modifying this area, follow these steps:
1. Keep popup surfaces outside primary terminal flex rows; use absolute overlay composition on a `relative()` root.
2. Keep overlay containers occluding to avoid background terminal hitboxes handling pointer/scroll input.
3. Keep settings body scrollable with explicit non-zero `scrollbar_width(...)` and `track_scroll(...)`.
4. Re-run keyboard-close behavior (`Esc`) and close-button behavior after layout updates.

## Do / Avoid

Do:
- keep the terminal surface as the sole child of `content_row`
- keep popup rendering conditional at root level via overlay child
- preserve existing settings state mutations and persistence behavior

Avoid:
- reintroducing settings panel as a sibling in terminal content flex flow
- removing occlusion from modal overlay layers
- hiding overflow controls without scroll affordance

## Typical Mistakes

- Reattaching settings panel to `content_row`, causing terminal width to shrink when settings opens.
- Positioning popup with static offsets but without a relative parent, causing inconsistent placement.
- Forgetting that popup body still needs independent scrolling behavior after style/layout changes.

## Verification Strategy

- Required automated checks:
  - `cargo fmt --all`
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - open settings and verify terminal width does not change
  - verify popup appears as overlay card and close button/`Esc` both work
  - scroll to bottom settings items using wheel/trackpad
- Signals of regression:
  - settings opening shifts or shrinks terminal content
  - popup appears docked like a drawer instead of floating
  - overflow settings are unreachable

## Related Artifacts

- Related docs:
  - `docs/evolution/0015-2026-02-24-settings-drawer-v1-and-live-persistence.md`
  - `docs/evolution/0016-2026-02-25-settings-drawer-scroll-and-menubar-command-reentrancy.md`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/terminal_view.rs`
