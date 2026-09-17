---
id: INC-A5-PUSH-RELEASE-MARKER-FRICTION
title: "Push/release protocol requires an empty ceremonial `chore(release): bump version` marker after a post-release documentation commit"
status: open
severity: low
priority: P2
fingerprint: "a5_push_release_marker_friction_v1"
fingerprint_aliases: ["a5_push_release_marker_friction_v1", "ceremonial_release_marker_v2"]
cluster_id: CL-RELEASE-DISCIPLINE
created: 2026-09-17
created_by: orchestrator (cycle p-63676b11dc0ef88f/a5-plan-base-production-ready)
owner: orchestrator
cycle_source: p-63676b11dc0ef88f/a5-plan-base-production-ready
finding_ref: A5-PLAN-push-contract-investigation
---

# INC-A5-PUSH-RELEASE-MARKER-FRICTION — ceremonial empty release marker to push docs

> Durable cross-cycle record. Registered during A5-PLAN (cycle
> `p-63676b11dc0ef88f/a5-plan-base-production-ready`). See
> `docs/architecture/a5/A5-PUSH-CONTRACT-INVESTIGATION.md`.

## Context

Across many cycle closes (A4-5a, A4-S15R, A4-5b, A4-5C, A4-CLOSEOUT),
after a release was published correctly, the subsequent
documentation/handoff push to `main` was rejected by `githooks/pre-push`
until an **empty** commit with subject
`chore(release): bump version (cycle close marker)` was added.

## Root cause

- `scripts/release.sh` (~line 232) runs `git push origin main`, pushing the
  release bump commit **before** the handoff docs exist.
- The hook checks the range `remote_sha..local_sha` for a release-marker
  subject or a `[workspace.package]` version bump.
- The docs-only range therefore contains neither, so the hook rejects it.

## Classification

**Independent operational defect** (not `INC-A4-RELEASE-VERSION-DRIFT`,
which is about pre-bump/tag confusion). The hook *intentionally* permits
the marker (INC-M7-9 option 3), but the *requirement* to emit a fake
release marker for a docs-only push is the defect.

Desired invariant:

```text
release commit ≠ handoff documentation commit ≠ cycle close marker
```

A push must never require asserting a version bump that did not happen.

## Remediation options (not implemented — A5-1)

1. Path-scoped allowance for documentation-only ranges (`docs/**`,
   `.sddk/**`, `**/*.md`).
2. Stop `release.sh` from pushing `main`; land the release commit and
   handoff docs in one push.
3. An explicit, honest `docs(cycle-close)` marker allowance instead of
   reusing the release subject.

## Impact

- A4 semantic certification is unaffected.
- It is operational friction that a production-ready tool should not
  require.

## Disposition

`MUST_CLOSE_A5` (P2), owner A5-1. Resolve independently of
`INC-A4-RELEASE-VERSION-DRIFT`.
