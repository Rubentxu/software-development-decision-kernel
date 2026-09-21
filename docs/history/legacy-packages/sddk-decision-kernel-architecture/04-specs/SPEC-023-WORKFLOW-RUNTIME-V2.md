# SPEC-023 — Workflow Runtime v2

**Status:** Proposed — refined for dynamic execution graphs

## Goals
Generic durable runtime for static, adaptive and exploratory workflows across SDD, UAT, incidents, security, releases and future packs.

## Three-level model

### WorkflowTemplate
Compact declaration of intent/invariants/policies/capability allowlist.

### WorkflowIR
Validated executable representation.

### ExecutionGraph
Runtime graph versioned through `ExecutionGraphRevision`. It may expand while running.

## Control operators

```text
Task
Sequence
Parallel
Map
Join
Race
Choice
Loop
Gate
Wait
SubWorkflow
Compensate
```

## Runtime entities

```text
WorkflowRun
  template_ref / ir_hash / graph_revision
  inputs/outputs/correlation_id/budget

NodeRun
  logical node identity/state/dependencies/attempts

Attempt
  route/timestamps/outcome/usage/context capsule

ExecutionGraphRevision
  revision/id/parent/events/nodes/edges/digest
```

## Dynamic expansion
A running node may emit:

```yaml
expansion:
  parent_node: discover-affected
  reason: affected_components_discovered
  nodes:
    - work_unit: auth
      capability: code.implement
    - work_unit: api
      capability: code.implement
  join:
    policy: all
```

Runtime validates and emits graph mutation events before scheduling.

## State transitions
All transitions are command-validated and event-emitting.

## Durability/idempotency
Process crash after append cannot duplicate side effects. Side-effect operations require idempotency keys/receipts. Graph revision hash detects conflicting expansion.

## Retry semantics
Retry creates another Attempt in the same NodeRun. Dynamic rediscovery may create new WorkUnits/NodeRuns, but never to hide a retry.

## Wait/pause/resume
Human approvals/external events are semantic waits. State survives IDE/machine restarts.

## Loop/convergence safety
- max iterations;
- event/reaction depth;
- graph node/depth budget;
- wall-time/token/cost budget;
- no-progress signature detection.

## Compatibility
Legacy `CyclePath` is a compiler preset/hint, not runtime state.

## Acceptance criteria
- three unrelated packs execute unchanged on same runtime;
- dynamic Map can discover N work units after start;
- graph can be reconstructed exactly from ledger;
- kill/restart resumes without duplicate effect;
- parallel/join/loop/human wait tested without LLM;
- invalid expansion is rejected deterministically.
