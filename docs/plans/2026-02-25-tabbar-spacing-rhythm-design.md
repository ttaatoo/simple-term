# Tab Bar Spacing Rhythm Design

## Goal

Improve tab-bar visual hierarchy by rebalancing margin/padding across tab chips and right controls while preserving compact density and all existing tab behaviors.

## Confirmed Constraints

- Keep `TAB_BAR_HEIGHT_PX = 40.0`.
- Keep fixed tab-width policy; widen slightly for readability.
- Preserve active/hover/close interaction behavior.
- Preserve find-strip width logic and mode-switch behavior.
- Keep change scope inside `apps/simple-term/src/terminal_view.rs`.

## Approaches Considered

### Option A (Chosen): Tokenized Rhythm Refresh

- Widen tab item width modestly.
- Increase tab-strip container gap and horizontal padding.
- Increase tab-chip inner padding and title-close separation.
- Increase right-cluster spacing and horizontal padding.
- Keep find strip structurally the same; align surrounding rhythm.

Pros:
- Best alignment with visual-clarity objective.
- No behavior-path changes.
- Predictable review and regression scope.

Cons:
- Slightly fewer tabs visible before overflow.

### Option B: Minimal Touch-Up

- Change one or two spacing values only.

Pros:
- Lowest risk, smallest diff.

Cons:
- Does not fully address inconsistent spacing rhythm between tabs and controls.

### Option C: Comfort-Forward Spacing

- Larger paddings and potentially larger bar height.

Pros:
- Highest readability.

Cons:
- Breaks compact-density and 40px height constraints.

## Chosen Layout Spec

- `TAB_ITEM_WIDTH_PX`: `140.0 -> 152.0`
- `#tab-items-scroll`: `.gap_1() -> .gap_2()`, `.px_2() -> .px_3()`
- `#tab-item` row: `.px_2() -> .px_3()`
- tab title/close row: `.gap_1() -> .gap_2()`
- right controls container: `.gap_1() -> .gap_2()`, `.px_2() -> .px_3()`

## Behavioral Guardrails

- Do not change keybindings or tab routing.
- Do not change hover-close logic or `cx.stop_propagation()` usage.
- Do not change active indicator token/color behavior.
- Do not add decorative children to tab-item vertical flow.

## Verification Plan

Automated:
- `cargo check --workspace`
- `cargo test --workspace`
- targeted: `cargo test -p simple-term-app tab_spacing_tokens_follow_balanced_compact_spec -- --nocapture`

Manual:
- validate spacing rhythm with 1, 2, and 6+ tabs
- validate long-title truncation + tooltip stability
- validate close-button hover spacing and click behavior
- validate find mode spacing parity with default controls
- validate narrow/wide window layouts for overlap/crowding regressions
