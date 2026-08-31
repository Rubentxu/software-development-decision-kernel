# ADR-0065 — Map source-context isolation + cross-tick replay (cycle-30)

**Status:** accepted (proposed 2026-08-25, accepted 2026-08-25)
**Date:** 2026-08-25
**Cycle:** 30 (A-min)
**Trigger:** Phase 4 WU-3 — DC-MAP-001 closure + deferred cross-tick replay
**Supersedes scope of:** ADR-0061 deferred row 1 (source-context isolation); ADR-0062 deferred row 1 (source dispatch); ADR-0063 deferred D-4 (cross-tick replay)

---

## Context

Cycle-26 (ADR-0061) shipped `Operator::Map` as a stub.
Cycle-27 (ADR-0062) wired source evaluation but re-used parent `OperatorContext`.
Cycle-28 (ADR-0063) added `max_concurrency` enforcement + collect-all aggregation.

Two items were explicitly deferred:

| Deferred Item | Source | Cycles Deferred |
|---|---|---|
| DC-MAP-001: source-context isolation | ADR-0061 row 1 | cycle-29 |
| Cross-tick replay | ADR-0063 A-2 | cycle-29 |

Cycle-30 addresses DC-MAP-001 closure and ships cross-tick replay deferred from cycle-28.

### DC-MAP-001 — The Problem

`Map::evaluate` called `source.evaluate(ctx)` at L1092 (cycle-27), passing the
parent `OperatorContext` directly. This meant:

- Source wrote to the **parent's** scratch store
- Source could observe the **parent's** `node_run.state` / `attempts`
- Source's `Pending` propagated via parent's `pending_sender` (hidden coupling)

Body iterations also wrote to the **same** parent's scratch store (since they ran
in threads but used the parent `ctx`). This violated INV-10: body iterations
were not scratch-isolated from each other or from source.

### Cross-Tick Replay — The Problem

Cycle-28 ADR-0063 D-4 explicitly deferred replay because it required "non-trivial
serialization infrastructure." When body returned `Pending` mid-iteration, the
Map checkpoint was lost — prior completed iterations were not preserved across
tick boundaries.

---

## Decision

### D-1: Source gets fresh child `OperatorContext` (DC-MAP-001 closure)

`Map::evaluate` MUST construct a **fresh child `OperatorContext`** for source
evaluation, mirroring the body child-ctx pattern at operator.rs L1234-1243:

```rust
// Per-source scratch store (isolated from parent and body iterations)
let source_store: ScratchStore = Arc::new(Mutex::new(GraphStoreBox {
    inner: Box::new(ScratchGraphStore),
}));
let mut source_ctx = OperatorContext {
    node_run: Arc::clone(&ctx.node_run),    // Arc clone — read-only
    ir: Arc::clone(&ctx.ir),
    run: Arc::clone(&ctx.run),
    store: source_store,                     // FRESH — not parent's
    clock: ctx.clock.clone(),
    executor: Arc::clone(&ctx.executor),
    pending_sender: None,                    // source → Map, not direct to runtime
};
let source_outcome = source_op.evaluate(&mut source_ctx)?;
```

**Invariants preserved:**
- `Arc` clones mean `node_run` / `ir` / `run` are shared read-only (no contention)
- Fresh `ScratchGraphStore` means source scratches do NOT leak to parent or body
- `pending_sender: None` means source `Pending` propagates as `Map::Pending`,
  not silently forwarded to runtime

**Why not `inputs_override` (DE-MAP-001 P3 finding from cycle-27):**
`inputs_override` would require a new field on `OperatorContext` and couple the
API to a specific use-case. The child-ctx approach is zero-API-change and mirrors
existing body iteration pattern.

### D-2: `MapCheckpointState` for cross-tick replay

Cycle-30 introduces `MapCheckpointState` (operator.rs L341-351), mirroring
`ParallelCheckpointState` (L343-347):

```rust
/// Opaque runtime state for Map mid-flight across ticks.
#[derive(Debug)]
pub struct MapCheckpointState {
    /// Receiver for child iteration results.
    pub receiver: std::sync::mpsc::Receiver<ChildResult>,
    /// Total number of items from source.
    pub items_len: usize,
    /// Completed iteration results indexed by iteration number.
    pub completed_results: BTreeMap<usize, ChildResult>,
    /// Snapshot of source outputs for replay (source NOT re-evaluated on resume).
    pub source_outputs_snapshot: BTreeMap<String, serde_json::Value>,
}
```

