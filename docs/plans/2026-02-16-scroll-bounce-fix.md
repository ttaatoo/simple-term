# Scroll Bounce After Typing Implementation Plan

**Goal:** Eliminate the “jump to bottom then bounce back” scroll behavior after typing while scrolled in terminal history.

**Scope:**
- In scope: input-triggered scroll suppression logic in `apps/zed-terminal/src/terminal_view.rs`, unit tests for the precise wheel/touch-phase sequence, targeted regression verification.
- Out of scope: redesigning general scroll behavior, changing scroll multiplier policy, modifying Alacritty internals.

**Assumptions:**
- The observed bounce is caused by a precise scroll event sequence that arrives after typing (`Started` then `Moved`) and should be suppressed within the input suppression window.
- Existing helper-level tests in `terminal_view.rs` are the right place for regression coverage.

**Risks:**
- Over-suppressing legitimate new user scroll gestures if suppression is not cleared at the correct boundary.
- Platform differences in touch-phase sequencing (trackpad vs wheel) could hide edge cases.

## Task 1: Lock Regression Test to Expected Behavior
Files:
- Modify: `apps/zed-terminal/src/terminal_view.rs`
- Test: `apps/zed-terminal/src/terminal_view.rs`

Steps:
1. Convert the current characterization test into a true regression expectation:
   - After `prepare_for_terminal_input(true, ...)`
   - `TouchPhase::Started` is ignored, but suppression remains active.
   - Follow-up precise `TouchPhase::Moved` within the window is also ignored.
2. Keep test name explicit about sequence and expected suppression retention.
3. Run targeted test command:
   - `cargo test -p zed-terminal-app precise_scroll_started_event_drops_input_suppression_for_followup_moved_event -- --nocapture`
   - Expected: fails before code change.

Done when:
- Test fails for the current implementation because suppression is currently cleared on `Started`.

## Task 2: Minimal Logic Fix in Scroll Event Gate
Files:
- Modify: `apps/zed-terminal/src/terminal_view.rs`
- Test: `apps/zed-terminal/src/terminal_view.rs`

Steps:
1. Update `should_ignore_scroll_event(...)` handling for `TouchPhase::Started`:
   - Reset `pending_scroll_lines`.
   - Do not clear `suppress_precise_scroll_until` unconditionally.
2. Preserve existing behavior for `TouchPhase::Ended` and line-based scroll clearing, unless tests prove a further adjustment is required.
3. Re-run targeted tests:
   - `cargo test -p zed-terminal-app precise_scroll_started_event_drops_input_suppression_for_followup_moved_event -- --nocapture`
   - Expected: passes after fix.

Done when:
- Regression test passes and code path prevents immediate precise follow-up scroll from applying after typing.

## Task 3: Broader Guardrail Verification
Files:
- Modify (only if needed): `apps/zed-terminal/src/terminal_view.rs`
- Test: `apps/zed-terminal/src/terminal_view.rs`

Steps:
1. Run related scroll/input helper tests to ensure no regression in intended behavior:
   - `cargo test -p zed-terminal-app precise_scroll_is_ignored_within_input_suppression_window`
   - `cargo test -p zed-terminal-app precise_scroll_is_allowed_after_suppression_expires`
   - `cargo test -p zed-terminal-app line_scroll_clears_precise_suppression`
2. If needed, adjust helper logic/tests for consistency with intended UX:
   - Old inertial events after typing are ignored inside window.
   - New explicit user gesture still works once suppression expires or non-precise gesture clears it.

Done when:
- All related targeted tests pass with coherent suppression semantics.

## Task 4: Workspace Verification and Diff Review
Files:
- Modify: as needed from prior tasks only

Steps:
1. Format:
   - `cargo fmt --all`
2. Run app crate tests (or workspace tests if requested):
   - `cargo test -p zed-terminal-app`
3. Review final diff:
   - `git diff -- apps/zed-terminal/src/terminal_view.rs`

Done when:
- Tests pass, formatting is clean, and diff is minimal and localized to suppression logic/tests.

## Rollback / Mitigation
- If retaining suppression on `TouchPhase::Started` causes missed intentional scrolls, introduce a narrower condition:
  - Only retain suppression for precise events while `now < suppress_precise_scroll_until`.
  - Clear suppression immediately for explicit line-based wheel input.
- If behavior differs by platform, add a second test that models alternate touch-phase ordering observed in logs.
