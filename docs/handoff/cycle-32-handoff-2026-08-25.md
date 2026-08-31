# Cycle-32 Handoff — Map Runtime Checkpoint Draining

**Cycle ID:** `p-52b95ef55999f9de/kernel-cycle-32-runtime-checkpoint-draining`
**Branch:** `feat/kernel-cycle-32-runtime-checkpoint-draining`
**Status:** GREEN committed (`7134b80`), 13 RED tests passing, 128 lib tests passing
**Date:** 2026-08-25

---

## What Was Built

cycle-32 wired the runtime side for Map cross-tick replay. The cycle-30 `MapCheckpointState` struct was being dropped immediately at `Map::evaluate` exit. cycle-32 added:

1. **`WorkflowRuntime.pending_map: HashMap<MapKey, Arc<Mutex<MapCheckpointState>>>`** — parallel to cycle-20's `pending_parallel`
2. **`drain_pending_map()`** — drains the map per tick, collects child results, finalizes via `aggregate_collect_all`
3. **`CheckpointHandle::MapChannel`** — new variant carrying `Arc<MapCheckpointState>` (not `Box`)
4. **INV-11 fix** — `source_outputs_snapshot` is now correctly populated from `source_outcome.outputs.clone()` in the concurrent path

---

## Key Technical Decisions

### Arc<Mutex<MapCheckpointState>> storage

`pending_map` stores `Arc<Mutex<MapCheckpointState>>` (not `Box`). Reason: `CheckpointHandle::MapChannel { state: Arc<MapCheckpointState> }` must implement `Clone` because the runtime clones checkpoints into attempt outcomes. `Arc<T>` is `Clone`; `Box<T>` requires `T: Clone` for the derive, and `MapCheckpointState` doesn't auto-derive `Clone` (contains `Receiver`).

### MapCheckpointState.receiver = Arc<Mutex<Receiver>>

The receiver inside `MapCheckpointState` is wrapped as `Arc<Mutex<Receiver<ChildResult>>>`. This satisfies:
- `Clone` (needed for `MapCheckpointState` to derive `Clone`)
- `Send + Sync` (needed for the `Arc<Mutex<>>` wrapper in `pending_map`)

### Manual PartialEq for CheckpointHandle

`CheckpointHandle` has a manual `PartialEq` + `Eq` impl that compares only `token` fields (since `Arc<Mutex<Receiver>>` inside `MapCheckpointState` has no equality). `Clone` is also manual, using `Arc::clone`.

### ChildResult derives Clone

`ChildResult` now derives `Clone` (was just `Debug`). This propagates to `MapCheckpointState::Clone`. `OperatorError` already derived `Clone` (part of existing `#[derive(Debug, Clone, PartialEq, Eq, Error)]`).

---

## Files Changed

### `crates/sddk-engine/src/operator.rs` (+117/-77 lines net)
- `CheckpointHandle::MapChannel` variant: `state: Arc<MapCheckpointState>` (was not present)
- `MapCheckpointState`: `receiver` changed to `Arc<Mutex<Receiver<ChildResult>>>`, added `#[derive(Clone)]`
- `ChildResult`: added `#[derive(Clone)]`
- `evaluate_map_body`: fixed `source_outputs_snapshot` population (INV-11 fix)
- `evaluate_concurrent`: passes `source_outputs_clone` for snapshot capture
- `CheckpointHandle`: manual `Clone` + `PartialEq` + `Eq` impls
- `build_map_composite_failure_reason`: made `pub`

### `crates/sddk-engine/src/workflow_runtime.rs` (+213/-0 lines net)
- `MapKey` type alias = `(RunId, OperatorId)`
- `pending_map` field added to `WorkflowRuntime`
- `drain_pending_map()` implemented (mirrors `drain_pending_parallel`)
- `tick()` calls both drains in sequence
- `spawn_pending_and_ready` checks `pending_map` to skip already-pending Map nodes
- `apply_outcomes_to_state` inserts MapChannel checkpoints into `pending_map`

### `crates/sddk-engine/tests/map_operator_tests.rs` (test updates)
- 6 structural tests updated to use `Arc<Mutex<Receiver>>` and `Arc<MapCheckpointState>`
- `map_checkpoint_handle_mapchannel_carries_boxed_state` → `map_checkpoint_handle_mapchannel_carries_arc_state`

### `crates/sddk-engine/tests/runtime_receiver_map_tests.rs` (9 tests, all green)
- `drain_pending_map_terminates_when_items_len_reached`
- `drain_pending_map_terminates_when_receiver_disconnected`
- `inv10_attempt_outcome_pendin_has_no_receiver`
- `mapkey_type_uses_runid_and_operatorid`
- `parallel_key_type_uses_runid_and_operatorid`
- `second_insert_for_same_key_overwrites`
- `new_workflow_runtime_has_empty_pending_map`
- `second_insert_for_same_mapkey_overwrites`
- `new_workflow_runtime_has_empty_pending_parallel`

### `crates/sddk-engine/tests/runtime_construction_tests.rs`
- `runtime_smoke_map_runs_through_one_tick`: updated docstring to reference cycle-32

---

## What Remains

### Before cycle closure:
1. **ADR-0067** — this document (`docs/adr/ADR-0067-map-runtime-checkpoint-draining.md`) — just created
2. **apply-progress.yaml** — must be written to `{cycle_artifacts_dir}/`
3. **push to origin** — GREEN commit already pushed (`7134b80`)

### NOT in scope for cycle-32:
- Sequential `Pending` body Task executor (cycle-33+)
- Token issuance policy (cycle-33+)
- Source re-evaluation on resume (cycle-33+)

---

## How to Verify

```bash
cd ~/Proyectos/agentesIA/sddk-framework
cargo test -p sddk-engine --test map_operator_tests       # 31 tests
cargo test -p sddk-engine --test runtime_receiver_map_tests  # 9 tests
cargo test -p sddk-engine --test runtime_construction_tests  # 4 tests
cargo test -p sddk-engine --lib                            # 128 tests
cargo fmt --all -- --check
cargo clippy -p sddk-engine --all-targets                  # warnings only, no errors
```

---

## Invariant Status

| Invariant | Status | Evidence |
|---|---|---|
| INV-10 (no lock on workflow state) | PRESERVED | `Arc<Mutex<...>>` guards receiver side-channel only |
| INV-11 (deterministic replay) | PRESERVED + FIXED | `source_outputs_snapshot` now non-empty on handoff |
| INV-9 (no thread leaks) | PRESERVED | No new threads spawned on resume |

---

## Cycle Artifacts Location

```
~/.local/share/sddk/projects/p-52b95ef55999f9de/cycle-artifacts/kernel-cycle-32-runtime-checkpoint-draining/
├── proposal.md    # Option A design
├── spec.md        # requirements + scenarios
└── (this handoff)
```
