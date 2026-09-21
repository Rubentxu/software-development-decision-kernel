# SPEC-045 — Adaptive Systems Review Workflow

**Status:** Proposed

## Goal

Provide deep engineering reasoning when risk/signals justify it without forcing systems-programming checks on every project.

## Entry

```text
systems.review
```

## Compilation

Use existing workflow algebra:

```text
Task(profile.resolve)
→ Task(review.plan)
→ Choice(signals/risk)
→ Parallel/Map(selected dimensions)
→ Join
→ Task(evidence.validate)
→ Gate(adjudication)
```

No new kernel primitive.

## Dimension activation

| Signal | Review dimension |
|---|---|
| architecture boundary change | architecture |
| async/channels/locks | concurrency/liveness |
| parser/binary/FFI/unsafe | representation |
| latency/throughput goal | performance |
| critical state transition | invariants + stronger verification |
| none | minimal architecture/invariant pass |

## Reviewer procedure

1. establish constraints;
2. identify invariants;
3. map trust and architecture boundaries;
4. trace representative operation;
5. account for resource liveness/blocking;
6. inspect concurrency/backpressure where relevant;
7. inspect copies/layout only on relevant paths;
8. choose proportional verification;
9. emit normalized findings/evidence gaps.

## Heuristics

- keep important core paths easy to reason about;
- validate at trust boundaries;
- make invalid states unrepresentable where practical;
- isolate incidental platform/runtime concerns;
- add execution-model complexity only for demonstrated need;
- distinguish fixed native layouts from variable representations;
- treat compiler/type guarantees as evidence;
- escalate tests → property/fuzz/formal according to consequence.

These are heuristics, not mandatory architecture.
