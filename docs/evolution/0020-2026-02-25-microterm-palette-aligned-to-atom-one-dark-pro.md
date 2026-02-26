# 0020-2026-02-25-microterm-palette-aligned-to-atom-one-dark-pro

## Metadata

- Date: 2026-02-25
- Sequence: 0020
- Status: active
- Scope: runtime

## Why This Entry Exists

The `atom_one_dark` preset originally used a legacy dark palette that no longer matched the requested visual baseline. We needed to align the default experience with Atom One Dark Pro while preserving existing user configuration compatibility.

This is not obvious from commit history alone because the key design constraint is schema stability: users should keep using `"theme": "atom_one_dark"` without migration, while runtime colors change.

## System Context

- Relevant directories/modules:
  - `apps/simple-term/src/terminal_view.rs`
  - `crates/simple-term/src/terminal_settings.rs`
  - `README.md`
- Upstream constraints (platform, library, policy):
  - Theme persistence uses serialized `TerminalTheme` enum values in `settings.json`.
  - `atom_one_dark` is already documented and used as the default theme key.
  - Terminal fallback colors are sourced from `theme_palette` and then mapped into `alacritty_terminal` color slots.
- Invariants already in force:
  - Default theme key must remain `atom_one_dark` for backward compatibility.
  - Theme presets must keep deterministic cycling order.
  - UI chrome and terminal fallback colors must stay in a single palette mapping function.

## Decision and Rationale

- Decision:
  - Keep preset identifier `atom_one_dark`.
  - Replace `atom_one_dark` palette values with Atom One Dark Pro-aligned colors (ANSI 16, foreground/background, chrome background).
  - Add a regression test for the `atom_one_dark` palette constants.
- Why this path was selected:
  - Delivers requested visual update immediately.
  - Avoids configuration migration and preserves existing `settings.json`.
  - Locks palette constants with tests to prevent accidental drift.
- Trade-offs accepted:
  - Existing users selecting `atom_one_dark` will see different colors after upgrade.
  - The `atom_one_dark` name no longer describes its historical palette origin.

## Alternatives Considered

1. Add a new `onedark_pro` preset and keep existing `atom_one_dark` colors
- Pros:
  - No visual change for current `atom_one_dark` users.
  - Naming matches palette identity directly.
- Cons:
  - User request explicitly targeted replacing `atom_one_dark`.
  - Adds one more preset to maintain and document.
- Why not chosen:
  - Did not satisfy requested behavior change.

2. Rename enum/config key from `atom_one_dark` to `onedark_pro`
- Pros:
  - Cleaner semantic naming.
- Cons:
  - Breaks existing `settings.json` unless migration logic is added.
  - Increases risk for persisted-theme compatibility regressions.
- Why not chosen:
  - Backward compatibility cost was unnecessary for this change.

## Safe Change Playbook

When modifying this area, follow these steps:
1. If changing preset colors, update `theme_palette` values and keep ANSI ordering (`black..white`, then bright variants).
2. Preserve persisted key compatibility unless migration is intentionally implemented and documented.
3. Update docs (`README` and `docs/evolution/INDEX.md`) when theme behavior or meaning changes.
4. Add or update unit tests that pin any preset constants you change.

## Do / Avoid

Do:
- Keep the persisted theme schema stable when only palette values change.
- Keep the source-of-truth palette mapping centralized in `theme_palette`.

Avoid:
- Splitting preset color constants across multiple files.
- Renaming serialized enum variants without an explicit migration plan.

## Typical Mistakes

- Updating terminal foreground/background but forgetting ANSI 16 colors, leading to inconsistent command output coloring.
- Introducing a new preset for a compatibility-sensitive change that should update an existing stable key.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - Launch app with `"theme": "atom_one_dark"` and verify terminal/chrome colors reflect Atom One Dark Pro style.
  - Cycle themes in settings and confirm `atom_one_dark` still appears and persists across restart.
- Signals of regression:
  - `atom_one_dark` renders legacy black-background palette after startup.
  - Persisted `theme` fails to deserialize or resets unexpectedly.

## Related Artifacts

- Related docs:
  - `docs/evolution/0014-2026-02-24-terminal-theme-presets-and-persistence.md`
  - `docs/evolution/INDEX.md`
- Optional references (PRs/commits/releases):
  - `https://github.com/Binaryify/OneDark-Pro`
