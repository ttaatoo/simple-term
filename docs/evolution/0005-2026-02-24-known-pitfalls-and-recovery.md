# 0005-2026-02-24-known-pitfalls-and-recovery

## Metadata

- Date: 2026-02-24
- Sequence: 0005
- Status: active
- Scope: workflow, governance, release, runtime

## Why This Entry Exists

This entry documents concrete failure patterns observed during project bootstrap, so future contributors and LLM agents can detect and recover quickly.

## System Context

Common failure surfaces:
- branch protection vs direct push behavior
- release runs with stale workflow assumptions
- local git state diverging from remote governance
- CI check-name mismatches
- terminal UI shortcuts colliding with shell-native control keys
- tab-bar flex layout drift causing centered-looking tabs and titlebar misalignment
- asynchronous tab-title updates causing tab chip width jitter

## Decision and Rationale

Decision:
- Maintain an explicit "pitfall + detection + recovery" playbook.

Rationale:
- These issues are procedural and easy to repeat.
- Commit history shows the event happened but does not teach recovery strategy.

## Alternatives Considered

1. Rely on troubleshooting as incidents occur
- Pros: minimal documentation effort
- Cons: repeated mistakes and slower incident response
- Why not chosen: poor operational learning

2. Keep guidance in PR comments only
- Pros: contextual to each event
- Cons: fragmented and hard for LLMs to ingest
- Why not chosen: weak discoverability

## Safe Change Playbook

For each operational issue:
1. Detect symptom from command/output.
2. Map symptom to known pitfall below.
3. Apply corresponding recovery steps.
4. Add new pitfall here if issue is novel.

## Do / Avoid

Do:
- Verify remote policy state before forceful operations.
- Confirm workflow/run IDs before cancellation/retry.
- Re-check release assets against support policy.

Avoid:
- Assuming local branch rules mirror remote settings.
- Retrying release blindly without cancelling outdated runs.
- Treating admin bypass as a standard merge path.

## Typical Mistakes

1. Direct push rejected on protected branch
- Detection: push error indicates PR/check/review requirements
- Recovery:
  - create/push feature branch
  - open PR
  - satisfy checks/reviews or use documented emergency path

2. Release run uses outdated policy (e.g., wrong platform set)
- Detection: active run building unexpected targets
- Recovery:
  - cancel stale run
  - update workflow on default branch
  - re-trigger release

3. Tag/release mismatch or duplicate tag
- Detection: release workflow fails tag checks
- Recovery:
  - inspect remote tag state
  - delete/recreate tag only with explicit operator intent
  - rerun release after version validation

4. Required check stuck due to wrong check name
- Detection: merge blocked with "required check expected" while CI appears green
- Recovery:
  - align branch rule required-check name with actual workflow job name

5. Terminal tab shortcuts hijack shell control sequences (e.g., `Ctrl+W`)
- Detection: shell editing/navigation shortcuts stop working after adding view-level keybindings
- Recovery:
  - keep tab-management shortcuts on platform modifier (`Cmd` on macOS) and reserve control-only combos for explicit exceptions (`Ctrl+Tab`)
  - add/adjust tests around shortcut routing before shipping keybinding changes

6. Pointer-to-grid mapping uses window coordinates instead of terminal-local coordinates
- Detection: text selection, hyperlink open, or mouse reports target the line below (or above) the cursor after adding/changing top UI chrome (tab bar/title area)
- Recovery:
  - ensure pointer mapping subtracts `TerminalBounds.bounds.origin` before computing row/column/side
  - add a regression test with non-zero bounds origin in `crates/simple-term/src/mappings/mouse.rs`
  - re-run targeted pointer tests and manual top-row selection checks

7. Tab bar items appear centered or stretched after style refactors
- Detection: tabs no longer hug the left edge; each tab expands to consume large equal-width blocks; active tab label appears centered in the window rather than within a compact tab chip
- Recovery:
  - keep tab items non-growing (`flex_none`) with bounded width policy (`min/max`)
  - reserve explicit left titlebar/traffic-light drag width before the tab viewport
  - keep right controls compact and fixed-count (`+`/dropdown) to avoid eating tab viewport width
  - verify with at least 1, 2, and 3 tab scenarios after every layout change

