# ADR-0057 — tick() phase extraction (cycle-23)

**Status:** accepted
**Date:** 2026-08-24
**Cycle:** 23 (A-min)
**Trigger:** cycle-20 debt-verify ARCH-LONG-METHOD-TICK (P1 medium)

---

## Context

cycle-21 introduced 3-phase tick() (DRAIN/SPAWN/LEGACY) but kept the 436-LOC body
in a single function. The debt-verify flagged this as a future regression risk.

## Decision

Extract 3 helper methods + thin orchestrator:

```rust
pub fn tick(&mut self) -> Result<TickOutcome> {
    if self.run.state != WorkflowRunState::Running { ... }
    let drain = self.drain_pending_parallel();
    let mut spawn = self.spawn_pending_and_ready(&drain);
    spawn.merge(drain);
    self.apply_outcomes_to_state(&spawn.outcomes);
    if spawn.any_failed { Ok(TickOutcome::Failed) }
    else if spawn.all_done { Ok(TickOutcome::AllComplete) }
    else { Ok(TickOutcome::Running) }
}
```

New struct `TickPhaseOutcome { outcomes, all_done, any_failed }` aggregates per-phase results.

## Consequences

### Positive
- tick() reduced from 436 LOC to ~21 LOC orchestrator
- Each helper is independently testable
- Phase 4 (Epic DW: Map/Join/Race/Loop) can extend each helper without touching the orchestrator

### Negative
- Indirection (3 function calls per tick)
- Borrow checker interactions may need explicit handling

### Trade-offs accepted
- Helpers are private (`fn` not `pub fn`) — internal refactor, not public API change
- `TickPhaseOutcome` is `struct` not `pub struct` — internal type

## INV preservation

- INV-1..INV-12 unchanged (pure refactor)
- INV-9 WARN log (cycle-22 fix) preserved in SPAWN phase
- INV-10 Arc<Mutex<NodeRun>> field type unchanged

## References
- cycle-20 debt-report.md §ARCH-LONG-METHOD-TICK
- cycle-21 HANDOFF (3-phase tick origin)
- cycle-22 fix (INV-9 WARN log preserved)
- workflow_runtime.rs:381-816 (pre-refactor)
