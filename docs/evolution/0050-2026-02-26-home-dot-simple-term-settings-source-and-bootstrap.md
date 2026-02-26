# 0050-2026-02-26-home-dot-simple-term-settings-source-and-bootstrap

## Metadata

- Date: 2026-02-26
- Sequence: 0050
- Status: active
- Scope: runtime

## Why This Entry Exists

The settings source was previously tied to platform-specific `ProjectDirs` resolution and did not guarantee that a physical `settings.json` file existed until a later save path was triggered. Users requested a VSCode-like model where settings are always backed by a known JSON file path that can be edited directly.

## System Context

- Relevant directories/modules:
  - `crates/simple-term/src/terminal_settings.rs`
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/terminal_view.rs`
  - `README.md`
- Upstream constraints (platform, library, policy):
  - UI controls already persist settings through `TerminalSettings::save(...)`.
  - Manual JSON editing must target the same source used by UI persistence.
  - Config bootstrap must not overwrite existing user files.
- Invariants already in force:
  - one JSON source of truth for persisted settings
  - missing config should fall back to defaults without crashing

## Decision and Rationale

- Decision:
  - pin config directory to `~/.simple-term`
  - add `TerminalSettings::load_or_create(...)` and use it at app startup
  - create `~/.simple-term/settings.json` with default JSON only when missing
  - keep existing save-path behavior for UI-driven updates
- Why this path was selected:
  - gives users a stable and discoverable path for direct JSON edits
  - preserves current UI settings flow while guaranteeing file bootstrap
  - avoids migration complexity because schema/persistence model remains unchanged
- Trade-offs accepted:
  - runtime manual edits are still loaded on startup (not hot-reloaded in this change)

## Alternatives Considered

1. Keep `ProjectDirs` path and only document the resolved location
- Pros:
  - no path behavior change
- Cons:
  - path remains less discoverable than an explicit home-dot folder
  - does not satisfy explicit `~/.simple-term` requirement
- Why not chosen:
  - does not match user-facing location contract

2. Create config only when UI first saves a setting
- Pros:
  - no startup write path
- Cons:
  - direct JSON editing flow is blocked until a UI save happens
  - first-run expectation of existing settings file is not met
- Why not chosen:
  - violates bootstrap requirement

## Safe Change Playbook

When modifying this area, follow these steps:
1. Keep `TerminalSettings::config_dir()` and `TerminalSettings::config_path()` aligned with the documented path contract.
2. Keep bootstrap creation logic in startup (`load_or_create`) and avoid writing defaults over existing files.
3. Preserve `TerminalSettings::save(...)` as the single persistence path used by UI controls and controller state updates.

## Do / Avoid

Do:
- treat `~/.simple-term/settings.json` as the canonical persisted settings file
- ensure missing directory/file bootstrap uses default serialized settings
- keep README and UI guidance strings aligned with the real path

Avoid:
- reintroducing platform-dependent hidden config paths for persisted settings
- overwriting an existing but invalid user-edited settings file during load
- splitting settings persistence across multiple files without an explicit migration plan

## Typical Mistakes

- Calling `load(...)` at startup and forgetting to materialize missing files.
- Updating docs/UI text without updating actual path resolution logic (or vice versa).
- Saving defaults unconditionally and unintentionally clobbering user config.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test -p simple-term`
- Recommended manual checks:
  - remove `~/.simple-term/settings.json` and start the app
  - verify `~/.simple-term/` and `settings.json` are created automatically
  - edit `~/.simple-term/settings.json`, restart app, and verify edited values load
- Signals of regression:
  - app starts without creating missing settings file
  - settings path in UI/docs differs from actual path used for read/write
  - first save writes to a different directory than startup load path

## Related Artifacts

- Related docs:
  - `docs/evolution/0015-2026-02-24-settings-drawer-v1-and-live-persistence.md`
  - `docs/evolution/0037-2026-02-26-settings-panel-global-hotkey-control.md`
- Optional references (PRs/commits/releases):
  - `crates/simple-term/src/terminal_settings.rs`
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/terminal_view.rs`
