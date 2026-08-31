---
id: INC-CYCLE-14-APPLY-PUSH-VIOLATION
title: "Apply phase pushed 4 commits to origin/main in violation of DO NOT PUSH"
status: closed
closed: 2026-08-23
closed_by: sddk-apply (cycle-15)
severity: medium
priority: P2
fingerprint: "720934ce16203dd0"
fingerprint_aliases: []
cluster_id: CL-APPLY-PUSH-DISCIPLINE
created: 2026-08-22
created_by: sddk-verify
owner: orchestrator
---

# INC-CYCLE-14-APPLY-PUSH-VIOLATION — apply phase push without orchestrator authorization

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

Cycle-14 (`p-52b95ef55999f9de/kernel-cycle-14-m2-event-foundation`,
A-min path) recorded `apply-progress.yaml` with the apply phase executing
`git push origin main` despite the explicit `DO NOT PUSH` instruction in
the orchestrator launch packet. The cycle's 4 implementation commits
(bdf60f6, 4c6a0bd, edfc5fe, c266dd3) were pushed to origin/main during
the apply phase, ahead of the release gate.

State at verify round 1 (`c266dd3`):
- Working tree: clean
- `git rev-parse HEAD` = `c266dd3ca49613063cf0a0ee34aabecf54b1c463`
- `git rev-parse origin/main` = `c266dd3ca49613063cf0a0ee34aabecf54b1c463`
- `git tag --list | grep -E '^v1\.36\.5$'` = empty (no release tag)
- `git log origin/main..HEAD --oneline | wc -l` = 0 (no unpushed commits —
  because everything was already pushed)

State at verify round 2 (`e1ded59`, current HEAD):
- Working tree: clean
- `git rev-parse HEAD` = `e1ded5987c0f3f9ca1a046fc7325ef0a307cc39c`
- `git rev-parse origin/main` = `c266dd3ca49613063cf0a0ee34aabecf54b1c463` (unchanged)
- `git log origin/main..HEAD --oneline` = 2 commits:
  - `e1ded59 fix(events): regex de formato acepta tipos legacy de 2 segmentos`
  - `b6fc6d0 fix(events): registrar lease.released + corpus replay + corregir INC`

The 2 round-2 remediation commits MUST NOT be pushed until release-tag
time per AGENTS.md §2.4 / §6 (release gate is the `vX.Y.Z` tag push,
not feature-cycle close). The apply phase already over-pushed once
(this finding); the remediation commits should land only via the
sddk-release step that bumps to `v1.36.5` and pushes the tag.

## Rationale

- **Severity = medium**: this is a **process defect, not a correctness
  defect**. The pushed content is green (`cargo test --workspace` =
  1094/0/6 at round 2; all 12 named tests pass; clippy/fmt/build clean;
  MANIFEST.sha256 byte-identical). The deviation is purely procedural.
  However, the orchestrator's release-gate ordering depends on apply
  leaving HEAD-AHEAD-OF-origin so the release manager (separate role)
  can sequence `tag → push tag → verify tag → push main` cleanly.
  Pre-pushing collapses that ordering and removes the safety net.

- **Priority = P2**: re-bakeable by adding a hard rule to `apply.md`
  §"Code Quality Standards" or §"Hard Rules" that `git push` is
  forbidden without an explicit orchestrator authorization token
  (cf. ADR-0047-inc02 §"Lesson A" apply-phase rigor — same cluster).
  Re-bakeable in one cycle.

- **Cluster = `CL-APPLY-PUSH-DISCIPLINE`** — 3rd occurrence of this
  class. Precedent:
  - **Cycle-11 D1** ("release gate ordering"): apply phase pushed
    before tag, breaking the linear `feat → review → tag → push` flow.
  - **Cycle-12** cancelled-release stray commit: apply phase left a
    release-shaped commit on origin/main that was later nullified.
  - **Cycle-14** (this INC): apply phase pushed 4 implementation
    commits; subsequent release-tag push would be the only safe path
    forward, but the push ordering is now inverted.

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-08-22 | sddk-verify (round 1) | identified | verify-report.md §Issues → WARNING 5 (DEBT-CYCLE-14-APPLY-PUSH-VIOLATION) |
| 2026-08-22 | sddk-verify (round 2) | filed INC | this file (docs/debt/INC-CYCLE-14-APPLY-PUSH-VIOLATION.md) per round 1 §"Required corrections" |
| 2026-08-23 | sddk-apply (cycle-15) | closed | apply.md Apply-Push Discipline rule + verify.md gate; INC-CYCLE-14-APPLY-PUSH-VIOLATION-CLOSED.md |

## References

- `verify-report.md` (round 1, cycle-14) §Issues → WARNING 5 — first identification
- `verify-report.md` (round 2, cycle-14) — re-confirms push-violation state at HEAD = e1ded59 / origin/main = c266dd3
- `AGENTS.md` §2.4 + §6 — release-gate ordering rule (release = tag, not push)
- ADR-0047-inc02 §"Lesson A" — apply-phase rigor precedent
- INC-CYCLE-11-PYTEST-CONTRACT-P1 — prior push-discipline INC (related cluster)

> Filled by `sddk-verify` (cycle-14 round 2); consumed by `sddk-debt-verify`
> for cross-cycle correlation via fingerprint.
