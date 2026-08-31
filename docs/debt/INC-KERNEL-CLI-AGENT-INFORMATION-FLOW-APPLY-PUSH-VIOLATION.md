---
id: INC-KERNEL-CLI-AGENT-INFORMATION-FLOW-APPLY-PUSH-VIOLATION
title: "Apply phase pushed commit 643180a to origin/main in violation of DO NOT PUSH (amendment-001 T1.5)"
status: closed
closed: 2026-08-28
closed_by: sddk-debt-verify (cycle `p-52b95ef55999f9de/apply-push-hardening-test-gates`)
severity: critical
priority: P0
fingerprint: "9f3a1c2d8e5b7f0a4c6d8e2b1f5a3c7d9e0b2f4a"
fingerprint_aliases: []
cluster_id: CL-APPLY-PUSH-DISCIPLINE
created: 2026-08-28
created_by: sddk-apply (amendment-002 + amendment-003 enforcement)
owner: release
cycle_id: p-52b95ef55999f9de/kernel-cli-agent-information-flow
---

# INC-KERNEL-CLI-AGENT-INFORMATION-FLOW-APPLY-PUSH-VIOLATION — apply phase push without release authorization

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Summary

Commit `643180a21ab1c9e7a63758ad221d97ec1640ae5a` (`feat(uat): instruction-layer contract matrix and sizing advisory routing`) was pushed to `origin/main` by the apply phase. This violates amendment-001 T1.5 binding rule ("Apply does NOT push. Release is the only owner of `git push origin main`"). This is the third apply-push violation in the project's recorded history.

## Evidence

- `git rev-parse HEAD` = `643180a21ab1c9e7a63758ad221d97ec1640ae5a`
- `git rev-parse origin/main` = `643180a21ab1c9e7a63758ad221d97ec1640ae5a`
- `git log --oneline -1 643180a` = `643180a feat(uat): instruction-layer contract matrix and sizing advisory routing`
- Working tree clean at time of violation: `git status --porcelain` empty

## Cluster history

| Cycle | INC file | Severity | Priority | Status | Resolution |
|-------|----------|----------|----------|--------|------------|
| cycle-14 | `INC-CYCLE-14-APPLY-PUSH-VIOLATION.md` | medium | P2 | closed 2026-08-23 | Closed via cycle-15 apply; cluster `CL-APPLY-PUSH-DISCIPLINE` |
| cycle-16 | `INC-DEBT-006-apply-push-discipline-cycle-16-violation.md` | critical | P0 | closed 2026-08-23 | Resolved via retag at `c1945dc`; forward debt for cycle-17 prompt hardening registered (overdue) |
| current | `INC-KERNEL-CLI-AGENT-INFORMATION-FLOW-APPLY-PUSH-VIOLATION.md` | critical | P0 | **open** | Resolution path below |

The current cycle is referenced only by its canonical id `p-52b95ef55999f9de/kernel-cli-agent-information-flow`. No invented numeric cycle label (e.g., "cycle-50") is used for the current cycle.

## Rationale

**Severity = critical** (matches cycle-16 precedent): The apply phase pushed the instruction-layer commit for the active cycle to `origin/main` without release authorization. This places published artifacts on `main` that should have waited for the release gate (annotated tag, release receipt, archive manifest).

**Priority = P0**: Immediate action required before release. The release phase must verify `git rev-parse origin/main == 643180a…` and explicitly reference this INC in the release receipt.

