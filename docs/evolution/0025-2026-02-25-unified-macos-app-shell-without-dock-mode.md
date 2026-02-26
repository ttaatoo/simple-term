# 0025-2026-02-25-unified-macos-app-shell-without-dock-mode

## Metadata

- Date: 2026-02-25
- Sequence: 0025
- Status: active
- Scope: runtime, architecture

## Why This Entry Exists

The macOS app shell previously retained two startup paths (`regular` and `menubar_only`) controlled by `dock_mode`. After window-behavior parity work, that split no longer represented a meaningful user-facing capability boundary, but it still created controller drift risk and duplicated control flow.

This entry records the deliberate breaking change: remove `dock_mode` and run one macOS app-shell model with startup-open window plus menubar/hotkey show-hide toggling.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/macos.rs`
  - `apps/simple-term/src/terminal_view.rs`
  - `crates/simple-term/src/terminal_settings.rs`
- Upstream constraints (platform, library, policy):
  - menubar status-item visibility depends on retaining a long-lived `StatusItemHandle`.
  - global hotkey registration can fail at parse/register time and must degrade gracefully.
  - hide/show state is controller-owned; `TerminalView` must request hide via callback to avoid state drift.
- Invariants already in force:
  - command dispatch to macOS controller remains async and re-entrancy-safe (`try_borrow_mut` + requeue).
  - terminal core behavior in `crates/simple-term` must remain unchanged by app-shell wiring.

## Decision and Rationale

- Decision:
  - Remove `DockMode` and `TerminalSettings.dock_mode`.
  - Replace dual controllers with a single `AppShellController` on macOS.
  - Always start with a visible terminal window, and keep menubar click/global hotkey as show-hide toggle commands.
  - Keep activation policy fixed to regular app policy.
- Why this path was selected:
  - removes duplicated startup/controller branches that can drift in behavior
  - preserves requested UX (startup-open window + menubar/hotkey quick toggle)
  - keeps status-item and hotkey integrations centralized and testable
- Trade-offs accepted:
  - breaking settings schema change (`dock_mode` removed)
  - historical docs referencing mode split remain as history, not current behavior

## Alternatives Considered

1. Keep `dock_mode` field but ignore it at runtime
- Pros:
  - lower migration pressure in short term
- Cons:
  - stale config surface and misleading UI semantics
  - keeps dead branches and ambiguity for future contributors
- Why not chosen:
  - conflicts with simplification goal and increases long-term maintenance risk

2. Keep two controllers but force behavior parity
- Pros:
  - less immediate code movement
- Cons:
  - preserves dual-path complexity and future drift hazards
  - mode split remains conceptual overhead without durable value
- Why not chosen:
  - does not actually simplify architecture

## Safe Change Playbook

When modifying macOS app-shell behavior in this model, follow these steps:
1. Keep all visibility mutations in the single controller (`visible`, `show_terminal`, `hide_terminal`).
2. Route hide intents from `TerminalView` through callback/command path, not direct controller-external state mutation.
3. Install and retain status item + hotkey manager in controller lifetime fields to avoid disappearing menubar affordances.
4. Treat window-handle invalidation as normal recovery flow: clear stale handle and recreate lazily on next show.

## Do / Avoid

Do:
- keep startup and toggle behavior in one macOS entrypoint (`run_macos_app`)
- preserve fallback behavior when hotkey parsing/registration fails
- keep outside-click auto-hide controlled by `settings.auto_hide_on_outside_click`

Avoid:
- reintroducing mode-based startup branching for shell behavior
- bypassing controller bookkeeping with direct view-level hide for controller-managed flows
- dropping status-item handle ownership after bootstrap

## Typical Mistakes

- Reintroducing `dock_mode` in settings/UI for behavior already represented by explicit feature flags (`button`, `global_hotkey`, `auto_hide_on_outside_click`).
- Adding a second macOS controller path for “just one special case,” recreating drift risk.
- Making window activation/deactivation behavior update UI only and not controller `visible` state.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
- Recommended manual checks:
  - app startup opens terminal window on macOS
  - menubar click toggles show/hide
  - global hotkey toggles show/hide
  - outside-click hide follows `auto_hide_on_outside_click` setting
  - last-tab close hide path (`Cmd+W` on last tab) remains controller-synchronized
- Signals of regression:
  - settings include or render `dock_mode` again
  - toggle state gets stuck after hide/reopen cycles
  - menubar icon appears briefly then disappears (lost handle ownership)

## Related Artifacts

- Related docs:
  - `docs/evolution/0018-2026-02-25-macos-menubar-window-behavior-and-status-icon-parity.md`
  - `docs/evolution/0023-2026-02-25-last-tab-close-hides-window-via-controller-path.md`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
  - `docs/plans/2026-02-25-unified-macos-app-shell-design.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/macos.rs`
  - `apps/simple-term/src/terminal_view.rs`
  - `crates/simple-term/src/terminal_settings.rs`
