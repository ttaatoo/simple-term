# UI Spacing Refresh Implementation Plan

**Goal:** Refine tab bar and settings drawer spacing rhythm for a cleaner, tighter premium look without changing behavior or top-bar height.

**Scope:**
- In scope: spacing/padding/gap and subtle border alpha tuning in `apps/simple-term/src/terminal_view.rs`.
- Out of scope: behavior changes, feature changes, hotkeys, event routing, state logic, theme model.

**Assumptions:**
- Existing tests around tab spacing and settings drawer metrics remain authoritative.
- GPUI spacing utilities (`px_*`, `gap_*`, `p_*`, `mt_*`) remain stable.

**Risks:**
- Over-relaxing spacing can reduce compactness.
- Under-tuning can leave visual inconsistency.
- Narrow viewport crowding regressions.

## Task 1: Introduce/Align Spacing Tokens and Constants

Files:
- Modify: `/Users/mt/Github/zed-terminal/apps/simple-term/src/terminal_view.rs`

Steps:
1. Identify all spacing literals used by tab bar and settings drawer sections.
2. Align token values to design-approved rhythm:
   - tab strip horizontal padding `px_3 -> px_4`
   - tab item horizontal padding `px_3 -> px_2`
   - right control cluster `gap_2 -> gap_3`
   - settings scroll right padding `12 -> 14`
   - settings section container `gap_3 -> gap_4`
   - card padding `p_2 -> p_3`
   - advanced text margins `mt_1 -> mt_2`
3. Keep top bar height and width helper functions unchanged.
4. Verification command: `cargo check --workspace` (expect success).

Done when:
- All spacing changes are confined to approved visual tokens.
- No behavior-affecting lines are changed in event handlers.

Rollback/Mitigation:
- If compactness regresses, first revert only group-level increases (`gap_4`, `px_4`) before touching control-level spacing.

## Task 2: Soften Divider/Border Presence

Files:
- Modify: `/Users/mt/Github/zed-terminal/apps/simple-term/src/terminal_view.rs`

Steps:
1. In tab bar and settings drawer containers, slightly reduce border alpha where used only as visual separators.
2. Keep borders that convey control focus/interaction clarity unchanged.
3. Ensure close-button and actionable control contrast remains sufficient.
4. Verification command: `cargo test -p simple-term-app tab_spacing_tokens_follow_balanced_compact_spec -- --nocapture` (expect pass).

Done when:
- Structural grouping remains readable.
- Border harshness is visibly reduced without losing affordance clarity.

Rollback/Mitigation:
- Restore prior alpha values for any separator that causes grouping ambiguity or control contrast issues.

## Task 3: Run Full Validation and Manual Smoke Checks

Files:
- Modify (if needed): `/Users/mt/Github/zed-terminal/apps/simple-term/src/terminal_view.rs`
- Optional docs update if implementation changes architectural behavior: `/Users/mt/Github/zed-terminal/docs/evolution/INDEX.md` and new evolution entry.

Steps:
1. Run `cargo test --workspace` and confirm all tests pass.
2. Manual smoke checks:
   - 1/2/6+ tabs, hover-close behavior, active indicator unchanged
   - find panel on/off spacing parity and no overlap
   - settings drawer readability with scroll content
   - narrow/wide window checks for clipping and crowding
3. If behavior or invariant changed unexpectedly, stop and split behavior fix from spacing PR.

Done when:
- Automated checks pass.
- Manual checks show cleaner + tight look with no regressions.

Rollback/Mitigation:
- Revert only the failing spacing subset and re-run focused tests before full suite.

## Final Integration Gate

1. `cargo check --workspace`
2. `cargo test --workspace`
3. `git diff -- apps/simple-term/src/terminal_view.rs` review for spacing-only scope

If all pass, prepare final summary with:
- exactly what spacing tokens changed,
- why each change maps to approved design direction,
- verification evidence.
