# Development Evolution (LLM-Oriented)

This directory is **not** a mirror of git commit history.

Its purpose is to capture durable engineering knowledge that commit logs do not explain well:
- repository layout rationale
- architectural boundaries and invariants
- recurring design decisions and trade-offs
- common mistakes and how to avoid them
- accepted development patterns for future work

Think of this directory as the project's "engineering memory layer" that complements commits/PRs.

## Relationship to Commit History

- Commit history answers: **when** and **what file changed**.
- Evolution history answers: **why this structure exists**, **what constraints matter**, **how to change safely**.

Do not write entries as commit summaries.
Use commits only as optional references when they clarify context.

## Numbering and Naming

Use this filename format:

`NNNN-YYYY-MM-DD-short-kebab-title.md`

Rules:
- `NNNN` is strictly increasing.
- Keep chronological order.
- One entry should describe one coherent architectural or process topic.

## What Must Be Captured in New Entries

For every new feature, architecture change, or process shift, add one new entry that includes:
1. the problem model and constraints
2. the chosen design and rejected alternatives
3. the safe-change playbook (what to touch, what not to break)
4. anti-patterns and known failure modes
5. verification strategy and rollout notes

## LLM-Focused Writing Rules

- Prefer stable concepts over transient commit details.
- Explicitly state invariants with "must" / "must not" language.
- Include "Do / Avoid" sections.
- Include cross-file pointers so an LLM can navigate quickly.
- Include "Typical mistakes" and "Recovery steps".

## Authoring Checklist

Before finalizing an entry, confirm:
- It teaches a future contributor how to reason about the system.
- It helps avoid repeating a known mistake.
- It remains useful even if commit hashes are unavailable.
- It links to related architecture docs where needed.

