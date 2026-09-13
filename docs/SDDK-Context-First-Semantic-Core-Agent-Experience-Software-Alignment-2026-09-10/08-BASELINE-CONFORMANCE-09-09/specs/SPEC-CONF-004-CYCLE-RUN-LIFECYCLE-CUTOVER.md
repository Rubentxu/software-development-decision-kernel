# SPEC-CONF-004 — Cycle / Run Lifecycle Cutover

## Baseline contract

Closes ADR-003, SPEC-002, M2 and M9 runtime-state retirement.

## Semantic rule

`Cycle` is the high-level change/delivery container. `Run` owns execution-time churn.

```text
Goal -> WorkItem -> WorkflowDefinition -> ExecutablePlan -> Run
                       \___________________________/
                                   |
                              execution truth

Cycle = high-level delivery grouping/summary, not a retry/wait machine
```

## Deprecated Cycle runtime meanings

The following MUST cease to be canonical Cycle states:

- `UatWaiting`;
- `ApprovalPending`;
- `Recovering`;
- `Remediating` unless an ADR proves a truly delivery-level meaning distinct from Run/WorkItem remediation.

## Requirements

- approval waits are Run/Authority facts;
- UAT waits/results are Run/gate/evidence facts;
- retry/recovery are Run execution facts;
- Cycle status is derived from durable high-level lifecycle plus child Run summaries;
- one Cycle may contain Run A failed, Run B approval-waiting and Run C passed without contradictory Cycle truth;
- legacy Cycle views MAY render compatibility labels but derive them from Run facts;
- new code cannot pattern-match deprecated runtime Cycle variants outside compatibility modules.

## Migration

1. identify all writers/readers of runtime-specific Cycle variants;
2. introduce derived summary mapping from Run/Authority facts;
3. stop new writes;
4. migrate fixtures/persistence decoders;
5. delete or make variants decode-only;
6. remove variants after compatibility window if wire/schema constraints permit.

## Acceptance

- UAT-03 passes with three heterogeneous Runs under one Cycle;
- UAT-04 approval timing remains correct;
- recovery/retry scenarios update Run, not Cycle authority;
- architecture scan fails on new runtime Cycle writes;
- legacy UI/CLI summary parity fixture remains green.
