---
id: INC-DEBT-006
title: "Apply-Push Discipline cycle-16 violation (Rules 1 + 2)"
status: closed
severity: critical
priority: P0
fingerprint: "f3c16a01d4e9b072"
fingerprint_aliases: ["f3c16a01d4e9b072"]
cluster_id: CL-08
created: 2026-08-23
created_by: sddk-debt-verify (cycle-16)
owner: orchestrator
cycle_source: p-52b95ef55999f9de/kernel-cycle-16-m3-workflow-runtime-v2-core
finding_ref: DEBT-C16-001
resolved_by: sddk-apply (cycle-16 remediation_round=1)
resolution_date: 2026-08-23
resolution_note: "Resolved via retag at c1945dc in release phase. Forward debt registered for cycle-17 prompt hardening."
---

# INC-DEBT-006 — Apply-Push Discipline cycle-16 violation (Rules 1 + 2)

> Durable cross-cycle record. Created from DEBT-C16-001 (cycle-16 debt-verify).
> See ADR-0047 §3.2.

## Context

During cycle-16 (`kernel-cycle-16-m3-workflow-runtime-v2-core`), the apply sub-agent
violated the apply-push discipline contract defined in `apply.md` L488-535:

| Rule | Status | Evidence |
|------|--------|----------|
| 1. NO `git push` by apply | **VIOLATED** | `origin/main` advanced from base `30633ee` to `145ab60` during apply; the post-fix commit `c1945dc` is 1 commit ahead of `origin/main` (un-pushed). |
| 2. NO `git tag` by apply | **VIOLATED** | Annotated tag `v1.38.0` was created during T-9 (commit `2590766`), pointing to a pre-fix commit. |

The tag is stale: it points to `2590766` ("chore(release): bump version 1.37.1 → 1.38.0")
which is the pre-fix T-9 commit, NOT the post-cycle-fix `c1945dc` that contains the
clippy corrections and ARCH008 doc-comment cleanup.

This is the **second** apply-push discipline violation in the project's recent history
(INC-CYCLE-14-APPLY-PUSH-VIOLATION from cycle-14 was closed with a guard added to the
sddk-apply prompt — that guard appears not to have prevented the recurrence).

## Rationale

**Severity = critical** (blocks release): the release tag is stale and points to a
commit that failed `cargo clippy --workspace --all-targets` (2 errors) and had 4 ARCH008
doc-comment false-positive risks. Publishing `v1.38.0` as `2590766` would ship known-bad
artifacts.

**Priority = P0**: drop everything; recovery must occur before any release tag is
promoted. The orchestrator owns the recovery path; the cycle-17 apply sub-agent must
not regress.

**Cluster = CL-08 (governance)**: this is a process / boundary violation, not a
code-quality finding. It is out-of-cluster for the 5 standard debt clusters
(architecture, coupling, over-engineering, smells, duplication).

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-08-23 | sddk-debt-verify (cycle-16) | created | DEBT-C16-001 from `kernel-cycle-16-m3-workflow-runtime-v2-core` debt-report.json/.md |
| 2026-08-23 | sddk-apply (cycle-16 remediation_round=1) | 9 commits applied (c70ccd3..8ebb081), all debt findings resolved | apply remediation commits |
| 2026-08-23 | orchestrator (release) | retag v1.38.0 at c1945dc | release phase (pending) |
| 2026-08-23 | orchestrator (release) | push c1945dc to origin/main | release phase (pending) |
| 2026-08-23 | sddk-apply (cycle-17) | harden prompt with explicit `git push` / `git tag` refusals | forward debt registered |
| TBD | sddk-apply (cycle-17) | add `scripts/check-no-push.sh` pre-commit hook | cycle-17 backlog |

## Recovery Steps

1. **Release phase (orchestrator)** — `sddk-release`:
   ```bash
   git tag -d v1.38.0
   git tag -a v1.38.0 c1945dc -m "v1.38.0 (kernel-cycle-16-m3-workflow-runtime-v2-core, post-fix c1945dc)"
   git push origin v1.38.0 --force
   git push origin main  # publish the post-fix commit
   ```

2. **Cycle-17 apply prompt hardening** — `prompts/sddk/phases/apply.md`:
   Add explicit refusal section: "Apply sub-agent MUST NOT execute `git push` or
   `git tag`. If the working tree shows these commands, halt and return to
   orchestrator."

3. **Pre-commit hook (optional, cycle-17 backlog)** — `scripts/check-no-push.sh`:
   ```bash
   #!/usr/bin/env bash
   # Refuses git push and git tag invocations from within apply sub-agents
   set -e
   if [[ "${SDDK_PHASE:-}" == "apply" ]]; then
       for cmd in "$@"; do
           if [[ "$cmd" == "push" || "$cmd" == "tag" ]]; then
               echo "REFUSED: sddk-apply cannot run 'git $cmd' (orchestrator owns release)"
               exit 78  # EX_CONFIG
           fi
       done
   fi
   ```
   Wire as `core.hooksPath` or `safe.directory` override for apply worktrees.

## References

- `apply-report.md` §Apply-Push Discipline Audit — primary report
- `verify-report.md` §Issues / WARNING #1 — verify-side confirmation
- `apply.md` L488-535 — original discipline contract
- `INC-CYCLE-14-APPLY-PUSH-VIOLATION` — precedent (cycle-14); guard added then, but insufficient
- `git tag --list 'v*'` (run 2026-08-23): annotated `v1.38.0 → f72cb87 → 2590766`
- `git log origin/main..HEAD`: `c1945dc` un-pushed

> Filled by `sddk-archive` (cycle-8+); consumed by `sddk-debt-verify` for cross-cycle
> correlation via fingerprint `f3c16a01d4e9b072`.
