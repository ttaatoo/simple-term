# 0028-2026-02-25-window-deactivation-hide-deferral-for-app-borrow-safety

## Metadata

- Date: 2026-02-25
- Sequence: 0028
- Status: active
- Scope: runtime, testing

## Why This Entry Exists

A reproducible macOS shell flow surfaced another `RefCell already borrowed` error:
- close the terminal window
- summon the app again from the menubar icon

Earlier guardrails focused on controller-level `RefCell` contention (`try_borrow_mut` + requeue), but this failure was in a different layer: app-level borrow re-entry timing.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
  - `apps/simple-term/src/main.rs`
- Upstream constraints (platform, library, policy):
  - `AsyncApp::update` in GPUI acquires `AppCell::borrow_mut()` and panics on concurrent mutable borrow.
  - `observe_window_activation` callbacks execute while a window/app update is already in progress.
  - GPUI activation observers invoke an initial callback immediately on registration.
  - sending shell commands synchronously from activation/deactivation callbacks can be consumed before the current update fully unwinds.
- Invariants already in force:
  - controller command routing remains channel-based and re-entrancy-aware for controller state (`try_borrow_mut` + requeue).
  - outside-click/window-deactivation hide behavior must remain intact when enabled.

## Decision and Rationale

- Decision:
  - make window-deactivation hide requests deferred instead of immediate by scheduling callback execution via `cx.defer(...)`.
  - gate deactivation-hide scheduling behind a "window has been active at least once" state so the observer's initial inactive callback cannot trigger an immediate hide.
  - centralize the scheduling predicate in `TerminalView::should_schedule_window_deactivation_hide(...)`.
  - centralize callback scheduling in `TerminalView::schedule_window_deactivation_hide(...)`.
  - add regression tests that enforce scheduling conditions and deferred callback semantics.
- Why this path was selected:
  - removes app-borrow re-entry timing hazard without redesigning the command architecture.
  - keeps behavior unchanged from user perspective (hide still occurs) while changing execution phase safely.
  - provides testable helper boundaries for future edits.
- Trade-offs accepted:
  - hide command dispatch now happens one deferred cycle later.
  - callback flow is slightly less direct than an inline call.

## Alternatives Considered

1. Keep synchronous deactivation callback and only harden controller borrow sites
- Pros:
  - minimal code movement
- Cons:
  - does not address app-level mutable borrow re-entry in `AsyncApp::update`
- Why not chosen:
  - leaves the close/reopen error path intact

2. Replace command loop architecture to avoid `AsyncApp::update`
- Pros:
  - could eliminate this class of timing hazard more globally
- Cons:
  - significantly broader refactor for a targeted runtime regression
- Why not chosen:
  - disproportionate scope/risk for the observed failure

## Safe Change Playbook

When modifying activation/deactivation-driven shell behavior:
1. treat activation observers as in-update callbacks; do not run command-emitting side effects synchronously there.
2. do not treat the observer's initial callback as "outside click"; only allow deactivation-hide after at least one active window state has been observed.
3. defer command-emitting callbacks (`cx.defer`) so they run after the current update frame.
4. keep the scheduling predicate in one helper and protect it with focused unit tests.
5. verify close/reopen and outside-click hide paths together after changes.

## Do / Avoid

Do:
- keep deactivation-triggered hide actions deferred from observer callback stack
- keep an explicit `window_has_been_active`-style guard before scheduling deactivation hide
- keep scheduling predicates explicit and test-backed
- preserve existing controller command pipeline and visibility semantics

Avoid:
- direct `on_window_deactivated()` invocation inside `observe_window_activation` callback body
- treating the activation observer's initial callback as a real deactivate transition
- coupling observer callback timing to immediate channel send side effects
- broad architectural rewrites for this targeted borrow-timing hazard

## Typical Mistakes

- assuming controller-level `try_borrow_mut` protection also prevents app-level borrow re-entry.
- dispatching commands synchronously from activation/deactivation observers.
- forgetting that activation observers may fire immediately at registration time, before the window first becomes active.
- adding new observer side effects without considering current GPUI update-stack timing.

## Verification Strategy

- Required automated checks:
  - `cargo test -p simple-term-app window_deactivation_hide`
  - `cargo test -p simple-term-app`
  - `cargo check --workspace`
- Recommended manual checks:
  - close window via titlebar controls, then click menubar icon to reopen; confirm no `RefCell already borrowed` log
  - with `auto_hide_on_outside_click` enabled, click outside the terminal and confirm hide still triggers
- Signals of regression:
  - logs show `RefCell already borrowed` in close/reopen or deactivation paths
  - outside-click deactivation no longer hides when setting is enabled

## Related Artifacts

- Related docs:
  - `docs/evolution/0016-2026-02-25-settings-drawer-scroll-and-menubar-command-reentrancy.md`
  - `docs/evolution/0025-2026-02-25-unified-macos-app-shell-without-dock-mode.md`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/terminal_view.rs`
  - `apps/simple-term/src/main.rs`
