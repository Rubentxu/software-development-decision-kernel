# ADR-0055 — P3 forward-debt closure (CountingSemaphore retained)

**Status:** accepted (closure note)
**Date:** 2026-08-24
**Cycle:** 21 (A-min, scope-revised)
**Supersedes:** HANDOFF-2026-08-25 §P3 forward-debt (medium severity)
**Trigger:** Discovery during cycle-21 that parking_lot::Semaphore does not exist

---

## Context

cycle-19 forward-debt P3 said: "True semaphore (not spin-wait) for backpressure — consider
adding `parking_lot` or using a proper OS semaphore". cycle-19 WU-10 (commit `32e65de`)
delivered `CountingSemaphore` (std `Mutex<usize>` + `Condvar`) — a TRUE semaphore that
fixed the original race ("multiple may acquire" when competing for 1 permit).

cycle-21 attempted to optimize this with `parking_lot::Semaphore`. Apply phase
discovered that **parking_lot 0.12.5 does not export a `Semaphore` type**. The crate
only provides: `Mutex`, `RwLock`, `Condvar`, `Once`, `ReentrantMutex`. `parking_lot_core`
(low-level API for building custom primitives) is also not a Semaphore substitute.

`std::sync::Semaphore` is stabilized in Rust 1.101+, but the workspace MSRV is 1.91.

## Decision

**P3 is functionally closed.** `CountingSemaphore` IS a true semaphore — it uses
`std::sync::Mutex<usize>` + `std::sync::Condvar` for correct acquire/release semantics
with zero spin-wait. The forward-debt's race condition is gone.

The "lightweight" optimization with `parking_lot::Semaphore` was based on a
documentation error — that type does not exist. No replacement is needed because:

1. `CountingSemaphore` is correct (zero race)
2. `parking_lot` does not provide `Semaphore`
3. `std::sync::Semaphore` requires Rust 1.101+ (workspace is 1.91)
4. `tokio::sync::Semaphore` requires tokio runtime (cycle-19 INV: stdlib-only for engine)

INV-10 §Permitted Exceptions is simplified: the `Mutex<usize>` permit-counter
exception is removed (since the mutex is inside CountingSemaphore, not directly on
workflow state — reclassified as "backpressure primitive" not "workflow state lock").

## Consequences

### Positive
- P3 closure documented (no orphan debt)
- INV-10 grep gate scope clarified
- Future cycles can use `std::sync::Semaphore` if MSRV is bumped to ≥1.101

### Negative
- None (CountingSemaphore retained; behavior unchanged)

### Out of Scope (deferred to cycle-22+)
- Tick() extraction (398 LOC)
- Arc::try_unwrap silent fallback fix
- GraphStoreBox dedup
- OperatorContext construction dedup
- ParallelKey struct
- Dynamic workflow Map/Join/Race/Loop operators (Epic DW)

## References
- HANDOFF-2026-08-25 §P3 forward-debt
- commit `32e65de` (cycle-19 WU-10: CountingSemaphore intro)
- ADR-0050 §Backpressure Implementation
- parking_lot 0.12.5 docs: https://docs.rs/parking_lot/0.12.5/parking_lot/
- Rust 1.101 release notes (std::sync::Semaphore stabilization)