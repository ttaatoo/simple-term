# 0037-2026-02-26-settings-panel-global-hotkey-control

## Metadata

- Date: 2026-02-26
- Sequence: 0037
- Status: active
- Scope: runtime

## Why This Entry Exists

Global shortcut configuration existed in `settings.json` but was not changeable from the in-app settings panel. Users requested changing the show/hide shortcut directly in UI, with immediate runtime effect.

This knowledge spans settings defaults, settings-panel controls, controller command routing, and hotkey re-registration lifecycle.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
  - `apps/simple-term/src/main.rs`
  - `crates/simple-term/src/terminal_settings.rs`
- Upstream constraints (platform, library, policy):
  - global hotkeys are registered through `global-hotkey` manager owned by app-shell controller
  - settings panel persists via `TerminalSettings::save`
  - app-shell remains authority for hide/show command handling
- Invariants already in force:
  - `TerminalView` should not register OS hotkeys directly
  - hotkey registration failures must not crash runtime

## Decision and Rationale

- Decision:
  - change default show/hide shortcut to `command+F4`
  - expose a settings-panel recording control for `global_hotkey` (`record` -> press shortcut)
  - apply shortcut changes immediately by sending `AppCommand::UpdateHotkeys` from `TerminalView` to `AppShellController`
  - when reapplying shortcuts, drop previous manager registrations first
- Why this path was selected:
  - preserves controller-owned lifecycle and avoids duplicate registration logic in view layer
  - immediate apply matches user expectation and reduces restart friction
  - recording mode avoids predefined shortcuts and captures the user’s intended key combo directly
- Trade-offs accepted:
  - recorder accepts a validated subset of keys that map cleanly to `global-hotkey` parser tokens
  - pin shortcut remains JSON-configurable only in this iteration

## Alternatives Considered

1. Persist-only update (apply on next launch)
- Pros:
  - no controller/runtime changes
- Cons:
  - poor UX; user sees changed setting that does not work immediately
- Why not chosen:
  - conflicts with explicit immediate-apply requirement

2. Direct registration from `TerminalView`
- Pros:
  - fewer command-bus edits
- Cons:
  - violates ownership boundary; risks duplicate managers and desynchronized state
- Why not chosen:
  - controller already owns app-shell hotkey lifecycle

## Safe Change Playbook

When modifying settings-driven global shortcut behavior, follow these steps:
1. Keep OS-level registration in `AppShellController` only.
2. Send settings-driven hotkey updates through `AppCommand` variants.
3. Drop old hotkey manager registrations before applying replacements.
4. Keep `TerminalSettings` default/fallback strings aligned with app-shell fallback `HotKey` values.
5. Add tests for key-capture helper behavior (valid combo, modifier-only rejection, no-modifier rejection).

## Do / Avoid

Do:
- keep immediate persistence and immediate runtime apply in sync
- keep hotkey compatibility remaps centralized in controller parse helpers
- keep settings UI labels explicit about scope (`Show/Hide Shortcut`)

Avoid:
- introducing separate hotkey parser logic in `TerminalView`
- re-registering hotkeys without first releasing previous manager state
- changing defaults in only one layer (settings schema or runtime fallback) without the other

## Typical Mistakes

- Updating default string in `TerminalSettings` but forgetting runtime fallback in `AppShellController`.
- Sending new hotkey values to controller without actually re-running registration.
- Re-registering on top of existing manager and accidentally preserving stale bindings.

## Verification Strategy

- Required automated checks:
  - `cargo fmt --all`
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - open settings panel, click `record`, then press `Cmd+F4` (or another combo)
  - verify new shortcut works immediately and old shortcut stops working
  - restart app and verify selected shortcut persists
- Signals of regression:
  - old shortcut continues to toggle after settings change
  - new shortcut only works after restart
  - duplicate toggle events from stale + new registrations

## Related Artifacts

- Related docs:
  - `docs/evolution/0036-2026-02-26-macos-toggle-and-pin-hotkeys.md`
  - `docs/evolution/0015-2026-02-24-settings-drawer-v1-and-live-persistence.md`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/terminal_view.rs`
  - `crates/simple-term/src/terminal_settings.rs`
