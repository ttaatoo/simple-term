# 0034-2026-02-26-multi-click-selection-modes

## Metadata

- Date: 2026-02-26
- Sequence: 0034
- Status: active
- Scope: runtime, testing

## Why This Entry Exists

The terminal supported drag selection but did not map multi-click gestures to semantic selection modes. Users expected double-click word selection and triple-click line selection, which are standard in terminal emulators. The gap was easy to miss because selection code always used `SelectionType::Simple` with no click-count branch.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
  - `apps/simple-term/src/terminal_view/utils.rs`
- Upstream constraints (platform, library, policy):
  - `gpui::MouseDownEvent` includes `click_count`.
  - `alacritty_terminal::selection::SelectionType` supports `Simple`, `Semantic`, and `Lines`.
  - When `TermMode::MOUSE_MODE` is active, pointer input may be owned by the PTY app and must keep passthrough behavior.
- Invariants already in force:
  - Selection creation must stay in the non-mouse-mode path.
  - PTY mouse-report behavior must remain unchanged when mouse mode is enabled.

## Decision and Rationale

- Decision:
  - Add explicit click-count-to-selection-mode mapping in a shared utility:
    - single click (`0/1`) -> `SelectionType::Simple`
    - double click (`2`) -> `SelectionType::Semantic`
    - triple or more (`>=3`) -> `SelectionType::Lines`
  - Use that mapping when initializing selection in the left-click non-mouse-mode path.
  - Add a unit test covering the mapping contract.
- Why this path was selected:
  - Aligns behavior with common terminal UX expectations.
  - Reuses `alacritty_terminal` built-in semantic/line expansion behavior instead of custom parsing.
  - Keeps implementation small and localized to selection creation.
- Trade-offs accepted:
  - Multi-click behavior remains disabled when PTY mouse mode is active (existing passthrough contract).

## Alternatives Considered

1. Keep drag-only selection behavior
- Pros:
  - No code changes.
- Cons:
  - Misses standard terminal interaction expectations.
  - Continues user-visible UX gap vs mainstream terminals.
- Why not chosen:
  - Does not solve the reported behavior problem.

2. Implement custom word/line expansion without `SelectionType`
- Pros:
  - Full control over boundaries.
- Cons:
  - Reinvents terminal selection semantics.
  - Higher maintenance and divergence risk from upstream.
- Why not chosen:
  - `alacritty_terminal` already exposes stable semantic/line modes.

## Safe Change Playbook

When modifying multi-click selection behavior, follow these steps:
1. Keep click-count mapping centralized in a utility helper; do not duplicate thresholds inline across handlers.
2. Preserve mouse-mode passthrough path (`TermMode::MOUSE_MODE`) so terminal apps with mouse tracking keep receiving pointer events.
3. Add or update unit tests for click-count mapping before changing thresholds or mode semantics.
4. Manually verify single/double/triple-click behavior in normal terminal mode and confirm no regression in mouse-mode apps.

## Do / Avoid

Do:
- Treat `click_count=0` as a single click fallback for platform consistency.
- Keep selection-mode decisions at selection creation time.
- Keep tests explicit about threshold behavior.

Avoid:
- Reverting selection creation to unconditional `SelectionType::Simple`.
- Branching on click count in multiple locations with inconsistent rules.
- Changing mouse-mode passthrough semantics as part of selection-only UX work.

## Typical Mistakes

- Reading `event.click_count` from GPUI events but not wiring it into `Selection::new`.
- Applying semantic/line selection logic in drag-update paths instead of initial selection type.
- Forgetting to add test coverage, allowing regressions back to drag-only behavior.

## Verification Strategy

- Required automated checks:
  - `cargo test -p simple-term-app selection_type_for_click_count_matches_terminal_conventions`
- Recommended manual checks:
  - Single-click drag selects exact range.
  - Double-click selects one semantic word.
  - Triple-click selects an entire line.
  - In a mouse-tracking app, click events still pass through as before.
- Signals of regression:
  - Double-click behaves like single click.
  - Triple-click fails to select full lines.
  - Mouse-tracking terminal apps stop receiving click events.

## Related Artifacts

- Related docs:
  - `docs/evolution/0010-2026-02-24-terminal-pointer-coordinate-space.md`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Optional references (PRs/commits/releases):
  - N/A
