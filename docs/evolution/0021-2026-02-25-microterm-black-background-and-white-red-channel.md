# 0021-2026-02-25-microterm-black-background-and-white-red-channel

## Metadata

- Date: 2026-02-25
- Sequence: 0021
- Status: active
- Scope: runtime

## Why This Entry Exists

After aligning `atom_one_dark` to One Dark Pro, users still needed a higher-contrast terminal look: darker black background and no red output text in common command output.

This is durable behavior knowledge because it changes color semantics, not just aesthetics: ANSI red channels are intentionally remapped to white under `atom_one_dark`.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
  - `docs/evolution/INDEX.md`
- Upstream constraints (platform, library, policy):
  - Terminal text colors are sourced from ANSI palette slots through `theme_palette`.
  - `atom_one_dark` key remains the stable persisted theme identifier.
- Invariants already in force:
  - Theme values must stay centralized in `theme_palette`.
  - Theme changes must be regression-tested and documented in `docs/evolution/`.

## Decision and Rationale

- Decision:
  - Set `atom_one_dark` terminal background to pure black (`#000000`) and app chrome to near-black (`#101010`).
  - Remap ANSI red (`index 1`) and bright red (`index 9`) to white-family colors.
  - Set `atom_one_dark` default foreground to bright white-family color for readable output.
- Why this path was selected:
  - Matches the requested visual outcome immediately.
  - Targets the ANSI channels that produce the observed red output in terminal listings.
- Trade-offs accepted:
  - Red semantic emphasis is reduced for programs that rely on ANSI red under `atom_one_dark`.

## Alternatives Considered

1. Keep One Dark Pro mapping and only darken background
- Pros:
  - Preserves canonical One Dark Pro terminal semantics.
- Cons:
  - Red output text remains.
- Why not chosen:
  - Does not satisfy user requirement.

2. Add another separate preset instead of changing `atom_one_dark`
- Pros:
  - Leaves existing `atom_one_dark` behavior untouched.
- Cons:
  - User requested direct `atom_one_dark` adjustment.
- Why not chosen:
  - Adds unnecessary preset sprawl.

## Safe Change Playbook

When modifying this area, follow these steps:
1. Change `theme_palette(TerminalTheme::AtomOneDark)` values in one place.
2. Keep ANSI ordering intact (standard 0..7, bright 8..15).
3. Update palette regression tests in `apps/simple-term/src/terminal_view.rs`.
4. Run `cargo check --workspace` and `cargo test --workspace`.

## Do / Avoid

Do:
- Keep `atom_one_dark` behavior explicit in tests.
- Prefer high-contrast defaults for terminal readability requests.

Avoid:
- Partial color tweaks without updating tests.
- Moving palette constants out of `theme_palette`.

## Typical Mistakes

- Updating background but forgetting foreground/ANSI channels, causing mixed contrast behavior.
- Changing only bright red while leaving standard red untouched.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - Run `ls -la` on a system directory and confirm previously red filename channel renders white.
  - Confirm terminal background is visually black.
- Signals of regression:
  - ANSI red content still renders as red under `atom_one_dark`.
  - Background appears dark gray instead of black.

## Related Artifacts

- Related docs:
  - `docs/evolution/0020-2026-02-25-microterm-palette-aligned-to-atom-one-dark-pro.md`
  - `docs/evolution/0014-2026-02-24-terminal-theme-presets-and-persistence.md`
- Optional references (PRs/commits/releases):
  - N/A
