# ADR-0050 — True-Concurrent Parallel with Pure-Return Contract

**Status:** accepted
**Date:** 2026-08-23
**Supersedes:** cycle-17 parallel implementation (sequential semantics)
**Trigger:** SDDK kernel-cycle-17 through cycle-19

---

## Context

cycle-17 introduced a `Parallel` operator with **sequential semantics** — one child per `evaluate` call, `NodeOutcome::Running` until all complete. This prevented concurrent execution across ticks and blocked cycle-17's true-concurrent goal.

cycle-18 attempted to upgrade `Parallel` to concurrent execution but was **BLOCKED** at WU-4 due to a fundamental Rust type-system barrier:

```
OperatorContext<'a> contains &mut references (node_run, store) which are not Send.
std::thread::spawn requires the captured closure to be Send.
```

The design in cycle-18 §3 attempted `Box::leak` on a shared context, but this requires `&'static mut OperatorContext` which cannot be obtained from `&mut OperatorContext<'_>` due to lifetime invariance.

cycle-19 resolves this by:

1. **Arc-wrap `ir` and `run` fields** of `OperatorContext` (WU-3.5): removes borrows from the context struct, making `OperatorContext<'static>: Send + 'static`
2. **`Box::leak` on owned `node_run` and `store`** in `snapshot_for_child` (WU-2, WU-4): produces truly owned `'static` data
3. **Concurrent fan-out** via `std::thread::spawn` with mpsc channel collection (WU-4)

---

## Decision

### Parallel::evaluate Concurrent Contract

`Parallel::evaluate` evaluates **all children concurrently** in a single call. It returns:

- `Ok(NodeOutcome::Succeeded { .. })` when all children succeed
- `Err(OperatorError::EvalFailed(..))` when any child fails with `NodeOutcome::Failed`
- `Ok(NodeOutcome::Failed { reason: "child N panicked" })` when any child panics

**Pure-return**: `Parallel::evaluate` returns an outcome without mutating `ctx.node_run`. The runtime persists attempts. This aligns with the `Task` and `Sequence` pure-return contract established in WU-3.

### snapshot_for_child Lifetime Strategy

```rust
impl<'a> OperatorContext<'a> {
    pub fn snapshot_for_child(&self, child_index: usize) -> OperatorContext<'static> {
        OperatorContext {
            node_run: Box::leak(Box::new(NodeRun { .. })),
            ir: Arc::clone(&self.ir),          // Arc: cheap clone, Send + Sync
            run: Arc::clone(&self.run),        // Arc: cheap clone, Send + Sync
            store: Box::leak(Box::new(ScratchGraphStore)),
            clock: self.clock.clone(),          // Clock: Clone, Send
            executor: Arc::clone(&self.executor), // Arc<dyn TaskExecutor>: Send + Sync
        }
    }
}
```

**Bounded Box::leak exception**: one leak per child invocation, scoped to process lifetime. cycle-20+ will refactor to eliminate this (P3).

### INV-8 Ordering Invariant

Despite concurrent evaluation, `attempt_seq == child_index` for all children. This is maintained by:

1. Spawning all N threads simultaneously
2. Collecting results in `BTreeMap<usize, ChildResult>` keyed by `child_index`
3. Applying outcomes in `child_index` order after all threads complete

The mpsc channel arrival order is **discarded**; `BTreeMap` insertion preserves declaration order.

### INV-9 No Thread Leaks

All `JoinHandle`s are joined inline after the mpsc drain. No thread leaks regardless of panic.

### INV-10 Zero Locks

Lock count: **zero `Mutex`, zero `RwLock` on workflow state**. Backpressure uses `Arc<AtomicUsize>` with spin-wait.

**INV-10 Permitted Exceptions** (cycle-20 WU-5, per ADR-0054): The following `Arc<Mutex<T>>` patterns are permitted:
- `Arc<Mutex<usize>>` for the permit counter (backpressure, uncontended)
- `Arc<Mutex<NodeRun>>` on `OperatorContext` fields (each Parallel child receives its own scratch `Arc<Mutex<ScratchGraphStore>>`, preserving cross-child isolation)

No `Mutex` or `RwLock` is permitted on shared **workflow** state (i.e., across distinct operators or ticks).

---

## Consequences

### Positive

- True concurrent parallel evaluation in a single tick
- Replay-safety: if `attempts.len() >= children.len()`, return `Succeeded` immediately (no re-evaluation)
- Pure-return contract: runtime is sole authority for attempt persistence
- `snapshot_for_child` works for any `OperatorContext<'a>` (generic impl)

### Negative

- Backpressure via atomic spin-wait does not guarantee strict sequential execution when `max_concurrency < num_children` and multiple waiters compete. This is a known trade-off of the stdlib-only constraint.
- cycle-20+ P3 refactor needed to eliminate `Box::leak` in `snapshot_for_child`

---

## References

- cycle-17: INC-FORWARD-001 (forward-debt from sequential parallel)
- cycle-18: BLOCKED at WU-4 (Rust lifetime barrier)
- cycle-19: WU-4, WU-5, WU-6 (concurrent fan-out implemented)
- ADR-0051: `Box<dyn Operator>` → `Arc<dyn Operator>` operator dyn safety resolution
