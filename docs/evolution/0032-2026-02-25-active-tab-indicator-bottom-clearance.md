# 0032-2026-02-25-active-tab-indicator-bottom-clearance

## Metadata

- Date: 2026-02-25
- Sequence: 0032
- Status: active
- Scope: runtime, testing

## Why This Entry Exists

The active tab accent indicator sat flush against the tab-bar bottom border. That made the accent visually merge with the border line and weakened hierarchy.

This is not obvious from commit history because all spacing tokens still satisfied size constraints; the issue is perceptual alignment, not a structural layout failure.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
- Upstream constraints (platform, library, policy):
  - tab bar height remains fixed at `TAB_BAR_HEIGHT_PX = 40.0`
  - tab item content (chip + indicator + decorative spacing) must remain within tab-bar height budget
  - active indicator color token remains the fixed brand purple
- Invariants already in force:
  - active indicator appears for active tab only
  - inactive tabs reserve indicator lane without visible fill
  - tab width and interaction behavior remain unchanged by spacing-only updates

## Decision and Rationale

- Decision:
  - introduce `TAB_ITEM_INDICATOR_BOTTOM_GAP_PX = 2.0`
  - keep indicator height unchanged (`TAB_ITEM_INDICATOR_HEIGHT_PX = 3.0`)
  - render a fixed transparent spacer below the indicator to create a consistent bottom clearance
  - include this clearance in `tab_item_vertical_footprint_px` so layout-budget tests stay accurate
- Why this path was selected:
  - achieves the requested visual separation with minimal change surface
  - avoids altering tab bar height, tab chip height, or interaction handlers
  - keeps active/inactive indicator logic untouched
- Trade-offs accepted:
  - tab content block consumes 2px more vertical budget (still below 40px limit)

## Alternatives Considered

1. Move indicator up by reducing tab chip height
- Pros:
  - no new spacer token
- Cons:
  - changes text-row hit area and vertical alignment
- Why not chosen:
  - higher behavioral/layout risk for a visual-only request

2. Add bottom margin directly on indicator node
- Pros:
  - smaller render-tree change
- Cons:
  - less explicit accounting in footprint tests and harder to reason about cross-framework spacing semantics
- Why not chosen:
  - explicit spacer + token is clearer and easier to keep invariant-checked

## Safe Change Playbook

When modifying this area, follow these steps:
1. Keep indicator color/state logic isolated from spacing changes.
2. Treat indicator height and bottom clearance as separate tokens; avoid hard-coding combined magic numbers in render code.
3. Update `tab_item_vertical_footprint_px` whenever indicator lane geometry changes.
4. Re-run tab spacing tests to ensure vertical budget invariants still hold.
5. Verify active and inactive tabs both preserve identical vertical footprint.

## Do / Avoid

Do:
- encode visual clearance as a named constant
- keep clearance applied uniformly for active and inactive tabs
- preserve tab hit targets while adjusting visual indicator position

Avoid:
- changing tab-bar or tab-chip heights for this class of visual tweak
- coupling indicator spacing with hover/active interaction branches
- introducing ad-hoc per-tab offsets

## Typical Mistakes

- Adjusting indicator position only for active tabs, causing vertical jitter when switching tabs.
- Changing indicator clearance without updating the vertical-footprint helper used by tests.
- Solving by shrinking text-row height and accidentally reducing pointer target comfort.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test -p simple-term-app tab_spacing_tokens_follow_balanced_compact_spec -- --nocapture`
- Recommended manual checks:
  - confirm active indicator has visible clearance from bottom border
  - switch active tab repeatedly and verify no vertical jump across tabs
  - verify hover close button behavior and click hit area remain unchanged
- Signals of regression:
  - active indicator touches or visually blends with bottom border again
  - tab content appears vertically cramped
  - active-tab switch causes subtle y-axis movement

## Related Artifacts

- Related docs:
  - `docs/evolution/0011-2026-02-24-tab-bar-vertical-alignment-invariants.md`
  - `docs/evolution/0030-2026-02-25-tabbar-spacing-rhythm-refresh.md`
  - `docs/evolution/0029-2026-02-25-tab-accent-purple-token.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/terminal_view.rs`
