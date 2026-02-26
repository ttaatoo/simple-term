# Evolution Index (Knowledge Map)

This index is organized by **engineering knowledge topics**, not by commit chronology.

## 0001 Repository Layout and Dependency Boundaries

File: `0001-2026-02-24-repository-layout-and-boundaries.md`

Covers:
- why the workspace is split into `apps/simple-term` and `crates/simple-term`
- dependency direction rules
- where new code should live
- common layering mistakes

## 0002 Runtime Model and Core Invariants

File: `0002-2026-02-24-runtime-model-and-invariants.md`

Covers:
- event/data flow from UI to PTY and back
- lock/snapshot rendering constraints
- input/scroll behavior guarantees
- backpressure and event reliability rules

## 0003 Feature Development Workflow and Test Strategy

File: `0003-2026-02-24-feature-workflow-and-test-strategy.md`

Covers:
- recommended flow for adding features safely
- test placement and verification expectations
- high-risk change areas and required safeguards

## 0004 Release and Governance Operating Model

File: `0004-2026-02-24-release-and-governance-model.md`

Covers:
- SemVer and release execution model
- macOS-only artifact policy
- branch protection intent and admin bypass policy
- operational failure modes during release

## 0005 Known Pitfalls and Recovery Playbooks

File: `0005-2026-02-24-known-pitfalls-and-recovery.md`

Covers:
- repeated mistakes observed during bootstrap
- how to detect and recover quickly
- preventative guardrails for future contributors/LLMs

## 0006 Legacy Path Cleanup

File: `0006-2026-02-24-legacy-path-cleanup.md`

Covers:
- removal of stale `simple-term` directory trees
- repository hygiene rules for active workspace-only sources
- how to prevent path ambiguity during future development

## 0007 macOS DMG Release Packaging

File: `0007-2026-02-24-macos-dmg-release-packaging.md`

Covers:
- adding `.dmg` packaging in release workflow
- app bundle assembly strategy for unsigned distribution
- verification expectations for multi-asset macOS releases

## 0008 macOS Menubar Quick Terminal Mode

File: `0008-2026-02-24-macos-menubar-quick-terminal-mode.md`

Covers:
- menubar + global shortcut quick-terminal app-shell architecture
- popup placement and hide/activate behavior contracts
- safe boundaries between platform glue and terminal core

## 0009 Terminal Tabs and Tab Bar UI

File: `0009-2026-02-24-terminal-tabs-and-tabbar-ui.md`

Covers:
- multi-tab session orchestration inside `TerminalView`
- Warp-like integrated tab bar with atom_one_dark-aligned color tokens, left-aligned tab items, and active-tab input routing
- resize/bounds invariants after introducing top tab bar layout

## 0010 Terminal Pointer Coordinate Space

File: `0010-2026-02-24-terminal-pointer-coordinate-space.md`

Covers:
- pointer-to-grid normalization when terminal content origin is offset by UI chrome
- centralized coordinate conversion invariants shared by selection, hyperlink hit-testing, and mouse protocol reporting
- regression detection and recovery for selection offset bugs near top-of-terminal rows

## 0011 Tab Bar Vertical Alignment Invariants

File: `0011-2026-02-24-tab-bar-vertical-alignment-invariants.md`

Covers:
- preventing state-dependent tab label drift when tab count changes
- keeping decorative separators out of tab-item vertical flow
- enforcing tab-item height budget and control centering checks

## 0012 Terminal Common Shortcut Routing

File: `0012-2026-02-24-terminal-common-shortcut-routing.md`

Covers:
- explicit app-level shortcut handling for canvas-based terminal output
- preserving shell control-key passthrough while adding copy/paste/select-all
- safe ordering and testing patterns for terminal shortcut routing

## 0013 Tab Bar Settings Panel and Runtime Appearance Controls

File: `0013-2026-02-24-tabbar-settings-panel-and-runtime-appearance-controls.md`

Covers:
- top-right tab-bar settings surface for runtime appearance updates
- live font/font-size reflow invariants and safe grid resync behavior
- dock-mode toggle persistence and cross-platform safety boundaries
- cursor shape/blink contracts across terminal core config and UI redraw loop

