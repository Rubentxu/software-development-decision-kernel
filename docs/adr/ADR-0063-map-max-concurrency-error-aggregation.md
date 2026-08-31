# ADR-0063 — Map max_concurrency + error aggregation (collect-all) (cycle-28)

**Status:** accepted (proposed 2026-08-24, accepted 2026-08-24)
**Date:** 2026-08-24
**Cycle:** 28 (A-min)
**Trigger:** Phase 4 (Epic DW) WU-3
**Supersedes scope of:** ADR-0061 limitations (2) and (4); ADR-0062 deferred rows 1 and 2.

---

## Context

Cycle-26 (ADR-0061) shipped `Operator::Map` as a bounded stub: source IGNORED,
`max_concurrency` IGNORED, first-failure aggregation, no concurrency throttling.

Cycle-27 (ADR-0062) promoted Map to real **source evaluation plumbing**:
- source is dispatched and evaluated against the parent `OperatorContext`
- `outputs["items"]: Array` collection-key convention enforced
- body Task-only invariant
- per-iteration `{item, index}` inputs injection
- results aggregated as `outputs["results"]: Array`
- error aggregation still **first-failure** (early-return on first body `Failed`)
- `max_concurrency` still IGNORED (sequential loop only)
- cross-tick replay still unsupported (cycle-29 deferred)

Cycle-28 WU-3 promotes Map to its **full runtime semantics**: concurrency
throttling via `CountingSemaphore` (ADR-0055) and **collect-all** error
aggregation with composite reason on total failure.

User-confirmed semantics (handoff cycle-27 §3, reproduced verbatim):

- `max_concurrency == 0` → unbounded (cycle-26/27 back-compat semantics where
  the field was ignored)
- `max_concurrency == 1` → sequential (zero thread spawn)
- `max_concurrency >= 2` → semaphore-gated thread pool
- Outcome: `Succeeded` if ≥1 body iteration succeeded; `Failed` only if ALL failed
- Aggregation keys: `outputs["results"]` (successes only, iteration order) +
  `outputs["failures"]` (all failures with `{index, reason}` regardless of outcome)

## Decision

### D-1: `max_concurrency` semantic (Map-specific, divergent from Parallel)

`Map` MUST honor its `max_concurrency` field as follows:

| `max_concurrency` | Behavior | Thread spawn? |
|---|---|---|
| `0` | **unbounded** — semaphore initialized with `items.len().max(1)` permits | yes (one thread per iteration) |
| `1` | **sequential** — preserve cycle-27 sequential loop | no |
| `>= 2` | **semaphore-gated** — `CountingSemaphore::new(max_concurrency as usize)` | yes |

**Divergence from `Parallel`**: `Parallel` uses `apply_default_max_concurrency(0)`
which returns 16 (default cap). `Map` MUST NOT reuse that helper because Map's
back-compat semantic for `0` is "unbounded" (semaphore = `items.len()`), not
"default-to-16". The local helper `map_max_concurrency_effective(mc, n)` returns
`n.max(1)` when `mc == 0`, otherwise `mc as usize`. This divergence is intentional
and documented here so future readers do not "fix" it.

### D-2: Collect-all error aggregation

Map MUST emit BOTH keys in `outputs`:

| Key | Type | Content | Ordering |
|---|---|---|---|
| `outputs["results"]` | `Array<Value>` | successful body outputs only (compact, no nulls for failed indices) | iteration order |
| `outputs["failures"]` | `Array<{index: u64, reason: string}>` | every failed iteration | iteration order |

Outcome rule:

| Scenario | Outcome |
|---|---|
| ≥1 body iteration returns `Succeeded` | `Succeeded { outputs: {results, failures} }` |
| ALL body iterations return `Failed` (or panic) | `Failed { reason: <composite>, ... }` |
| Any body iteration returns `Pending` | propagate `Pending` immediately (early return; cycle-20 receiver-map compat) |
| Source returns `Failed` | `Failed` with source reason (body not evaluated) |
| Source returns `Pending` / `Running` | propagate as-is (cycle-27 invariant) |

### D-3: Composite reason format (with truncation)

When `all-fail` (D-2 row 2), the `reason` MUST be:

```
map body failed at all N iterations: [0]reason0; [1]reason1; ... [K-1]reasonK-1; ...
```

where `K = min(N, 10)` and `...` (literal three dots) is appended iff `N > 10`.
The full list of failures is preserved in `outputs["failures"]` (no truncation
in the array).

### D-4: Scope guard — DC-MAP-002 deferred to cycle-29

`source.evaluate(ctx)` continues to reuse the parent `OperatorContext` in
cycle-28. Per-iteration child contexts are introduced ONLY for body iterations.
This means:

