# 0022-2026-02-25-theme-rename-microterm-to-atom-one-dark

## Metadata

- Date: 2026-02-25
- Sequence: 0022
- Status: active
- Scope: runtime

## Why This Entry Exists

The theme identifier and display name were still using `microterm` while the intended name is `AtomOneDark`.
This change records the naming migration and compatibility rules for persisted settings.

## System Context

- Relevant directories/modules:
  - `crates/simple-term/src/terminal_settings.rs`
  - `apps/simple-term/src/terminal_view.rs`
  - `README.md`
- Upstream constraints (platform, library, policy):
  - Theme values are serialized/deserialized via `serde`.
  - Existing user configs may still contain `"theme": "microterm"`.
- Invariants already in force:
  - Default theme must remain the same visual palette.
  - Settings deserialization must not break existing user files.

## Decision and Rationale

- Decision:
  - Rename enum variant `TerminalTheme::Microterm` to `TerminalTheme::AtomOneDark`.
  - Change user-facing label to `AtomOneDark`.
  - Change documented key to `atom_one_dark`.
  - Keep `alias = "microterm"` for backward compatibility in `serde`.
- Why this path was selected:
  - Aligns naming across code/UI/docs.
  - Preserves compatibility for already persisted configs.
- Trade-offs accepted:
  - Serialized value is now `atom_one_dark` for newly saved settings.
  - Legacy `microterm` remains as a compatibility alias and must be maintained unless a formal migration is introduced.

## Alternatives Considered

1. Keep enum and key unchanged, only update UI label
- Pros:
  - Minimal code churn.
- Cons:
  - Leaves schema/docs naming mismatch.
- Why not chosen:
  - Does not satisfy full rename intent.

2. Hard break to `atom_one_dark` without alias
- Pros:
  - Cleaner schema.
- Cons:
  - Breaks existing user settings.
- Why not chosen:
  - Backward compatibility regression.

## Safe Change Playbook

When modifying this area, follow these steps:
1. Keep `serde` rename/alias attributes synchronized with documented config keys.
2. Update all `TerminalTheme` match arms and preset lists when renaming variants.
3. Verify theme cycling tests and palette tests still pass after rename.

## Do / Avoid

Do:
- Keep compatibility aliases when renaming persisted enum values.
- Keep label/key/variant naming aligned.

Avoid:
- Renaming enum variants without checking serialization behavior.
- Updating docs without updating runtime deserialization rules.

## Typical Mistakes

- Changing the variant name but forgetting `THEME_PRESETS`, causing runtime cycle issues.
- Removing backward alias and breaking existing `settings.json` files.

## Verification Strategy

- Required automated checks:
  - `cargo check --workspace`
  - `cargo test --workspace`
- Recommended manual checks:
  - Start with `"theme": "microterm"` and verify it loads as AtomOneDark.
  - Save settings and verify persisted key is `atom_one_dark`.
- Signals of regression:
  - Theme fails to load from old configs.
  - UI still displays old name.

## Related Artifacts

- Related docs:
  - `docs/evolution/0014-2026-02-24-terminal-theme-presets-and-persistence.md`
  - `docs/evolution/0020-2026-02-25-microterm-palette-aligned-to-atom-one-dark-pro.md`
  - `docs/evolution/0021-2026-02-25-microterm-black-background-and-white-red-channel.md`
- Optional references (PRs/commits/releases):
  - N/A
