# SPEC-021 — Progressive Software Knowledge

## KnowledgeAssertion

```text
id
subject
predicate
value
epistemic_status
producer
basis
evidence_refs[]
valid_from
valid_until?
observed_at
supersedes[]
invalidation_dependencies[]
```

Epistemic states:

`OBSERVED | DECLARED | INFERRED | DECIDED | VERIFIED | CONTRADICTED | STALE | SUPERSEDED | UNKNOWN`.

## Reevaluation dispositions

`CONFIRM | MODIFY | RETRACT | SUPERSEDE | STALE | UNKNOWN | NEW`.

History is never overwritten. Current cards are projections over valid assertions.