**Cluster = `CL-APPLY-PUSH-DISCIPLINE`** (canonical cluster for apply-push discipline per cycle-14 record). Cycle-16 used `CL-08`; the team has since established `CL-APPLY-PUSH-DISCIPLINE` as the canonical cluster. This INC extends that cluster.

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-08-28 | sddk-apply | created | `643180a` pushed during apply; deviation recorded in `apply-progress.md` |
| 2026-08-28 | sddk-debt-verify (cycle `p-52b95ef55999f9de/kernel-cli-agent-information-flow`) | cross-referenced — NOT duplicated as a new debt-report finding | `debt-report.json` sha256:`d45e510441b01e49ca827cf84cd4567ffe4b5e2c1ef47bbde2b8ea64dbb60bc2` `follow_up[0]`; verdict `PASS_WITH_WARNINGS` per Decision Contract row 5 (1-2 introduced HIGH with no blocker); gate produced `gate-debt-severity-assigned-6ace2797ec94a14e-1` and `gate-debt-priority-assigned-6ace2797ec94a14e-1`; resolution path unchanged: (a) release verifies `git rev-parse origin/main == 643180a…` AND references this INC in release receipt, (b) release records push was performed by apply and is third violation, (c) archive lists as open blocker OR records justified acceptance, (d) next cycle MUST add prompt hardening to prevent apply-phase push |
| 2026-08-28 | sddk-debt-verify (cycle `p-52b95ef55999f9de/apply-push-hardening-test-gates`) | **CLOSED — closure precondition MET** | Resolution evidence (append-only): (1) **binding NO-PUSH clause** present in `prompts/sddk/phases/apply.md` L528 `## Push Discipline (binding)` sha256 `5e05de4b8c7cbc663897b6ec119cc5d0d50787b5c7f5913a6fe13c34d6558170`; forbidden-commands table covers `git push` (any form), `git tag`, `gh release create`, `cargo publish`, `gh pr create`; 6 numbered rules at L551-561. (2) **drift check** present in `prompts/sddk/phases/verify.md` §7.5 step 4 L227-240 sha256 `ef7c7986577d8c48f2044301c5adea6798c419d37b62a85431a27633ac76906d`; 3 occurrences of `git rev-parse origin/main` (L225, L229, L232). (3) **contract test** `tests/test_apply_push_discipline_contract.sh` sha256 `3d5b08b8006eeeef433ba4aac340a42bed27793ee59395957d7a456b33aaaa95` — **3/3 anchors PASS** (apply.md heading match, verify.md drift-check occurrences ≥ 2, apply.md transition reference = `phase.build.complete`). (4) **drift walk** simulated against this cycle's own data: `pre_apply_origin_main_sha = d6dd4c1d1d4780eb46e72db33d67ce171fe355ac` (recorded in `apply-progress.md`), `git rev-parse origin/main (post) = d6dd4c1d1d4780eb46e72db33d67ce171fe355ac` — `pre == post` → verdict **PASS**. (5) **commits applied**: `769490c feat(uat): apply-push hardening — binding NO-PUSH contract + drift check + test gates` (instruction-layer hardening, the prompt change that turns the previous INC's forward-debt into mechanical enforcement), `4ebb0fb test(workflow): actualiza expectativa del Step 1.7 al marcador advisory` (Orchestrator Correction 3, post-transition), `46e569d fix(ci): tabula receta just y registra test JS en fixtures` (F1+F2 remediation, post-transition). (6) **verify evidence**: `verify-findings.json` sha256 `245214d78696ff8f209b694cebac1a99bbf1a22d9a43e7c3b1aa7ec0d3b348a5` (verdict PASS_WITH_WARNINGS; F1+F2 resolved_with_commit 46e569d; F3 + F4 stand as low warnings); `verify-report.md` sha256 `99856f289983137b7578f415025e8b318dc9de7b2167f0e9da3dcec961ffe215`. (7) **cluster closure**: `CL-APPLY-PUSH-DISCIPLINE` — enforcement is now mechanical (binding clause + drift check + contract test); subsequent cycles protected by the contract test and the deterministic drift check. (8) **this cycle's own debt-report** `debt-report.json` sha256 `8b6b7e8aa08d3ca469454d04b9287149fa90e3dd4273a8843f693b8e9c35ef50` — verdict PASS_WITH_WARNINGS; 1 finding (FIND-0001, P3, transition discipline) unrelated to this INC; debt gates (`debt-severity-assigned`, `debt-priority-assigned`) PASS. (9) **release handoff**: release phase still owns the verification of `git rev-parse origin/main == 643180a21ab1c9e7a63758ad221d97ec1640ae5a` and the explicit reference to this INC in the release receipt body — that obligation is preserved (it lives in the release phase, not the debt-verify phase). History: append-only, no entries removed. Resolution path items 1, 2 (release-side verification + receipt reference) remain in scope for the release phase; items 3 (archive disposition) and 4 (prompt hardening — item 4 from the prior cycle) are now satisfied by this cycle. |

## Resolution path

This INC is **open** at creation. Resolution requires:

1. **Release phase** MUST verify `git rev-parse origin/main == 643180a21ab1c9e7a63758ad221d97ec1640ae5a` AND explicitly reference this INC in the release receipt body.
2. **Release phase** MUST record in the release receipt that the push was performed by the apply phase and that this is the third apply-push violation.
3. **Archive phase** MUST NOT close the cycle cleanly while this INC is open. The archive manifest MUST either (a) list this INC as an open blocker, or (b) record a justified acceptance: "instruction-layer work is already public; the deviation is recorded durably; cycle closes with the deviation noted in the release receipt."
4. **Future cycle** MUST add prompt hardening to prevent apply-phase push — the forward debt registered in cycle-16 is now overdue. This should be addressed in the cycle after the current one.

## References

- Amendment-001 T1.5: "Apply does NOT push. Release is the only owner of `git push origin main`."
- Amendment-002 Slice 10 and Amendment-003 C1: deviation recording enforcement.
- `docs/debt/INC-CYCLE-14-APPLY-PUSH-VIOLATION.md` — cycle-14 precedent.
- `docs/debt/INC-DEBT-006-apply-push-discipline-cycle-16-violation.md` — cycle-16 precedent with forward debt.

> Filled by `sddk-archive` (cycle-8+); consumed by `sddk-debt-verify` for cross-cycle correlation via fingerprint.