Note: panel placement from this entry is superseded by `0015` (right-side drawer).

## 0014 Terminal Theme Presets and Persistence

File: `0014-2026-02-24-terminal-theme-presets-and-persistence.md`

Covers:
- persisted `TerminalTheme` schema and stable preset names in `settings.json`
- runtime theme cycling in tab-bar settings controls
- unified palette mapping for chrome, cursor, and terminal fallback ANSI colors

## 0015 Settings Drawer V1 and Live Persistence

File: `0015-2026-02-24-settings-drawer-v1-and-live-persistence.md`

Covers:
- replacing inline tab-bar settings with a dedicated right-side drawer
- immediate apply + immediate persistence rules for V1 settings controls
- focus/keyboard-close invariants (`Esc`) and platform-specific `dock_mode` UI boundaries

Note: drawer layout is superseded by `0017` popup-overlay composition.

## 0016 Settings Drawer Scroll and Menubar Command Re-entrancy

File: `0016-2026-02-25-settings-drawer-scroll-and-menubar-command-reentrancy.md`

Covers:
- GPUI overflow-scroll requirement for non-zero `scrollbar_width`
- safe menubar command dispatch under `RefCell` contention
- regression guardrails for drawer scrollability and requeue-on-busy handling

## 0017 Settings Popup Overlay Window

File: `0017-2026-02-25-settings-popup-overlay-window.md`

Covers:
- migrating settings presentation from in-flow drawer to absolute overlay popup
- preserving settings scroll behavior and close invariants under popup composition
- preventing terminal width/layout shifts when settings opens

## 0018 macOS Menubar Window Behavior and Status Icon Parity

File: `0018-2026-02-25-macos-menubar-window-behavior-and-status-icon-parity.md`

Covers:
- moving menubar quick-terminal shell from pinned popup semantics to normal movable/resizable desktop window behavior
- keeping Dock visibility in menubar startup mode while preserving quick-terminal command flow
- adding menubar status icon parity for regular startup mode

## 0019 Tab Title Width Stability and Tooltip Overflow

File: `0019-2026-02-25-tab-title-width-stability-and-tooltip-overflow.md`

Covers:
- fixed-width tab-title chips to prevent post-create width jitter on asynchronous title updates
- stable truncation behavior for long tab titles without pushing tab-strip controls
- GPUI stateful tooltip pattern for showing full title on hover

## 0020 AtomOneDark Palette Aligned to Atom One Dark Pro

File: `0020-2026-02-25-microterm-palette-aligned-to-atom-one-dark-pro.md`

Covers:
- replacing legacy `atom_one_dark` palette constants with Atom One Dark Pro-aligned ANSI/foreground/background values
- preserving persisted theme compatibility by keeping the stable `atom_one_dark` key
- adding regression guardrails for palette constant drift

## 0021 AtomOneDark Black Background and White Red Channel

File: `0021-2026-02-25-microterm-black-background-and-white-red-channel.md`

Covers:
- forcing `atom_one_dark` terminal background to black for higher contrast
- remapping ANSI red channels to white-family colors to avoid red output text
- keeping palette semantics verified through unit tests

## 0022 Theme Rename atom_one_dark to AtomOneDark

File: `0022-2026-02-25-theme-rename-microterm-to-atom-one-dark.md`

Covers:
- renaming `TerminalTheme::Microterm` to `TerminalTheme::AtomOneDark`
- switching documented theme key to `atom_one_dark`
- preserving backward compatibility with `alias = "microterm"` during deserialization

## 0023 Last-Tab Close Hides Window via Controller Path

File: `0023-2026-02-25-last-tab-close-hides-window-via-controller-path.md`

Covers:
- `Cmd+W` behavior at the last-tab boundary (hide window instead of no-op)
- controller-aware hide routing to keep quick-terminal visibility state consistent
- regression guardrails for tab-close boundary logic and verification

## 0024 Selection Highlight Tint and Contrast

File: `0024-2026-02-25-selection-highlight-tint-and-contrast.md`

