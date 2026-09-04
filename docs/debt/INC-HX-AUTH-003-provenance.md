---
id: INC-HX-AUTH-003
title: "LedgerEvent / GateReceipt / JournalEntry / EventContext lose actor_kind at engine boundary"
status: open
severity: high
priority: P1
fingerprint: "hx-auth-003-provenance-loss-4types"
fingerprint_aliases: []
cluster_id: CL-HX-PR-003
created: 2026-09-04
created_by: orchestrator
owner: EVT-LEDGER-001
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

## References

- [ADR-069 §5](docs/sddk-decision-kernel-architecture/03-adrs/ADR-069-EXPLICIT-AUTHORITY-MATRIX.md#-decision-4--provenance-baseline) — provenance baseline
- [ledger.rs:23-38](crates/sddk-domain/src/models/ledger.rs:23-38) — LedgerEvent.actor
- [gate_receipt.rs:108-123](crates/sddk-domain/src/models/gate_receipt.rs:108-123) — GateReceipt.actor
- [journal.rs:13-32](crates/sddk-domain/src/projections/journal.rs:13-32) — JournalEntry
- [engine/lib.rs:434-445,1442-1463](crates/sddk-engine/src/lib.rs) — EventContext.actor
- EVT-LEDGER-001 (order 90, H0) — typed-actor event schema widening
