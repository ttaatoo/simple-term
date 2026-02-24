# 0004-2026-02-24-release-and-governance-model

## Metadata

- Date: 2026-02-24
- Sequence: 0004
- Status: active
- Scope: release, governance

## Why This Entry Exists

Release/governance decisions are easy to forget because they live partly in workflow files and partly in GitHub settings. This entry centralizes the operational model and decision rationale.

## System Context

Current release model:
- SemVer-based versions from workspace manifest
- GitHub Release publication via `.github/workflows/release.yml`
- Artifact policy: macOS-only (`.dmg` as primary installer format, `.tar.gz` as fallback binary package)

Current governance model:
- Protected branch rules with required checks
- Admin bypass allowed for emergencies
- squash-merge oriented repository settings

## Decision and Rationale

Release decisions:
- Keep SemVer discipline with tag prefix `v`.
- Publish only macOS artifacts until official multi-platform support exists.

Governance decisions:
- Keep strict required checks for normal development.
- Permit admin bypass for urgent unblock/incident response.

Trade-offs:
- Admin bypass is operationally useful but introduces process-abuse risk.
- macOS-only reduces release noise but narrows immediate platform reach.

## Alternatives Considered

1. Multi-platform release regardless of support status
- Pros: broad surface for future adoption
- Cons: confusion and maintenance overhead for unsupported targets
- Why not chosen: mismatched product scope

2. Zero-governance fast merges
- Pros: speed
- Cons: high regression risk and low audit quality
- Why not chosen: unacceptable for infrastructure-critical changes

## Safe Change Playbook

When touching release/governance:
1. Update workflow/config and this entry in the same change set.
2. Validate versioning assumptions against workspace manifest.
3. Perform dry-run style checks where possible.
4. If bypass is used, record reason and follow-up action.

## Do / Avoid

Do:
- Keep release artifact policy aligned with actual support policy.
- Treat admin bypass as exception handling, not default flow.
- Preserve stable required-check naming.

Avoid:
- Triggering release with mismatched manifest version.
- Re-enabling unsupported platforms without explicit product decision.
- Frequent direct pushes to protected branches.

## Typical Mistakes

- Running outdated release workflow while policy changed.
- Requiring checks that no longer exist by name.
- Mixing governance changes with unrelated feature patches.

## Verification Strategy

Before release:
- verify `Cargo.toml` workspace version
- verify release workflow is current

After release:
- verify assets match policy (macOS-only)
- verify both `.dmg` and `.tar.gz` are present
- verify checksum file exists
- verify release notes and tag correctness

For governance changes:
- verify rule state in GitHub settings/API
- test a normal PR path and (if needed) documented bypass path

## Related Artifacts

- Related docs: `docs/evolution/README.md`, `docs/evolution/0005-2026-02-24-known-pitfalls-and-recovery.md`
- Optional references: GitHub branch protection and releases pages
