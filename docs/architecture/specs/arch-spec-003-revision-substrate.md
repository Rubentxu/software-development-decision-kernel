---
id: arch-spec-003-revision-substrate
package_local_id: SPEC-003
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-003-REVISION-SUBSTRATE.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# arch-spec-003 — REVISION-SUBSTRATE

> **Mirror of package SPEC `SPEC-003`.** Repository-native identifier is `arch-spec-003-revision-substrate` (range arch-spec-001..018). Per SUPERSESSION.md, the packages original SPEC ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `SPEC-003` |
| Repository native | `arch-spec-003-revision-substrate` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-003-REVISION-SUBSTRATE.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

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
