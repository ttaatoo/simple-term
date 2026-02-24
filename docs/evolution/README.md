# Evolution History

This directory is the project's chronological engineering memory.
Its purpose is to help both humans and LLM agents quickly answer:
- What changed?
- Why was it changed?
- What alternatives were considered?
- What constraints drove the final decision?
- What was the impact and verification evidence?

## File Naming and Ordering

Use this filename format for every entry:

`NNNN-YYYY-MM-DD-short-kebab-title.md`

Rules:
- `NNNN` is a 4-digit, strictly increasing sequence.
- Sequence order must match event chronology.
- Never reuse a sequence number.
- If multiple events happen on the same date, keep incrementing `NNNN`.

Examples:
- `0006-2026-03-02-input-method-refactor.md`
- `0007-2026-03-05-shell-startup-optimization.md`

## Required Structure for Each Entry

Each entry should follow this order:

1. `Metadata`
2. `Summary`
3. `Context and Problem`
4. `Goals and Non-Goals`
5. `Decision`
6. `Alternatives Considered`
7. `Implementation Details`
8. `Impact`
9. `Verification`
10. `Risks and Follow-ups`
11. `Traceability`

## When a New Entry Is Mandatory

Create a new entry for any of these:
- New feature delivery
- API or behavior change
- Architecture/module boundary change
- CI/CD or release process change
- Branch protection/governance change
- Any tag or GitHub Release publication
- Incident-driven fix or rollback

## LLM-Friendly Writing Guidelines

- Prefer short, declarative sentences.
- Explicitly state causal reasoning ("because X, we chose Y").
- Separate facts from assumptions.
- Include concrete identifiers (commit SHA, PR number, run ID, tag).
- List rejected alternatives with reasons.
- Describe user-visible impact and developer workflow impact separately.

## Operating Rule Going Forward

For every new feature or infrastructure change:
1. Add one new evolution document in this directory.
2. Link it in `INDEX.md`.
3. Include traceability to commits, PRs, runs, and releases.

