# 0051-2026-02-26-tab-and-settings-spacing-polish

## Metadata

- Date: 2026-02-26
- Sequence: 0051
- Status: active
- Scope: runtime, testing

## Why This Entry Exists

Tab bar and settings drawer were functionally stable but still felt visually dense in edge padding and group rhythm. The change is intentionally visual-only (spacing and subtle border presence), so rationale and guardrails are not obvious from commit diffs alone.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
- Upstream constraints (platform, library, policy):
  - top tab bar must remain `TAB_BAR_HEIGHT_PX = 40.0`
  - no behavioral changes to tab/find/settings interactions
  - settings drawer width policy and viewport guards remain unchanged
- Invariants already in force:
  - active tab affordance is indicator-first
  - hover-close behavior and click propagation boundaries remain intact
  - settings overlay close paths and keyboard handling stay unchanged

## Decision and Rationale

- Decision:
  - apply spacing polish across tab bar and settings drawer:
    - tab strip edge breathing increased (`px_3 -> px_4`)
    - tab item horizontal padding tightened (`px_3 -> px_2`)
    - right-side tab controls group spacing increased (`gap_2 -> gap_3`)
    - settings content rhythm increased (`gap_3 -> gap_4`, cards `p_2 -> p_3`)
    - settings scroll right padding increased (`12 -> 14`)
    - subtle separator contrast softened (`0.06 -> 0.05`, `0.08 -> 0.07`)
    - advanced text sub-spacing increased (`mt_1 -> mt_2`)
- Why this path was selected:
  - achieves "cleaner but still tight" visual target without changing layout height budget or interaction code paths
- Trade-offs accepted:
  - slightly more horizontal breathing can reduce maximal visual density in tight windows

## Alternatives Considered

1. Minimal single-token tweak
- Pros:
  - smallest possible diff
- Cons:
  - inconsistent rhythm between tab and settings surfaces remains
- Why not chosen:
  - insufficient for cross-surface polish objective

2. Broader visual redesign
- Pros:
  - larger perceived redesign impact
- Cons:
  - higher regression risk, exceeds spacing-only scope
- Why not chosen:
  - violated low-risk and behavior-preserving constraint

## Safe Change Playbook

When modifying this area, follow these steps:
1. Preserve `TAB_BAR_HEIGHT_PX = 40.0` and tab vertical footprint invariants.
2. Treat tab strip and settings drawer as one spacing system; avoid isolated tweaks.
3. Keep behavior logic untouched while editing spacing/border tokens.
4. Verify find panel mode and default controls mode both after spacing changes.
5. Validate narrow and wide viewports for crowding/overlap regressions.

## Do / Avoid

Do:
- keep spacing changes tokenized and localized to layout/style calls
- soften separators incrementally, not by removing all borders
- run focused spacing tests plus full workspace tests

Avoid:
- mixing spacing edits with interaction or state-machine changes
- changing width/height helpers unless explicitly redesigning geometry policy
- over-increasing gaps that break compact terminal-shell feel

## Typical Mistakes

- Increasing group gaps but forgetting edge padding balance, causing lopsided composition.
- Softening all borders uniformly, which can reduce actionable control clarity.
- Adjusting settings spacing without checking custom scrollbar-side crowding.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test --workspace`
  - `cargo test -p simple-term-app tab_spacing_tokens_follow_balanced_compact_spec -- --nocapture`
- Recommended manual checks:
  - tab counts: 1, 2, and overflow (6+)
  - find panel open/close rhythm parity
  - settings drawer readability and scrollbar-side spacing
  - narrow/wide window checks for clipping/crowding
- Signals of regression:
  - controls look detached or overly loose
  - tab close affordance feels cramped
  - settings sections lose hierarchy or appear noisy

## Related Artifacts

- Related docs:
  - `docs/plans/2026-02-26-ui-spacing-refresh-design.md`
  - `docs/plans/2026-02-26-ui-spacing-refresh.md`
  - `docs/evolution/0030-2026-02-25-tabbar-spacing-rhythm-refresh.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/terminal_view.rs`