- Body iterations are thread-isolated (each gets `child_ctx` with per-thread
  scratch store and Arc-cloned `node_run` / `ir` / `run`).
- Source evaluation is still serialized against parent ctx.

This limitation (DC-MAP-002, P2 in cycle-27 debt-report) is **explicitly
deferred to cycle-29**, scoped-out of cycle-28. Acceptable trade-off because:

1. Source is evaluated exactly once per Map execution (not per iteration), so
   the parent-ctx coupling is a single borrow, not a per-iteration hazard.
2. Cycle-28's blast radius is bounded to the body-fan-out path; touching source
   plumbing would risk cycle-27 regressions.

### D-5: Helper divergence from `Parallel` (justification)

The `Parallel` operator (operator.rs lines 680-935) already implements the
exact pattern that Map needs:

- per-thread child context with Arc-cloned shared state
- per-thread scratch `GraphStoreBox { inner: Box::new(ScratchGraphStore) }`
- `CountingSemaphore` + `PermitGuard` (ADR-0055)
- `mpsc::channel<ChildResult>` + `BTreeMap<usize, ChildResult>` aggregator
- `std::panic::catch_unwind` → `OperatorError::ChildPanicked { child_index }`
- `JoinHandle::join` to satisfy INV-9 (no thread leaks)

Cycle-28 adopts this pattern verbatim for Map's body fan-out. The only
divergence is the **outcome aggregation rule**: Parallel collapses all failures
into a single `first_failure` (`OperatorError::EvalFailed`), while Map collects
all per-iteration outcomes into a `failures` array. This is a deliberate
non-uniformity because:

- Parallel's children are heterogeneous operators chosen at graph-build time;
  a single failure is sufficient to abort the join.
- Map's body is homogeneous (the same Task applied N times); partial-success
  semantics are useful for the "process batch, log failures" pattern.

The outcome divergence is captured in the spec
(`REQ-Map-Collect-All-Errors`) and is the user-confirmed semantics.

### D-6: JSON ordering determinism

Both `outputs["results"]` and `outputs["failures"]` MUST preserve iteration
order (not insertion order of the runtime BTreeMap). The implementation MUST
iterate `0..items.len()` and push to `Vec<Value>` / `Vec<{index, reason}>`
in order. Top-level keys in the `outputs` BTreeMap MUST be in sorted order
(cycle-20 INV-8 invariant, preserved).

`outputs["failures"][i]` is a `serde_json::Value::Object` with two keys
(`index`, `reason`) in sorted-key order via `serde_json::Map::from_iter`.

## Alternatives Considered

### A-1: Fail-fast on first failure (cycle-27, rejected for cycle-28)

Map stops on the first body `Failed` and returns `Failed` immediately.
Rejected because the user explicitly requested "log all failures, succeed if
≥1 success" for batch-processing workloads (handoff cycle-27 §3).

### A-2: Cross-tick replay in cycle-28 (rejected — deferred to cycle-29)

Add checkpointing of Map state between iterations so a tick boundary can pause
and resume Map evaluation. Rejected because:

1. Adds non-trivial serialization infrastructure (per-iteration checkpoint
   BTreeMap; runtime-side restore logic).
2. Cycle-28 is A-min (single apply phase, no architectural fork); adding
   checkpointing inflates scope beyond the WU-3 contract.
3. The receiver-map runtime (cycle-20) handles per-node Pending propagation;
   cross-tick replay is orthogonal to concurrency + collect-all.

Deferred to cycle-29 (handoff §3 confirms).

### A-3: Parent-ctx body iteration (rejected)

Keep `body_op.evaluate(ctx)` with the parent `OperatorContext`, sharing it
across all spawned threads. Rejected because:

- `OperatorContext::node_run` is `Arc<Mutex<NodeRun>>`; sharing `&mut`
  references across threads is undefined behavior.
- Would reintroduce DC-MAP-001 (P2 in cycle-27 debt) for the body-fan-out path.
- Breaks INV-10 (no lock on workflow state) — body iterations would contend
  on `node_run` mutex instead of being read-only via Arc clone.

### A-4: Reuse `apply_default_max_concurrency` from `Parallel` (rejected)

Apply the same `0 → 16` default that `Parallel` uses. Rejected because
cycle-26/27 semantics had `max_concurrency == 0` mean "ignored" (i.e., zero
throttling), and the user-confirmed semantic is "unbounded". Defaulting to 16
would silently change behavior for adopters who relied on the cycle-26/27
back-compat (no cap).

## Consequences

### Positive

- Map fans out concurrently with bounded parallelism (no over-spawn on big
  collections).
