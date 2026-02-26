# Local Pin Hotkey and Tab-Bar Indicator Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make `pin_hotkey` window-local (focused terminal only), keep pin toggle independent from show/hide, and add a clickable tab-bar pin status indicator.

**Architecture:** Keep `AppShellController` as the single source of truth for `pinned`. OS-global hotkey registration remains only for show/hide. `TerminalView` handles local pin-key matching and UI interactions, then requests controller toggles through callbacks.

**Tech Stack:** Rust workspace (`apps/simple-term`, `crates/simple-term`), GPUI, `global-hotkey`, macOS AppKit bridge (`cocoa`/`objc`).

---

### Task 1: Restrict OS-global hotkeys and keep controller-owned pin state sync

**Files:**
- Modify: `apps/simple-term/src/main.rs`
- Test: `apps/simple-term/src/main.rs` (existing `#[cfg(all(test, target_os = "macos"))]` module)

**Step 1: Add failing tests for desired controller behavior**

Add tests that cover:
- global registration path no longer uses `pin_hotkey` as an OS-global binding.
- pin toggle path does not call show/hide logic and only mutates pinned/window-level behavior.
- controller pushes current pinned state to view after toggles (through window update callback path).

**Step 2: Run targeted tests and confirm failure**

Run: `cargo test -p simple-term-app parse_command_five_function_key_remaps_to_non_reserved_combo -- --nocapture`  
Expected: existing test passes; newly added behavior tests fail until implementation is complete.

**Step 3: Implement controller changes**

Implement:
- `install_global_hotkeys()` registers only `global_hotkey`.
- remove `pin_hotkey` ID registration/event handling from global listener.
- keep `AppCommand::TogglePinned` handling unchanged as controller-owned pin entrypoint.
- after `toggle_terminal_pin()`, update active view pinned state via `window_handle.update(...)`.
- keep hide gate (`should_process_hide_terminal_request`) pinned logic unchanged.

**Step 4: Re-run targeted tests**

Run: `cargo test -p simple-term-app hide_terminal_request_is_processed_when_visible_flag_is_false -- --nocapture`  
Expected: PASS, plus new controller tests PASS.

**Step 5: Commit**

```bash
git add apps/simple-term/src/main.rs
git commit -m "refactor: keep pin hotkey local to focused terminal window"
```

**Rollback/Mitigation:** If callback wiring causes runtime borrow issues, temporarily keep controller-side state updates only and guard view updates behind `if let Some(window_handle)` while preserving global-hotkey restriction.

---

### Task 2: Add window-local pin hotkey handling in `TerminalView`

**Files:**
- Modify: `apps/simple-term/src/terminal_view.rs`
- Test: `apps/simple-term/src/terminal_view.rs` (unit tests near hotkey parsing tests)

**Step 1: Add failing tests for pin-hotkey parsing/matching**

Add tests for helpers like:
- accepts `command+Backquote` pattern.
- rejects no-modifier keystrokes.
- rejects modifier-only keystrokes.
- matches configured `settings.pin_hotkey` only when keystroke is valid.

**Step 2: Run focused tests to confirm failure**

Run: `cargo test -p simple-term-app global_hotkey_from_keystroke_accepts_backquote_toggle_style -- --nocapture`  
Expected: existing tests pass; new pin-local matching tests fail before implementation.

**Step 3: Implement local pin keybinding path**

Implement:
- add `on_toggle_pin_requested` callback field in `TerminalView`.
- add helper to parse/validate configured `pin_hotkey` and compare with current `KeyDownEvent`.
- in `on_key_down`, check pin-hotkey match before terminal escape/input routing; on match, request pin toggle and consume event.
- ensure this path never calls show/hide.

**Step 4: Re-run focused tests**

Run: `cargo test -p simple-term-app global_hotkey_from_keystroke -- --nocapture`  
Expected: all global + pin-local parsing tests PASS.

**Step 5: Commit**

```bash
git add apps/simple-term/src/terminal_view.rs apps/simple-term/src/main.rs
git commit -m "feat: handle pin hotkey as focused-window keybinding"
```

**Rollback/Mitigation:** If matching logic is unstable across key token variants, fall back to normalized string comparison using the same tokenization path already used for global hotkey recording.

---

### Task 3: Add tab-bar pin indicator and settings recorder for `pin_hotkey`