Covers:
- replacing selection fg/bg inversion with theme-aware tinted background blending
- preserving selected text readability by keeping foreground colors stable
- regression guardrails for selection color math and dark-theme highlight quality

## 0025 Unified macOS App Shell Without Dock Mode

File: `0025-2026-02-25-unified-macos-app-shell-without-dock-mode.md`

Covers:
- removing `dock_mode` from settings/runtime and collapsing macOS startup into one controller path
- preserving startup-open window with menubar/hotkey show-hide toggling in a single state machine
- guardrails for controller-owned visibility state, callback-based hide routing, and status-item lifetime

## 0026 Cursor Blink Suppression During Input

File: `0026-2026-02-25-cursor-blink-suppression-during-input.md`

Covers:
- suppressing cursor blink for a short window after terminal input so typing keeps a steady cursor
- keeping suppression logic consistent across blink timer and render visibility paths
- regression guardrails for suppression expiry, tab reset behavior, and input-entry routing

## 0027 Responsive Settings and Find UI Hardening

File: `0027-2026-02-25-responsive-settings-and-find-ui-hardening.md`

Covers:
- viewport-aware width policies for tab-bar find strip and settings popup
- theme-derived active-tab accent and control-size normalization for better UI consistency
- overlay close-path hardening (backdrop click + existing close routes) and find-strip guidance cleanup

## 0028 Tab Hover Close Action

File: `0028-2026-02-25-tab-hover-close-action.md`

Covers:
- showing a tab-local close button only while hovering the corresponding tab item
- safe click routing so close action does not also trigger parent tab activation
- hover-state guardrails for tab close behavior and stale-hover cleanup

## 0029 Tab Accent Purple Token

File: `0029-2026-02-25-tab-accent-purple-token.md`

Covers:
- introducing a fixed tab brand-purple token for tab highlight states
- applying that token to active-tab indicator and tab-close hover styling
- preserving boundaries between fixed accent states and theme-derived neutral tints

## 0028 Window Deactivation Hide Deferral for App-Borrow Safety

File: `0028-2026-02-25-window-deactivation-hide-deferral-for-app-borrow-safety.md`

Covers:
- eliminating close/reopen `RefCell already borrowed` timing by deferring deactivation-triggered hide callbacks
- clarifying app-level borrow re-entry risk in GPUI observer callback timing
- regression guardrails for deactivation scheduling predicates and deferred callback behavior

## 0030 Tab Bar Spacing Rhythm Refresh

File: `0030-2026-02-25-tabbar-spacing-rhythm-refresh.md`

Covers:
- balanced-compact spacing refresh for tab strip, tab chips, and right-side controls
- preserving fixed tab geometry and 40px tab-bar height invariants while improving visual hierarchy
- regression guardrails for spacing token drift and tab interaction/layout stability

## 0031 Settings Overlay Position Invariant and Focus Dim

File: `0031-2026-02-25-settings-overlay-position-invariant-and-focus-dim.md`

Covers:
- preventing popup overlays from re-entering flex flow due to conflicting position setters
- preserving terminal layout isolation while settings overlay is open
- standardizing subtle backdrop dimming for focus without over-darkening context

## 0032 Active Tab Indicator Bottom Clearance

File: `0032-2026-02-25-active-tab-indicator-bottom-clearance.md`

Covers:
- lifting the active-tab purple indicator above the tab-bar bottom border with a fixed gap token
- preserving tab geometry invariants by accounting for indicator-bottom clearance in vertical footprint checks
- keeping active/inactive indicator rendering behavior unchanged except for vertical offset

## 0033 macOS App Icon Pipeline and Rounded Asset

File: `0033-2026-02-25-macos-app-icon-pipeline-and-rounded-asset.md`

Covers:
- introducing repository-tracked rounded app-icon assets for macOS releases
- wiring `CFBundleIconFile` and bundle resource copy so `SimpleTerm.app` resolves a custom icon
- safe-change and regression guardrails for icon filename/resource/plist alignment

## 0034 Multi-Click Selection Modes

File: `0034-2026-02-26-multi-click-selection-modes.md`

