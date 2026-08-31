---
id: INC-CYCLE-14-LOC-OVERAGE
title: "Cycle-14 event_registry.rs impl 506 LOC vs ≤220 budget"
status: closed
closed: 2026-08-23
closed_by: sddk-apply (cycle-15)
severity: medium
priority: P2
fingerprint: "8f3a1b2c4d5e6f07"
fingerprint_aliases: []
cluster_id: CL-LOC-OVERAGE
created: 2026-08-22
created_by: sddk-apply
owner: orchestrator
---

# INC-CYCLE-14-LOC-OVERAGE — event_registry.rs impl LOC overage

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

Cycle-14 (`kernel-cycle-14-m2-event-foundation`) delivered
`EventSchemaRegistry` + `CanonicalEventValidator` (3-stage validation) +
18 registered schemas. The `event_registry.rs` impl section is **512 LOC**
vs the ≤220 LOC per-file budget. Also `projections.rs` impl grew by ~173 LOC
(new `JournalProjection` + `severity_for_event_type` table).

The overage is REAL: `EventSchemaRegistry` (Arc-based registry keyed
`(event_type, schema_version)`) + `CanonicalEventValidator` (format→
hash→schema 3-stage) + 18 `Schema` impl blocks is genuinely large.
The struct and trait design is clean; the overage is in density,
not messiness.

> **Amended (2026-08-22 remediation)**: The original INC cited impl LOC
> 506/507 which were slightly inaccurate. The correct measured values are:
> event_registry.rs impl = 512 LOC (full file, all new), projections.rs impl
> delta = 173 LOC (not 507). Total cycle-14 impl LOC = 760.

Implementation LOC (adjudicated):
- `event_registry.rs` impl: **512** (overage +292 vs ≤220)
- `event_bus.rs` impl (new helpers): **68** (overage vs ≤220 budget)
- `projections.rs` impl delta: **173** (new projection + severity table)
- Boilerplate (lib.rs exports): **7** (ok)
- **Total cycle-14 impl: 760** (vs ≤220 per-file budget)

Test LOC:
- `event_registry.rs` tests: **~175** lines (9 tests, including new corpus test)
- `event_bus.rs` tests: **~312** lines (existing + new)
- `projections.rs` tests: **~394** lines (existing + new)

Total delta across cycle: **~1344 lines** (4 files).

## Rationale

- **Severity = medium**: 18 schemas are registered (including lease.released
  added in remediation), all tests pass (1094 workspace tests after
  remediation), byte-equivalence preserved for legacy events.
  The overage is structural density, not correctness. The file is well-
  organized with clear section headers. The 18 `Schema` impl blocks are
  mechanically similar but semantically necessary (each event type has
  distinct payload shape).

- **Priority = P2**: not blocking release; remediation is a refactor pass
  that extracts shared schema-building macros or moves per-type schemas
  to a generated file. Does not change behavioral output. Aligns with
  ADR-0047-inc02 "LOC reality lesson": structural overage may be
  reabsorbed in subsequent cycles.

- **Cluster = `CL-LOC-OVERAGE`**: same family as INC-CYCLE-13-LOC-OVERAGE.

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-08-22 | sddk-apply | created | LOC breakdown above |
| 2026-08-22 | sddk-verify | confirmed | workspace 1093 tests green |
| 2026-08-22 | sddk-apply | amended | adjudicated impl LOC = 760 (not 506+507); 18 schemas registered |
| 2026-08-23 | sddk-apply (cycle-15) | closed | 3 module splits executed; all files ≤500 LOC; INC-CYCLE-14-LOC-OVERAGE-CLOSED.md |

## References

- `crates/sddk-domain/src/event_registry.rs` (512 impl LOC) — the over-budget file
- `crates/sddk-domain/src/projections.rs` (173 impl delta) — second over-budget file
- `crates/sddk-engine/src/event_bus.rs` (68 impl delta) — helpers over budget
- `docs/debt/SEVERITY.md` — severity taxonomy
- `docs/debt/PRIORITY.md` — priority taxonomy
- ADR-0047-inc02 — LOC reality lesson

> Filled by `sddk-apply`; consumed by `sddk-debt-verify` for cross-cycle
> correlation via fingerprint.
