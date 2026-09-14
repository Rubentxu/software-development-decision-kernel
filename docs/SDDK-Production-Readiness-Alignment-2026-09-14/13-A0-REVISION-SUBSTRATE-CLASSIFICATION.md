# A0 — Revision Substrate / OID / Ref / CAS Classification (PR-GAP-004)

**Audit basis:** `main@f690f4f` (after Slice A0-1).
**Requirement:** classify every OID/ref/CAS/ancestry primitive as `shared`, `decision-semantic`, `compatibility` or `redundant`; semantically identical generic invariants have one implementation; narrow differences require an ADR.

## Primitive classification

| Primitive | Location | Class | Rationale | Consumers |
|---|---|---|---|---|
| `Oid(pub String)` (`sha256:<64hex>` over canonical JSON) | `sddk-engine/src/revision_substrate.rs` | **shared** | Generic cross-domain content identity; canonical JSON keeps serialization deterministic. | none yet (additive substrate) |
| `Revision<T>` (`oid`, `parents[]`, `payload_ref`, `provenance`, `metadata`) | `sddk-engine/src/revision_substrate.rs` | **shared** | Generic immutable revision envelope for cross-domain lineage (plan ↔ execution graph). | none yet |
| `Ref { namespace, name, current }`, `RefUpdate`, `RefStore::cas` | `sddk-engine/src/revision_substrate.rs` | **shared** | The single generic fail-closed compare-and-swap invariant. | none yet (tested) |
| `CasRef(pub String)` (+ `digest()`) | `sddk-engine/src/canonical_event_log.rs` | **shared** | Content-addressed handle reused by `Revision::payload_ref`; already the canonical-event-log ref. | canonical event log, revision substrate |
| `FilesystemCas` | `sddk-storage/src/cas.rs` | **shared (different responsibility)** | Bytes-on-disk CAS root; a persistence mechanism, not a revision identity invariant. | storage |
| `MemoryId` (`[u8;32]` sha256 over `canonical_payload_bytes`) + `id_hex` | `sddk-engine/src/decision_memory.rs` | **decision-semantic** | Typed durable *curated decision history* identity; binary digest with its own canonicalization. | Decision Memory store, CLI memory commands |
| `Reflog`, `ReflogEntry`, `RefMovement` | `sddk-engine/src/decision_memory.rs` | **decision-semantic** | Append-only decision ref history (`refs/heads/*` semantics) with reset/tombstone/dropped tracking; not a generic ref store. | Decision Memory |
| `DecisionMemoryCommit` DAG (`merge`, ancestry) | `sddk-engine/src/decision_memory.rs` | **decision-semantic** | Commit graph with domain merge rules over decision memory. | Decision Memory |
| `PlanRevisionV1` lineage (`ancestry()`, `parents`) | `sddk-domain/src/plan_revision.rs` | **decision-semantic / domain** | Plan-specific revision lineage with semantic mutation kinds. | planning |
| `GraphRevision` (u64), `ExecutionGraphRevision` | `sddk-domain` | **domain** | Domain counters/revisions for graph materialization. | graph/execution |
| `workflow_ir::RevisionId(pub String)` | `sddk-domain/src/workflow_ir.rs` | **domain** | Workflow IR revision label. | workflow IR |

## Conclusion

- **No semantically-identical duplication found.** The generic substrate (`Revision<T>`/`Oid`/`RefStore`) and Decision Memory's `MemoryId`/`Reflog`/commit DAG are **not** the same invariant:
  - canonicalization differs (canonical JSON vs `canonical_payload_bytes`),
  - identity shape differs (`String` `sha256:<hex>` vs `[u8;32]` binary),
  - Decision Memory adds decision-specific diff/merge/reset/tombstone semantics.
- **Narrow differences are already governed by an accepted ADR:** `docs/architecture/adrs/ADR-0097-COMMON-REVISION-SUBSTRATE.md` (status `accepted`). Its `supersedes_history_note` explicitly keeps the specialised types authoritative and makes `Revision<T>` additive per AGENTS §2.10 (strangler migration). No new ADR is required to justify the differences.
- **One implementation per invariant holds:** there is exactly one generic CAS invariant (`RefStore::cas`); Decision Memory and the domain revision types are distinct semantic layers on top, not competing generic CAS implementations. The cross-domain substrate has zero production consumers yet, which is the documented intent of ADR-0097.

## Evidence

- `PR-UAT-004` — two writers racing the same `expected_old` are decided by the one generic CAS invariant: `crates/sddk-engine/src/revision_substrate.rs::tests::ref_store_cas_race_has_single_winner` (exactly one `Updated`, one `Stale`, ref ends on the winner's Oid).
- Existing fail-closed evidence: `ref_store_cas_fails_closed_on_stale_expected`, `ref_store_rejects_reserved_namespace`/`_name`, `ref_store_multiple_namespaces_isolated`.
- ADR-0097 records the accepted narrow differences and the additive (strangler) stance.

## Ownership / disposition of the consumer-less substrate (PR-GAP-009 hand-off)

- **Owner:** ADR-0097 (accepted, `stale_after: 2027-09-12`).
- **Reason it survives `allow(dead_code)`-free:** it is public API, not flagged dead; it is an intentionally additive substrate.
- **Exit/revisit trigger:** first cross-domain consumer (e.g. lineage spanning plan revisions and execution graph revisions) OR the ADR-0097 `stale_after` review date, whichever comes first.
