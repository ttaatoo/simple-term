# 0006-2026-02-24-legacy-path-cleanup

## Metadata

- Date: 2026-02-24
- Sequence: 0006
- Status: active
- Scope: architecture, workflow

## Why This Entry Exists

A clean repository layout is critical for both humans and LLM agents. Parallel legacy trees with similar names can cause edits in the wrong location, broken assumptions about active code, and noisy code search results.

## System Context

Before cleanup, the repository included:
- active workspace paths: `apps/simple-term`, `crates/simple-term`
- stale non-workspace paths: `apps/zed-terminal`, `crates/zed-terminal`

Only `simple-term` paths were listed in root workspace members.

## Decision and Rationale

Decision:
- Remove `apps/zed-terminal` and `crates/zed-terminal` from the repository.

Rationale:
- Eliminate ambiguity during navigation/search.
- Prevent accidental edits to non-built code.
- Keep repository topology aligned with workspace definition.

Trade-offs:
- Historical snapshots are no longer present in-tree.
- Any needed historical context should come from git history, not duplicate live paths.

## Alternatives Considered

1. Keep legacy directories with warning notes
- Pros: easy local reference to old snapshots
- Cons: recurring confusion and accidental modifications
- Why not chosen: high long-term maintenance cost

2. Move legacy directories under an `archive/` folder
- Pros: preserves files while reducing top-level noise
- Cons: still searchable and editable by mistake
- Why not chosen: repository is cleaner with source-of-truth in git history

## Safe Change Playbook

When cleaning repository structure:
1. Verify active workspace members in root `Cargo.toml`.
2. Confirm candidate directories are not referenced by build/test workflows.
3. Remove stale directories atomically.
4. Update developer-facing docs to match the final layout.

## Do / Avoid

Do:
- Keep only active product paths in source tree.
- Use git history for legacy inspection.
- Keep docs synchronized with real directory structure.

Avoid:
- Leaving duplicate code trees that are not built.
- Documenting non-existent paths in README.
- Mixing cleanup with unrelated behavior changes.

## Typical Mistakes

- Assuming similarly named directories are all active.
- Running searches and patching files in stale paths.
- Forgetting to update docs after structural cleanup.

## Verification Strategy

Required checks:
- `cargo check --workspace`
- `rg -n "apps/zed-terminal|crates/zed-terminal" -S .` should return no active references (except historical git metadata)

Manual checks:
- validate README project layout section
- ensure workspace still builds and tests normally

## Related Artifacts

- Related docs: `README.md`, `docs/evolution/0001-2026-02-24-repository-layout-and-boundaries.md`
- Optional references: git history containing removed legacy paths

