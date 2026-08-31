# ADR-0056 — Arc::try_unwrap defensive sync (cycle-22)

**Status:** accepted
**Date:** 2026-08-24
**Cycle:** 22 (A-min)
**Trigger:** cycle-20 debt-verify finding COUPLE-TRY-UNWRAP-SILENT-SYNC (P1 medium)

---

## Context

cycle-20 introduced `Arc<Mutex<NodeRun>>` (per ADR-0054) as the field type for
`OperatorContext.node_run`. `workflow_runtime.rs::tick()` syncs state from the
Arc back to `self.nodes[id]` after each evaluate, using:

```rust
if let Ok(nr) = Arc::try_unwrap(node_run_arc) {
    *node_run = nr.into_inner().unwrap();
}
```

This `if let Ok(...)` silently swallows the failure case when `Arc::try_unwrap`
returns `Err` (other Arc refs still alive). In cycle-20's `Parallel::evaluate`
Pending branch (`operator.rs:776`), the spawned supervisor thread holds an
`Arc::clone(&ctx.node_run)` clone, so `try_unwrap` always returns `Err` for
Pending outcomes. The mutation is silently dropped.

Currently nil data loss because Parallel::Pending doesn't mutate `node_run`
during evaluate (per ADR-0054 Pure contract). **However**, the silent fallback:

1. Hides future operator bugs — any operator that mutates `node_run` mid-evaluate
   without an outcome-driven state transition will have its mutation lost
2. Has zero test coverage — the failure path is not exercised

## Decision

Replace the silent fallback with a defensive `match` that syncs via lock when
other Arc refs exist:

```rust
match Arc::try_unwrap(node_run_arc) {
    Ok(mutex) => {
        *node_run = mutex.into_inner()
            .expect("Mutex<NodeRun> poisoned at sync point");
    }
    Err(arc) => {
        let count = Arc::strong_count(&arc);
        *node_run = arc.lock()
            .expect("Mutex<NodeRun> poisoned at sync point")
            .clone();
        eprintln(
            "WARN: Arc<Mutex<NodeRun>> sync via lock fallback ({} refs at sync point) \
             — INV-9 audit: investigate thread leak source",
            count
        );
    }
}
```

### Properties

- **No data loss**: mutation is preserved via lock clone
- **No silent failure**: WARN log surfaces the case for INV-9 audit
- **Panic on poison**: consistent with existing `into_inner().unwrap()` patterns
- **Fast path preserved**: uncontended case still does `into_inner()`

### INV-9 Compliance

INV-9 says "zero thread leaks from `Parallel::evaluate`". The WARN log provides
an audit trail — if the fallback fires, the runtime can detect a leaked
supervisor thread (or future operator that clones `ctx.node_run`).

## Consequences

### Positive
- Latent data-loss bug fixed
- INV-9 audit observability
- 4 RED tests document the behavior

### Negative
- Lock acquire in fallback path (rare, only when extra Arc refs exist)
- WARN log noise (acceptable: signals real anomaly)

### Trade-offs accepted
- `eprintln!` instead of `tracing::warn!` — engine is stdlib-only per cycle-19 INV
- Panic on Mutex poison (consistent with the existing `into_inner().unwrap()`
  pattern at the same site)

## References
- cycle-20 debt-report.md §COUPLE-TRY-UNWRAP-SILENT-SYNC
- ADR-0054 §Implementation (Arc<Mutex<NodeRun>> permitted exception)
- operator.rs:776 (Parallel::evaluate Pending branch)
- workflow_runtime.rs:582 (node_run_arc creation), :591 (ctx clone), :604/:668 (sync sites)
- INV-9 (zero thread leaks)
