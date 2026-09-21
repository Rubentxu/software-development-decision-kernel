# SPEC-047 — Goal Registry, Planner & Convergent Execution

**Status:** Proposed

## Purpose

Allow agents to request semantic outcomes while preserving the complete deterministic behavior of SDDK.

## GoalDefinition

```yaml
goal:
  id: cycle.verified
  desired_invariants:
    - implementation.complete
    - functional.verification.passed
    - required.assurance.satisfied
    - debt.gate.satisfied
  required_reports:
    - verify-report
    - debt-report
  required_receipts: [...]
  allowed_effect_classes: [...]
  planning_policy: ...
```

Required reports/receipts are first-class goal obligations.

## Goal Registry

Sources:

- kernel/application goals;
- pack-declared goals subject to pack rules.

Validation:

- unique semantic ID;
- all invariants resolvable;
- report/evidence obligations resolvable;
- effects declared;
- no authority redefinition.

## DecisionSnapshot

A bounded deterministic projection used for planning.

It includes:

- project/workspace;
- repository;
- cycle/workflow;
- lease status;
- knowledge coverage;
- graph revision;
- evidence state;
- report completeness;
- blockers;
- available goals.

## OperationContract

```yaml
operation:
  id: testing.execute
  requires:
    invariants: [...]
    schemas: [...]
  produces:
    invariants: [...]
    schemas: [...]
    reports: [...]
    evidence: [...]
  effects:
    class: read_only|modifies|irreversible
  idempotency:
    mode: pure|cached|convergent|effect_once|compensatable
  retry: ...
  cache: ...
```

## Planner v1

Inputs:

```text
GoalDefinition
DecisionSnapshot
OperationRegistry
Policy
Budget
```

Algorithm:

1. compute unsatisfied invariants/report/evidence obligations;
2. find operations that can establish each missing obligation;
3. recursively resolve prerequisites;
4. reject unresolved/cyclic dependencies;
5. apply policy/effect constraints;
6. topologically order ready work;
7. preserve independent parallelism;
8. return typed blockers.

No LLM is needed for baseline planning.

## GoalPlan

```yaml
goal_plan:
  goal: cycle.verified
  snapshot_fingerprint: ...
  satisfied: [...]
  required_steps: [...]
  required_reports: [...]
  expected_receipts: [...]
  blockers: [...]
  estimated_effects: [...]
```

## GoalRun

```yaml
goal_run:
  id: gr-...
  goal: cycle.verified
  state: running|waiting|blocked|succeeded|failed|cancelled
  plan_revision: ...
  obligation_status:
    invariants: ...
    reports: ...
    evidence: ...
    receipts: ...
  completed_operations: [...]
  blockers: [...]
```

## Reconciliation loop

```text
refresh snapshot
→ goal complete?
→ validate/rebuild plan
→ execute ready operations
→ verify postconditions
→ persist detailed outputs
→ append events
→ refresh
```

## Cognitive operations

When a semantic capability is required:

```yaml
status: waiting
request:
  capability: engineering.assess
  context_capsule_ref: ...
  input_schema: ...
  output_schema: ...
resume_token: ...
```

The typed result resumes the same GoalRun.

## Idempotency

Before retrying an effectful operation, runtime checks the declared postcondition.

Reconciliation must prefer:

```text
already satisfied
```

over blindly repeating the effect.

## Work avoidance

Safe reusable operation output:

```text
fingerprint(
  operation_version,
  declared_inputs,
  git_revision,
  graph_revision,
  ledger_head,
  policy_hash,
  profile/tool versions
)
```

If fresh and valid:

```text
UP_TO_DATE
```

Detailed reuse evidence is still recorded.

## Plan revision

Replan if state/evidence/policy changes or dynamic workflow work is discovered.

Every material revision emits an event.

## CLI adapter examples

```bash
sddk state --format json

sddk goal plan cycle.verified --cycle CYCLE --format json

sddk goal apply cycle.verified --cycle CYCLE --format json
```

## Backward compatibility

Existing low-level commands remain available.

High-level execution calls application services directly; it must not recursively shell out to `sddk`.

## Completeness

A GoalRun cannot return `succeeded` while a mandatory report/evidence/receipt obligation remains missing.

See SPEC-049.
