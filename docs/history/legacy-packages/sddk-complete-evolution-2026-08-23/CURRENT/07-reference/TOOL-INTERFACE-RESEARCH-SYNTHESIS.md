# Tool / CLI Interface Research Synthesis

## Purpose

Extract patterns that improve the LLM ↔ deterministic Rust boundary without importing unrelated product models.

## Maven — lifecycle / goal ergonomics

Useful idea:

```text
ask for a lifecycle outcome
instead of manually enumerating every prior prerequisite step
```

SDDK translation:

```text
cycle.verified
cycle.closed
```

Not adopted:

- Maven POM/domain model;
- fixed software build lifecycle as SDDK kernel ontology.

Reference:
https://maven.apache.org/guides/introduction/introduction-to-the-lifecycle.html

## Gradle — task graph and work avoidance

Useful:

- dependency DAG;
- declared inputs/outputs;
- independent execution;
- `UP-TO-DATE` style work avoidance.

SDDK translation:

```text
OperationContract
GoalPlan DAG
OperationFingerprint
UP_TO_DATE
```

Not adopted:

- Gradle task model/configuration language as-is.

Reference:
https://docs.gradle.org/current/userguide/build_lifecycle.html

## Controller / reconciliation pattern

Useful:

```text
actual state
vs
desired state
→ reconcile
```

SDDK translation:

```text
DecisionSnapshot
+ Goal
→ GoalRun
```

This is especially appropriate for retries/idempotency.

Reference:
https://kubernetes.io/docs/concepts/architecture/controller/

## Agent tool design

General useful patterns observed across modern tool-calling systems:

- fewer overlapping tools;
- semantic names;
- precise use/avoid descriptions;
- typed inputs/outputs;
- retry/effect semantics;
- examples for non-obvious calls;
- dynamic tool selection.

SDDK translation:

```text
state
goal.plan
goal.apply
query
evidence.submit
```

## Schema/tool graph planning

Recent research explores representing tool input/output compatibility as a graph/hypergraph and planning from missing requirements.

SDDK fit:

```text
OperationContract.requires
OperationContract.produces
Goal.unsatisfied_obligations
```

Decision:

- defer advanced search;
- deterministic dependency planner first;
- future planners implement the same contract.

## Process mining

Useful idea:

Use actual agent/tool traces to identify:

- redundant sequences;
- interface confusion;
- repeated reads;
- poor granularity.

SDDK has a natural source: Event Ledger.

Decision:

- adopt telemetry and deterministic trajectory projection;
- use GCI for candidate interface redesign;
- no automatic macro generation.

## Tool-call restraint

Research on when-not-to-call tools reinforces a simple SDDK principle:

```text
if fresh information is already in ContextCapsule,
do not make the model rediscover it through another tool.
```

## Tool-schema compilation

Compact/generated model-facing tool definitions can reduce context and schema-use errors.

SDDK fit:

```text
canonical operation/capability contract
→ Rust
→ JSON Schema
→ CLI help
→ agent tool descriptor
```

## Final adoption matrix

| Idea | Decision |
|---|---|
| semantic goals | adopt |
| actual→desired reconciliation | adopt |
| declared operation dependencies | adopt |
| work avoidance | adopt conditionally |
| small agent tool surface | adopt |
| detailed report preservation | mandatory |
| tool telemetry | adopt |
| process mining | later |
| schema-hypergraph planner | research |
| generated arbitrary shell planner | reject |
| expose every CLI command as a tool | reject |