8. Tab label vertical alignment shifts when tab-count changes
- Detection: first tab label appears vertically lower/higher after opening a second tab (or when `is_last` toggles), despite unchanged font and bar height
- Recovery:
  - keep decorative separators out of the tab item's vertical flex flow (use border/overlay, not extra stacked children with margins)
  - ensure tab item vertical footprint is invariant across tab positions (`is_last` true/false)
  - add a regression test that enforces tab-item height budget against `TAB_BAR_HEIGHT_PX`

9. Common clipboard shortcuts are missing or inconsistent in terminal canvas
- Detection: selecting text then pressing expected shortcuts (`Cmd+C` / `Ctrl+Shift+C`) does nothing, while terminal still accepts typing
- Recovery:
  - keep app-level shortcut detection explicit and test-backed in `TerminalView` utility helpers
  - route app-level shortcuts before terminal escape mapping in keydown handling
  - preserve control-only shell shortcuts (`Ctrl+C`, `Ctrl+W`, etc.) as PTY input unless there is explicit product intent otherwise

10. Cursor settings exist but UI still shows static full-cell block cursor
- Detection: changing `cursor_shape` / `blinking` in `settings.json` has no visible effect, or cursor never blinks despite terminal events
- Recovery:
  - wire `TerminalSettings::default_cursor_style()` into terminal config at spawn time
  - render cursor by reported `cursor.shape` (`Beam`/`Underline`/`HollowBlock`) instead of always filling full cell
  - add/keep a UI-level blink timer that toggles visibility and triggers redraw; `alacritty_terminal` provides blink state but not frontend repaint scheduling

11. Settings drawer changes break terminal-focused close/input behavior
- Detection: pressing plain `Esc` no longer closes the open settings drawer, or drawer key handling interferes with normal terminal input when drawer is closed
- Recovery:
  - keep drawer close logic as a plain-keystroke predicate (`Esc` without platform/control/alt) in terminal key handling path
  - add/keep unit tests for drawer toggle and plain-escape close behavior
  - verify key-routing order: settings close check -> find/common shortcuts -> terminal key mapping

12. GPUI `overflow_*_scroll` is configured without a positive scrollbar width
- Detection: overflow content is clipped and cannot be scrolled; expected scrollbar never appears for long settings/content panels
- Recovery:
  - set explicit non-zero `scrollbar_width(...)` on scroll containers
  - when a scrollable panel overlays terminal content, ensure the panel occludes background mouse hitboxes so wheel input does not leak to terminal listeners
  - keep regression test coverage for the configured width constant
  - manually verify scroll-to-bottom behavior after layout changes

13. Menubar command path re-enters app/controller borrows during async + observer timing overlap
- Detection: runtime logs include `RefCell already borrowed` during rapid toggle/hide, close/reopen, or window deactivation flows
- Recovery:
  - replace direct mutable borrow with `try_borrow_mut()` guard
  - requeue deferred commands when controller is temporarily busy instead of panicking
  - in `observe_window_activation` paths, ignore the initial registration callback until the window has entered an active state at least once
  - when hide/toggle is triggered from window activation/deactivation observers, defer the callback (`cx.defer`) instead of dispatching synchronously inside the observer stack
  - if close/reopen still logs borrow errors, inspect observer callbacks first for immediate command sends during in-progress app updates
  - add guard-path unit tests to preserve non-panicking behavior

14. Settings popup is rendered inside terminal flex flow instead of overlay layer
- Detection: opening settings shrinks/pushes terminal surface horizontally (drawer-like behavior) instead of floating above content
- Recovery:
  - keep terminal content row dedicated to terminal surface only
  - render settings through an absolute overlay attached to a `relative()` root container
  - do not chain `.absolute()` and `.relative()` on the same overlay element; the later call overrides GPUI `position` and can push overlay back into normal layout flow
  - keep overlay `occlude()` enabled so background terminal hitboxes do not consume interactions
  - verify popup close paths (`Esc`, close button) and scroll-to-bottom behavior after layout edits

