# SPEC-002 — Goal, WorkItem, Cycle, Plan and Run lifecycle

## Definitions

- Goal: durable intent/outcome sought.
- WorkItem: planning unit required to satisfy a goal.
- Cycle: bounded change/delivery container grouping work and releases.
- WorkflowDefinition: declarative process/template.
- ExecutablePlan: compiled revisioned execution DAG.
- Run: one execution instance of a plan.

## Requirements

- LC-001: runtime waiting/retry/approval/UAT states belong to Run, not Cycle.
- LC-002: Spine imports/reconciles WorkItems but is not operational authority after reconciliation.
- LC-003: one Cycle may contain multiple Runs and WorkItems.
- LC-004: plan revisions are immutable and reference their semantic parent(s).
- LC-005: Run projections are derived from canonical run events.

## Compatibility

Legacy CycleStatus values remain readable until migration is complete and map to derived Run/summary state.
