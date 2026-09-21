# SPEC-034 — Static/Runtime Alignment

Maintain separate planes:

```text
Declared architecture
       vs
Static observed architecture
       vs
Runtime observed architecture
```

Runtime evidence may reveal dynamic edges, races, temporal coupling and hot paths not visible statically.

`BehaviorFingerprint` stores scenario-level digests and EvidenceRef/session ref, not a duplicate of the entire provider trace.

Missing runtime provider -> `NOT_EVALUATED`.
