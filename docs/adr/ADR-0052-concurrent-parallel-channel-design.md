# ADR-0052 — Concurrent Parallel Channel Design (cycle-20)

**Status:** implemented (cycle-20 WU-2 + WU-4 + WU-5)
**Date:** 2026-08-24
**Supersedes:** ADR-0050 (single-tick concurrent parallel)
**Trigger:** SDDK kernel-cycle-20 WU-2, WU-4, WU-5
**References:** [ADR-0054](ADR-0054-operatorcontext-arc-mutex-fields.md) (Arc<Mutex<T>> field-type decision)

---

## Context

ADR-0050 established true-concurrent `Parallel::evaluate` with thread-based fan-out and mpsc collection. However, that design **blocks** until all children complete — suitable for tests but not for the runtime where the runtime must return to the caller between ticks.

cycle-20 requires a **non-blocking** `Parallel::evaluate` that:
1. Spawns a supervisor thread
2. Returns `NodeOutcome::Pending` immediately
3. Delivers results via a side channel (`pending_sender`) for the runtime to collect on subsequent ticks

---

## Decision

### Dual-Path Parallel::evaluate

`Parallel::evaluate` supports two execution paths:

#### Path A — Non-Blocking (Runtime Path)

When `ctx.pending_sender` is `Some(tx)`:

1. Supervisor thread is spawned immediately
2. Supervisor creates its own mpsc channel `(tx, rx)`
3. Children are spawned as `std::thread::spawn` threads, each receiving a fresh `OperatorContext`:
   `let node_run = Arc::clone(&ctx.node_run); let store = Arc::new(Mutex::new(Box::new(ScratchGraphStore::new())));`
4. Children report results to supervisor via supervisor's channel
5. Supervisor drains results and **forwards** each `ChildResult` to `pending_sender`
6. `Parallel::evaluate` returns `NodeOutcome::Pending { checkpoint: CheckpointHandle::Channel { resume_token: 0 } }` **immediately**

```rust
if let Some(pending_sender) = ctx.pending_sender.take() {
    // Spawn supervisor thread
    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel::<ChildResult>();
        // ... spawn children, collect results ...
        for result in collected {
            let _ = pending_sender.send(result);
        }
    });
    return Ok(NodeOutcome::Pending {
        checkpoint: CheckpointHandle::Channel { resume_token: 0 },
    });
}
```

#### Path B — Blocking (Test Path)

When `ctx.pending_sender` is `None` (legacy tests without runtime):

- Uses the original ADR-0050 implementation
- Blocks on `rx.recv()` until all children complete
- Returns final outcome synchronously

### Supervisor Thread Design

The supervisor thread is a **fire-and-forget** background task:
- Lives until all children send results
- Forwards results to runtime via `pending_sender`
- No cleanup needed (thread terminates after forwarding)

### CheckpointHandle for Pending

`NodeOutcome::Pending { checkpoint }` carries a `CheckpointHandle` token:

```rust
pub enum CheckpointHandle {
    None,                            // cycle-19 (sync completion)
    Channel { resume_token: u64 },    // cycle-20 (async, side-channel)
}
```

`resume_token: 0` is a placeholder — cycle-20 keys the receiver by `(RunId, OperatorId)` so no separate token is needed.

---

## Consequences

### Positive

- Runtime can return `Pending` and accept more work between ticks
- Supervisor thread handles concurrent children without blocking the runtime
- Tests continue to work with blocking path (backward compatible)

### Negative

- Supervisor thread adds overhead for small parallel evaluations
- `Box::leak` eliminated in cycle-20 WU-5 by refactoring OperatorContext field types to `Arc<Mutex<T>>` (per ADR-0054); `snapshot_for_child` deleted entirely; each Parallel child thread receives a freshly-constructed `OperatorContext<'static>` via `Arc::clone` of shared fields plus a per-child scratch store.
- `CheckpointHandle::Channel { resume_token: 0 }` is a placeholder (full token system deferred)

---

## References

- [ADR-0050](ADR-0050-true-concurrent-parallel.md) — concurrent Parallel base design
- [ADR-0053](ADR-0053-tick-receiver-map-design.md) — runtime tick/receiver-map integration
