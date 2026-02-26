# 0016-2026-02-25-settings-drawer-scroll-and-menubar-command-reentrancy

## Metadata

- Date: 2026-02-25
- Sequence: 0016
- Status: active
- Scope: runtime, testing

## Why This Entry Exists

Two runtime bugs surfaced together in day-to-day usage:
- settings drawer overflow content could not be scrolled to the bottom
- menubar quick-terminal path occasionally logged `RefCell already borrowed`

The file-level diff does not explain why these failures happen in GPUI/menubar event flow, or how to harden future changes without reintroducing them. This entry captures those invariants.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
  - `apps/simple-term/src/main.rs`
- Upstream constraints (platform, library, policy):
  - GPUI `Overflow::Scroll` needs non-zero `scrollbar_width`; zero width behaves like `Hidden` in layout semantics.
  - macOS menubar quick-terminal command routing is asynchronous (`smol::channel`) and can re-enter controller command handling.
- Invariants already in force:
  - settings drawer must remain usable for all controls even when content exceeds viewport height
  - menubar command handling must not panic on transient re-entrancy

## Decision and Rationale

- Decision:
  - Mark the settings drawer as mouse-occluding so background terminal hitboxes cannot steal wheel events.
  - Track drawer scroll state via `ScrollHandle` and render an explicit right-edge thumb for visible scroll affordance.
  - Keep an explicit non-zero scrollbar width on `settings-drawer-scroll` for GPUI scroll-container semantics.
  - Replace direct `borrow_mut()` in the menubar command loop with a guarded `try_borrow_mut()` path and requeue-on-busy fallback.
  - Add regression tests for both guardrails.
- Why this path was selected:
  - minimal behavior change with clear deterministic guarantees
  - avoids broad refactors to terminal view structure or command architecture
  - directly removes runtime panic surface while preserving command semantics
- Trade-offs accepted:
  - requeue introduces at-most-one extra dispatch hop when controller is temporarily busy
  - drawer scrollbar consumes a small fixed width in the panel layout

## Alternatives Considered

1. Keep `overflow_y_scroll()` only and rely on default style
- Pros:
  - no UI style changes
- Cons:
  - GPUI default scrollbar width is zero; overflow remains effectively non-scrollable
- Why not chosen:
  - does not resolve the bug under current GPUI semantics

2. Replace `Rc<RefCell<QuickTerminalController>>` with a larger architectural rewrite
- Pros:
  - could remove interior mutability entirely
- Cons:
  - high churn in menubar control flow with unnecessary risk for targeted bugfix
- Why not chosen:
  - disproportionate scope for the observed failure mode

## Safe Change Playbook

When modifying this area, follow these steps:
1. For any GPUI element using `overflow_*_scroll()`, set an explicit `scrollbar_width(...)` that is greater than zero unless you intentionally want hidden-like behavior.
2. In menubar command loops, never assume single-entry mutable access; guard interior mutability with `try_borrow_mut()` or equivalent.
3. If mutable access is temporarily unavailable, prefer deferring/requeueing commands over panicking.
4. Keep regression tests near these helpers to prevent accidental rollback to panic-prone patterns.

## Do / Avoid

Do:
- keep settings drawer scroll container style explicit (`overflow_y_scroll` + non-zero `scrollbar_width`)
- keep the drawer as an occluding overlay when open so wheel input does not leak to terminal surface listeners
- keep visible scrollbar thumb metrics derived from `ScrollHandle` (`bounds`, `max_offset`, `offset`)
- treat asynchronous menubar commands as potentially re-entrant
- log deferred command dispatches so contention is diagnosable

Avoid:
- relying on implicit GPUI defaults for scroll container behavior
- direct `borrow_mut()` in high-frequency async command handlers
- dropping commands silently when controller state is temporarily unavailable

## Typical Mistakes

- Adding `overflow_y_scroll()` without setting a positive scrollbar width.
- Assuming command callbacks cannot overlap in menubar mode.
- Fixing `RefCell` borrow failures by swallowing errors without retries.

## Verification Strategy

- Required automated checks:
  - `cargo fmt --all`
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - open settings drawer and scroll to bottom items with mouse wheel/trackpad
  - in menubar mode, rapidly toggle terminal via hotkey and status item, then check logs for missing `RefCell already borrowed`
- Signals of regression:
  - drawer stops before bottom content
  - runtime logs show repeated `RefCell already borrowed`
  - toggle/hide behavior becomes flaky under rapid command input

## Related Artifacts

- Related docs:
  - `docs/evolution/0008-2026-02-24-macos-menubar-quick-terminal-mode.md`
  - `docs/evolution/0015-2026-02-24-settings-drawer-v1-and-live-persistence.md`
  - `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/terminal_view.rs`
