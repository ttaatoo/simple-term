# 0044-2026-02-26-macos-fast-toggle-latency-guardrails

## Metadata

- Date: 2026-02-26
- Sequence: 0044
- Status: active
- Scope: runtime

## Why This Entry Exists

Users still observed latency during rapid show/hide on the same monitor. Two avoidable costs remained: unnecessary frame-restore calls when the window was already in place, and strict float equality causing redundant settings writes.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/macos.rs`
  - `crates/simple-term/src/terminal_settings.rs`
- Upstream constraints (platform, library, policy):
  - native frame updates are deferred to main queue for safety
  - hidden-window reuse should be near-instant when geometry is unchanged
- Invariants already in force:
  - per-monitor placement persistence
  - show-on-cursor-monitor behavior

## Decision and Rationale

- Decision:
  - add frame-diff check with tolerance before scheduling native frame updates
  - add tolerant placement comparison before persisting settings on hide
- Why this path was selected:
  - removes no-op native frame work and avoids unnecessary disk writes
  - preserves safety model and behavior correctness
- Trade-offs accepted:
  - tiny (<0.5pt) drift is treated as equivalent by design

## Alternatives Considered

1. Persist on every hide unconditionally
- Pros:
  - simple
- Cons:
  - repeated disk IO under rapid toggling
- Why not chosen:
  - worsens latency

2. Remove all frame restoration for existing windows
- Pros:
  - fast path
- Cons:
  - breaks cross-monitor and per-monitor restore correctness
- Why not chosen:
  - correctness regression

## Safe Change Playbook

When modifying this area, follow these steps:
1. Keep tolerance-based comparison centralized (`approximately_equals`, `window_needs_frame_update`).
2. Use the same tolerance consistently between save and apply paths unless there is a proven reason to diverge.
3. Verify rapid same-monitor toggles and cross-monitor restores both remain correct.

## Do / Avoid

Do:
- skip native frame updates when current frame already matches target
- avoid persisting placement on insignificant float jitter

Avoid:
- strict float equality in runtime geometry paths
- scheduling frame updates unconditionally during every show

## Typical Mistakes

- Treating tiny coordinate jitter as meaningful change.
- Optimizing same-monitor toggles by dropping cross-monitor restore logic.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - rapid show/hide on same monitor should feel immediate
  - cross-monitor show/hide should still restore independent size/position
- Signals of regression:
  - delayed popup on same-monitor rapid toggles
  - settings file rewritten on every hide without actual movement/resize

## Related Artifacts

- Related docs:
  - `docs/evolution/0043-2026-02-26-macos-cross-monitor-popup-latency-reduction.md`
  - `docs/evolution/0042-2026-02-26-macos-existing-window-frame-restore.md`
- Optional references (PRs/commits/releases):
  - `apps/simple-term/src/main.rs`
  - `apps/simple-term/src/macos.rs`
  - `crates/simple-term/src/terminal_settings.rs`
