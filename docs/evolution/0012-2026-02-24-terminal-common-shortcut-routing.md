# 0012-2026-02-24-terminal-common-shortcut-routing

## Metadata

- Date: 2026-02-24
- Sequence: 0012
- Status: active
- Scope: runtime, testing

## Why This Entry Exists

`TerminalView` renders terminal output in a custom canvas rather than a native text control, so expected desktop shortcuts are not provided by the OS automatically. Without explicit routing, common actions like copy/paste/select-all are either missing or inconsistent.

This is not obvious from commit-level diffs because keyboard behavior is split between app-level shortcut handling and terminal escape-sequence mapping.

## System Context

Relevant modules:
- `apps/simple-term/src/terminal_view.rs`
- `apps/simple-term/src/terminal_view/utils.rs`
- `crates/simple-term/src/mappings/keys.rs`

Upstream constraints:
- The terminal must preserve shell/control-sequence semantics (`Ctrl+C`, `Ctrl+W`, etc.).
- App-level shortcuts must be handled before terminal key passthrough.
- Selection lives in terminal state (`term.selection`) and must be converted to text explicitly.

Invariants already in force:
- Control-only shortcuts must continue to flow to PTY unless explicitly intended otherwise.
- Platform shortcuts (`Cmd` on macOS) should remain reserved for app UX actions.
- Cross-platform parity may require explicit `Ctrl+Shift+<key>` variants for app-level clipboard actions.

## Decision and Rationale

Decision:
- Add a pure shortcut classifier (`common_shortcut_action`) in `terminal_view::utils`.
- Route four common app actions in `TerminalView` before terminal key mapping:
  - copy selection (`Cmd+C` and `Ctrl+Shift+C`)
  - paste (`Cmd+V` and `Ctrl+Shift+V`)
  - select all terminal content (`Cmd+A` and `Ctrl+Shift+A`)
  - find mode (`Cmd+F` and `Ctrl+Shift+F`)
- Implement select-all against terminal grid bounds (`topmost_line` to `bottommost_line`).
- Keep selection-clearing behavior aligned with `keep_selection_on_copy`.
- Render an inline top-bar find panel (input-like display, match counter, next/previous, close).
- Support keyboard find navigation (`Enter` next, `Shift+Enter` previous, `Esc` close).

Why this path was selected:
- Keeps shortcut parsing deterministic and testable as pure logic.
- Preserves existing shell control-key behavior by avoiding control-only interception.
- Improves expected terminal UX without changing PTY backend behavior.

Trade-offs accepted:
- Shortcut surface is still intentionally minimal beyond clipboard/select/find.
- Search UI is intentionally lightweight (styled inline panel with keyboard-driven query update).

## Alternatives Considered

1. Keep only `copy_on_select` and platform paste
- Pros: no new key-routing logic
- Cons: misses baseline terminal shortcuts; inconsistent with user expectations
- Why not chosen: poor usability and discoverability

2. Intercept control-only clipboard combos
- Pros: fewer modifier variants to support
- Cons: collides with shell-native controls (`Ctrl+C`, `Ctrl+V`, readline bindings)
- Why not chosen: violates terminal input invariants and known pitfall guidance

## Safe Change Playbook

When modifying keyboard shortcuts in terminal view:
1. Keep app-level shortcut detection in a pure helper and add/adjust unit tests first.
2. Handle app-level shortcuts before terminal escape mapping in `on_key_down`.
3. Do not intercept control-only combos unless there is explicit, justified terminal UX intent.
4. For selection actions, update `term.selection` while holding the lock, then drop lock before mutating unrelated view state.
5. Re-run shortcut-focused tests plus full app test suite.

## Do / Avoid

Do:
- Keep shortcut matching explicit and constrained by modifier policy.
- Test both platform and `Ctrl+Shift` variants.
- Use terminal dimension helpers (`topmost_line`, `bottommost_line`, `last_column`) for full-buffer selections.

Avoid:
- Hiding shortcut behavior inside unrelated key mapping branches.
- Introducing control-only app shortcuts that hijack shell behavior.
- Mutating view fields while terminal lock guards are still alive.

## Typical Mistakes

- Adding shortcut logic directly into escape-mapping paths, making ownership and precedence unclear.
- Consuming `Ctrl+C`/`Ctrl+V` as app actions and breaking terminal control semantics.
- Selecting only visible rows for "Select All" instead of full terminal buffer range.

## Verification Strategy

Required automated checks:
- `cargo test -p simple-term-app common_shortcut_action -- --nocapture`
- `cargo test -p simple-term-app`
- `cargo check --workspace`

Recommended manual checks:
- select text and press `Cmd+C` / `Ctrl+Shift+C`, then paste into another app
- press `Cmd+V` / `Ctrl+Shift+V` to paste into terminal prompt
- press `Cmd+A` / `Ctrl+Shift+A` and confirm full-buffer highlight
- press `Cmd+F` / `Ctrl+Shift+F`, type a query, then press `Enter` to jump to next result
- verify `Ctrl+C` still sends interrupt to foreground process

Signals of regression:
- clipboard shortcuts do nothing despite active selection
- shell control keys stop working after shortcut changes
- select-all highlights only partial output unexpectedly

## Deferred Shortcut Backlog

Planned later (explicitly deferred to keep this change focused):
- clear screen / clear scrollback shortcut
- reopen closed tab (`Cmd+Shift+T`)
- dedicated next/previous find-result shortcut bindings (e.g., `Cmd+G` / `Cmd+Shift+G`)
- richer find-result status/UI behavior beyond the current lightweight inline panel

## Related Artifacts

- Related docs: `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`, `docs/evolution/0009-2026-02-24-terminal-tabs-and-tabbar-ui.md`
- Optional references: `apps/simple-term/src/terminal_view.rs`, `apps/simple-term/src/terminal_view/utils.rs`