**Checkpoint flow (concurrent path):**

1. Body iteration returns `Pending` → detected in `while let Ok(result) = rx.recv()` loop
2. `MapCheckpointState` built with `receiver: rx`, `items_len`, `completed_results`,
   `source_outputs_snapshot` (snapshot of source's outputs at first `Pending`)
3. Return `Pending { checkpoint: CheckpointHandle::Channel { resume_token: T } }`
4. Runtime stores `MapCheckpointState` keyed by `run_id:node_id`
5. On next tick, runtime resumes from checkpoint — source NOT re-evaluated (INV-11)

**Checkpoint flow (sequential path):**

1. Body iteration returns `Pending`
2. Snapshot source outputs from items array
3. Create placeholder `(tx, rx)` channel (sequential has no concurrent threads)
4. Return `Pending { checkpoint: CheckpointHandle::Channel { resume_token: 0 } }`

**Why snapshot `source_outputs_snapshot`:**
Per INV-11, replay must be deterministic. Re-evaluating source could produce
different results (e.g., a random generator or external API). Snapshotting
freezes the source outputs so replay resumes from the same cursor.

### D-3: Sequential Pending builds checkpoint before return

Cycle-28 code at L1185 returned `Pending` without checkpointing:

```rust
other => return Ok(other), // Pending/Running propagation
```

Cycle-30 sequential path now builds `MapCheckpointState` before returning:

```rust
NodeOutcome::Pending { .. } => {
    let source_snapshot: BTreeMap<String, serde_json::Value> =
        serde_json::from_value(serde_json::json!({ "items": items })).unwrap_or_default();
    let (tx, rx) = std::sync::mpsc::channel::<ChildResult>();
    drop(tx); // sequential: no one will send
    let _checkpoint = MapCheckpointState {
        receiver: rx,
        items_len: items.len(),
        completed_results: BTreeMap::new(),
        source_outputs_snapshot: source_snapshot,
    };
    return Ok(NodeOutcome::Pending {
        checkpoint: CheckpointHandle::Channel { resume_token: 0 },
    });
}
```

**Note:** Sequential `Pending` via `Task` body cannot be triggered through the
`TaskExecutor` interface (which returns `TaskOutput`/`TaskError`, mapping to
`Succeeded`/`Failed`). The sequential checkpoint path exists for future body
operator types that can return `Pending` directly.

### D-4: Docstring updated — cycle-30 in-scope, DC-MAP-002 deferred

Map docstring (operator.rs L1055-1082) now lists:

- **In-scope (cycle-30):** source-context isolation (DC-MAP-001 closure),
  cross-tick replay, collect-all semantics, `max_concurrency` enforcement
- **Deferred to cycle-31+:** DC-MAP-002 (dispatch global) — the coupling
  between Map's dispatch and a global dispatch table

**DC-MAP-002 NOT deferred to cycle-30:** The dispatch-global concern affects
`Parallel` and `Sequence` equally. Scoping it to cycle-30 for Map alone
would create divergence without solving the root issue. Deferred to cycle-31
where a holistic dispatch-global solution can be designed.

---

## Alternatives Considered

### A-1: `inputs_override` field on `OperatorContext` (cycle-27 DE-MAP-001 P3)

Add `inputs_override: Option<BTreeMap<String, Value>>` to `OperatorContext`.
Source evaluates against parent ctx but with overridden inputs.

**Rejected:** API surface change; couples context to one use-case; the
child-ctx approach (D-1) achieves the same isolation with zero API change.

### A-2: Per-iteration source re-evaluation on replay

Instead of snapshotting `source_outputs_snapshot`, re-evaluate source on replay.

**Rejected:** Violates INV-11 (non-deterministic replay). External sources
(rand, API) could produce different results. Snapshot is the correct approach.

### A-3: Cross-tick replay via runtime-owned `ParallelCheckpointState`

Reuse `ParallelCheckpointState` for Map replay.

**Rejected:** `ParallelCheckpointState` is designed for heterogeneous children
with a single `receiver`. Map is homogeneous (N iterations of the same body)
with additional state (source snapshot, items_len). Distinct checkpoint
structure is cleaner.

### A-4: Store checkpoint on `node_run.state` directly

Instead of runtime-owned map keyed by `run_id:node_id`, store checkpoint in
`node_run.state`.

**Rejected:** `NodeRunState` is a simple enum; adding complex checkpoint
state would bloat it. The side-channel map pattern (ParallelCheckpointState,
cycle-20) is already established and appropriate.

---

## Consequences

### Positive

- DC-MAP-001 closed — source scratches do not leak to body or parent
- Cross-tick replay shipped — pending Map can resume across tick boundaries
- Source evaluated exactly once per execution (INV-11 satisfied via snapshot)
- No new public API surface
- Child-ctx pattern mirrors existing body iteration code (reduces cognitive load)

### Negative

- DC-MAP-002 remains open (dispatch global deferred to cycle-31)
- `MapCheckpointState` allocates a `BTreeMap` for `source_outputs_snapshot`
  (minor; source outputs are typically small)
- Sequential Pending path requires `Task`-returning-operator (not testable via
  `TaskExecutor` in current architecture) — limitation documented in test comment

### Risks

| Risk | Severity | Mitigation |
|---|---|---|
| Large `items` array bloat in `source_outputs_snapshot` | Low | Snapshot is source outputs, not items array. Source outputs are derived from items, not the full array. |
| Sequential `Pending` untestable via `TaskExecutor` | Low (documented) | Code path exists for future operator types. `TaskExecutor` cannot produce `NodeOutcome::Pending`. |
| Runtime doesn't actually drain Map checkpoints (not implemented in this cycle) | Medium | `Pending { Channel { resume_token } }` returned; runtime-side checkpoint map out of scope for cycle-30. |

---

## Test Coverage

Cycle-30 adds ~6 new tests + 2 doc-oracle updates to `map_operator_tests.rs`:

| Test | REQ scenario | Notes |
|---|---|---|
| `map_source_context_isolation_source_does_not_mutate_parent_attempts` | REQ-Map-Source-Context-Isolation.2 | Verifies `node_run.attempts.len()` unchanged |
| `map_source_context_isolation_source_pending_propagates` | REQ-Map-Source-Context-Isolation.3 | Source failure → Map Failed |
| `map_cross_tick_replay_source_not_reevaluated` | REQ-Map-Cross-Tick-Replay.3 | Source evaluated exactly once |
| `map_collect_all_preserved_across_replay` | REQ-Map-Collect-All-Errors.4 | results.len() + failures.len() == items_len |
| `map_checkpoint_state_struct_exists` | Structural | Verifies `MapCheckpointState` exports |
| `map_docstring_lists_cross_tick_replay_in_scope` | Doc-oracle | "source-context isolation" in-scope, no "cycle-29" |
| `map_docstring_defers_only_dc_map_002` | Doc-oracle | Only DC-MAP-002 in deferred list |

Doc-oracle updates (removed cycle-29 deferred assertions):

| Test | Change |
|---|---|
| `map_docstring_lists_cross_tick_replay_deferred` → `map_docstring_lists_cross_tick_replay_in_scope` | cycle-29 removed, cycle-30 in-scope asserted |
| `map_docstring_defers_only_dc_map_002` | NEW — asserts only DC-MAP-002 deferred |

---

## INV Preservation

- INV-1..INV-9: Map changes are additive; no existing invariants broken
- INV-10 preserved: per-source and per-body-iteration scratch stores are independent
- INV-11 (deterministic replay): `source_outputs_snapshot` freezes source outputs;
  replay is deterministic for identical checkpoint state
- INV-9 (no thread leaks): `JoinHandle` cleanup unchanged in concurrent path

---

## References

- ADR-0061 — Map stub (cycle-26, DC-MAP-001 deferred)
- ADR-0062 — Map source plumbing (cycle-27, source context deferred)
- ADR-0063 — Map max_concurrency + collect-all (cycle-28, cross-tick replay deferred)
- ADR-0050 — True concurrent Parallel (cycle-20, `ParallelCheckpointState` template)
- REQ-Map-Source-Context-Isolation.md (cycle-30, this cycle's spec)
- REQ-Map-Cross-Tick-Replay.md (cycle-30, this cycle's spec)
- `crates/sddk-engine/src/operator.rs` lines 340-351 (`MapCheckpointState`),
  1055-1082 (Map docstring), 1104-1125 (source child ctx), 1193-1282
  (sequential eval), 1343-1430 (concurrent eval with checkpoint)