Covers:
- enabling double-click semantic-word selection and triple-click line selection
- centralizing click-count-to-selection-mode mapping and test guards
- preserving PTY mouse-mode passthrough while improving normal-mode selection UX

## 0035 Dock Reopen Hide-Command Consistency

File: `0035-2026-02-26-dock-reopen-hide-command-consistency.md`

Covers:
- handling macOS Dock-driven reopen where window visibility can bypass controller `show_terminal` bookkeeping
- making controller hide path idempotent so stale `visible` flags do not drop valid hide requests
- preserving last-tab `Cmd+W` hide behavior parity across Dock-open and menubar-open flows

## 0036 macOS Toggle and Pin Hotkeys

File: `0036-2026-02-26-macos-toggle-and-pin-hotkeys.md`

Covers:
- adding dedicated global shortcuts for show/hide (`global_hotkey`) and pin/unpin (`pin_hotkey`)
- keeping pin behavior controller-owned while applying native macOS floating window level
- preserving fallback/compatibility handling for shortcut parsing and registration conflicts

## 0037 Settings Panel Global Hotkey Control

File: `0037-2026-02-26-settings-panel-global-hotkey-control.md`

Covers:
- setting the show/hide shortcut default to `command+F4`
- exposing settings-panel shortcut recording for `global_hotkey`
- applying hotkey updates live via controller command routing and safe re-registration

## 0038 macOS Show Terminal on Mouse Monitor

File: `0038-2026-02-26-macos-show-terminal-on-mouse-monitor.md`

Covers:
- ensuring `show_terminal` resolves placement from current mouse monitor on every show request
- applying monitor-aware move logic to existing-window reuse path before finalizing show handling
- preserving controller-owned placement boundaries between `main.rs` and `macos.rs`

## 0039 macOS Deferred Window Move to Avoid GPUI Re-entry

File: `0039-2026-02-26-macos-deferred-window-move-to-avoid-gpui-reentry.md`

Covers:
- resolving `RefCell already borrowed` logs caused by synchronous native window frame moves inside GPUI update flow
- deferring existing-window move/pin update via `App::defer`, avoiding in-update window activation, and deferring native frame move via main-queue dispatch to avoid in-stack GPUI re-entry
- preserving monitor-follow placement behavior without recreating terminal windows

## 0040 macOS Per-Monitor Window Position Persistence

File: `0040-2026-02-26-macos-per-monitor-window-position-persistence.md`

Covers:
- persisting last known window origin per monitor key in `settings.json`
- capturing monitor+position during hide and reapplying on next show for that monitor
- clamping restored positions to visible monitor bounds with centered fallback when no persisted position exists

## 0041 macOS Per-Monitor Window Size Persistence

File: `0041-2026-02-26-macos-per-monitor-window-size-persistence.md`

Covers:
- extending per-monitor placement persistence with optional `width`/`height`
- capturing position and size together on hide and restoring both on next show
- preserving backward compatibility for older settings by treating saved size as optional and clamping restored geometry to visible bounds

## 0042 macOS Existing-Window Frame Restore

File: `0042-2026-02-26-macos-existing-window-frame-restore.md`

Covers:
- fixing existing-window show path to apply full frame (position + size) rather than top-left-only movement
- preserving deferred main-queue native frame mutation for GPUI borrow safety
- preventing cross-monitor size bleed when reusing a hidden window

## 0043 macOS Cross-Monitor Popup Latency Reduction

File: `0043-2026-02-26-macos-cross-monitor-popup-latency-reduction.md`

Covers:
- reducing extra deferred-hop latency in existing-window reopen path
- applying monitor placement/pin update inline for reused windows while keeping deferred native frame mutation
- activating the app after frame scheduling so quick monitor switches feel immediate

## 0044 macOS Fast-Toggle Latency Guardrails

File: `0044-2026-02-26-macos-fast-toggle-latency-guardrails.md`

Covers:
- skipping no-op native frame updates when existing-window frame already matches target
- using tolerant placement comparisons to avoid redundant settings writes on hide
- preserving fast same-monitor toggle behavior without breaking per-monitor restore correctness

## 0045 Tab-Bar Pin Indicator and Controller Sync

