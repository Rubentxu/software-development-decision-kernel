---
id: ADR-0097-COMMON-REVISION-SUBSTRATE
package_local_id: ADR-004
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-004-COMMON-REVISION-SUBSTRATE.md
status: accepted
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
accepted_at: 2026-09-12
accepted_by_cycle: p-63676b11dc0ef88f/revision-substrate
implementation_evidence:
  - "crates/sddk-engine/src/revision_substrate.rs:71 — `pub struct Revision<T>` envelope with `oid: Oid`, `parents: Vec<Oid>`, `payload_ref: CasRef`, `provenance: ProvenanceV1`, `metadata: BTreeMap<String, String>` per ADR-0097 §Decision"
  - "crates/sddk-engine/src/revision_substrate.rs:33 — `pub struct Oid(pub String)` content identity (`sha256:<64hex>` of canonical JSON payload)"
  - "crates/sddk-engine/src/revision_substrate.rs:166 — `pub struct RefStore` with `cas(namespace, name, expected_old?, new_oid)` fail-closed compare-and-swap"
  - "crates/sddk-engine/src/revision_substrate.rs:155 — `RefUpdate::Stale { actual }` returned on stale writes; ref is NOT mutated (pin-tested by `ref_store_cas_fails_closed_on_stale_expected`)"
  - "crates/sddk-engine/src/revision_substrate.rs:191 — reserved prefix (`_`) on namespace/name is rejected with `RefError::Reserved`"
  - "crates/sddk-engine/src/lib.rs:25 — `pub mod revision_substrate` wired into the engine library"
  - "11 unit tests in revision_substrate::tests cover: deterministic Oid, parent-dependent child Oid, root/child/metadata constructors, CAS unconditional/expected-match/stale-fail-closed/reserved/multiple-namespaces"
supersedes_history_note: "Existing specialised revision types (`GraphRevision` u64-only, `PlanRevisionV1`, `ExecutionGraphRevision`) remain authoritative in their domains. The `Revision<T>` envelope is additive per AGENTS §2.10 (strangler migration); future cross-domain work (cycle 7+) adopts `Revision<T>` without forking. Domain layers continue to provide semantic diff/merge rules."
superseded_by: []
related_adrs:
  - "ADR-0001-ADR-PROMOTION-PROCESS"
  - "ADR-0094-ONE-CANONICAL-FACT-LOG"
stale_after: 2027-09-12

---

# ADR-0097 — COMMON-REVISION-SUBSTRATE

> **Mirror of package ADR `ADR-004`.** Repository-native numbering is `ADR-0097-COMMON-REVISION-SUBSTRATE` (range ADR-0094..0110). Per SUPERSESSION.md, the package's original ADR ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Promotion (2026-09-12 review)

Promoted from `proposed` to `accepted` by cycle `p-63676b11dc0ef88f/revision-substrate` (v1.168.36). The construction gap documented in the previous `deferred_until` and `deferred_reason` frontmatter is now closed:

- `Revision<T>` generic envelope shipped in `crates/sddk-engine/src/revision_substrate.rs` with the exact field shape ADR-0097 specifies (`oid`, `parents[]`, `payload_ref`, `provenance`, `metadata`).
- `Ref = { namespace, name, expected_old?, new_oid }` CAS primitive shipped as `RefStore::cas(...)` with fail-closed semantics on stale writes.
- Deterministic `Oid` derived from canonical JSON serialisation of the payload.
- 11 unit tests cover the construction: deterministic Oid, parent-dependent child Oid, root/child/metadata constructors, CAS unconditional/expected-match/stale-fail-closed/reserved/multiple-namespaces.

The 3 existing specialised revision types (`GraphRevision` u64-only, `PlanRevisionV1`, `ExecutionGraphRevision`) are NOT migrated in this cycle — AGENTS §2.10 (strangler migration) explicitly defers their conversion until domain-specific work adopts `Revision<T>`. The substrate is additive: it provides the cross-domain primitive for future cycles (e.g. cross-domain lineage between plan revisions and execution graph revisions) without breaking the existing specialised types.

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
