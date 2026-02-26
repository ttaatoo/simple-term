# 0041-2026-02-26-macos-per-monitor-window-size-persistence

## Metadata

- Date: 2026-02-26
- Sequence: 0041
- Status: active
- Scope: runtime

## Why This Entry Exists

Position persistence per monitor was in place, but resized window dimensions still reset to defaults. This records the per-monitor size persistence behavior and safety constraints.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/macos.rs`
  - `apps/simple-term/src/main.rs`
  - `crates/simple-term/src/terminal_settings.rs`
- Upstream constraints (platform, library, policy):
  - monitor and window geometry are sourced from AppKit frame APIs
  - startup/open path still uses `WindowOptions::window_bounds` before native adjustment
- Invariants already in force:
  - terminal appears on mouse monitor
  - restored placement must remain visible on current monitor

## Decision and Rationale

- Decision:
  - extend persisted monitor placement with optional `width` and `height`
  - capture width/height together with x/y on hide
  - on show, use saved per-monitor size when present, then clamp to visible monitor area
- Why this path was selected:
  - restores user-resized dimensions without changing existing monitor-position keying scheme
  - backward compatible with older settings that only stored x/y
- Trade-offs accepted:
  - size is persisted on hide, not continuously during drag-resize
  - layout changes can cause clamp-adjusted dimensions on next show

## Alternatives Considered

1. Persist one global size only
- Pros:
  - simpler
- Cons:
  - incorrect when monitors have different resolutions/scales
- Why not chosen:
  - conflicts with per-monitor UX expectation

2. Persist every bounds change
- Pros:
  - newest value always available
- Cons:
  - frequent writes and noisier lifecycle coupling
- Why not chosen:
  - hide-time capture is sufficient and safer

## Safe Change Playbook

When modifying this area, follow these steps:
1. Keep monitor placement schema backward compatible (`width`/`height` optional).
2. Sanitize invalid saved dimensions (`<= 0` or non-finite) to `None`.
3. Clamp restored size and origin against current `visibleFrame` before applying.
4. Verify hide/show flow on at least two monitors after resizing on each.

## Do / Avoid

Do:
- keep per-monitor placement in one settings object containing x/y/optional size
- preserve fallback to configured defaults when saved size is missing

Avoid:
- trusting saved dimensions without visible-frame clamping
- introducing required size fields that break existing settings files

## Typical Mistakes

- Applying saved position math before final width/height clamp.
- Treating missing historical width/height as invalid settings.
- Persisting size but forgetting to compare and save only when changed.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - resize + move on monitor A, hide/show, verify both restored
  - resize + move on monitor B, hide/show, verify independent restore
  - switch monitor layout/resolution and confirm clamped on-screen restore
- Signals of regression:
  - size always resets after hide/show
  - off-screen/oversized restored window on smaller monitor

## Related Artifacts

- Related docs:
  - `docs/evolution/0040-2026-02-26-macos-per-monitor-window-position-persistence.md`
  - `docs/evolution/0038-2026-02-26-macos-show-terminal-on-mouse-monitor.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/macos.rs`
  - `apps/simple-term/src/main.rs`
  - `crates/simple-term/src/terminal_settings.rs`
