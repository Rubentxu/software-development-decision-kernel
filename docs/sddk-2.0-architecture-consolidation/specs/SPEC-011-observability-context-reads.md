# SPEC-011 — Observability and Context-Read Tracing

**Status:** Proposed

## 1. Goal

Explain agent/behavior decisions through inspectable inputs and causal metadata without relying on private chain-of-thought.

## 2. Execution trace

Each behavior/agent execution SHOULD expose:

- triggering event/pattern;
- actor/version hashes;
- frame/cycle/fork;
- objects/artifacts read;
- tools/capabilities requested;
- evidence consulted;
- outputs/proposals;
- terminal status;
- timings and budget usage.

## 3. Context reads

Optional `context.read` events SHOULD record a bounded ordered set of graph/artifact IDs accessed during an execution. These events are bookkeeping and MUST NOT themselves trigger reactive behaviors or consume workflow budget.

Recommended fields:

- behavior/agent execution ID;
- triggering event ID;
- ordered de-duplicated object IDs;
- exact count;
- truncation flag;
- read categories;
- optional content hashes.

## 4. Privacy and scale

Context tracing defaults MAY be off or sampled for high-volume paths. It must record IDs/hashes rather than secret content. Caps prevent unbounded trace size.

## 5. User-facing explanation

`why` surfaces SHOULD answer using provenance:

```text
Finding F-19 was produced by architecture-critic v7
triggered by commit C
using SPEC-019, ADR-021 and src/payment.rs@hash
with evidence E1/E2
```

This is sufficient causal transparency without storing model chain-of-thought.
