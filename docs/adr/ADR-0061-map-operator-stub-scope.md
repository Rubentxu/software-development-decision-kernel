# ADR-0061 — Operator::Map stub (cycle-26)

**Status:** superseded-by-ADR-0062 (limitations 1, 3) + ADR-0063 (limitations 2, 4)
**Date:** 2026-08-24 (original) / 2026-08-24 (supersession marker)
**Cycle:** 26 (A-min)
**Trigger:** Phase 4 (Epic DW) WU-1

> **Supersession note (2026-08-24, cycle-28 housekeeping):** Limitations (2)
> "max_concurrency ignored" and (4) "error aggregation first-failure" listed
> below were promoted to in-scope by [ADR-0062](ADR-0062-map-source-plumbing.md)
> (cycle-27 source plumbing) and finalized by
> [ADR-0063](ADR-0063-map-max-concurrency-error-aggregation.md) (cycle-28 max
> concurrency + collect-all aggregation). This ADR remains the canonical record
> of the cycle-26 stub contract; do not edit prior content.

---

## Context

Epic DW Map/Join/Race/Loop operators are the foundation of dynamic workflows
(per ROADMAP.md Phase 4 exit criteria: "a discovery node can create N runtime
work units after workflow start and replay reconstructs the same graph").

The runtime currently returns `OperatorError::NotImplementedInCycle16` for
the Map variant (operator.rs:1079-1081).

## Decision

### WU-1: Add Map runtime struct

Add `pub struct Map { source, body, max_concurrency }` to operator.rs and
implement `Operator::evaluate(ctx)` as a **bounded stub**:

| behavior | value | rationale |
|---|---|---|
| source evaluation | IGNORED | source plumbing requires runtime inputs/outputs (cycle-27+) |
| N | 3 (hardcoded) | arbitrary but bounded for testing |
| body inputs | `{"item": Null, "index": 0..2}` | shape contract documented but not enforced |
| max_concurrency | IGNORED | concurrency throttling deferred (cycle-28+) |
| error aggregation | first-failure | conservative; collect-all deferred |
| order | preserved (sequential evaluation) | deterministic for replay |

### WU-2 to WU-5: flip dispatch, mod tests, re-export, docstring

Standard housekeeping. Map becomes in-scope; 7 arms remain out-of-scope.

### WU-6: RED tests

`tests/map_operator_tests.rs` — 2-3 minimal tests for the stub contract.

## Consequences

### Positive
- 1 of 8 deferred operators promoted from `NotImplemented` to `Implemented`
- Foundation laid for cycle-27 (source plumbing) and cycle-28 (concurrency throttling)
- Mod test infrastructure for adding remaining 7 operators is now established

### Negative (limitations documented)
- Source is not evaluated — bodies run against a stubbed N=3 collection
- max_concurrency is ignored — bodies run unbounded in parallel
- No error aggregation — first failure aborts
- No cross-tick checkpoint support
- Body recursion not guarded (if body is itself Map, infinite loop possible)

### Out of scope (deferred to cycle-27+)
- Source operator evaluation (requires OperatorContext inputs/outputs)
- Concurrency throttling (cycle-20 semaphore reusable)
- Collection type semantics (Vec vs BTreeSet vs custom)
- Cross-tick deterministic replay
- Join, Race, Loop, Gate, Wait, SubWorkflow, Compensate

## INV Preservation

- INV-1..INV-12 unchanged (Map is purely additive)
- INV-2 dyn GraphStore preserved
- INV-10 Arc<Mutex<NodeRun>> field types unchanged
- Domain Operator enum unchanged

## References

- ROADMAP.md Phase 4 — Dynamic Workflow Engine
- BACKLOG.md Epic DW — item 5
- cycle-26 explore-report.md
- ADR-0052 (Parallel channel design, cycle-20)
- [ADR-0062](ADR-0062-map-source-plumbing.md) — Map source plumbing (cycle-27)
- [ADR-0063](ADR-0063-map-max-concurrency-error-aggregation.md) — Map max_concurrency + collect-all (cycle-28)

## Supersession Log (append-only)

| Date | Cycle | Action | Linked ADR |
|---|---|---|---|
| 2026-08-24 | 27 | Limitation (1) source-evaluation + (3) body-Inputs-Injection promoted to in-scope | [ADR-0062](ADR-0062-map-source-plumbing.md) |
| 2026-08-24 | 28 | Limitation (2) max_concurrency + (4) error-aggregation promoted to in-scope | [ADR-0063](ADR-0063-map-max-concurrency-error-aggregation.md) |
| (open) | 29 | Limitation (5) cross-tick replay — deferred | — |
