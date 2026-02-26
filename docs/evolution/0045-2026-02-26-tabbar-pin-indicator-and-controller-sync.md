# 0045-2026-02-26-tabbar-pin-indicator-and-controller-sync

## Metadata

- Date: 2026-02-26
- Sequence: 0045
- Status: active
- Scope: runtime

## Why This Entry Exists

Pin/unpin behavior existed at controller/window level, but tab-bar UI had no visible pinned-state indicator. Users could toggle pin state without a persistent in-window affordance confirming current state.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/terminal_view.rs`
- Upstream constraints (platform, library, policy):
  - `AppShellController` is the source of truth for app-shell window state.
  - `TerminalView` renders UI state and routes user interactions via callbacks.
- Invariants already in force:
  - pinning must not bypass controller command routing
  - native window pin level remains applied in `macos::set_window_pinned`

## Decision and Rationale

- Decision:
  - add a dedicated tab-bar pin indicator button (`📌` pinned, `○` unpinned)
  - add controller-to-view pinned-state synchronization (`set_pinned`)
  - route indicator clicks back to controller through `on_toggle_pin_requested`
  - keep pin/unpin independent from show/hide placement flow (must not move window between monitors)
- Why this path was selected:
  - keeps state ownership centralized while exposing explicit UX feedback
  - avoids view-local state divergence from native window pin level
- Trade-offs accepted:
  - indicator occupies one fixed tab-bar control slot

## Alternatives Considered

1. Infer pin state only from window behavior (no indicator)
- Pros:
  - no UI changes
- Cons:
  - poor discoverability and ambiguous state
- Why not chosen:
  - does not satisfy visible state affordance requirement

2. Let view own pin state independently
- Pros:
  - simpler local UI toggling
- Cons:
  - risks drift from controller/native pin level
- Why not chosen:
  - violates controller-owned state invariant

## Safe Change Playbook

When modifying this area, follow these steps:
1. Keep controller as the only mutator of canonical pinned state.
2. Push pinned state into `TerminalView` on every pin-affecting controller path.
3. Keep view actions callback-based (`on_toggle_pin_requested`), not direct native calls.

## Do / Avoid

Do:
- update both existing-window and new-window paths with view pin sync
- keep indicator text derived from the synchronized `pinned` field
- treat pin/unpin as window-level change only, not a placement command

Avoid:
- duplicating pin mutation logic inside `TerminalView`
- adding alternate pin pathways that bypass `AppCommand::TogglePinned`
- calling `show_terminal()` from pin-toggle handling

## Typical Mistakes

- Rendering a pin indicator without wiring state synchronization, resulting in stale UI.
- Toggling native pin level directly from the view, causing controller/view divergence.
- Reusing show/hide logic in pin toggle, which repositions window using current mouse monitor.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test -p simple-term-app pin_indicator_symbol_maps_pinned_and_unpinned_states`
- Recommended manual checks:
  - toggle pin via hotkey and verify indicator updates immediately
  - click indicator and verify pin/unpin behavior and icon state stay in sync
- Signals of regression:
  - indicator does not change after pin toggle
  - pinned window behavior changes while indicator remains unchanged

## Related Artifacts

- Related docs:
  - `docs/plans/2026-02-26-local-pin-hotkey-and-tab-indicator-design.md`
  - `docs/evolution/0036-2026-02-26-macos-toggle-and-pin-hotkeys.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/terminal_view.rs`
