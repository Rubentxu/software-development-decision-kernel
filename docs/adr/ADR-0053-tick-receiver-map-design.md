# ADR-0053 — Tick/Receiver-Map Design (cycle-20)

**Status:** implemented (cycle-20 WU-2 + WU-3 + WU-4 + WU-5)
**Date:** 2026-08-24
**Trigger:** SDDK kernel-cycle-20 WU-3, WU-4, WU-5
**References:** [ADR-0054](ADR-0054-operatorcontext-arc-mutex-fields.md) (Arc<Mutex<T>> field-type decision)

---

## Context

cycle-20 introduces `NodeOutcome::Pending` for `Parallel` operators that return before all children complete. The runtime must:

1. Store the receiver side of a channel so it can collect child results on subsequent ticks
2. Continue evaluating other nodes while children run concurrently
3. Reconcile child results when they arrive

ADR-0052 covers the `Parallel::evaluate` side. This ADR covers the **runtime tick redesign** and the **receiver map**.

---

## Decision

### WorkflowRuntime.pending_parallel Map

```rust
pub type ParallelKey = (sddk_domain::RunId, sddk_domain::OperatorId);

pub struct WorkflowRuntime<R: RunStore> {
    // ...
    pending_parallel: HashMap<ParallelKey, Arc<Mutex<mpsc::Receiver<ChildResult>>>>,
}
```

Key design:
- `ParallelKey = (RunId, OperatorId)` uniquely identifies a parallel slot
- `HashMap` (not BTreeMap) for O(1) lookup
- `Arc<Mutex<Receiver>>` guards the receiver handle (INV-10 compliant)

### ParallelKey Hash Derivation

`RunId` and `OperatorId` are newtype wrappers around `String`. Both require `Hash` derive:

```rust
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct RunId(pub String);

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct OperatorId(pub String);
```

This enables `HashMap<ParallelKey, Arc<Mutex<mpsc::Receiver<ChildResult>>>>` compilation.

### Tick() Three-Phase Redesign

The runtime `tick()` is restructured into three phases:

#### Phase 1 — DRAIN

Iterates `pending_parallel.drain()` to collect completed child results:

```rust
let mut reinsert_entries = Vec::new();
for (key, rx_arc) in self.pending_parallel.drain() {
    let rx_lock = rx_arc.lock().expect("mutex poisoned");
    let mut collected = BTreeMap::new();
    let mut disconnected = false;

    // Drain all available results from the receiver
    loop {
        match rx_lock.try_recv() {
            Ok(result) => { collected.insert(result.child_index, result); }
            Err(TryRecvError::Disconnected) => { disconnected = true; break; }
            Err(TryRecvError::Empty) => break,
        }
    }

    // If all children reported OR channel disconnected: build attempts
    // Else: re-insert entry for next tick
}
```

**INV-8 ordering invariant**: results collected in `BTreeMap<usize, ChildResult>` (keyed by `child_index`) preserves insertion order.

#### Phase 2 — SPAWN

For nodes in `Pending` or `Ready` state that are **not** in `pending_parallel`:

1. Create `(tx, rx)` channel pair
2. Store `rx` in `pending_parallel` keyed by `(run_id, operator_id)`
3. If operator is `Parallel` → pass `pending_sender = Some(tx)` to context
4. If operator is not `Parallel` → pass `pending_sender = None` (legacy path)
5. Evaluate operator

```rust
let pending_sender = if is_parallel {
    let (tx, rx) = mpsc::channel::<ChildResult>();
    let rx_arc = Arc::new(Mutex::new(rx));
    self.pending_parallel.insert(key, rx_arc);
    Some(tx)
} else {
    None
};

let mut ctx = OperatorContext {
    node_run,
    ir: Arc::new(self.ir.clone()),
    run: Arc::new(self.run.clone()),
    store: &mut self.store,
    clock: self.clock.clone(),
    executor,
    pending_sender,
};
```

#### Phase 3 — LEGACY

Non-Parallel nodes evaluated in Phase 2 receive `pending_sender=None`, triggering the blocking path in `Parallel::evaluate`. (Integrated into Phase 2 above for clarity.)

### AttemptOutcome::Pending Variant

The `AttemptOutcome` enum gains a `Pending` variant for cross-tick resumption:

```rust
pub enum AttemptOutcome {
    Succeeded { outputs: HashMap<String, Value> },
    Failed { error: String },
    Pending { resume_token: u64, attempt_seq: u32 },
}
```

This allows the runtime to record that a parallel is in-flight with a specific attempt sequence number.

### NodeOutcome::Pending Checkpoint

`Parallel::evaluate` returns:

```rust
NodeOutcome::Pending {
    checkpoint: CheckpointHandle::Channel { resume_token: 0 }
}
```

The runtime stores the receiver in `pending_parallel` before calling evaluate, so no additional checkpoint correlation is needed — the receiver IS the checkpoint.

---

## Consequences

### Positive

- Runtime can handle concurrent parallel children without blocking
- Clean separation: Phase 1 drains old results, Phase 2 spawns new work
- Receiver map keyed by `(RunId, OperatorId)` is natural and unique

### Negative

- `HashMap` over `HashMap<ParallelKey, ...>` requires `Hash` derive on `RunId` and `OperatorId` (added in cycle-20 WU-2)
- `Arc<Mutex<Receiver>>` adds synchronization overhead
- `Box::leak` eliminated in cycle-20 WU-5 by refactoring OperatorContext field types to `Arc<Mutex<T>>` (per ADR-0054); `snapshot_for_child` deleted entirely (ADR-0054)

---

## References

- [ADR-0050](ADR-0050-true-concurrent-parallel.md) — concurrent Parallel base design
- [ADR-0052](ADR-0052-concurrent-parallel-channel-design.md) — channel design
