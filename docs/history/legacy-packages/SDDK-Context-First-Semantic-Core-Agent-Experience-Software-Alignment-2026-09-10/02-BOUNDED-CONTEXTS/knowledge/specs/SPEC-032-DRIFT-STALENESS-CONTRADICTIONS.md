# SPEC-032 — Drift, Staleness and Contradictions

Drift dimensions:

`SOURCE | STRUCTURAL | DEPENDENCY | ARCHITECTURE | SEMANTIC | BEHAVIORAL | KNOWLEDGE | DECISION | CONTEXT`.

Staleness causes include source/dependency/analyzer/policy/decision/tradeoff/runtime/child changes.

Contradiction example:

```text
DECLARED: domain must not import storage
OBSERVED: domain -> storage
PROJECTED: ALIGNED
```

Action: stale projection, create contradiction finding, link proof, reevaluate assessment. Never silently choose which side is correct.
