# 0014-2026-02-24-terminal-theme-presets-and-persistence

## Metadata

- Date: 2026-02-24
- Sequence: 0014
- Status: active
- Scope: runtime, architecture, testing

## Why This Entry Exists

The terminal previously used one hardcoded atom_one_dark-like palette for app chrome, cursor, and fallback ANSI colors. Users could not choose a different visual theme, and there was no stable configuration key for palette selection in `settings.json`.

This entry records the new invariant: theme selection is now explicit, persisted, and consumed by both terminal fallback colors and app-shell rendering.

## System Context

- Relevant directories/modules:
  - `crates/simple-term/src/terminal_settings.rs`
  - `apps/simple-term/src/terminal_view.rs`
  - `README.md`
- Upstream constraints (platform, library, policy):
  - terminal cell colors still come from `alacritty_terminal` renderable content; theme presets only provide fallback palette values when terminal colors are not explicitly set
  - tab-bar settings controls must remain compact and coexist with existing font/mode controls
  - settings updates must persist via `TerminalSettings::save` without breaking backward compatibility for existing `settings.json`
- Invariants already in force:
  - settings schema must remain serde-loadable with defaults for missing fields
  - tab-scoped rendering/input behavior must stay unchanged
  - runtime appearance updates must trigger repaint without requiring app restart

## Decision and Rationale

- Decision:
  - Add a `TerminalTheme` enum in `TerminalSettings` with six presets (`atom_one_dark`, `gruvbox_dark`, `tokyo_night`, `catppuccin_mocha`, `nord`, `solarized_dark`).
  - Add a theme selector to the tab-bar settings panel with previous/next controls.
  - Route theme values into both:
    - tab/terminal chrome backgrounds and cursor fill
    - ANSI/foreground/background fallback palette used by `ColorsSnapshot`.
- Why this path was selected:
  - keeps user-facing configuration simple (single `theme` field)
  - avoids introducing per-color custom editing UI and schema complexity
  - aligns with existing runtime settings mutation + persistence pipeline
- Trade-offs accepted:
  - preset-only model (no arbitrary custom palettes yet)
  - theme changes apply globally, not per-tab

## Alternatives Considered

1. Hardcode one alternate palette only (light/dark toggle)
- Pros:
  - minimal code changes
- Cons:
  - still too limited for user preference
- Why not chosen:
  - requirement is broader palette choice, not binary mode

2. Full custom color editor in settings panel
- Pros:
  - maximum flexibility
- Cons:
  - high UI/state complexity and larger persistence schema surface
- Why not chosen:
  - over-scoped for current requirement

## Safe Change Playbook

When modifying theme support, follow these steps:
1. Keep `TerminalTheme` serde names stable once published; treat renamed variants as migration work.
2. If adding a preset, update all three places together: enum variant list, `THEME_PRESETS` cycling order, and `theme_palette` mapping.
3. Keep fallback ANSI, foreground, and background palette values bundled in one theme mapping function to avoid split-brain defaults.
4. Preserve runtime persistence flow (`self.settings.theme` update -> `persist_settings()` -> `cx.notify()`).

## Do / Avoid

Do:
- keep theme selection and rendering logic in `TerminalView` (UI layer)
- keep persisted theme schema in `terminal_settings` (config layer)
- add/maintain tests for deterministic wraparound theme cycling

Avoid:
- scattering hardcoded color constants outside `theme_palette`
- adding theme-dependent behavior to terminal input or tab lifecycle paths
- changing enum variant serde names without explicit migration handling

## Typical Mistakes

- Adding a new `TerminalTheme` variant but forgetting to include it in `THEME_PRESETS`, causing selector wraparound gaps.
- Updating tab/chrome colors but not fallback ANSI colors, causing mixed old/new appearance in terminal content.
- Changing display labels without preserving stable persisted enum names.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
- Recommended manual checks:
  - open settings panel and cycle theme left/right in a running app session
  - restart app and confirm selected theme is restored from `settings.json`
  - verify both terminal canvas and tab chrome change together
- Signals of regression:
  - selector changes text but not render colors
  - selected theme not persisted after restart
  - cursor color or default ANSI colors remain from previous theme

## Related Artifacts

- Related docs:
  - `docs/evolution/0013-2026-02-24-tabbar-settings-panel-and-runtime-appearance-controls.md`
  - `docs/architecture-invariants.md`
- Optional references (PRs/commits/releases):
  - `crates/simple-term/src/terminal_settings.rs`
  - `apps/simple-term/src/terminal_view.rs`
