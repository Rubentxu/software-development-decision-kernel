---
name: systems-reasoning
description: "Trigger: systems review, invariants, boundaries, resources, concurrency, performance. Apply evidence-oriented systems engineering reasoning."
license: MIT
metadata:
  author: "Rubentxu"
  version: "1.0"
---

## Activation Contract

Use when invariants, architecture boundaries, resource liveness, concurrency, hot paths, failure modes or verification materially affect correctness.

## Hard Rules

- Establish constraints before recommending mechanisms.
- Validate material claims against evidence.
- Treat core/adapters and sync/async placement as decisions, not dogma.
- Activate memory-layout, zero-copy or formal checks only when relevant.
- Emit normalized findings with evidence; never verdict-only prose.

## Decision Gates

| Signal | Action |
|---|---|
| architecture change | trace boundaries/dependencies |
| async/concurrency | trace waits, ownership, queues, blocking |
| binary/FFI/memory | inspect representation invariants |
| performance goal | define workload + measurable budget |
| high-consequence invariant | strengthen verification |

## Execution Steps

1. Identify constraints and invariants.
2. Map trust/runtime boundaries.
3. Trace a representative operation.
4. Account for resources, failure and concurrency.
5. Select proportional verification.
6. Return findings, evidence obligations and unknowns.

## Output Contract

Return constraints, invariants, boundary/control-flow model, findings with evidence refs, verification obligations and recommended verdict.

## References

- `references/principles.md`
