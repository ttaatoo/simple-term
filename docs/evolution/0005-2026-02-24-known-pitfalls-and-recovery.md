# 0005-2026-02-24-known-pitfalls-and-recovery

## Metadata

- Date: 2026-02-24
- Sequence: 0005
- Status: active
- Scope: workflow, governance, release

## Why This Entry Exists

This entry documents concrete failure patterns observed during project bootstrap, so future contributors and LLM agents can detect and recover quickly.

## System Context

Common failure surfaces:
- branch protection vs direct push behavior
- release runs with stale workflow assumptions
- local git state diverging from remote governance
- CI check-name mismatches

## Decision and Rationale

Decision:
- Maintain an explicit "pitfall + detection + recovery" playbook.

Rationale:
- These issues are procedural and easy to repeat.
- Commit history shows the event happened but does not teach recovery strategy.

## Alternatives Considered

1. Rely on troubleshooting as incidents occur
- Pros: minimal documentation effort
- Cons: repeated mistakes and slower incident response
- Why not chosen: poor operational learning

2. Keep guidance in PR comments only
- Pros: contextual to each event
- Cons: fragmented and hard for LLMs to ingest
- Why not chosen: weak discoverability

## Safe Change Playbook

For each operational issue:
1. Detect symptom from command/output.
2. Map symptom to known pitfall below.
3. Apply corresponding recovery steps.
4. Add new pitfall here if issue is novel.

## Do / Avoid

Do:
- Verify remote policy state before forceful operations.
- Confirm workflow/run IDs before cancellation/retry.
- Re-check release assets against support policy.

Avoid:
- Assuming local branch rules mirror remote settings.
- Retrying release blindly without cancelling outdated runs.
- Treating admin bypass as a standard merge path.

## Typical Mistakes

1. Direct push rejected on protected branch
- Detection: push error indicates PR/check/review requirements
- Recovery:
  - create/push feature branch
  - open PR
  - satisfy checks/reviews or use documented emergency path

2. Release run uses outdated policy (e.g., wrong platform set)
- Detection: active run building unexpected targets
- Recovery:
  - cancel stale run
  - update workflow on default branch
  - re-trigger release

3. Tag/release mismatch or duplicate tag
- Detection: release workflow fails tag checks
- Recovery:
  - inspect remote tag state
  - delete/recreate tag only with explicit operator intent
  - rerun release after version validation

4. Required check stuck due to wrong check name
- Detection: merge blocked with "required check expected" while CI appears green
- Recovery:
  - align branch rule required-check name with actual workflow job name

## Verification Strategy

After any recovery action:
- re-run relevant CI or release workflow
- verify branch/release state via GitHub API or UI
- confirm policy and docs are still consistent

## Related Artifacts

- Related docs: `docs/troubleshooting.md`, `docs/release-strategy.md`
- Optional references: GitHub Actions run history and branch protection settings