**Files:**
- Modify: `apps/simple-term/src/terminal_view.rs`
- Modify: `crates/simple-term/src/terminal_settings.rs` (only if sanitization/helper adjustments are needed)
- Test: `apps/simple-term/src/terminal_view.rs`
- Test: `crates/simple-term/src/terminal_settings.rs` (if new sanitizer behavior added)

**Step 1: Add failing tests for recorder and conflict guard**

Add tests for:
- recording-mode toggles for `pin_hotkey`.
- applying recorded `pin_hotkey` persists and notifies runtime.
- reject `pin_hotkey == global_hotkey` updates.

**Step 2: Run focused tests and confirm failure**

Run: `cargo test -p simple-term-app toggled_settings_panel_open_flips_boolean_state -- --nocapture`  
Expected: existing tests pass; new recorder/conflict tests fail pre-implementation.

**Step 3: Implement UI + state changes**

Implement:
- tab-bar right-side clickable indicator: `📌` when pinned, `○` when unpinned.
- indicator click triggers `on_toggle_pin_requested`.
- settings section adds `Pin/Unpin Shortcut` row with `record/cancel`, mirrored from show/hide recorder flow.
- add pin recorder state fields and keydown capture path.
- on save, call `on_hotkeys_updated(global_hotkey, pin_hotkey)`.
- conflict guard rejects identical show/hide and pin shortcuts.

**Step 4: Re-run focused tests**

Run: `cargo test -p simple-term-app global_hotkey_from_keystroke_rejects_shortcuts_without_modifier -- --nocapture`  
Expected: PASS for recorder/conflict + existing hotkey tests.

**Step 5: Commit**

```bash
git add apps/simple-term/src/terminal_view.rs crates/simple-term/src/terminal_settings.rs
git commit -m "feat: add pin status indicator and pin-hotkey recorder"
```

**Rollback/Mitigation:** If adding recorder UI introduces event-order regressions, keep indicator feature and temporarily gate recorder controls while preserving `settings.json` compatibility.

---

### Task 4: Update evolution docs for safe-change knowledge

**Files:**
- Create: `docs/evolution/0038-2026-02-26-local-pin-hotkey-and-tabbar-indicator.md`
- Modify: `docs/evolution/INDEX.md`
- Modify: `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`

**Step 1: Draft evolution entry**

Document:
- why pin moved from global to local.
- invariants: controller owns pin state; local keybinding only.
- safe playbook for future shortcut/pin edits.

**Step 2: Update index + pitfall entry**

Add entry pointer in `INDEX.md` and pitfall guidance on accidental global pin registration.

**Step 3: Verify docs consistency**

Run: `rg -n "0038-2026-02-26-local-pin-hotkey-and-tabbar-indicator|pin hotkey|global" docs/evolution`  
Expected: new entry indexed and referenced.

**Step 4: Commit**

```bash
git add docs/evolution/0038-2026-02-26-local-pin-hotkey-and-tabbar-indicator.md docs/evolution/INDEX.md docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md
git commit -m "docs(evolution): record local pin hotkey and indicator invariants"
```

**Rollback/Mitigation:** If sequence number conflicts with concurrent docs work, bump to next free sequence and update index links in the same commit.

---

### Task 5: Full verification and integration checkpoint

**Files:**
- Modify: any touched files from Tasks 1-4 (if fixes required)

**Step 1: Format and static checks**

Run: `cargo fmt --all`  
Expected: no formatting drift after re-run.

**Step 2: Workspace validation**

Run: `cargo check --workspace`  
Expected: PASS.

**Step 3: Test suite**

Run: `cargo test --workspace`  
Expected: PASS.

**Step 4: Final commit (if verification fixed anything)**

```bash
git add -A
git commit -m "chore: finalize local pin hotkey and indicator verification fixes"
```

**Step 5: Manual macOS smoke test**

Manual checklist:
- `Cmd+F4` still global show/hide.
- `Cmd+\`` only works when terminal window focused.
- `Cmd+\`` and indicator click toggle only pin state.
- pinned blocks outside-click hide and last-tab hide.
- pin recorder persists and reloads.

**Rollback/Mitigation:** If a regression appears only in manual macOS flow, revert the last feature commit and reintroduce changes incrementally behind focused tests.