15. Menubar quick-terminal is treated as a pinned utility popup instead of a normal desktop window
- Detection: in `menubar_only` mode, window cannot be freely resized/moved/maximized, or each toggle snaps it back under the menubar
- Recovery:
  - use normal-window semantics for quick-terminal (`WindowKind::Normal`, movable/resizable/minimizable, titlebar-backed style)
  - apply panel placement only when creating a new window, not on every toggle of an existing one
  - keep status-item handles in long-lived controllers for both startup modes so menubar icon parity remains stable
  - verify both `regular` and `menubar_only` startup paths after app-shell changes

16. Tab width jumps when shell title updates after creating a new tab
- Detection: a newly created tab starts narrow, then expands horizontally as soon as a longer title arrives (e.g., prompt path / command context), causing visible tab strip jitter
- Recovery:
  - keep tab items at explicit fixed width for this UI (`TAB_ITEM_WIDTH_PX`) instead of content-responsive `min/max` range behavior
  - keep visible tab label truncated to preserve strip stability under long dynamic titles
  - expose full title via hover tooltip on a stateful label element (`.id(...)` + `.tooltip(...)`)
  - verify create-tab + title-update flow and confirm right-side controls do not shift

17. Last-tab close path bypasses quick-terminal controller visibility state
- Detection: `Cmd+W` on the last tab either does nothing or hides the app without updating `QuickTerminalController.visible`, causing follow-up toggle behavior to feel inconsistent
- Recovery:
  - route last-tab close through the same hide callback/command path used by controller-managed hide events (`AppCommand::HideTerminal`)
  - only fall back to direct `cx.hide()` when no controller callback is present
  - keep a unit guard around tab-count boundary logic (`0/1 => hide`, `>=2 => close tab`)
  - manually verify in `menubar_only` mode that hide + re-toggle remains stable

18. Selection highlight reverts to fg/bg inversion and produces harsh white blocks
- Detection: selecting text on dark themes paints a near-white rectangle with dark text, and token foreground colors are replaced by background-ish colors
- Recovery:
  - keep selection preprocessing as background tinting, not `fg/bg` swapping
  - derive selection tint from theme palette accent (`cursor`) and blend over the resolved cell background
  - keep blend behavior guarded by unit tests for RGB interpolation and selected-background derivation
  - manually verify selection readability across at least two themes before shipping

19. macOS app-shell logic splits back into mode-specific startup controllers
- Detection: runtime behavior diverges by startup mode again (for example, one path toggles hide/show while another only activates; one path receives fixes while the other regresses)
- Detection: Dock-reopened windows can be visible while controller `visible` remains false, and hide commands (for example last-tab `Cmd+W`) are ignored
- Recovery:
  - keep a single macOS app-shell controller as the only owner of visibility state and command handling
  - avoid reintroducing `dock_mode` as a runtime branch selector; model UX differences through explicit feature flags instead
  - ensure `TerminalView` hide requests route through controller callback/command flow so controller `visible` state stays authoritative
  - treat hide requests as idempotent; do not gate `hide_terminal` on stale `visible` preconditions
  - verify startup-open + menubar/hotkey toggle + outside-click behavior after every app-shell edit
  - verify Dock reopen + last-tab `Cmd+W` hide behavior after visibility-state changes

20. Fixed-width find/settings surfaces regress on narrow windows
- Detection: find strip overlaps tab controls or becomes unusably cramped; settings popup exceeds viewport padding or cannot be dismissed by clicking backdrop
- Recovery:
  - keep find/settings width policy in dedicated viewport-aware helpers instead of inline fixed widths
  - clamp widths with viewport margin constraints before applying preferred width constants
  - preserve overlay backdrop click-to-close alongside existing close paths (`Esc`, close button)
  - keep regression tests for responsive width helper behavior and run manual resize checks before shipping

21. Tab close button click bubbles into parent tab activation
- Detection: clicking tab close also triggers tab selection/activation or briefly changes active state before closing
- Recovery:
  - keep tab close button as its own interactive child and call `cx.stop_propagation()` in close handler
  - keep hover state scoped by tab id and clear stale hovered id when tab closes
  - add/keep unit guard for hover-state transition helper and run manual hover+close checks

