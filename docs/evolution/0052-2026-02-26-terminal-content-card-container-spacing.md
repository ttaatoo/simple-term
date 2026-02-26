# 0052-2026-02-26-terminal-content-card-container-spacing

## Metadata

- Date: 2026-02-26
- Sequence: 0052
- Status: active
- Scope: runtime, testing

## Why This Entry Exists

Terminal text and background were rendered flush against the window edge in the content area, producing a cramped visual feel. The fix intentionally introduces lightweight container-level spacing without changing terminal input/paint coordinate semantics.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
- Upstream constraints (platform, library, policy):
  - keep top tab bar height and behavior unchanged
  - avoid touching terminal coordinate mapping and scroll behavior
  - preserve existing canvas rendering pipeline
- Invariants already in force:
  - terminal interaction behavior (typing/selection/scrolling) must remain stable
  - settings overlay layering must remain intact

## Decision and Rationale

- Decision:
  - wrap `terminal_surface` with a lightweight container in `content_row`
  - add outer content padding while keeping frame visuals transparent (no visible border/background)
  - keep canvas rendering and event logic untouched
- Why this path was selected:
  - solves edge-clinging text issue at container level with minimal behavioral risk
- Trade-offs accepted:
  - slightly reduced raw drawable area due to new content padding

## Alternatives Considered

1. Shift canvas draw origin inward
- Pros:
  - no extra container layer
- Cons:
  - high risk for click/selection/scroll coordinate regressions
- Why not chosen:
  - unnecessary complexity for a visual spacing issue

2. Increase only root padding without container wrapper
- Pros:
  - smaller diff
- Cons:
  - weaker control over future styling hooks
- Why not chosen:
  - did not meet "more beautiful" requirement

## Safe Change Playbook

When modifying this area, follow these steps:
1. Apply spacing at container composition level before touching canvas math.
2. Keep `terminal_surface` rendering and event handlers unchanged unless fixing a specific bug.
3. Keep wrapper layout simple (`flex_1`, `min_h(0)`) to avoid accidental sizing regressions.
4. Re-run full test suite and manual interaction smoke checks.

## Do / Avoid

Do:
- isolate visual spacing to layout wrappers
- keep frame visuals transparent unless explicitly requested
- validate interaction behavior after visual container changes

Avoid:
- mixing coordinate/selection refactors with pure visual polish
- adding visible border/background before validating terminal readability
- changing tab/settings behavior in the same patch

## Typical Mistakes

- Adding padding inside the canvas draw math and accidentally offsetting mouse hit testing.
- Using a non-flex wrapper that can collapse child layout.
- Adding visible frame contrast that overpowers terminal content.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - typing, selection drag, scroll wheel
  - window resize behavior with lightweight wrapper
  - settings overlay open/close layering
- Signals of regression:
  - click/selection offset mismatch
  - collapsed content area due to wrapper sizing
  - reduced text readability from visible frame styling

## Related Artifacts

- Related docs:
  - `docs/plans/2026-02-26-ui-spacing-refresh-design.md`
  - `docs/plans/2026-02-26-ui-spacing-refresh.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/terminal_view.rs`