File: `0045-2026-02-26-tabbar-pin-indicator-and-controller-sync.md`

Covers:
- adding tab-bar pin state indicator (`📌`/`○`) as explicit pinned-status affordance
- synchronizing controller-owned pinned state into `TerminalView` across existing/new window paths
- routing indicator clicks through controller command flow to preserve pin-state ownership

## 0046 macOS Hidden-Window Frame Apply Ordering

File: `0046-2026-02-26-macos-hidden-window-frame-apply-ordering.md`

Covers:
- fixing cross-monitor reopen flash by applying hidden-window frame updates before activation visibility
- keeping deferred main-queue frame mutation only for visible windows to preserve re-entry safety
- clarifying hidden-vs-visible frame-apply strategy as a runtime invariant

## 0047 macOS Cross-Monitor Activation Ordering Without Re-entry

File: `0047-2026-02-26-macos-cross-monitor-activation-ordering-without-reentry.md`

Covers:
- eliminating `RefCell already borrowed` re-entry by keeping frame mutation deferred for hidden-window cross-monitor reopen
- moving hidden-window activation into the same native deferred callback after frame apply
- making activation ownership explicit between controller and native callback paths

## 0048 Local macOS Packaging Entrypoint with Icon Contract

File: `0048-2026-02-26-local-macos-packaging-entrypoint-with-icon-contract.md`

Covers:
- adding a one-command local packaging entrypoint (`make package-macos-local`) for release-like app bundle assembly
- mirroring icon contract requirements (`CFBundleIconFile` + `SimpleTerm.icns` copy into `Contents/Resources/`) in local packaging path
- reducing false icon-regression reports caused by validating raw release binaries instead of packaged `.app` bundles

## 0049 macOS Toggle Terminal Focus Restoration

File: `0049-2026-02-26-macos-toggle-terminal-focus-restoration.md`

Covers:
- restoring explicit terminal view focus on show/toggle paths so `Cmd+F4` reopen accepts immediate typing
- preserving existing activation ownership split without reintroducing window-level activation in borrowed update paths
- documenting focus restoration as a show-path invariant alongside existing activation-ordering guardrails

## 0050 Home `~/.simple-term` Settings Source and Bootstrap

File: `0050-2026-02-26-home-dot-simple-term-settings-source-and-bootstrap.md`

Covers:
- standardizing persisted settings path to `~/.simple-term/settings.json`
- creating missing config directory/file on startup while preserving existing user files
- keeping UI persistence and direct JSON editing on one shared settings source

## 0051 Tab and Settings Spacing Polish

File: `0051-2026-02-26-tab-and-settings-spacing-polish.md`

Covers:
- spacing rhythm polish for tab strip and settings drawer while keeping `TAB_BAR_HEIGHT_PX = 40.0`
- cleaner visual separators via subtle border contrast reduction without behavior changes
- safe-change guardrails for compact-density preservation and viewport regression checks

## 0052 Terminal Content Card Container Spacing

File: `0052-2026-02-26-terminal-content-card-container-spacing.md`

Covers:
- resolving edge-clinging terminal content with container-level spacing in `content_row`
- introducing a rounded, subtly bordered terminal frame without changing canvas coordinate logic
- guardrails for preserving interaction behavior while applying visual polish

## 0053 Pin Shortcut Focus Scope and Cursor Blink Default

File: `0053-2026-02-26-pin-shortcut-focus-scope-and-cursor-blink-default.md`

Covers:
- enforcing `pin_hotkey` as focused-window scope instead of global hotkey scope
- preventing hidden-window popup side effects from `Cmd+Backquote` pin toggles
- restoring blinking-by-default behavior for `blinking = "terminal"` cursor mode

## 0054 Cmd+W Last Tab Force-Hide Even When Pinned

File: `0054-2026-02-26-cmdw-last-tab-force-hide-even-when-pinned.md`

Covers:
- introducing explicit `ForceHideTerminal` command for user-initiated last-tab close
- preserving pinned protection for passive hide paths while allowing `Cmd+W` hide on the last tab
- keeping hide-policy ownership centralized in app-shell controller logic
