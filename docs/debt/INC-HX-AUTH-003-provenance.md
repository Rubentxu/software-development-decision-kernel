---
id: INC-HX-AUTH-003
title: "LedgerEvent / GateReceipt / JournalEntry / EventContext lose actor_kind at engine boundary"
status: closed
severity: high
priority: P1
fingerprint: "hx-auth-003-provenance-loss-4types"
fingerprint_aliases: []
cluster_id: CL-HX-PR-003
created: 2026-09-04
created_by: orchestrator
owner: EVT-LEDGER-001
resolved_at: 2026-09-04
resolved_by: sddk-apply (EVT-LEDGER-001 cycle)
resolved_commits:
  - "0a18982 actor_ref widening (additive) for LedgerEvent, GateReceipt, JournalEntry, EventContext"
  - "c63167d EVT-LEDGER-001 closure commit"
resolution_note: |
  Closed at v1.168.48 (EVT-LEDGER-001 closure): ActorRef gained the sixth
  canonical field `role: Option<String>` (additive, `#[serde(default)]`
  keeps the pre-widening corpus deserializable). All ActorRef literal
  construction sites across domain/engine/storage/cli were updated; the
  baseline pin test flipped from `actor_ref_carries_five_required_fields`
  to `actor_ref_carries_six_required_fields` plus a legacy-deserialization
  regression test. This unblocks the ADR-0073 `role=secretary` runtime
  binding (paths 6-7 of INC-HX-AUTH-004). Reconciled during v1.168.8 INC
  hygiene to add explicit `resolved_at`/`resolved_by` frontmatter.
last_updated: 2026-09-12
---

# INC-HX-AUTH-003 — LedgerEvent / GateReceipt / JournalEntry / EventContext lose actor_kind at engine boundary

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

Four event/carrier types carry `actor: String` instead of `ActorRef` at the engine boundary, losing the `actor_kind` field that `EventEnvelopeV1.actor` preserves. The affected types are:

- `LedgerEvent.actor: String` (`models/ledger.rs:23-38`)
- `GateReceipt.actor: String` (`models/gate_receipt.rs:108-123`)
- `JournalEntry` has no actor fields (`projections/journal.rs:13-32`)
- `EventContext.actor: String` (`engine/lib.rs:434-445, 1442-1463`)

ADR-069 §5 declares the canonical ActorRef 5-field contract (`kind`, `id`, `definition_hash`, `policy_hash`, `model`); widening deferred to `EVT-LEDGER-001`.

## Rationale

This is severity **high** because provenance — the ability to trace who performed an action — is lost for these four carriers. Priority **P1** because it requires schema migration design (EVT-LEDGER-001, order 90, H0) which should be sequenced after ARCH-HEX-001's engine-boundary work.

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-09-04 | orchestrator | created | HX-AUTHORITY-001 cycle; ADR-069 §5; INC-HX-AUTH-003 |
| 2026-09-04 | sddk-apply | resolved: actor_ref widened (additive) in LedgerEvent, GateReceipt, JournalEntry, EventContext | commits 0a18982, c63167d (EVT-LEDGER-001 cycle) |
| 2026-09-12 | orchestrator | closed: EVT-LEDGER-001 role field added to ActorRef (6-field contract); baseline test flipped; legacy-deser pin added | this cycle's diff (roadmap-sweep) |

## References

- [ADR-069 §5](docs/sddk-decision-kernel-architecture/03-adrs/ADR-069-EXPLICIT-AUTHORITY-MATRIX.md#-decision-4--provenance-baseline) — provenance baseline
- [ledger.rs:23-38](crates/sddk-domain/src/models/ledger.rs:23-38) — LedgerEvent.actor
- [gate_receipt.rs:108-123](crates/sddk-domain/src/models/gate_receipt.rs:108-123) — GateReceipt.actor
- [journal.rs:13-32](crates/sddk-domain/src/projections/journal.rs:13-32) — JournalEntry
- [engine/lib.rs:434-445,1442-1463](crates/sddk-engine/src/lib.rs) — EventContext.actor
- EVT-LEDGER-001 (order 90, H0) — typed-actor event schema widening
