---
id: INC-DEBT-019
title: "SystemTime::now() hidden coupling in Engine::cycle_supersede breaks determinism"
status: open
severity: medium
priority: P2
fingerprint: "4c7a1e2fb918ce3f4d8a92b1c0e3f7a25d3b9e4f1c8a27d0b5e6f1a3c8b4d1f2"
fingerprint_aliases: []
cluster_id: CL-CC-01
created: 2026-09-02
created_by: sddk-archive
owner: unassigned
---

# INC-DEBT-019 — SystemTime::now() hidden coupling in Engine::cycle_supersede breaks determinism

> Durable record for one debt finding across cycles. See ADR-0047 §3.2.

## Context

`Engine::cycle_supersede` dereferences wall-clock time via `std::time::SystemTime::now().duration_since(UNIX_EPOCH)` at line 86 to compute `now_ms` passed to `ledger.verify_cycle_lease`. This couples the lease-fence verification step to real wall-clock, breaking determinism for golden tests and making any replay/repro path non-hermetic. The coupling cluster's hidden-dependency catalog flags time-randomness inside business logic at default MEDIUM. Attribution is pre_existing: the call exists at base commit b4ea9450. The cycle-51 finalization diff does NOT add this call, but does transitively make it more load-bearing (GAP-BUG-1 fix now requires correct `now_ms` to release the lease atomically; before the fix, an incorrect `now_ms` only delayed failure, after the fix it could cause stale-lease gate failure modes).

## Rationale

Severity: medium — time coupling in production path; deterministic test reproduction blocked; workaround exists (tests can avoid asserting on now_ms-derived values). Priority: P2 — Fix in a later cycle (cycle-52 DRY pass).
Cluster: CL-CC-01 (coupling cluster, time-randomness).

## Lifecycle

| Date | Actor | Change | Evidence |
|------|-------|--------|----------|
| 2026-09-02 | sddk-archive | created | FIND-000001 from debt-report cycle-51 |

## References

- `crates/sddk-engine/src/cycle_supersede.rs:86-89` (SystemTime::now().duration_since(UNIX_EPOCH))
- `crates/sddk-cli/src/cycle.rs` (precedent: CLI already computes wall-clock time; can be passed as parameter)
- debt-report: `FIND-000001`, fingerprint `4c7a1e2f...`

## Remediation (cycle-52 DRY pass)

Inject a `Clock` trait (or take `now_ms: i64` as a parameter from the CLI composition root) so `Engine::cycle_supersede` does not dereference wall-clock. Pattern already exists in domain ports as `NoopTaskExecutor`. Tests must either avoid asserting on now_ms-derived values or freeze time via `tokio::time::pause`.
