# 0018-2026-02-25-macos-menubar-window-behavior-and-status-icon-parity

## Metadata

- Date: 2026-02-25
- Sequence: 0018
- Status: active
- Scope: runtime, architecture

## Why This Entry Exists

The original menubar quick-terminal shell optimized for a dropdown-like panel (`PopUp` + pinned placement). That conflicted with desktop-window expectations: users could not freely resize/move/zoom the window, Dock presence differed by mode, and regular mode lacked status-item parity.

Code diffs alone do not capture the shell-level behavioral contract change, so this entry records the new invariants.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/macos.rs`
- Upstream constraints (platform, library, policy):
  - GPUI on macOS only applies resizable style-mask behavior when titlebar configuration includes the resizable path.
  - `WindowKind::PopUp` behaves like a utility panel and is not suitable for normal desktop move/zoom expectations.
  - menubar status item visibility depends on retaining `StatusItemHandle` for app lifetime.
- Invariants already in force:
  - menubar command dispatch remains async and re-entrancy-safe (`try_borrow_mut` + requeue).
  - core terminal behavior (`crates/simple-term`) remains unchanged.

## Decision and Rationale

- Decision:
  - In menubar mode, create terminal windows as normal desktop windows (not popup utility panels), keep them movable/resizable/minimizable, and stop re-pinning/re-sizing on every toggle.
  - Keep Dock visible in menubar mode by using regular activation policy.
  - In regular mode, also install/status-manage menubar icon and route icon commands to activate/reopen the standard window.
- Why this path was selected:
  - directly resolves missing resize/move/zoom behavior in menubar mode
  - aligns app-shell affordances across runtime modes (Dock + menubar icon)
  - preserves existing shortcut + outside-click hide flow in quick-terminal path
- Trade-offs accepted:
  - menubar mode no longer enforces strict “always under menu bar” placement after first creation
  - `dock_mode = menubar_only` now mainly controls startup workflow (quick-terminal lifecycle), not Dock visibility

## Alternatives Considered

1. Keep `PopUp` window kind and patch individual move/resize behaviors
- Pros:
  - minimal scope in the short term
- Cons:
  - still fights platform semantics for zoom/move behavior
  - repeated regressions likely when toggling/showing window
- Why not chosen:
  - does not reliably provide normal desktop window behavior

2. Introduce a third dock mode (e.g., `menubar_with_dock`)
- Pros:
  - explicit mode naming for Dock policy differences
- Cons:
  - schema/UI churn for a behavior request that can be satisfied without new config complexity
- Why not chosen:
  - unnecessary for current scope; startup-behavior split already exists

## Safe Change Playbook

When modifying macOS app-shell behavior in this area, follow these steps:
1. Keep quick-terminal show/hide logic separate from window geometry persistence; do not force-move existing windows on every toggle.
2. If users need desktop semantics (move/resize/zoom), prefer `WindowKind::Normal` with titlebar-backed style settings over `PopUp`.
3. Retain status-item handles in long-lived controllers; do not create transient status items in short-lived startup frames.
4. Verify both startup modes (`regular`, `menubar_only`) for Dock visibility, status icon presence, and toggle behavior.

## Do / Avoid

Do:
- preserve async command dispatch guards when adding controller commands
- keep regular and menubar startup paths explicit and testable
- apply placement only for first quick-terminal window creation unless explicit re-pin behavior is desired

Avoid:
- coupling menubar toggle to unconditional `resize + move_window_to` on every show
- assuming popup utility windows will match normal desktop window shortcuts/zoom behavior
- dropping status-item handles after startup

## Typical Mistakes

- Reintroducing `WindowKind::PopUp` for quick-terminal while expecting standard resize/zoom UX.
- Forcing geometry reset on each toggle, which makes user-driven window positioning impossible.
- Installing menubar icon in regular mode without keeping the handle alive, causing icon disappearance.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - in `menubar_only`, window can be resized and moved freely
  - in `menubar_only`, Dock icon is visible and app remains summonable
  - in `regular`, menubar icon is visible and clicking it activates/reopens terminal window
  - in `menubar_only`, global shortcut still toggles terminal visibility
- Signals of regression:
  - quick terminal snaps back to menubar position after every toggle
  - menubar mode loses Dock icon
  - regular mode status icon disappears or does nothing

## Related Artifacts

- Related docs:
  - `docs/evolution/0008-2026-02-24-macos-menubar-quick-terminal-mode.md`
  - `docs/evolution/0016-2026-02-25-settings-drawer-scroll-and-menubar-command-reentrancy.md`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/macos.rs`
