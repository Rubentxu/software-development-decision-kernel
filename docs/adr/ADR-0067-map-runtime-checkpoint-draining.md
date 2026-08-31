# ADR-0067 — Map Runtime Checkpoint Storage + Drain (cycle-32)

**Status:** implemented (cycle-32 GREEN commit `7134b80`)
**Date:** 2026-08-25
**Trigger:** SDDK kernel-cycle-32 runtime-side checkpoint draining
**References:** [ADR-0053](ADR-0053-tick-receiver-map-design.md) (cycle-20 pending_parallel design), [ADR-0052](ADR-0052-concurrent-parallel-channel-design.md) (ParallelCheckpointState), [ADR-0065](ADR-0065-map-source-context-isolation-cross-tick-replay.md) (cycle-30 MapCheckpointState struct)

---

## Context

cycle-30 introduced `MapCheckpointState` as a struct holding `receiver: mpsc::Receiver<ChildResult>`, `items_len`, `completed_results`, and `source_outputs_snapshot`. However, the cycle-30 implementation discarded the checkpoint immediately (`let _checkpoint = ...` at L1263/L1380), defeating the cross-tick replay goal.

cycle-32 completes the wiring: `WorkflowRuntime<R>` owns a `pending_map` parallel to cycle-20's `pending_parallel`, drains it per tick, and finalizes Map operators via `aggregate_collect_all`.

---

## Decision

### 1. WorkflowRuntime.pending_map Field

```rust
pub type MapKey = (sddk_domain::RunId, sddk_domain::OperatorId);

pub struct WorkflowRuntime<R: RunStore> {
    // ... existing fields ...
    /// Receiver map for in-flight `Map` operators that returned `NodeOutcome::Pending`
    /// on a prior tick. Cycle-32: stores `MapCheckpointState` keyed by (RunId, OperatorId).
    /// The outer Arc<Mutex<>> allows mutable access to MapCheckpointState during drain.
    pending_map: HashMap<MapKey, Arc<std::sync::Mutex<MapCheckpointState>>>,
}
```

Key design decisions:
- `MapKey = (RunId, OperatorId)` mirrors `ParallelKey` from cycle-20
- `HashMap` for O(1) lookup (same rationale as `pending_parallel`)
- `Arc<Mutex<MapCheckpointState>>` (NOT `Box`) because:
  - `CheckpointHandle::MapChannel { state }` must be `Clone` (runtime clones checkpoints into attempt outcomes)
  - `Arc<MapCheckpointState>` is `Clone`; `Box<MapCheckpointState>` would require `MapCheckpointState: Clone`
  - `MapCheckpointState` contains `receiver: Arc<Mutex<mpsc::Receiver<ChildResult>>>` (not `Receiver` directly) — the inner `Arc` satisfies the `Clone` requirement for `CheckpointHandle`'s manual `Clone` impl
- `Arc<Mutex<...>>` guards mutable access during drain (INV-10 compliant)

### 2. CheckpointHandle MapChannel Variant

```rust
pub enum CheckpointHandle {
    None,
    Channel { resume_token: u64 },
    /// Cycle-32: Map cross-tick replay via MapCheckpointState.
    /// `state` is Arc so that CheckpointHandle can implement Clone
    /// (required because runtime clones checkpoints into attempt outcomes).
    MapChannel {
        state: std::sync::Arc<MapCheckpointState>,
        token: u64,
    },
}
```

Manual `Clone` impl uses `Arc::clone` for the `MapChannel` variant.
Manual `PartialEq` + `Eq` impl compares only the `token` field (receiver inside `MapCheckpointState` has no equality).

### 3. MapCheckpointState Struct (cycle-32 updated)

```rust
#[derive(Debug, Clone)]
pub struct MapCheckpointState {
    /// Receiver for child iteration results.
    /// Wrapped in Arc<Mutex<...>> to satisfy Send+Sync requirements.
    pub receiver: std::sync::Arc<std::sync::Mutex<std::sync::mpsc::Receiver<ChildResult>>>,
    pub items_len: usize,
    pub completed_results: BTreeMap<usize, ChildResult>,
    /// Snapshot of source outputs for replay (source NOT re-evaluated on resume).
    /// INV-11: MUST be non-empty when checkpoint is handed to runtime.
    pub source_outputs_snapshot: BTreeMap<String, serde_json::Value>,
}
```