22. Tab accent color drifts across themes after brand-color requests
- Detection: active-tab indicator and close-button hover color change with theme cursor instead of staying on requested purple brand color
- Recovery:
  - keep tab accent color centralized in a dedicated helper token (`tab_brand_purple(...)`)
  - apply token consistently to indicator and all hover-capable interactive controls in this surface
  - keep active-tab state represented by indicator; avoid reintroducing active chip background highlight unless explicitly requested
  - keep tests that assert token values and run manual cross-theme checks

23. Multi-click text selection silently regresses to drag-only behavior
- Detection: double-click selects a single cell (or nothing semantic), and triple-click does not select whole lines
- Recovery:
  - keep click-count mapping centralized in `terminal_view::utils` (`1 -> Simple`, `2 -> Semantic`, `>=3 -> Lines`)
  - ensure left-click non-mouse-mode selection creation uses that mapping instead of hardcoded `SelectionType::Simple`
  - add/keep a focused unit test for click-count mapping and manually verify single/double/triple click behavior
  - preserve PTY mouse-mode passthrough behavior while adding normal-mode selection improvements

24. macOS dual-hotkey or pin-state wiring drifts out of sync
- Detection: show/hide shortcut works but pin shortcut does nothing (or vice versa), pinned windows can still auto-hide, or pin state is lost after reopen/toggle
- Recovery:
  - keep hotkey parsing/fallback and registration centralized in `AppShellController::install_global_hotkeys`
  - treat `Cmd+F5` as reserved by macOS VoiceOver and remap legacy defaults to the current toggle default (`Cmd+F4`)
  - keep `pin_hotkey` distinct from `global_hotkey`; if IDs collide, warn and disable pin registration explicitly
  - apply pin state in both existing-window update path and new-window creation path (`macos::set_window_pinned`)
  - keep hide gating pin-aware (`should_process_hide_terminal_request(..., pinned)`) so “always show” mode is enforced consistently
  - manually verify `Cmd+F4` (or configured toggle) and `Cmd+\`` pin/unpin flows together after app-shell edits

25. Settings-panel hotkey edits persist but do not apply until restart
- Detection: settings UI shows updated `global_hotkey`, `settings.json` persists, but old shortcut still drives toggle behavior in the running app
- Recovery:
  - route settings-driven hotkey updates through `AppCommand` so controller remains the single owner of registration state
  - drop previous `GlobalHotKeyManager` registrations before re-registering updated shortcuts
  - when implementing recorder UI, capture shortcut keydown before tab/find/common shortcut handlers so recorded combos do not trigger unrelated actions
  - keep registration failure behavior non-fatal (warn + continue)

26. Cross-monitor reopen re-enters borrowed app/controller state when activation ordering is wrong
- Detection: moving pointer to another monitor then pressing `Cmd+F4` logs `RefCell already borrowed`, often with old-monitor flash before final placement
- Recovery:
  - keep native frame mutation deferred (`dispatch_async_f`) in `macos.rs`; avoid synchronous `setFrame` in controller update stack
  - when reopening a hidden window that requires frame move, run `unhide/orderFront/activateIgnoringOtherApps` inside the same deferred native callback after frame apply
  - return activation ownership from native move API and skip `cx.activate(true)` when native callback is responsible
  - preserve ordering invariant: `setFrame -> reveal/orderFront -> activate`
  - verify both symptoms are gone: no borrow logs and no old-monitor flash during repeated cross-monitor toggles

27. Toggle-reopened terminal window appears but does not accept typing until click
- Detection: after `Cmd+F4` show, terminal is visible but first keypresses are ignored until the user clicks terminal content
- Recovery:
  - keep terminal view focus restoration explicit in show paths (`focus_handle.focus(window)` via `TerminalView::focus_terminal`)
  - apply focus restore for both existing-window reuse and new-window creation paths
  - do not rely on app/window activation alone as a substitute for GPUI element focus
  - avoid using `window.activate_window()` in borrowed show-update closures as a focus workaround

## Verification Strategy

After any recovery action:
- re-run relevant CI or release workflow
- verify branch/release state via GitHub API or UI
- confirm policy and docs are still consistent

## Related Artifacts

- Related docs: `docs/troubleshooting.md`, `docs/evolution/0004-2026-02-24-release-and-governance-model.md`
- Optional references: GitHub Actions run history and branch protection settings
