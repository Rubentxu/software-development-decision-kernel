# `sddk-uat` Workflow

## High-level flow

```mermaid
flowchart LR
  P[Plan] --> A[Plan Approval]
  A --> E[Execute scenarios]
  E --> O[Oracle assessments]
  O --> R{Human review required?}
  R -->|yes| H[Human decision]
  R -->|no| S[Scenario acceptance]
  H --> S
  S --> D{Defects?}
  D -->|yes| F[Fix / Retest loop]
  F --> E
  D -->|no| G[Sign-off]
  G --> RD[Release decision]
```

## Key semantics
`PASSED` is not automatically `ACCEPTED`. Machine assessment and human decision remain separate.

## Event examples
- `uat.plan.created`
- `uat.plan.approved`
- `uat.scenario.started`
- `uat.evidence.recorded`
- `uat.oracle.assessed`
- `uat.human.review.requested`
- `uat.defect.opened`
- `uat.retest.completed`
- `uat.signoff.recorded`

## Change-driven retest
A change can emit `evidence.stale` and schedule only affected scenarios when policy allows.
