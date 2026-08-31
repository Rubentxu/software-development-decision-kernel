---
id: INC-CYCLE-11-PYTEST-CONTRACT-P1
title: "Test contract gaps: 17 pre-existing failures in test_workflow_contract.py"
status: closed
severity: high
priority: P1
fingerprint: "cycle11-pytest-contract-p1-v1"
fingerprint_aliases: []
cluster_id: CL-CYCLE-11
created: 2026-08-22
created_by: sddk-apply
owner: sddk-team
resolved-by: cycle-12
resolved-date: 2026-08-22
---

# INC-CYCLE-11-PYTEST-CONTRACT-P1 — Test contract gaps: 17 pre-existing failures in test_workflow_contract.py

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

The test_workflow_contract.py regression suite has 17 pre-existing failures that are unrelated to the cycle-11 A-full coherence gate ordering changes. These failures represent gaps in the SDDK test contract that were present before the cycle-11 apply phase.

The failures fall into 5 regression clusters:

| Cluster | Regression | Count | Description |
|---------|-----------|-------|-------------|
| I | Propose/debt artifact store | 2 | sddk-propose.md and sddk-debt-verify.md missing `sddk artifact store` reference |
| J | Verify skill CLI contract | 8 | sddk-verify/SKILL.md missing path-scoped transition strings and gate outcome patterns |
| B | Transition artifact refs | 1 | Fewer than 15 transition artifact refs found (found 5) |
| C | Release authority contract | 5 | sddk-release.md, SKILL.md, release.md missing local release authority + after-verify patterns |
| D | Knowledge pipeline ordering | 1 | orchestrator.md missing explicit scan→verify→import ordering |

## Rationale

**Severity: high** — Multiple regression clusters indicate systematic gaps in the SDDK contract surface. The verify skill (J) and release authority (C) gaps directly impair the release and verification pipeline.

**Priority: P1** — Fix in the current or next cycle. These gaps block full contract compliance and should be resolved alongside the cycle-11 coherence ordering work.

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-08-22 | sddk-apply | created | cycle-11 apply phase, test_workflow_contract.py xfail registry |
| 2026-08-22 | sddk-spec | closed (cycle-12) | cycle-12 spec; REGRESSION I/J/B/C/D resolved; XFAIL emptied; threshold 15→5; allowlist precise 6 entries |

## Items (17 failures)

| # | Regression | Failure Message | Severity | Priority |
|---|-----------|----------------|----------|----------|
| 1 | I | sddk-debt-verify.md: missing sddk artifact store | medium | P1 |
| 2 | I | sddk-propose.md: missing sddk artifact store | medium | P1 |
| 3 | J | sddk-verify: missing status includes cycle | high | P1 |
| 4 | J | sddk-verify: missing A-full transition | high | P1 |
| 5 | J | sddk-verify: missing A-min transition | high | P1 |
| 6 | J | sddk-verify: missing A-lite transition | high | P1 |
| 7 | J | sddk-verify: missing B-direct transition | high | P1 |
| 8 | J | sddk-verify: missing failed gate outcome | high | P1 |
| 9 | J | sddk-verify: missing failed transition state | high | P1 |
| 10 | J | sddk-verify: missing conditional lease flags | high | P1 |
| 11 | B | Expected >= 15 transition artifact refs, found 5 | medium | P1 |
| 12 | C | sddk-release.md: missing local release authority contract | high | P1 |
| 13 | C | sddk-release.md: missing positive after verify | high | P1 |
| 14 | C | SKILL.md: missing local release authority contract | high | P1 |
| 15 | C | SKILL.md: missing positive after verify | high | P1 |
| 16 | C | release.md: missing positive after verify | high | P1 |
| 17 | D | orchestrator.md: missing explicit scan→verify→import ordering | medium | P1 |

## References

- `tests/test_workflow_contract.py` — xfail registry at line ~35; REGRESSION R `KNOWN_MCW_WORKFLOW_DRIFT_ALLOWLIST` constant (cycle-11 remediation)
- `prompts/sddk/phases/apply.md` — apply phase context
- `prompts/sddk/phases/verify.md` — verify phase authority
- `prompts/sddk/workflows/sddk-a-full.yaml` — coherence gates with explicit `depends_on`
- `workflow/workflow.yaml` — simplified 10-phase workflow; divergences from MCW captured in `KNOWN_MCW_WORKFLOW_DRIFT_ALLOWLIST`

> Filled by `sddk-archive` (cycle-8+); consumed by `sddk-debt-verify` for cross-cycle correlation via fingerprint.
