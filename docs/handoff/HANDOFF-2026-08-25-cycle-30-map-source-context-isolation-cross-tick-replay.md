# HANDOFF — cycle-30 CLOSED

> **Cycle:** `p-52b95ef55999f9de/kernel-cycle-30-map-source-context-isolation-cross-tick-replay`
> **Status:** CLOSED ✅
> **Tag:** `v1.46.0` peels to `e56ce0be9cda1a9f399b248aeae10f47311a6f3f`
> **HEAD:** `e56ce0be9cda1a9f399b248aeae10f47311a6f3f` on `feat/kernel-cycle-30-map-source-context-isolation-cross-tick-replay`
> **Path:** A-min
> **Date closed:** 2026-08-25
> **Next cycle:** **cycle-31** — DC-MAP-002 dispatch global refactor

---

## 1. What cycle-30 shipped

Closed DC-MAP-001 (source-context isolation) and shipped cross-tick replay deferred from cycle-28:

- **Source-context isolation (DC-MAP-001 closure):** `source.evaluate` now uses fresh child `OperatorContext` with Arc-cloned shared fields, own `ScratchGraphStore`, `pending_sender: None`. Source MUST NOT mutate parent's `node_run.state`/`attempts`.
- **Cross-tick replay (cycle-28 deferred):** `MapCheckpointState` struct introduced; sequential and concurrent `Pending` paths now build checkpoint before returning. Source outputs snapshotted for replay (INV-11: source NOT re-evaluated on resume).
- **Docstring updated:** `source-context isolation` and `cross-tick replay` now listed as in-scope (cycle-30); only `DC-MAP-002 (dispatch global)` remains deferred to cycle-31+.

### Final 3 commits

```
e56ce0b docs(debt): INC-DEBT-007 preexisting sddk-cli clippy (cycle-30)
761073d docs(handoff): cycle-30 handoff (cycle-30)
0bd581a docs(adr): ADR-0065 map source-context isolation + cross-tick replay (cycle-30)
7dd9502 test(engine): RED tests for Map source-context isolation + cross-tick replay (cycle-30)
```

`<commit-1>` touches only the test file + impl (GREEN tests included in same commit as implementation per TDD protocol); `<commit-2>` touches ADR; `<commit-3>` touches only the handoff.

### Files affected (4)

| File | Δ | What |
|---|---|---|
| `crates/sddk-engine/src/operator.rs` | +98/-12 | `MapCheckpointState` struct (L341-351); `Checkpoint::MapChannel` variant (L357); source child ctx at L1104-1125; sequential Pending checkpoint (L1225-1277); concurrent Pending checkpoint (L1395-1422); docstring update (L1055-1082) |
| `crates/sddk-engine/tests/map_operator_tests.rs` | +404/-22 | 24 tests total (was 18 cycle-28 + 6 new cycle-30); removed `map_docstring_lists_cross_tick_replay_deferred`; added 2 doc-oracle updates + 3 isolation tests + 1 replay test + note on sequential Pending limitation |
| `docs/adr/ADR-0065-map-source-context-isolation-cross-tick-replay.md` | NEW | Cycle-30 decision: source child ctx + checkpoint shape + D-4 DC-MAP-002 deferral |
| `docs/handoff/HANDOFF-2026-08-25-cycle-30-map-source-context-isolation-cross-tick-replay.md` | NEW | This file (cycle-30 handoff to cycle-31) |

Total: ~502 insertions, ~34 deletions across 4 files.

### Spec coverage (5/5 COMPLIANT)

`{cycle-artifacts-dir}/spec.md` — 2 new requirements, 2 modified:

| Requirement | Scenarios | Status |
|---|---|---|
| REQ-Map-Source-Context-Isolation | 3 | ✅ |
| REQ-Map-Cross-Tick-Replay | 3 | ✅ (sequential Pending path limitation documented) |
| REQ-Map-Doc-Cycle30-OutOfScope | 2 | ✅ |
| REQ-Map-Collect-All-Errors (cycle-30 appended) | 1 | ✅ |

---

## 2. Debt findings (resolved / deferred to cycle-31)

`debt-report.json` at `/home/rubentxu/.local/share/sddk/projects/p-52b95ef55999f9de/cycle-artifacts/kernel-cycle-30-map-source-context-isolation-cross-tick-replay/debt-report.json` (SHA256: `c5e7c6a0c087786c42dd79da06b046aa1c3c9e4eb3d0a7a02bbdb409a9ebf6a9`).

| ID | Cluster | Severity | Priority | Summary | Status |
|---|---|---|---|---|---|
| DC-MAP-001 | coupling | medium | P2 | `source.evaluate(ctx)` reused parent `OperatorContext` | ✅ CLOSED (cycle-30) |
| DC-MAP-002 | coupling | medium | P2 | `Map` calls `dispatch()` global for both source and body | Deferred to cycle-31+ |

**Cycle-31 picks up DC-MAP-002 (P2 medium) as dispatch global refactor.** Affects `Map`, `Parallel`, and `Sequence` equally — requires holistic solution.

---

## 3. Next cycle — cycle-31

**Goal:** DC-MAP-002 dispatch global refactor. Spec: [REQ-Map-Dispatch-Global](`REQ-Map-Dispatch-Global` (vault)) (to be created).

**Path:** A-min.

**Scope:**
1. **Dispatch global concern:** Map, Parallel, and Sequence all call `dispatch()` global. DC-MAP-002 is that this global coupling is a P2 medium issue.
2. **Approach TBD:** Options include operator registry, context-local dispatch, or inline operator resolution.
3. **Not in scope:** Runtime-side checkpoint map draining (out of scope for engine-only cycle).

**Operational notes:**
- `MapCheckpointState` is built but runtime-side checkpoint draining is not implemented in this cycle. `Pending { Channel { resume_token } }` is returned but the runtime would need to store and drain the checkpoint map.
- Sequential `Pending` via `Task` body is untestable in current architecture (cycle-30 code exists but cannot be triggered via `TaskExecutor`). Documented in test comment.

---

## 4. Recovery cheat sheet

### If `cargo test -p sddk-engine` fails after merge

1. Check `map_operator_tests.rs` — 24 tests expected (18 cycle-28 + 6 new cycle-30)
2. Run `cargo test -p sddk-engine --test map_operator_tests` to isolate
3. Check if `MapCheckpointState` is exported: `use sddk_engine::operator::MapCheckpointState`
4. Check docstring: `grep -n "source-context isolation" crates/sddk-engine/src/operator.rs`

### If spec coverage regresses

1. Verify `MapCheckpointState` struct exists with fields: `receiver`, `items_len`, `completed_results`, `source_outputs_snapshot`
2. Verify `Checkpoint::MapChannel` variant exists
3. Verify source child ctx pattern at `operator.rs ~L1104-1125`
4. Check sequential Pending path builds checkpoint at `operator.rs ~L1225-1277`
5. Check concurrent Pending path builds checkpoint at `operator.rs ~L1395-1422`

### If DC-MAP-002 resurfaces

1. Verify source dispatch is still via `dispatch(source_ir)?` at `operator.rs L1109`
2. DC-MAP-002 deferred to cycle-31+ — do NOT close without holistic dispatch solution
