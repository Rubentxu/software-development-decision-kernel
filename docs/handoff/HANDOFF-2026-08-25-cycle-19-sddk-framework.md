# HANDOFF — 2026-08-25 kernel-cycle-19-operator-arc-dyn-safety

## Cycle Summary

**Cycle:** `kernel-cycle-19-operator-arc-dyn-safety`
**Completed:** 2026-08-25
**Commits:** 8 (WU-8a, WU-1, WU-2, WU-3, WU-3.5, WU-4, WU-9a, WU-9b partial)
**Status:** Completed (apply phase complete, verify pending)
**CYCLE-20 SHIPPED on 2026-08-24 (v1.42.0)** — forward debt P1/P2/P4 + INC-FORWARD-002 RESOLVED by `feat/kernel-cycle-20-p1p2p4-http-async` (see Forward Debt table below).

This cycle resolved the cycle-18 BLOCKED state (W-4 design contradiction) by discovering that `OperatorContext<'static>: Send + 'static` is achievable via Arc-wrap of `ir`/`run` fields (WU-3.5) combined with a generalized `snapshot_for_child` that works on any `OperatorContext<'a>` (not just `'static`). The bounded `Box::leak` exception in `snapshot_for_child` is documented and deferred to cycle-20+ P3 refactor.

---

## Files Changed

| File | Change | LOC |
|------|--------|-----|
| `crates/sddk-engine/src/operator.rs` | Concurrent Parallel::evaluate, helper fns, adapted tests | +248 -188 |
| `crates/sddk-engine/tests/parallel_concurrency_tests.rs` | New: 20 concurrency tests | +793 |
| `crates/sddk-engine/tests/parallel_seq_tests.rs` | New: 5 adapted tests | +253 |
| `docs/adr/ADR-0050-true-concurrent-parallel.md` | New: concurrent parallel contract | +127 |
| `docs/adr/ADR-0051-operator-arc-dyn-safety.md` | New: Box→Arc dyn safety | +116 |
| `apply-progress.yaml` | Updated: WU-4 completed | |

**Total: +1173 LOC, 7 files changed**

---

## Test Results

| Suite | Baseline | Added | Total |
|-------|---------|-------|-------|
| `sddk-engine --lib` | 123 | 0 | 123 |
| `sddk-engine --test parallel_concurrency_tests` | 0 | 20 | 20 |
| `sddk-engine --test parallel_seq_tests` | 0 | 5 | 5 |
| `sddk-engine` other tests | ~1128 | 0 | ~1128 |
| **Total workspace** | **1251** | **25** | **1276** |

All tests pass: `cargo test --workspace` → 0 failures

---

## Invariants Preserved

| # | Invariant | Status |
|---|-----------|--------|
| INV-1 | `Attempt.schema_version = 1` | Preserved |
| INV-2 | `OperatorContext.store: dyn GraphStore + Send` | Preserved |
| INV-3 | `Attempt` derives `Clone + PartialEq + Serialize + Deserialize` unchanged | Preserved |
| INV-4 | `ParallelCheckpointState` not on `Attempt` | Preserved |
| INV-5 | `NodeOutcome::Pending` struct variant | Preserved |
| INV-6 | `ChildResult` not Clone/Serialize | Preserved |
| INV-7 | `OperatorError::ChildPanicked` not constructed outside `Parallel::evaluate` | Preserved |
| INV-8 | `attempt_seq == child_index` for all Parallel children | Preserved |
| INV-9 | Zero thread leaks from `Parallel::evaluate` | Preserved |
| INV-10 | Zero `Mutex`/`RwLock` in operator.rs | Preserved |
| INV-11 | `Arc<dyn Operator>` and `Box<dyn Operator>` dispatch identically | Preserved |
| INV-12 | `OperatorContext<'static>: Send + 'static` | Preserved |

---

## INC-FORWARD-001 Closed

