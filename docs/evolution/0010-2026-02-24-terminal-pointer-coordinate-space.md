# 0010-2026-02-24-terminal-pointer-coordinate-space

## Metadata

- Date: 2026-02-24
- Sequence: 0010
- Status: active
- Scope: runtime, testing

## Why This Entry Exists

After adding the top tab bar, terminal rendering moved to a non-zero Y origin, but pointer-to-grid mapping still treated pointer coordinates as if terminal content started at `(0, 0)`. This created selection and mouse-hit offset bugs that are easy to reintroduce when changing UI chrome.

## System Context

Relevant modules:
- `apps/simple-term/src/terminal_view.rs`
- `crates/simple-term/src/mappings/mouse.rs`

Upstream constraints:
- GPUI mouse event positions are window-relative.
- Terminal content is rendered inside a canvas region whose origin can be offset by top chrome (tab bar/title controls).
- Selection, hyperlink hit detection, mouse reporting, and scroll reporting all depend on the same point-to-grid conversion.

Invariants already in force:
- Pointer mapping must use terminal-local coordinates before converting to terminal row/column.
- Side (`Left`/`Right`) calculations must use local cell-relative X, not window X.
- Any tab-bar/chrome height change must keep render bounds and hit-test mapping aligned.

## Decision and Rationale

Decision:
- Normalize pointer coordinates inside `grid_point_and_side` by subtracting `TerminalBounds.bounds.origin` before computing line, column, and side.
- Add regression coverage for non-zero terminal origins.

Why this path was selected:
- This is the narrowest fix and keeps all call sites consistent automatically.
- Centralized coordinate normalization prevents future call-site drift between selection and mouse-report paths.

Trade-offs accepted:
- Mapping logic now depends on `TerminalBounds.bounds.origin` semantics being maintained correctly by the UI layer.

## Alternatives Considered

1. Subtract origin at every call site in `terminal_view.rs`
- Pros: no mapping helper changes
- Cons: duplicated logic and high risk of inconsistent behavior between selection/mouse paths
- Why not chosen: fragile and error-prone

2. Keep pointer mapping unchanged and force terminal bounds origin to `(0, 0)`
- Pros: fewer changes in mapping helpers
- Cons: breaks scrollbar geometry and diverges from actual rendered canvas position
- Why not chosen: violates render/hit-test consistency

## Safe Change Playbook

When modifying terminal layout or pointer handling:
1. Confirm whether terminal content origin moved (tab bar height, titlebar integration, padding, overlays).
2. Keep pointer conversion centralized in `grid_point_and_side`; do not add ad-hoc origin math in event handlers.
3. Add or update tests that exercise non-zero `TerminalBounds.bounds.origin`.
4. Re-run targeted pointer-mapping tests and full crate tests.
5. Manually verify selection start/end, drag selection, hyperlink hit, and mouse-mode reporting near the first visible row.

## Do / Avoid

Do:
- Treat render bounds origin and hit-test origin as a single invariant.
- Keep `grid_point` and `grid_point_and_side` behavior aligned for all pointer consumers.
- Include at least one regression test with non-zero origin whenever layout chrome changes.

Avoid:
- Assuming `(0, 0)` coordinate space in input mapping helpers.
- Mixing window-relative and terminal-local coordinates in the same calculation.
- Validating mapping behavior only with tests that use origin `(0, 0)`.

## Typical Mistakes

- Computing row/column from window coordinates without subtracting terminal bounds origin.
- Computing `Side` using unnormalized X, causing incorrect character-edge selection.
- Updating tab bar height without verifying pointer mapping regression tests.

## Verification Strategy

Required automated checks:
- `cargo test -p simple-term grid_point_ -- --nocapture`
- `cargo test -p simple-term`

Recommended manual checks:
- select text on the first visible row immediately below tab bar
- drag selection across line boundaries near the top edge
- right-click hyperlink detection on first visible row
- verify mouse-mode scroll/button reports still target expected row/column

Signals of regression:
- text selection starts one or more rows below pointer
- hyperlink open/hit-test targets line below cursor
- mouse protocol reports wrong row when clicking near top of terminal content

## Related Artifacts

- Related docs: `docs/evolution/0009-2026-02-24-terminal-tabs-and-tabbar-ui.md`, `docs/architecture-invariants.md`
- Optional references: `crates/simple-term/src/mappings/mouse.rs`, `apps/simple-term/src/terminal_view.rs`
