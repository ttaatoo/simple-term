# 0030-2026-02-25-tabbar-spacing-rhythm-refresh

## Metadata

- Date: 2026-02-25
- Sequence: 0030
- Status: active
- Scope: runtime, testing

## Why This Entry Exists

Tab-bar spacing had become visually uneven across three adjacent areas: tab strip, tab chips, and the right-side controls/find surface. Existing values worked functionally, but hierarchy and grouping looked cramped compared with the intended balanced-compact visual direction.

This is not obvious from commit history because the issue was not a single bug; it was an accumulation of small spacing choices (`gap_1`, `px_2`, fixed tab width) across multiple render branches.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
- Upstream constraints (platform, library, policy):
  - tab bar height remains fixed at `TAB_BAR_HEIGHT_PX = 40.0`
  - tab label + indicator vertical budget must stay within tab-bar height
  - tab width must stay fixed to avoid title-update jitter
  - close/hover interactions and keyboard routing must remain behaviorally unchanged
- Invariants already in force:
  - active tab is communicated by indicator, not active-chip fill
  - tab close button appears on hover and must stop click propagation
  - tab title remains truncated with tooltip full-title access

## Decision and Rationale

- Decision:
  - keep 40px tab-bar and existing interaction behavior
  - widen fixed tab width from `140.0` to `152.0`
  - rebalance spacing rhythm:
    - tab strip (`#tab-items-scroll`): `gap_1 -> gap_2`, `px_2 -> px_3`
    - tab chip row (`#tab-item`): `px_2 -> px_3`
    - title/close row inside chip: `gap_1 -> gap_2`
    - right control cluster: `gap_1 -> gap_2`, `px_2 -> px_3`
  - keep find-strip structure and width helpers unchanged while aligning cluster rhythm around it
- Why this path was selected:
  - improves visual hierarchy without changing interaction semantics
  - retains fixed-geometry protections already added for stability
  - keeps change scope small and easy to reason about
- Trade-offs accepted:
  - slightly fewer tabs visible before horizontal overflow due to wider chips and spacing

## Alternatives Considered

1. Minimal touch-up (single spacing tweak)
- Pros:
  - very low risk and tiny diff
- Cons:
  - does not fully solve cross-section rhythm inconsistency
- Why not chosen:
  - did not meet visual-clarity priority

2. Comfort-forward redesign (larger paddings + taller bar)
- Pros:
  - strongest readability uplift
- Cons:
  - breaks compact-density and 40px bar-height constraints
- Why not chosen:
  - conflicts with explicit product-direction constraints

## Safe Change Playbook

When modifying this area, follow these steps:
1. Keep tab geometry stable: fixed width and fixed vertical footprint budget.
2. Treat tab strip, chip internals, and right controls as one spacing system; avoid isolated tweaks.
3. Preserve event/interaction logic exactly when doing visual spacing work (hover tracking, close propagation stop, active indicator states).
4. Re-verify find-panel and normal-control modes after spacing edits so rhythm remains coherent across both render branches.
5. Keep layout refactors free of new conditional children in tab-item vertical flow.

## Do / Avoid

Do:
- centralize spacing intent with explicit constants and consistent utility usage
- validate spacing changes in 1-tab, 2-tab, and overflow scenarios
- keep fixed-width tab policy unless a deliberate redesign supersedes it

Avoid:
- mixing unrelated behavior changes into spacing-only PRs
- reintroducing content-driven tab width policies that jitter on title updates
- adjusting only one side of the bar (tabs or controls) without rechecking whole-bar rhythm

## Typical Mistakes

- Increasing tab width without rebalancing right-cluster padding, causing perceived right-side crowding.
- Changing chip internal padding but leaving title/close gap unchanged, which compresses title readability.
- Treating find-strip mode as a separate visual system and accidentally making mode-switch density inconsistent.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test --workspace`
  - `cargo test -p simple-term-app tab_spacing_tokens_follow_balanced_compact_spec -- --nocapture`
- Recommended manual checks:
  - compare visual spacing with 1, 2, and 6+ tabs
  - verify close-button hover spacing and click behavior on active/inactive tabs
  - toggle find mode and confirm spacing rhythm parity with default controls
  - resize narrow/wide windows and confirm no overlap regressions
- Signals of regression:
  - tab chips appear crowded or inconsistent across states
  - right controls feel detached from tab-strip rhythm
  - spacing changes accidentally alter tab interaction behavior

## Related Artifacts

- Related docs:
  - `docs/evolution/0009-2026-02-24-terminal-tabs-and-tabbar-ui.md`
  - `docs/evolution/0011-2026-02-24-tab-bar-vertical-alignment-invariants.md`
  - `docs/evolution/0019-2026-02-25-tab-title-width-stability-and-tooltip-overflow.md`
  - `docs/evolution/0028-2026-02-25-tab-hover-close-action.md`
  - `docs/evolution/0029-2026-02-25-tab-accent-purple-token.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/terminal_view.rs`
