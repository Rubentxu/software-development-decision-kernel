# ADR-0062 — Map source evaluation plumbing (cycle-27)

**Status:** accepted
**Date:** 2026-08-24
**Cycle:** 27 (A-min)
**Trigger:** Phase 4 (Epic DW) WU-2

---

## Context

Cycle-26 (ADR-0061) shipped a bounded stub for `Map::evaluate`:
- `source` was IGNORED (hardcoded N=3)
- body ran 3 times sequentially with `{"item": Null, "index": 0..2}`
- `max_concurrency` ignored

Cycle-27 WU-2 requires real source operator evaluation and per-iteration inputs injection.

## Decision

### Collection Key Convention

The source operator MUST produce `outputs["items"]: serde_json::Value::Array`.

| key | value | rationale |
|---|---|---|
| `outputs["items"]` | `Array` | explicit contract; avoids ambiguity with other output keys |
| missing key | `Failed` with reason containing `expected outputs["items"]` | fail-fast, clear error |
| `null` / non-Array | `Failed` with reason containing `expected outputs["items"]` | fail-fast, type-safe |

### Body Restriction (cycle-27)

The Map body MUST be `DomainOperator::Task` in cycle-27.

| body variant | behavior |
|---|---|
| `Task` | fan-out runs (per-item evaluate) |
| `Sequence`, `Parallel`, `Choice`, `Map`, `Join`, `Race`, `Loop`, `Gate`, `Wait`, `SubWorkflow`, `Compensate` | `Failed{reason: "cycle-27 map body must be Task"}` |

Rationale: Task-only reduces scope for cycle-27. Concurrency + error aggregation (cycle-28) and cross-tick replay (cycle-29) are orthogonal concerns.

### Inputs Injection (per-iteration)

On iteration `i` over `source.items[i]`, the operator MUST merge into body's base inputs:

```rust
iter_task.inputs.insert("item".to_string(), item.clone());
iter_task.inputs.insert("index".to_string(), serde_json::Value::Number(i as u64));
```

- **Non-destructive**: body's base `inputs` map is cloned per iteration before merge
- **0-indexed**: `index` field starts at `0`
- **Ordering preserved**: iteration order matches source array order

### Result Aggregation

Map returns `Succeeded { outputs: { "results": [...] } }` where `results[i]` is the
`outputs` BTreeMap from body iteration `i` serialized as a JSON object.

| scenario | outcome |
|---|---|
| all body runs succeed | `Succeeded{results: [...]}` |
| body run `i` fails | `Failed{reason: "map body failed at iteration i: <reason>"}` |
| source fails | `Failed{reason: "<source reason>"}` (body not evaluated) |
| source Pending/Running | propagated as-is |

### Deferred Limitations (documented in Map docstring)

| limitation | deferred to |
|---|---|
| `max_concurrency` enforcement | cycle-28 |
| error aggregation = collect-all | cycle-28 |
| cross-tick replay support | cycle-29 |

## Consequences

### Positive
- Source operator evaluation is now functional
- body receives real `item` and `index` inputs per iteration
- Non-Task body is rejected at evaluation time (fail-fast)
- Docstring explicitly lists cycle-27 limitations

### Negative (limitations documented)
- `max_concurrency` is still ignored — bodies run unbounded sequentially
- Error aggregation is still first-failure — no partial results on body failure
- Cross-tick replay is unsupported — Map state is not checkpointed between iterations

## INV Preservation

- INV-1..INV-12 unchanged (Map plumbing is additive)
- INV-2 dyn GraphStore preserved
- INV-10 Arc<Mutex<NodeRun>> field types unchanged
- Domain Operator enum unchanged

## References

- ADR-0061 (Map stub, cycle-26)
- REQ-Map-Source-Evaluation.md (5 requirements, 14 scenarios)
- cycle-27 explore-report.md
