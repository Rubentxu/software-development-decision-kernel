---
id: ADR-0097-COMMON-REVISION-SUBSTRATE
package_local_id: ADR-004
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-004-COMMON-REVISION-SUBSTRATE.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
deferred_until: "Generic Revision<T> type and CAS-Ref abstraction implemented as cross-domain substrate"
deferred_reason: |
  CAS primitive (crates/sddk-storage/src/cas_object_store.rs) exists. However, the spec calls for a common Revision<T> = { oid, parents[], payload_ref, provenance, metadata } with Ref = { namespace, name, expected_old?, new_oid } and compare-and-swap updates, implemented as a cross-domain substrate. The shipped revisions are domain-specialized: GraphRevision is u64-only (crates/sddk-engine/src/semantic_graph.rs), PlanRevisionV1 (crates/sddk-domain/src/plan_revision.rs:298) and ExecutionGraphRevision (crates/sddk-domain/src/graph.rs:1241) each carry their own lineage without sharing a generic envelope. Promotion blocked on construction, not on audit. Per ADR-0001 §3.2 criterion 1.

---

# ADR-0097 — COMMON-REVISION-SUBSTRATE

> **Mirror of package ADR `ADR-004`.** Repository-native numbering is `ADR-0097-COMMON-REVISION-SUBSTRATE` (range ADR-0094..0110). Per SUPERSESSION.md, the package's original ADR ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `ADR-004` |
| Repository native | `ADR-0097-COMMON-REVISION-SUBSTRATE` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-004-COMMON-REVISION-SUBSTRATE.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# ADR-004 — Common content-addressed revision substrate

**Status:** Proposed

## Context

PlanRevision, fork/replay and Decision Memory all need parentage, content identity, refs and diff semantics.

## Decision

Introduce a generic immutable substrate:

```text
ObjectId = sha256(canonical object envelope)
Revision<T> = { oid, parents[], payload_ref, provenance, metadata }
Ref = { namespace, name, expected_old?, new_oid }
```

Refs use compare-and-swap updates. Reflog is an append-only fact/projection depending on implementation. Structured objects use canonical serialization with schema version.

Domain layers add semantic diff/merge rules; the substrate remains domain-agnostic.

## Non-goal

Do not reimplement Git transport, packfiles, index/staging area or filesystem checkout semantics.
