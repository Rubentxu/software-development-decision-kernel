# SPIKE-004 — Event-derived Active Graph & Replay

## Question
Can the operational graph be rebuilt deterministically from the ledger and support useful causal queries?

## Fixture
Events representing:
- workflow creation;
- NodeRun/Attempts;
- context capsule;
- provider failure;
- circuit breaker;
- route change;
- evidence;
- completion.

## Queries

```text
why did Attempt #1 fail?
what was affected by provider A outage?
which evidence supports decision D?
what context was used by Attempt #2?
```

## Invariants
- graph state after replay equals graph state after live projection;
- every graph relationship has provenance;
- deleting/rebuilding graph changes no ledger events;
- behavior replay does not repeat side effects.

## Success criteria
The same fixture supports Journal, causal graph and `sddk why` without separate hand-maintained state.
