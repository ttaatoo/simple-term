# 0031-2026-02-25-settings-overlay-position-invariant-and-focus-dim

## Metadata

- Date: 2026-02-25
- Sequence: 0031
- Status: active
- Scope: runtime, testing

## Why This Entry Exists

The settings popup is intended to be an overlay that does not affect terminal layout. A subtle but high-impact regression appeared when the overlay chain combined `.absolute()` and `.relative()` on the same element. In GPUI, later position calls overwrite earlier ones, so the overlay silently re-entered normal flex flow and compressed terminal content.

This behavior is easy to miss in diffs because both calls are individually valid, but their combination breaks the core popup invariant.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Upstream constraints (platform, library, policy):
  - GPUI position utilities (`relative`, `absolute`) write the same `position` style field.
  - Popup overlays must remain out of flow and attached to a `relative()` parent root.
- Invariants already in force:
  - opening settings must not resize or reflow terminal content
  - settings overlay must occlude background interactions
  - backdrop dim should provide focus affordance without obscuring terminal context

## Decision and Rationale

- Decision:
  - keep `#settings-popup-overlay` strictly absolute-positioned
  - remove conflicting `.relative()` from the overlay root
  - set backdrop dim to a lighter token (`SETTINGS_OVERLAY_BACKDROP_ALPHA = 0.28`)
- Why this path was selected:
  - addresses the layout regression at the source with minimal structural churn
  - keeps existing popup close paths and settings controls unchanged
  - improves focus cue while preserving background readability
- Trade-offs accepted:
  - backdrop darkness is opinionated; future tuning may be needed per theme feedback

## Alternatives Considered

1. Keep current overlay structure and only lower dim alpha
- Pros:
  - tiny visual-only change
- Cons:
  - does not fix terminal clipping/layout influence regression
- Why not chosen:
  - symptom-only; root cause remains

2. Move settings popup into separate native window
- Pros:
  - total separation from terminal layout tree
- Cons:
  - significantly larger lifecycle/focus complexity
- Why not chosen:
  - too heavy for this regression class

## Safe Change Playbook

When modifying this area, follow these steps:
1. Keep popup overlay root absolute and anchored with full-screen insets (`top/right/bottom/left = 0`).
2. Never chain mutually exclusive position setters (`.absolute()` then `.relative()`) on the same overlay container.
3. Keep popup positioning wrappers (`.relative()`, `.size_full()`, centering layout) in child containers, not the absolute overlay root.
4. Preserve `occlude()` so terminal hitboxes do not consume overlay interactions.
5. Re-verify first-row terminal rendering after popup open/close, not just popup visuals.

## Do / Avoid

Do:
- use a single backdrop alpha token for consistent focus dim behavior
- keep overlay composition at terminal root level, outside terminal content flow
- validate popup-open behavior on both wide and narrow window sizes

Avoid:
- adding non-absolute position calls to the overlay root
- embedding popup card directly into terminal content flex rows
- increasing dim opacity until terminal context becomes unreadable

## Typical Mistakes

- Treating `.relative()` as additive after `.absolute()`; in GPUI it overrides `position`.
- Debugging clipped terminal rows as font/render issues when layout flow is the true cause.
- Changing backdrop styling without re-checking overlay layering and flow isolation.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test -p simple-term-app settings_overlay_backdrop_alpha_stays_subtle -- --nocapture`
- Recommended manual checks:
  - open settings and verify terminal row geometry does not shift or clip
  - confirm backdrop dim is present but subtle
  - close popup via backdrop click and `Esc`
- Signals of regression:
  - opening settings reduces terminal content area or clips top rows
  - popup appears to push content instead of overlaying it
  - backdrop either missing or so dark that terminal context is lost

## Related Artifacts

- Related docs:
  - `docs/evolution/0017-2026-02-25-settings-popup-overlay-window.md`
  - `docs/evolution/0027-2026-02-25-responsive-settings-and-find-ui-hardening.md`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/terminal_view.rs`
