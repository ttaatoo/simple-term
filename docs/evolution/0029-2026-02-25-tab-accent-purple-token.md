# 0029-2026-02-25-tab-accent-purple-token

## Metadata

- Date: 2026-02-25
- Sequence: 0029
- Status: active
- Scope: runtime, testing

## Why This Entry Exists

Tab accent styling was previously coupled to theme cursor color, which made the active-tab indicator and tab-close hover state vary across themes. Product direction requested a stable branded purple accent for these high-salience tab interactions.

This entry records where the accent must remain fixed and where theme-derived color is still allowed.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
- Upstream constraints (platform, library, policy):
  - GPUI style composition allows centralized color helper functions for repeated use.
- Invariants already in force:
  - tab title width remains fixed and truncated
  - close button appears only on hovered tab item

## Decision and Rationale

- Decision:
  - introduce `tab_brand_purple(alpha)` helper with the requested value (`hsla(272/360, 0.91, 0.65, alpha)`)
  - use this purple for:
    - active tab bottom indicator
    - tab close button hover background (and hover border)
    - all existing hover-capable tab-bar controls and settings controls
  - remove active-tab background highlight so active state is communicated by indicator, not chip fill
- Why this path was selected:
  - delivers exact requested palette while minimizing impact to the broader theme system
  - keeps repeated color values centralized and testable
- Trade-offs accepted:
  - hover accents are no longer theme-adaptive for those specific states

## Alternatives Considered

1. Keep accent derived from `theme.cursor`
- Pros:
  - full theme cohesion
- Cons:
  - does not satisfy fixed purple brand requirement
- Why not chosen:
  - conflicts with explicit visual preference request

2. Replace all tab states with fixed purple
- Pros:
  - complete visual consistency
- Cons:
  - over-applies brand color and reduces subtle hierarchy
- Why not chosen:
  - only key attention states required fixed purple

## Safe Change Playbook

When modifying this area, follow these steps:
1. Keep fixed purple usage centralized via `tab_brand_purple(...)` helper.
2. Apply fixed purple only to intended high-salience states unless product direction changes.
3. Keep active state visual hierarchy explicit: indicator for active, hover fill for interaction preview.
4. Preserve stop-propagation behavior on close button click while adjusting hover visuals.
5. Re-run tab interaction and styling tests after any accent edits.

## Do / Avoid

Do:
- use named helper for brand accent
- keep hover/indicator color behavior aligned

Avoid:
- scattering literal purple HSLA values in multiple render branches
- mixing unrelated theme tokens into the close-button hover state

## Typical Mistakes

- Reverting indicator color back to theme-derived cursor token during refactors.
- Updating close-button hover to another color without updating indicator, causing inconsistent tab affordance.

## Verification Strategy

- Required automated checks:
  - `cargo fmt --all`
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - verify active-tab indicator is the requested purple in all themes
  - hover close button and confirm hover fill uses the same purple
- Signals of regression:
  - indicator color varies by theme
  - close-button hover no longer matches indicator accent

## Related Artifacts

- Related docs:
  - `docs/evolution/0028-2026-02-25-tab-hover-close-action.md`
  - `docs/evolution/0019-2026-02-25-tab-title-width-stability-and-tooltip-overflow.md`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/terminal_view.rs`
