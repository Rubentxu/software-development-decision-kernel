---
id: INC-CYCLE-14-APPLY-PUSH-VIOLATION
title: "Apply phase pushed 4 commits to origin/main in violation of DO NOT PUSH"
status: closed
severity: medium
priority: P2
fingerprint: "720934ce16203dd0"
fingerprint_aliases: []
cluster_id: CL-APPLY-PUSH-DISCIPLINE
created: 2026-08-22
created_by: sddk-verify
closed: 2026-08-23
closed_by: sddk-apply (cycle-15)
owner: orchestrator
---

# INC-CYCLE-14-APPLY-PUSH-VIOLATION-CLOSED — Resolution

## Closure

This INC is **CLOSED** by cycle-15 (`kernel-cycle-15-hardening-loc-absorption-apply-discipline`).

## Resolution

### Apply-Push Discipline rule (Commit 4)

Added a new section to `prompts/sddk/phases/apply.md` between the existing
"Pre-commit Discipline" and "Code Quality Standards" sections:

- **Title**: `## Apply-Push Discipline (NON-NEGOTIABLE)`
- **5 numbered rules**: apply agents MUST NOT invoke `git push`; push-to-main
  is reserved for `sddk-release`; push-to-feature-branch is reserved for
  orchestrator's branch-creation; read-only remote ops remain permitted;
  only explicit orchestrator authorization token enables push.
- **Why section**: documented the 3-occurrence history (cycle-11, cycle-13,
  cycle-14).
- **Anti-patterns table**: 5 rows mirroring Pre-commit Discipline format.

Section: 45 LOC, passes REQ-CYCLE-15-003.

### Verify gate (Commit 5)

Added a new mandatory gate to `prompts/sddk/phases/verify.md`:

- Row in "Mandatory Gates" table: "Apply-Push discipline" — PASS if no push
  in apply-report, FAIL if push without `orchestrator_authorization` token.
- Procedural step "7.5. Apply-Push Discipline Gate" in §Procedure.

Section: 13 LOC, passes REQ-CYCLE-15-004.

### Enforcement

The rule is now self-policing: `sddk-verify` checks the apply-report for any
`git push` invocation and rejects the cycle if found without authorization.
The orchestrator owns push; apply owns commits only.

## References

- `prompts/sddk/phases/apply.md` — Apply-Push Discipline section (L488-532)
- `prompts/sddk/phases/verify.md` — Apply-Push Discipline gate (Mandatory Gates row + §7.5)
- `prompts/sddk/git-contract.md` — release-phase push ownership
- INC-CYCLE-11-D1 (cycle-11 push precedent)
- INC-CYCLE-13 (cycle-13 push precedent)
