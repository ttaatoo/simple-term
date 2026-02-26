# UI Spacing Refresh Design (Tab Bar + Settings Drawer)

## Goal

Improve visual polish by refining padding/margin rhythm in the top tab bar and settings drawer while preserving compact density, existing behavior, and layout invariants.

## Scope

In scope:
- `apps/simple-term/src/terminal_view.rs`
- Spacing, padding, gap, subtle border-presence tuning for:
  - Tab bar strip and right control cluster
  - Settings drawer sections and spacing rhythm

Out of scope:
- Interaction logic, shortcuts, event propagation, state transitions
- Theme system changes
- Structural redesign of tab/find/settings features
- Height budget changes for top bar

## Confirmed Constraints

- Keep top tab bar height at `40px` (`TAB_BAR_HEIGHT_PX` unchanged).
- Keep behavior unchanged: tab switching, hover-close, find panel flow, settings toggle.
- Keep fixed geometry policies and width helpers unchanged unless required for spacing harmony.
- Visual target combines:
  - Cleaner feel (reduced divider harshness, improved breathing room)
  - Tighter premium feel (avoid over-loose spacing)

## Design Direction

### 1. Global Rhythm Rules

- Keep vertical density compact; improve perceived order mostly through horizontal spacing and section rhythm.
- Use a two-level rhythm:
  - Group-level spacing (clearer separation between functional groups)
  - Control-level spacing (tight and precise inside each group)
- Reduce visual noise from separators by slightly lowering border contrast.

### 2. Tab Bar Spec

Keep:
- `TAB_BAR_HEIGHT_PX = 40.0`
- tab item height/indicator budget and behavior

Adjust:
- `#tab-items-scroll` horizontal padding: increase (`px_3 -> px_4`) for cleaner edge breathing.
- `#tab-item` content horizontal padding: tighten (`px_3 -> px_2`) to keep controls precise.
- Internal title-close spacing remains balanced (`gap_2`) to avoid cramped close affordance.
- Right control cluster container gap: increase (`gap_2 -> gap_3`) to reduce visual stickiness.
- Divider/border alpha in tab-bar sections: slightly reduce for cleaner look without removing structure.

### 3. Settings Drawer Spec

Keep:
- drawer width logic (`settings_drawer_width_for_viewport`)
- interaction model and controls

Adjust:
- Scroll content right padding near custom scrollbar: increase (`12px -> 14px`) to avoid near-track crowding.
- Card container inner padding: increase (`p_2 -> p_3`) for readability.
- Main vertical rhythm between groups: increase (`gap_3 -> gap_4`) for clearer section hierarchy.
- Keep most intra-control rows compact (`gap_1`), selectively elevate numeric control rows to `gap_2`.
- Advanced text block sub-spacing: `mt_1 -> mt_2` for better scan breaks.
- Slightly soften border presence (alpha reduction) to align with cleaner visual direction.

## Alternatives Considered

### Option A (Chosen): Tokenized rhythm refresh

- Consistent spacing updates across both UI regions, no behavior changes.
- Best balance of coherence, maintainability, and low risk.

### Option B: Minimal local touch-up

- Faster but leaves inconsistency between tab and settings systems.

### Option C: Visual layering overhaul

- Highest visual delta but exceeds spacing-only intent and increases regression risk.

## Risks and Mitigations

Risk: spacing adjustments could accidentally break compact feel.
- Mitigation: preserve 40px top bar and avoid blanket large-gap increases.

Risk: visual changes could interfere with hover/close affordance clarity.
- Mitigation: keep close button sizing and hover behavior unchanged; only tune surrounding spacing.

Risk: narrow viewports may expose crowding regressions.
- Mitigation: explicitly test narrow/wide windows and both find-enabled and default states.

## Verification Plan

Automated:
1. `cargo check --workspace`
2. `cargo test --workspace`
3. Keep spacing-related tests passing (tab spacing + settings overlay/drawer metrics tests)

Manual:
1. Tab bar with 1 / 2 / 6+ tabs: rhythm, alignment, and close affordance.
2. Find panel on/off: right cluster spacing parity and non-overlap.
3. Settings drawer scrolling and section readability.
4. Narrow and wide window checks for clipping/crowding.

## Acceptance Criteria

1. Top bar remains exactly `40px` high.
2. UI feels cleaner with reduced border harshness.
3. Controls still feel tight and precise (not loose).
4. No behavior regressions in tab/find/settings interactions.
