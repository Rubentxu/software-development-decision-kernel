# HANDOFF-2026-08-26-cycle-26-map-operator-stub

## Cycle-26 Summary

Cycle-26 promotes `Operator::Map` from `NotImplementedInCycle16` to a bounded
stub in sddk-engine. First of the Phase 4 (Epic DW) operator implementations.

## What was done

- WU-1: Added `pub struct Map { source, body, max_concurrency }` + `impl Operator` stub (returns Succeeded)
- WU-2: Flipped `dispatch()` Map arm from `Err` to `Ok(Arc::new(Map { ... }))`
- WU-3: Flipped mod test `dispatch_maps_map` + added `map_implements_operator`
- WU-4: Re-export Map via `lib.rs` named export list
- WU-5: Updated docstring: 5 in-scope, 7 out-of-scope
- WU-6: Added `tests/map_operator_tests.rs` (3 RED tests)
- WU-7: Wrote ADR-0061 documenting stub limitations
- WU-8: This handoff
- WU-9: Version bump 1.42.4 → 1.42.5

## Limitations (per ADR-0061)

| behavior | value | cycle to fix |
|---|---|---|
| source evaluation | IGNORED | 27 |
| max_concurrency | IGNORED | 28 |
| N | hardcoded 3 | 27 |
| error aggregation | first-failure | 28 |
| cross-tick replay | unsupported | 29 |

## 7 remaining operators

Join, Race, Loop, Gate, Wait, SubWorkflow, Compensate — still
`NotImplementedInCycle16`. Each gets its own cycle.

## Next session

Cycle-27 (A-min): source operator evaluation (real OperatorContext inputs/outputs plumbing)
