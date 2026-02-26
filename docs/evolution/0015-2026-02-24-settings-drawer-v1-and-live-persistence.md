# 0015-2026-02-24-settings-drawer-v1-and-live-persistence

## Metadata

- Date: 2026-02-24
- Sequence: 0015
- Status: active
- Scope: runtime, architecture, testing

## Why This Entry Exists

The settings interaction model changed from an inline tab-bar strip to a dedicated right-side drawer. This is a structural UI/runtime change because settings now occupy a persistent side region while terminal rendering remains active, and settings writes happen immediately on every supported control update.

Commit diffs alone do not capture the invariants around focus routing, runtime apply order, and save-path centralization, so this entry records those rules.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
  - `crates/simple-term/src/terminal_settings.rs`
  - `README.md`
- Upstream constraints (platform, library, policy):
  - GPUI terminal input is canvas-driven; keyboard routing must preserve shell input behavior.
  - Settings persistence must remain backward compatible with existing `settings.json` schema.
  - `dock_mode` behavior is macOS-specific and must not introduce non-macOS runtime coupling.
- Invariants already in force:
  - terminal geometry updates must go through shared typography + grid-resync flow
  - settings writes must stay centralized through `persist_settings()`
  - tab-scoped terminal state must not leak across tabs

## Decision and Rationale

- Decision:
  - Replace inline tab-bar settings panel with a 360px right drawer opened by the top-right `⚙` control.
  - Keep V1 scope to frequent controls only and apply/save changes immediately.
  - Preserve `TerminalSettings` schema; no breaking config changes in V1.
  - Keep non-macOS `dock_mode` in disabled/read-only presentation with explicit UI explanation.
- Why this path was selected:
  - avoids tab-bar crowding and layout coupling with find controls
  - supports grouped settings growth without adding schema risk
  - keeps runtime behavior simple (`update -> apply -> persist`) with low rollback complexity
- Trade-offs accepted:
  - no transactional Apply/Cancel model in V1
  - numeric controls currently use stepper interactions instead of a full form editor

## Alternatives Considered

1. Keep inline panel and continue extending controls
- Pros:
  - minimal code churn
- Cons:
  - tab bar becomes crowded and harder to maintain with find panel and tab controls
- Why not chosen:
  - poor scalability and weaker readability for grouped settings

2. Add a separate preferences window
- Pros:
  - more room for advanced editors
- Cons:
  - slower quick-adjust workflow during active terminal use
- Why not chosen:
  - does not match the lightweight in-session tuning model for V1

## Safe Change Playbook

When modifying this area, follow these steps:
1. Keep drawer visibility state and key handling consistent: open/close from tab-bar control, close on plain `Esc`, and do not block normal terminal key routing when drawer is closed.
2. For runtime settings updates, preserve operation order: mutate `self.settings` -> apply runtime effect (if needed) -> persist through `persist_settings()`.
3. Route typography-affecting settings (`font_family`, `font_size`, `line_height`) through `apply_typography_settings(...)` so grid/cell recalculation stays consistent for every tab.
4. Keep platform checks in UI behavior only (`dock_mode` availability), without introducing platform-specific fields or schema branches.
5. Maintain V1 scope boundaries; advanced editors (`shell`, `working_directory`, `env`, regex lists, hotkey recorder) should remain explicit V2 work.

## Do / Avoid

Do:
- keep settings sections explicit (`Appearance`, `Behavior`, `Window`, `Advanced`)
- keep persistence centralized via `persist_settings()` and helpers that call it
- add pure helper tests for keyboard-close logic and value normalization

Avoid:
- reintroducing inline settings controls in the tab bar flow
- adding ad-hoc file writes for settings from multiple code paths
- binding non-macOS behavior to AppKit-specific dock mode logic

## Typical Mistakes

- Updating settings UI state without applying runtime side effects (e.g., font changes without grid resync).
- Saving from multiple places with inconsistent write timing.
- Handling `Esc` only in the drawer and accidentally breaking close behavior when terminal surface owns focus.
- Expanding V1 drawer with unscoped advanced fields that require complex validation/editing semantics.

## Verification Strategy

- Required automated checks:
  - `cargo fmt --all`
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - open/close drawer via `⚙`, `×`, and `Esc`
  - verify theme/font/font-size/line-height updates apply immediately and persist after restart
  - verify `keep_selection_on_copy` controls are disabled when `copy_on_select=false`
  - verify non-macOS `dock_mode` shows disabled state and explanation
- Signals of regression:
  - terminal input focus lost after opening/closing drawer
  - settings values revert after restart
  - typography changes leave stale row/column sizing

## Related Artifacts

- Related docs:
  - `docs/evolution/0013-2026-02-24-tabbar-settings-panel-and-runtime-appearance-controls.md`
  - `docs/evolution/0014-2026-02-24-terminal-theme-presets-and-persistence.md`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/terminal_view.rs`
  - `README.md`
