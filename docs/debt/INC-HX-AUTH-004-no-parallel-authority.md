---
id: INC-HX-AUTH-004
title: "no-parallel-authority invariant not enforced at runtime"
status: open
severity: high
priority: P1
fingerprint: "hx-auth-004-no-parallel-authority-7gap"
fingerprint_aliases: []
cluster_id: CL-HX-NPA-004
created: 2026-09-04
created_by: orchestrator
owner: ARCH-HEX-001
---

# INC-HX-AUTH-004 — no-parallel-authority invariant not enforced at runtime

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

ADR-069 §6 declares the no-parallel-authority invariant: exactly one canonical writer and one canonical approver per decision point. Today this invariant is violated by seven dual-writer paths:

| Decision point | Dual-writer gap |
|--------------|----------------|
| Approval decision | `sddk approval grant\|deny` (CLI; forced Human per `emit.rs:259`) AND direct `event_store.append(emit_approval_decision(...))` from any code path |
| Cycle transition | `sddk cycle transition` (CLI; lease fence + plan revalidation) AND direct `engine::apply_transition` from any code path |
| Cycle pause/resume | `sddk cycle pause` (CLI) AND direct engine path |
| Gate receipt | Engine path + manual CLI |
| Knowledge ingest | CLI + manual vault write |
| Secretary closed-set (ADR-0073 prose) | ADR-0073 prohibits `release.*` / `gate.*` / `lease.*` / `receipt.*` but no code enforcement |
| Secretary escalation | No runtime enforcement of escalation path |

## Rationale

This is severity **high** because it means the system's authority model is not enforced — any code path can bypass the CLI and emit events with a forged identity. Priority **P1** because fixes are distributed across ARCH-HEX-001 (approval/cycle paths), EVT-LEDGER-001 (ActorKind enum), and RX-SECRETARY-001/002 (Secretary closed-set).

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-09-04 | orchestrator | created | HX-AUTHORITY-001 cycle; ADR-069 §6; INC-HX-AUTH-004 |

## References

- [ADR-069 §6](docs/sddk-decision-kernel-architecture/03-adrs/ADR-069-EXPLICIT-AUTHORITY-MATRIX.md#-decision-5--no-parallel-authority-invariant) — no-parallel-authority invariant
- [emit.rs:259](crates/sddk-engine/src/event_bus/emit.rs:259) — forced Human
- [engine/lib.rs:1111](crates/sddk-engine/src/lib.rs:1111) — apply_transition dual-writer
- ARCH-HEX-001 (order 80, H0) — engine-side authority enforcement
- EVT-LEDGER-001 (order 90, H0) — ActorKind enum widening
- RX-SECRETARY-001/002 (order 300/310) — Secretary runtime admission predicate