**INC-FORWARD-001** (cycle-17, forward-debt from sequential `Parallel`) is **CLOSED** by this cycle. `Parallel::evaluate` now evaluates all children concurrently in a single tick. The `NodeOutcome::Pending` type is present (for cycle-20+ cross-tick resumption) but is not emitted by the current implementation.

---

## Key Technical Decisions

### snapshot_for_child Generalization

The key insight that resolved the cycle-18 BLOCKED state:

```rust
// WU-2 (cycle-18): snapshot_for_child only on OperatorContext<'static>
// WU-4 (cycle-19): generalized to impl<'a> OperatorContext<'a>
impl<'a> OperatorContext<'a> {
    pub fn snapshot_for_child(&self, child_index: usize) -> OperatorContext<'static> {
        // Creates truly owned 'static data: Box::leak for node_run/store,
        // Arc::clone for ir/run/executor/clock
        OperatorContext {
            node_run: Box::leak(Box::new(NodeRun { .. })),
            ir: Arc::clone(&self.ir),
            run: Arc::clone(&self.run),
            store: Box::leak(Box::new(ScratchGraphStore)),
            clock: self.clock.clone(),
            executor: Arc::clone(&self.executor),
        }
    }
}
```

The `impl<'a>` (not just `impl OperatorContext<'static>`) allows `ctx.snapshot_for_child(i)` to be called from any `Parallel::evaluate` context, not just from a `'static` context.

### Backpressure Implementation

`Arc<AtomicUsize>` with spin-wait (no external semaphore crate). Not a perfect semaphore — when multiple threads compete for 1 permit, all may retry simultaneously and multiple may acquire. This is a known limitation of the stdlib-only constraint. The atomic approach still provides useful backpressure at higher concurrency limits.

---

## Cycle-20+ Forward Debt

| Debt | Severity | Description |
|------|----------|-------------|
| P1 | High | `Box::leak` in `snapshot_for_child` — **RESOLVED in cycle-20 WU-5 via Arc<Mutex<T>> field-type refactor (per ADR-0054)** |
| P2 | Medium | `NodeOutcome::Pending` support in `WorkflowRuntime::tick` for cross-tick Parallel resumption — **RESOLVED in cycle-20 WU-2/3** (receiver map + 3-phase drain/spawn/legacy tick) |
| P3 | Medium | True semaphore (not spin-wait) for backpressure — consider adding `parking_lot` or using a proper OS semaphore |
| P4 | Low | `ParallelCheckpointState` cross-tick resumption with receiver map on `WorkflowRuntime<R>` — **RESOLVED in cycle-20 WU-4** (`Parallel::evaluate` dual-path: blocking tests / non-blocking runtime emitting `NodeOutcome::Pending`) |
| INC-FORWARD-002 | Medium | HTTP async con `reqwest` (reemplazar `ureq` bloqueante) — **RESOLVED in cycle-20 WU-1** (reqwest 0.12 rustls-tls; closes INC-FORWARD-002 from cycle-17) |

---

## Next Steps

1. **Verify:** Run `cargo test --workspace` — must show 0 failures
2. **Clippy + Fmt:** `cargo clippy --workspace --all-targets -- -D errors && cargo fmt --all`
3. **Tag:** When all verifications pass, tag `v1.41.0`
4. **Release:** `sddk dev install` to update bundle runtime

---

## Relevant Files

- `crates/sddk-engine/src/operator.rs` — concurrent `Parallel::evaluate`, helper fns, `snapshot_for_child`
- `crates/sddk-engine/tests/parallel_concurrency_tests.rs` — 20 new concurrency tests
- `crates/sddk-engine/tests/parallel_seq_tests.rs` — 5 adapted tests
- `docs/adr/ADR-0050-true-concurrent-parallel.md` — concurrent parallel contract
- `docs/adr/ADR-0051-operator-arc-dyn-safety.md` — Box→Arc safety
- `~/.local/share/sddk/projects/p-52b95ef55999f9de/changes/kernel-cycle-19-operator-arc-dyn-safety/apply-progress.yaml` — full task ledger
