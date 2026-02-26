# 0026-2026-02-25-cursor-blink-suppression-during-input

## Metadata

- Date: 2026-02-25
- Sequence: 0026
- Status: active
- Scope: runtime

## Why This Entry Exists

The cursor previously continued normal blink cadence even while the user was actively typing. That can feel visually unstable during command entry and was explicitly reported as undesirable behavior.

This entry records the runtime rule that typing should temporarily suppress cursor blinking.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
- Upstream constraints (platform, library, policy):
  - cursor blink visibility is frontend-driven by a periodic timer (`spawn_cursor_blink_loop`)
  - terminal snapshot render path decides final cursor draw visibility each frame
- Invariants already in force:
  - blinking mode selection still follows `Blinking` setting semantics (`Off`, `On`, `TerminalControlled`)
  - cursor remains visible when blinking is effectively disabled

## Decision and Rationale

- Decision:
  - add a short post-input suppression window (`CURSOR_BLINK_SUPPRESSION_AFTER_INPUT`) that disables blinking immediately after terminal input
  - apply suppression in both blink timer and render visibility calculation
  - reset suppression state on active-tab frame-state reset
- Why this path was selected:
  - gives deterministic "typing = steady cursor" behavior
  - avoids changing persisted settings schema for a transient UX rule
  - keeps implementation localized to `TerminalView` state machine
- Trade-offs accepted:
  - blinking resumes only after suppression window and next blink tick cycle
  - introduces one additional timing state in view runtime state

## Alternatives Considered

1. Disable blinking globally when any key is pressed until user re-enables it
- Pros:
  - simple runtime behavior
- Cons:
  - unexpectedly mutates long-lived preference semantics
  - loses intended blinking behavior after typing stops
- Why not chosen:
  - too blunt for a transient interaction request

2. Keep existing behavior and only force cursor visible on input events
- Pros:
  - minimal code churn
- Cons:
  - blink cadence resumes immediately and can still flicker during continuous typing
- Why not chosen:
  - does not satisfy requested behavior in practice

## Safe Change Playbook

When modifying cursor blink behavior in this area, follow these steps:
1. Keep blink suppression checks centralized and shared between render and timer paths.
2. Ensure input-entry points route through `begin_terminal_input(...)` so suppression is consistently armed.
3. Preserve explicit tests for both mode-based blinking and suppression-window behavior.

## Do / Avoid

Do:
- keep suppression duration as a dedicated constant
- force cursor visibility to `true` when input arrives while cursor is currently hidden
- clear stale suppression timestamps once the window expires

Avoid:
- scattering independent blink suppression logic across multiple handlers
- tying suppression behavior to persisted user settings without product intent
- applying suppression to non-input interactions (e.g., passive repaint events)

## Typical Mistakes

- Updating timer toggle logic but forgetting render-path visibility calculation, causing inconsistent cursor states.
- Adding suppression state but not resetting it during tab/frame-state resets.
- Bypassing `begin_terminal_input(...)` for new input paths, resulting in behavior drift.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
- Recommended manual checks:
  - type continuously and verify cursor remains steady during active typing
  - stop typing and verify blinking resumes after a short delay
  - switch tabs and verify cursor state remains consistent
- Signals of regression:
  - cursor still blinks while rapidly typing
  - cursor remains permanently non-blinking after typing stops
  - cursor appears hidden after keypress until next long repaint cycle

## Related Artifacts

- Related docs:
  - `docs/evolution/0025-2026-02-25-unified-macos-app-shell-without-dock-mode.md`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/terminal_view.rs`
