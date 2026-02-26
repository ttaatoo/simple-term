# 0011-2026-02-24-tab-bar-vertical-alignment-invariants

## Metadata

- Date: 2026-02-24
- Sequence: 0011
- Status: active
- Scope: runtime, testing

## Why This Entry Exists

Tab labels in the first tab shifted vertically after opening a second tab. The visual issue came from conditional tab separator nodes participating in vertical layout flow, so the same tab item had different effective height depending on whether it was the last tab.

This is not obvious from commit-level diffs because the bug is caused by interaction between flex direction, conditional children, and fixed tab-bar height budget.

## System Context

Relevant modules:
- `apps/simple-term/src/terminal_view.rs`

Upstream constraints:
- Tab bar height is fixed (`TAB_BAR_HEIGHT_PX = 40`).
- Tab item content (label row + active indicator) must remain visually stable as tab order/count changes.
- GPUI flex-column layout includes conditional children in total height calculations.

Invariants already in force:
- Switching from one tab to multiple tabs must not change label baseline/vertical centering for existing tabs.
- Decorative chrome (separators, borders) must not consume vertical budget intended for text content.
- Right-side tab controls (`+`, dropdown) should be explicitly center-aligned and not depend on font baseline heuristics.

## Decision and Rationale

Decision:
- Remove separator blocks from the tab item's vertical child stack and render separation as a right border on the tab-label row.
- Promote tab item heights to constants (`TAB_ITEM_HEIGHT_PX`, `TAB_ITEM_INDICATOR_HEIGHT_PX`) and keep a test-only footprint helper to enforce budget invariants.
- Explicitly center the right controls using flex (`items_center` + `justify_center`).

Why this path was selected:
- It fixes the root cause (layout-flow pollution) instead of masking with spacing tweaks.
- Border separators preserve intended visual separation while avoiding state-dependent vertical drift.
- The regression test makes the height-budget contract explicit for future refactors.

Trade-offs accepted:
- Separator visuals are slightly simplified (border-based rather than separate mini-divider node with custom margin offsets).

## Alternatives Considered

1. Keep separator child and tune margins/padding
- Pros: minimal visual change from previous structure
- Cons: fragile; future style changes can reintroduce drift
- Why not chosen: still couples text alignment to conditional child layout

2. Increase tab bar height to absorb overflow
- Pros: easy to implement
- Cons: hides, rather than fixes, state-dependent layout behavior
- Why not chosen: violates stable alignment expectations and wastes vertical space

## Safe Change Playbook

When modifying tab bar layout:
1. Keep decorative separators outside vertical flow for tab chips in `flex_col` stacks.
2. Treat tab-row height and indicator height as a fixed budget; verify their sum stays within `TAB_BAR_HEIGHT_PX`.
3. If adding conditional visual nodes, validate both `is_last = true` and `is_last = false` paths.
4. Keep control buttons (`+`, dropdown) explicitly centered via flex.
5. Re-run tab layout regression tests and manual 1-tab/2-tab checks.

## Do / Avoid

Do:
- Use borders/overlays for separators when parent uses vertical stack alignment.
- Keep tab-item height constants centralized in `terminal_view.rs`.
- Add/maintain tests that compare tab-item footprint across position states.

Avoid:
- Appending decorative children with margins under tab-label rows in a vertical stack.
- Relying on implicit text baseline centering for button glyphs in fixed-height controls.
- Shipping tab bar refactors without validating 1-tab to 2-tab transition.

## Typical Mistakes

- Implementing per-tab separators as extra stacked children in a `flex_col` item.
- Assuming "only non-last tabs" visual branches cannot influence vertical alignment.
- Verifying tab alignment only with a single tab open.

## Verification Strategy

Required automated checks:
- `cargo test -p simple-term-app tab_item_vertical_footprint -- --nocapture`
- `cargo check --workspace`

Recommended manual checks:
- open 1 tab and verify label vertical centering
- open 2+ tabs and compare first-tab label baseline to single-tab state
- verify `+` and dropdown glyphs remain vertically centered after tab count changes

Signals of regression:
- first tab text shifts vertically after opening/closing another tab
- tab separators reintroduce extra stacked height in non-last tabs
- right controls look vertically offset despite fixed control height

## Related Artifacts

- Related docs: `docs/evolution/0009-2026-02-24-terminal-tabs-and-tabbar-ui.md`, `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Optional references: `apps/simple-term/src/terminal_view.rs`