Key changes from cycle-30:
- `receiver` is now `Arc<Mutex<Receiver<ChildResult>>>` (was `Receiver<ChildResult>`) — satisfies `Clone` and `Send+Sync`
- `source_outputs_snapshot` is populated from `source_outcome.outputs.clone()` in the concurrent path (INV-11 fix)

### 4. drain_pending_map() Implementation

```rust
fn drain_pending_map(&mut self) -> TickPhaseOutcome {
    // 1. Drain all entries from pending_map
    // 2. For each entry:
    //    a. Lock outer Arc<Mutex<MapCheckpointState>>
    //    b. Clone inner Arc<Mutex<Receiver>> 
    //    c. Lock inner receiver and try_recv until Empty or Disconnected
    //    d. Insert each ChildResult into state.completed_results
    //    e. If completed_results.len() == items_len OR Disconnected:
    //       - Build results/failures via aggregate_collect_all
    //       - Emit NodeOutcome::Succeeded or NodeOutcome::Failed
    //    f. Else: re-insert entry into pending_map for next tick
}
```

Drain order in `tick()`: `drain_pending_map()` runs BEFORE `spawn_pending_and_ready()` (same phase ordering as cycle-20's `drain_pending_parallel`).

### 5. INV-11 Source Outputs Snapshot Fix

The cycle-30 concurrent path had a bug: `source_outcome` was consumed in the match arm for items extraction, so the snapshot was built from an empty/moved value. cycle-32 fixes this by cloning `source_outcome.outputs` into `source_outputs_clone` before using `source_outcome` for the checkpoint:

```rust
let source_outputs_clone: BTreeMap<String, serde_json::Value>;
let items: Vec<serde_json::Value> = match &source_outcome {
    NodeOutcome::Succeeded { outputs, .. } => {
        source_outputs_clone = outputs.clone();
        // ... extract items from source_outputs_clone ...
    }
    // ...
};
// source_outputs_clone available for checkpoint building
```

---

## Rejected Alternatives

### A. Box<MapCheckpointState> instead of Arc

`Box<T>` requires `T: Clone` to derive `Clone`. `MapCheckpointState` contains `Arc<Mutex<Receiver>>` which is `Clone` but `Receiver` alone is not. A manual `Clone` impl for `CheckpointHandle` could wrap the `Box` without requiring `MapCheckpointState: Clone`. However, `Arc` was chosen because:

- `Arc` is `Sync + Send` (needed for runtime's `Arc<Mutex<MapCheckpointState>>` field)
- `Arc::clone` is O(1) (same as `Box::new`)
- Avoids heap dereference on every state access in the drain loop

### B. Storing Receiver directly in HashMap without Arc<Mutex<>>

`mpsc::Receiver` is `!Sync`. A raw `HashMap<MapKey, Receiver<ChildResult>>` would not compile. The `Arc<Mutex<>>` wrapper provides interior mutability for drain (INV-10: no lock on workflow state, only on the receiver side-channel).

### C. BTreeMap instead of HashMap for pending_map

BTreeMap would provide ordered iteration but adds O(log n) lookup overhead. HashMap's O(1) average lookup is consistent with cycle-20's `pending_parallel` design and sufficient since keys are unique per tick.

---

## Invariant Preservation

- **INV-10 (no lock on workflow state)**: `Arc<Mutex<MapCheckpointState>>` guards the checkpoint side-channel (receiver + completed_results), NOT workflow state.
- **INV-11 (deterministic replay)**: `pending_map` keyed by `(RunId, OperatorId)` ensures one checkpoint per Map node-run. `source_outputs_snapshot` is non-empty on handoff (verified by RED test `map_source_outputs_snapshot_non_empty_on_concurrent_pending`).
- **INV-9 (no thread leaks)**: Concurrent path transfers the existing `rx` and existing `tx` clones held by spawned threads; no new threads spawned on resume.

---

## Changelog

- 2026-08-25T14:00 | implemented | cycle=kernel-cycle-32-runtime-checkpoint-draining | GREEN commit `7134b80` | status=implemented
