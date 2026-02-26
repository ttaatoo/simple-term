# 0027-2026-02-25-responsive-settings-and-find-ui-hardening

## Metadata

- Date: 2026-02-25
- Sequence: 0027
- Status: active
- Scope: runtime, testing

## Why This Entry Exists

The tab-bar find strip and settings popup used fixed dimensions and placeholder-style controls that were acceptable for V1 but not robust enough for production behavior. On narrower windows, controls could feel cramped, and the settings popup lacked an outside-click close path.

This entry records the new UI invariants so future updates keep the interface responsive and coherent across window sizes.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Upstream constraints (platform, library, policy):
  - GPUI layout primitives are pixel-based and require explicit width/height policy for responsive behavior.
  - Overlay interaction must remain occluded so background terminal hitboxes do not leak input.
- Invariants already in force:
  - settings panel opens in an overlay (not in terminal flex flow)
  - settings body remains scrollable with explicit non-zero scrollbar width
  - tab strip remains width-stable per tab item

## Decision and Rationale

- Decision:
  - add viewport-aware width helpers for find strip and settings popup
  - derive active tab indicator/background accent from the active theme cursor color (remove hardcoded accent)
  - normalize settings control heights for denser but usable click targets
  - replace placeholder-like find controls with actionable guidance text
  - allow clicking the overlay backdrop to close settings popup
  - clean labels/icons for consistency (`Atom One Dark`, chevron glyphs, close glyph)
- Why this path was selected:
  - low-risk upgrade within existing architecture
  - preserves current runtime state model while removing obvious UX rough edges
  - improves behavior on small and medium window sizes without adding new settings schema
- Trade-offs accepted:
  - responsive sizing still uses simple pixel heuristics, not breakpoint-specific design variants
  - find strip remains keyboard-driven (guidance text clarifies behavior instead of adding full text field controls)

## Alternatives Considered

1. Keep fixed-width panels and only adjust colors
- Pros:
  - minimal code churn
- Cons:
  - does not solve cramped/overflow behavior on smaller windows
- Why not chosen:
  - misses the main production-readiness problem

2. Build a full responsive layout system with breakpoints and component primitives
- Pros:
  - strongest long-term design scalability
- Cons:
  - much larger refactor than needed for current UI issues
- Why not chosen:
  - disproportionate scope for this stabilization pass

## Safe Change Playbook

When modifying this area, follow these steps:
1. Keep viewport-dependent widths centralized in helper methods (`find_panel_width_for_viewport`, `settings_drawer_width_for_viewport`) instead of scattering constants in render code.
2. Keep active accent colors theme-derived, not hardcoded to a specific hue.
3. Keep settings controls at or above the shared control-height token to avoid shrinking hit targets during style tweaks.
4. Preserve overlay close paths (`Esc`, close button, backdrop click) whenever changing popup layering.
5. Re-run unit checks for width helpers and settings-key-close logic after any layout update.

## Do / Avoid

Do:
- prefer responsive width clamps over fixed absolute widths in top-bar and overlay surfaces
- keep UI labels human-readable and consistent
- keep advanced-section copy actionable without placeholder language

Avoid:
- reintroducing hardcoded accent colors detached from theme data
- adding decorative, non-functional controls in the find strip
- removing backdrop close behavior from modal-like overlays

## Typical Mistakes

- Reverting find/settings widths back to fixed values, causing clipping or control crowding on narrow windows.
- Hardcoding a visual accent color that clashes with non-default themes.
- Styling controls below usability size when trying to “fit more” content in settings rows.
- Keeping only `Esc`/button close while forgetting backdrop-close expectations for overlays.

## Verification Strategy

- Required automated checks:
  - `cargo fmt --all`
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - resize window and verify find strip + settings popup remain usable
  - open settings and close via `Esc`, close button, and backdrop click
  - cycle themes and verify active tab accent follows selected theme
  - verify settings rows remain legible and clickable after size normalization
- Signals of regression:
  - find strip overlaps or disappears on narrow widths
  - settings popup cannot be closed by backdrop interaction
  - active tab indicator remains fixed color regardless of theme

## Related Artifacts

- Related docs:
  - `docs/evolution/0017-2026-02-25-settings-popup-overlay-window.md`
  - `docs/evolution/0019-2026-02-25-tab-title-width-stability-and-tooltip-overflow.md`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/terminal_view.rs`
