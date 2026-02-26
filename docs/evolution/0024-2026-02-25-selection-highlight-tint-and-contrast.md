# 0024-2026-02-25-selection-highlight-tint-and-contrast

## Metadata

- Date: 2026-02-25
- Sequence: 0024
- Status: active
- Scope: runtime

## Why This Entry Exists

Selection rendering previously relied on foreground/background inversion per selected cell. On dark themes this often produced near-white blocks with dark text, which looked visually harsh and disconnected from each theme's palette. This behavior was not obvious from commit history because the inversion logic lived in snapshot preprocessing, not in an explicit "selection style" API.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
- Upstream constraints (platform, library, policy):
  - `alacritty_terminal` provides cell fg/bg values and selection range membership.
  - This frontend owns final visual composition of selected cells.
- Invariants already in force:
  - theme palette values remain the source of visual identity for terminal UI and cursor.
  - snapshot preprocessing must stay deterministic and cheap.

## Decision and Rationale

- Decision:
  - Replace selection fg/bg inversion with a themed background tint.
  - Compute selection background as `blend(cell_bg, theme_cursor, 0.30)` and keep foreground unchanged.
- Why this path was selected:
  - Preserves syntax/token foreground readability.
  - Keeps selection visually consistent with active theme accents.
  - Avoids extreme white highlight blocks in dark palettes.
- Trade-offs accepted:
  - Colored backgrounds now receive a tint instead of semantic inverse.
  - Selection styling is more aesthetic than terminal-traditional inverse behavior.

## Alternatives Considered

1. Keep inversion and only clamp white backgrounds
- Pros:
  - Preserves historical terminal inverse semantics.
- Cons:
  - Requires brittle, branchy heuristics by color range.
  - Still causes abrupt contrast jumps on many color combinations.
- Why not chosen:
  - Harder to reason about and less theme-consistent.

2. Use a fixed neutral selection color for all themes
- Pros:
  - Simple implementation and predictable snapshots.
- Cons:
  - Ignores theme accents and can mismatch warm/cool palettes.
- Why not chosen:
  - Theme-specific cursor tint already exists and gives a better visual anchor.

## Safe Change Playbook

When modifying selection styling, follow these steps:
1. Keep selection color derivation inside snapshot preprocessing (`take_snapshot`) so render paths remain data-driven.
2. Preserve `resolve_alac_rgb` as the canonical conversion path before blending.
3. Add/adjust unit tests for blend math and selected-background derivation before changing constants.
4. Manually verify selection readability on at least AtomOneDark and one non-black-background theme.

## Do / Avoid

Do:
- Keep selection tint tied to theme palette data (cursor/accent).
- Keep blend alpha explicit and centrally declared.
- Validate selected background remains different from both raw background and foreground fallback colors.

Avoid:
- Reintroducing fg/bg swaps for selection in preprocessing.
- Hardcoding per-theme selection colors in multiple render branches.
- Coupling selection color logic to cursor blink or cursor shape logic.

## Typical Mistakes

- Treating selection as "just inverse" and bypassing theme palette contracts.
- Mixing colors in HSL/HSLA without a stable conversion path from terminal RGB values.
- Duplicating color resolution logic outside `resolve_alac_rgb`, leading to drift.

## Verification Strategy

- Required automated checks:
  - `cargo test -p simple-term-app blend_rgb_interpolates_channels`
  - `cargo test -p simple-term-app selection_background_color_uses_soft_tint_instead_of_foreground_swap`
- Recommended manual checks:
  - Select multiline text in dark themes and confirm highlight is tinted (not pure white).
  - Confirm token foreground colors remain readable while selected.
- Signals of regression:
  - Selection appears as bright white/gray inversion blocks.
  - Selected text foreground flips unexpectedly to background color.

## Related Artifacts

- Related docs:
  - `docs/evolution/0014-2026-02-24-terminal-theme-presets-and-persistence.md`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Optional references (PRs/commits/releases):
  - N/A