- Partial-failure semantics enable batch-processing patterns ("process 1000
  items, log the 3 that failed").
- Composite reason preserves all failure context for diagnostics.
- Pattern reuse from `Parallel` (cycle-20) — battle-tested semaphore + child
  context + BTreeMap aggregation. Zero new concurrency primitives.
- Cycle-27 invariants (Task-only body, collection-key convention) preserved.

### Negative

- DC-MAP-002 (source-context coupling) persists as cycle-28 outstanding debt
  (P2 fingerprint, deferred to cycle-29). Acceptable per scope guard D-4.
- Two divergent max_concurrency=0 semantics (Map vs Parallel). Documented
  in D-1 to prevent accidental "unification" in future cycles.
- Composite reason truncation at 10 entries is opinionated. Adopters needing
  the full list must read `outputs["failures"]` (which is always complete).
- Body iteration scratch `GraphStoreBox` per child — minor allocation overhead
  for high-N maps. Mitigated by `Box::new(ScratchGraphStore)` (no heap
  thrash); profiling deferred to cycle-29 if needed.

### Risks

| Risk | Severity | Mitigation |
|---|---|---|
| `CountingSemaphore` permits > workers (e.g., `max_concurrency=100`, 5 items) | Low | Semaphore is over-permissive by design; behavior is natural. Spec covers it in REQ-Map-Max-Concurrency.3. |
| Body panic masks partial successes | Medium | `catch_unwind` → `OperatorError::ChildPanicked` → treated as `Failed { reason: "child i panicked" }` in `failures` (consistent with `Parallel` lines 918-924). |
| JSON key order non-deterministic | Low (DE-MAP-002 P3) | All output maps use `BTreeMap` (sorted keys). `serde_json::Map::from_iter` preserves BTreeMap order. |
| Cycle-28 debt-verify flags DC-MAP-002 as `introduced` | Low | Fingerprint stable; cycle-29 will remediate. Documented in D-4. |

## Cross-phase test patterns

Cycle-28 introduces ~5-6 new tests + 3 doc-oracle updates in
`crates/sddk-engine/tests/map_operator_tests.rs`:

| Test | REQ scenario | RED→GREEN→TRIANGULATE |
|---|---|---|
| `map_max_concurrency_one_runs_sequentially` | REQ-Map-Max-Concurrency.1 | sequential branch, no thread spawn |
| `map_max_concurrency_two_gates_to_two_at_a_time` | REQ-Map-Max-Concurrency.2 | semaphore N=2 + AtomicUsize peak counter |
| `map_max_concurrency_zero_runs_all_in_parallel_unbounded` | REQ-Map-Max-Concurrency.3 | helper `map_max_concurrency_effective(0, n) == n` |
| `map_partial_failures_returns_succeeded_with_failures` (rewrite of cycle-27 `map_item_wrong_type_propagates_failure`) | REQ-Map-Collect-All-Errors.1 | collect-all + any_success flag |
| `map_all_failures_returns_failed_with_composite_reason` | REQ-Map-Collect-All-Errors.2 | composite reason builder |
| `map_composite_reason_truncates_at_ten` | REQ-Map-Collect-All-Errors.3 | `take(10)` + elision marker |

Plus 3 doc-oracle updates:

| Test | Change |
|---|---|
| `map_docstring_lists_max_concurrency` | assert NOT `"IGNORED"` |
| `map_docstring_lists_first_failure` → `map_docstring_lists_collect_all` | rename + assert `"collect-all"` |
| `map_docstring_lists_cross_tick_replay` | unchanged (cycle-29 still deferred) |

## INV Preservation

- INV-1..INV-9 unchanged (Map fan-out is purely additive to existing patterns).
- INV-10 preserved: `CountingSemaphore` reuse (ADR-0055); no new `Mutex<usize>`
  on workflow state; per-thread scratch stores are per-child `GraphStoreBox`.
- INV-11 (deterministic replay): cycle-28 does NOT add cross-tick replay. The
  receiver-map runtime still treats Map as a single-shot operator; cycle-29
  will address checkpointable replay.

## References

- ADR-0061 — Map stub (cycle-26, limitations (2) and (4))
- ADR-0062 — Map source plumbing (cycle-27, deferred rows 1 and 2)
- ADR-0055 — P3 closure (CountingSemaphore retained, cycle-21)
- ADR-0050 — True concurrent Parallel (cycle-20, pattern template)
- REQ-Map-Max-Concurrency-Errors.md (cycle-28, this ADR's spec)
- REQ-Map-Source-Evaluation.md (cycle-27, collection-key convention)
- HANDOFF-2026-08-26-cycle-27-source-operator-evaluation.md §3
- `crates/sddk-engine/src/operator.rs` lines 680-935 (Parallel template),
  1037-1166 (Map current), 552-592 (CountingSemaphore + PermitGuard),
  535-537 (`apply_default_max_concurrency` — NOT to be reused)
