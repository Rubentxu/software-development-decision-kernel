---
id: INC-DEBT-018
title: "SPEC-SUPERSEDE-001 §2 ledger-event count drift: impl N+3 vs spec N+2"
status: open
severity: medium
priority: P2
fingerprint: "9b3e7f1a4d2c5e8b0a6f3d1c2e9b4f7a5d3c8b1e2f6a4d9c3b7e1f2a8c5d4b9e"
fingerprint_aliases: []
cluster_id: CL-CC-02
created: 2026-09-02
created_by: sddk-archive
owner: unassigned
---

# INC-DEBT-018 — SPEC-SUPERSEDE-001 §2 ledger-event count drift: impl N+3 vs spec N+2

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

`Engine::cycle_supersede` at line 204 calls `update_cycle_with_event(..., release_lease_on_phase_change=true)` (GAP-BUG-1 fix materialization). This emits a third ledger event (`lease.released`) in addition to the two user-visible events (`cycle.supersede.requested` and `cycle.supersede.applied`). The implementation now appends N+3 events; SPEC-SUPERSEDE-001 §2 still documents N+2. The verify-report flagged this as S-LEDGER-INVARIANT PARTIAL with owner:debt-verify. Attribution is pre_existing from a documentation-drift perspective: the spec was written before the lease-release atomicity fix was required.

## Rationale

Severity: medium — spec/impl contract drift; workaround exists (the test documents the actual N+3 behavior, so implementation is self-consistent). Priority: P2 — Fix in a later cycle (cycle-52).
Cluster: CL-CC-02 (coupling cluster, spec-impl drift).

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-09-02 | sddk-archive | created | FIND-000002 from debt-report cycle-51 |

## References

- `crates/sddk-engine/src/cycle_supersede.rs:204-207` (release_lease_on_phase_change=true)
- `docs/sddk-decision-kernel-architecture/04-specs/SPEC-SUPERSEDE-001.md` §2 (still reads N+2)
- `crates/sddk-engine/tests/cycle_supersede.rs` — `supersede_preserves_ledger_event_digests` asserts N+3
- debt-report: `FIND-000002`, fingerprint `9b3e7f1a...`

## Remediation (cycle-52)

1. Update SPEC-SUPERSEDE-001 §2 ledger appendix to document the 3-event invariant (`requested → lease.released → applied`)
2. Update AGENTS.md §9 cycle supersede workflow to record the same
3. Add a contract test that fails if a future change reintroduces only 2 events
