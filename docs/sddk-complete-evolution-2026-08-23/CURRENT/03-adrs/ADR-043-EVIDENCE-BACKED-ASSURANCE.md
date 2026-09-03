# ADR-043 — Engineering Assurance is evidence-backed and deterministically adjudicated

**Status:** Proposed

## Context

Probabilistic reviewers can discover important problems but should not unilaterally own gate verdicts.

## Decision

Separate:

```text
Observation
→ Evidence
→ Finding / Obligation
→ Deterministic Adjudication
→ PASS | PASS_WITH_WARNINGS | FAIL | INCONCLUSIVE
```

Evidence may include source+revision, compiler/static analysis, tests, benchmarks, property/fuzz/model checks, graph queries, human review receipts and existing SDDK receipts.

Agent prose alone cannot support a blocking finding.

Missing or stale required evidence yields `INCONCLUSIVE`, not PASS.

Material findings contain stable fingerprint, dimension/rule, severity, scope, evidence refs, violated obligation when applicable and remediation/disposition.

Replay reconstructs assessments; it does not rerun analyzers or agents.

## Consequence

Assurance becomes gate-compatible, auditable and queryable by future `sddk why` views.
