# `sddk-incident` Workflow

## Why this pack matters
Incident response is intentionally unlike SDD: it is event-driven, urgency-sensitive, often parallel, includes waits and governed side effects. It tests the generality of Workflow v2.

## Flow

```mermaid
flowchart TD
  DET[Incident detected] --> TRI[Triage]
  TRI --> SEV{Severity}
  SEV --> DIAG[Parallel diagnosis]
  SEV --> OBS[Collect observability]
  DIAG --> CONTAIN{Containment required?}
  CONTAIN -->|yes| APP[Approval / policy]
  APP --> ACT[Contain capability]
  CONTAIN -->|no| FIX[Remediation]
  ACT --> FIX
  OBS --> FIX
  FIX --> VERIFY[Verify recovery]
  VERIFY --> POST[Post-incident analysis]
```

## Reactive events
- monitoring alert can start workflow;
- deployment event can enrich context;
- provider outage can reroute diagnostic agent;
- human severity override changes priority/budget.

## Governed effects
Containment, rollback and production changes require capability scopes and receipts.
