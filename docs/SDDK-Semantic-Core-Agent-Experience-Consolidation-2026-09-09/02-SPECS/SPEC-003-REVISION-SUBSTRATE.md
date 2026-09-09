# SPEC-003 — Content-addressed revision substrate

## Object envelope

```text
ObjectEnvelope {
  kind
  schema_version
  payload
}
OID = sha256(canonical_bytes(ObjectEnvelope))
```

## Revision

```text
Revision {
  revision_oid
  parents[]
  payload_ref
  provenance_ref
  metadata
}
```

## Requirements

- RV-001 deterministic IDs for identical canonical content;
- RV-002 0..N parents (root/linear/merge);
- RV-003 atomic compare-and-swap ref update;
- RV-004 reflog/audit of ref movement;
- RV-005 reachability traversal and merge-base primitive;
- RV-006 domain-specific semantic diff is layered above raw object diff;
- RV-007 no Git checkout/index/network protocol in core.

## Consumers

PlanRevision, DecisionMemory and experiment/fork metadata reuse the substrate.
