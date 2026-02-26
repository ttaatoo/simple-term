# 0019-2026-02-25-tab-title-width-stability-and-tooltip-overflow

## Metadata

- Date: 2026-02-25
- Sequence: 0019
- Status: active
- Scope: runtime

## Why This Entry Exists

Tab labels were previously configured with a flexible width range (`min/max`). When a new tab was created with a short default title and then quickly received a longer shell title, the tab width expanded immediately. That produced visible horizontal jitter in the tab strip.

This behavior was not obvious from commit history alone because the root cause is a style policy interaction (flex layout + width bounds + late title update), not a single algorithmic bug.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
- Upstream constraints (platform, library, policy):
  - GPUI stateful hover tooltips require an `id(...)` on the target element.
  - Tab labels are user-facing shell-derived text and can change after tab creation.
- Invariants already in force:
  - Tab strip should remain visually stable as titles update.
  - Long titles must not push surrounding controls.
  - Full title remains discoverable without sacrificing compact layout.

## Decision and Rationale

- Decision:
  - Use a fixed tab item width (`TAB_ITEM_WIDTH_PX`) instead of a width range.
  - Keep label rendering as single-line truncated text (`truncate`) so overflow is shown as `...`.
  - Attach a hover tooltip on the tab title text node to reveal the full title.
- Why this path was selected:
  - Fixed width removes post-create width expansion and keeps tab geometry stable.
  - Truncation preserves compact tab strip behavior under long titles.
  - Tooltip preserves access to full context without widening the tab.
- Trade-offs accepted:
  - Tabs no longer use extra horizontal space for medium-length titles.
  - Tooltip introduces one extra renderable view per hovered tab label.

## Alternatives Considered

1. Keep min/max width and lower max width only
- Pros:
  - Minimal style changes.
- Cons:
  - Width still changes after title updates.
- Why not chosen:
  - Does not remove the jitter root cause.

2. Auto-resize all tabs to equal available width on each title change
- Pros:
  - Could maximize visible text for low tab counts.
- Cons:
  - More layout churn and complexity; still risks perceptual jitter.
- Why not chosen:
  - Over-engineered for this issue.

## Safe Change Playbook

When modifying this area, follow these steps:
1. Keep tab item width policy explicit and single-sourced (`TAB_ITEM_WIDTH_PX`) unless there is a deliberate UX redesign.
2. If using GPUI tooltip APIs on a `div`, ensure the element is stateful via `.id(...)` before calling `.tooltip(...)`.
3. Preserve truncation on the visible tab label when allowing long dynamic titles.
4. Validate both create-tab flow and post-create title update flow (OSC title updates from shell).

## Do / Avoid

Do:
- Keep tab geometry stable across title changes.
- Keep full-title discoverability via hover affordances when truncating.

Avoid:
- Re-introducing content-driven width growth for individual tabs.
- Attaching tooltip behavior to non-stateful GPUI elements (missing `.id(...)`).

## Typical Mistakes

- Using `min_w/max_w` ranges for tabs that receive asynchronous title updates, which causes visible width jumps.
- Calling `.tooltip(...)` on a plain `div()` without first making it stateful.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
- Recommended manual checks:
  - Create a new tab and verify the tab width does not expand when the shell sets a longer title.
  - Hover a truncated tab title and confirm the tooltip shows the full text.
  - Open multiple tabs and verify right-side controls (`+`, dropdown, settings) do not shift unexpectedly.
- Signals of regression:
  - Tab chip width changes immediately after title updates.
  - Hover on tab label shows no tooltip or causes runtime interactivity errors.

## Related Artifacts

- Related docs:
  - `docs/evolution/0009-2026-02-24-terminal-tabs-and-tabbar-ui.md`
  - `docs/evolution/0011-2026-02-24-tab-bar-vertical-alignment-invariants.md`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Optional references (PRs/commits/releases):
  - N/A
