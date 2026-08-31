# ADR-0054 — OperatorContext field types: Arc<Mutex<T>>

**Status:** accepted (cycle-20 WU-5)
**Date:** 2026-08-24
**Supersedes:** design.md Decision 1 (Box::new moved into closure) — explicitly overridden by WU-5 expanded-scope refactor
**Trigger:** SDDK kernel-cycle-20 WU-5

---

## Context

cycle-19 forward-debt P1 was to eliminate `Box::leak` in `snapshot_for_child`. cycle-20 design.md Decision 1 chose `Box::new(...)` moved into closure to satisfy `'static`. However, this requires changing OperatorContext field types from `&'a mut T` to `Box<T>` (or Arc<T>) for ALL construction sites (24 sites across operator.rs, workflow_runtime.rs, and 4 test files).

The `Box::new` moved-into-closure pattern works ONLY for `Parallel::evaluate`'s spawn closure. Other construction sites (workflow_runtime, tests) require different ownership models.

## Decision

Change OperatorContext field types:

| Field | Before | After |
|-------|--------|-------|
| `node_run` | `&'a mut NodeRun` | `Arc<Mutex<NodeRun>>` |
| `store` | `&'a mut dyn GraphStore + Send` | `Arc<Mutex<dyn GraphStore + Send>>` |

This was design.md Decision 1's explicitly REJECTED alternative ("serializes children, defeats cycle-19"). We accept it with the following justification:

### Why it does NOT serialize children in practice

Per `operator.rs:309` (cycle-19 invariant), `child.evaluate(&mut ctx)` is documented **PURE**: returns `NodeOutcome` without mutating `ctx.node_run`. Children only push `ChildResult` via `mpsc::Sender`. The `Arc<Mutex<NodeRun>>` is only locked at construction (workflow_runtime writes initial NodeRun) and at completion (workflow_runtime collects attempts). **No contention during `child.evaluate`**.

For `store`: each Parallel child receives its OWN `Arc::new(Mutex::new(ScratchGraphStore::new()))` — NOT a clone of the parent's store. Cross-child isolation preserved (cycle-19 invariant).

### Implementation

- `snapshot_for_child` method **deleted** entirely.
- All construction sites (24 sites: operator.rs inline tests + workflow_runtime.rs + 3 test files) updated to `Arc::new(Mutex::new(...))`.
- Use sites dereference via `ctx.node_run.lock().unwrap()` (helper: `fn nr(ctx: &OperatorContext) -> MutexGuard<NodeRun>`).
- INV-10 wording updated to "no Mutex on workflow state"; `Arc<Mutex<NodeRun>>` permitted (replaces cycle-19 `Box::leak` exception).

## Consequences

### Positive
- `Box::leak` eliminated entirely (grep returns 0)
- `snapshot_for_child` deleted (grep returns 0)
- INV-9 zero thread leaks preserved (Arc drops on thread join)
- ABI simplification: OperatorContext is now a value type with owned shared state

### Negative
- Every OperatorContext use site requires `.lock()` to deref node_run/store
- Slight runtime overhead from Mutex acquire/release (uncontended in current code paths)
- 24 construction sites updated (mechanical refactor)

### Trade-offs accepted
- Mutex on NodeRun is a structural change vs. cycle-19's atomic-free design. Justified by:
  1. The Mutex is uncontended in current patterns (children are pure)
  2. The alternative (different field types for borrowed vs owned contexts) doubles the API surface
  3. INV-10 wording explicitly permits this pattern, preserving the invariant's intent (no shared workflow state mutation)

## References
- `docs/adr/ADR-0050-true-concurrent-parallel.md` — INV-10 invariant origin
- `docs/adr/ADR-0051-operator-arc-dyn-safety.md` — `Arc<dyn Operator>` precedent
- `prompts/sddk-decision-kernel-architecture/` — forward-debt P1 origin
- `docs/handoff/HANDOFF-2026-08-25-cycle-19-sddk-framework.md` §101-109 — original forward debt
- design.md Decision 1 — REJECTED alternative that this ADR overrides
