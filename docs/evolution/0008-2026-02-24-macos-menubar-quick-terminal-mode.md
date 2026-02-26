# 0008-2026-02-24-macos-menubar-quick-terminal-mode

## Metadata

- Date: 2026-02-24
- Sequence: 0008
- Status: active
- Scope: runtime, architecture

## Why This Entry Exists

`simple-term` originally booted as a normal desktop window only. This change adds a macOS quick-terminal mode controlled by menubar and global shortcut behavior. The key risks are app-shell regressions (focus/hide loops, wrong-screen placement, Dock policy surprises), which are not obvious from code diffs alone.

## System Context

Relevant modules:
- `apps/simple-term/src/main.rs`
- `apps/simple-term/src/macos.rs` (new)
- `apps/simple-term/src/terminal_view.rs`
- `crates/simple-term/src/terminal_settings.rs`

Platform/runtime constraints:
- Global shortcut manager must be created on a thread with active app event loop (main app lifecycle).
- Menubar integration and window positioning rely on AppKit (`NSStatusItem`, `NSScreen`, `NSWindow`).
- Existing terminal core (`crates/simple-term`) should remain unchanged.

## Decision and Rationale

Decision:
- Keep terminal backend/rendering unchanged.
- Add a macOS app-shell controller for quick-terminal behavior.
- Add config fields for dock policy, shortcut, and panel placement offset.
- Keep startup default in `regular` mode; enable menubar shell only when `dock_mode = "menubar_only"`.

Rationale:
- Preserves proven terminal invariants and test coverage.
- Isolates high-risk AppKit behavior in `macos.rs`.
- Supports requested UX: menubar trigger + global shortcut + outside-click hide.

Trade-offs:
- Adds Objective-C bridge code in app layer.
- Introduces platform-specific behavior that is intentionally macOS-only.
- Uses popup-style app-shell flow (activate/hide) instead of persistent standard desktop window semantics.

## Alternatives Considered

1. Keep normal window app and add only in-app shortcut
- Pros: minimal code
- Cons: cannot provide true global summon UX
- Why not chosen: does not satisfy quick-terminal requirements

2. Rewrite using a different framework with built-in menubar abstractions
- Pros: possibly simpler platform APIs
- Cons: high migration cost and risk to terminal behavior
- Why not chosen: too disruptive relative to scope

## Safe Change Playbook

When changing menubar quick-terminal behavior:
1. Keep `TerminalView` rendering/input logic independent from menubar control flow.
2. Treat app-shell commands (`toggle`, `hide`) as the only state transition entry points.
3. Validate popup placement math against multi-display setups and menu bar reserved space.
4. When touching global shortcut wiring, keep fallback behavior if shortcut parsing/registration fails.
5. Keep non-macOS startup path unchanged (`open_standard_window`).

## Do / Avoid

Do:
- Keep AppKit-specific logic in `apps/simple-term/src/macos.rs`.
- Keep `regular` as the default mode during non-menubar feature development.
- Preserve existing terminal test suite and runtime invariants.

Avoid:
- Coupling PTY lifecycle directly to menubar event callbacks.
- Adding platform conditionals deep in core terminal crate.
- Assuming every machine has only one display.

## Typical Mistakes

- Creating focus/hide loops by firing hide on every activation transition.
- Forgetting to keep status item / hotkey manager alive after startup.
- Regressing non-macOS startup path while iterating on macOS-only code.
- Accidentally enabling menubar flow for all macOS users when the mode should be opt-in.

## Verification Strategy

Required checks:
- `cargo check --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`

Manual checks:
- menubar icon toggles terminal show/hide
- global shortcut toggles terminal show/hide
- clicking outside terminal hides window
- multi-display summon follows mouse screen
- dock mode switch via settings (`menubar_only` vs `regular`)

Regression signals:
- window no longer appears after first hide
- shortcut events fire but window stays hidden
- terminal appears on wrong monitor repeatedly

## Related Artifacts

- Related docs: `docs/architecture-invariants.md`, `docs/plans/2026-02-24-menubar-quick-terminal-design.md`
- Optional references: `apps/simple-term/src/main.rs`, `apps/simple-term/src/macos.rs`
