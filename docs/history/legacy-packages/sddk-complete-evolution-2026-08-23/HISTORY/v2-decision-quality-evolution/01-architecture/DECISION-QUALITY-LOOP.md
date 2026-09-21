# Decision Quality Loop

## Purpose

Connect Event Ledger, assurance and controlled improvement with one conceptual feedback cycle.

```text
1. DECIDE
   goal → workflow → capabilities → decisions

2. ACT
   agents/tools/humans execute through governed capabilities

3. VERIFY
   acceptance + assurance + policies + evidence

4. RECORD
   canonical events + receipts + artifacts

5. LEARN
   derive experience + detect repeated patterns

6. EXPERIMENT
   candidate variants through Workflow Laboratory

7. PROMOTE
   policy + evidence + rollout + rollback
```

This is not a mandatory phase sequence. It is an architectural feedback loop.

## Why it fits the project name

A Decision Kernel should preserve and improve:

```text
decision quality
decision evidence
decision traceability
decision reproducibility
decision adaptation
```

## Learning timescales

### T0 — execution-time adaptation
retry/failover, dynamic WorkUnits, run-scoped IR revision, context refresh.

### T1 — operational consolidation
repeated failures, stale skill/config, recurring human correction, route effectiveness.

### T2 — offline improvement
skill/prompt/context/workflow/verifier candidates.

### T3 — durable promotion
activate new versions through policy/approval + receipts.

This separation prevents one successful episode from mutating durable behavior immediately.

## Diversity without product bloat

Preserve alternative candidates when evidence is incomplete:

```text
Candidate A
├── Candidate B
├── Candidate C
└── Candidate D
```

Do not introduce scientific `Theory/Hypothesis` into core domain.

## Negative learning

Preserve what did not work:

```text
candidate rejected because...
route failed under...
workflow expanded unnecessarily when...
skill regressed on...
```

This negative knowledge can reduce repeated rediscovery.

## Causal caution

Operational correlation does not prove improvement. Prefer controlled forks:

```text
same goal / same base revision
          │
      fork point
      /        baseline   candidate
      \       /
        diff
```
