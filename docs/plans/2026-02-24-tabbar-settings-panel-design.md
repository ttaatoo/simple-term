# Tab Bar Settings Panel Design

## Goal

Add a right-side settings affordance in the top tab bar so users can quickly tune typography and app-shell mode without leaving terminal context.

Requested controls:

- change font family
- change font size
- toggle mode between default desktop app and menubar mode

## Constraints

- Keep existing terminal rendering/input behavior stable.
- Keep top tab bar height and terminal pointer coordinate assumptions stable.
- Persist preference changes to `settings.json`.
- Keep implementation in app/UI layer (`apps/simple-term`) and configuration layer (`crates/simple-term/src/terminal_settings.rs`).

## UX Brainstorm

### Option A (Recommended): Inline Expandable Toolbar in Top-Right

- Add compact settings button in tab bar right controls.
- Clicking button expands inline controls immediately left of the button.
- Controls include:
  - font family previous/next (`<` / `>`) + current font label
  - font size decrement/increment (`-` / `+`) + current size value
  - mode toggle button (`mode: Default` / `mode: Menubar`)

Pros:

- Lowest implementation risk (reuses existing tab bar flex layout).
- No overlay/absolute positioning complexity.
- Keeps context visible while applying typography changes.

Cons:

- Dense control area on smaller widths.
- Not as visually "panel-like" as a floating popover.

### Option B: Floating Popover Anchored to Right Settings Button

Pros:

- Better visual separation and clearer panel metaphor.
- More space for labels/hints.

Cons:

- Requires new positioning and z-order behavior in current GPUI view tree.
- Higher risk of clipping/overlap regressions.

### Option C: Separate Settings Window

Pros:

- Unlimited space for richer preferences.

Cons:

- Breaks quick-adjust workflow.
- Does not satisfy "top-right panel" intent.

## Chosen Interaction Model

Use Option A.

- Right-top button always visible.
- Expanded controls shown only when not in find-panel mode.
- Font and size changes apply immediately to active terminal view geometry.
- Mode toggle persists immediately; app policy is applied on macOS at runtime.

## Behavioral Notes

- Font candidates come from active font + configured fallbacks + curated monospace list.
- Font family cycling wraps around.
- Font size is clamped to safe range (6.0 to 72.0).
- Dock mode toggle persists to config and maps `Regular <-> MenubarOnly`.

## Verification Plan

Automated:

- unit tests for font option derivation, font cycling wraparound, and dock mode toggle mapping
- config save/load roundtrip tests
- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`

Manual:

- open settings from tab bar right side
- cycle font and verify terminal text metrics update instantly
- change font size and verify rows/cols resize correctly
- toggle mode and verify persisted config reflects selection
