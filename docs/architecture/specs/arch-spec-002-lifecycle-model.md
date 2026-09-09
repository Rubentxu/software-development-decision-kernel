---
id: arch-spec-002-lifecycle-model
package_local_id: SPEC-002
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-002-LIFECYCLE-MODEL.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# arch-spec-002 — LIFECYCLE-MODEL

> **Mirror of package SPEC `SPEC-002`.** Repository-native identifier is `arch-spec-002-lifecycle-model` (range arch-spec-001..018). Per SUPERSESSION.md, the packages original SPEC ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `SPEC-002` |
| Repository native | `arch-spec-002-lifecycle-model` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-002-LIFECYCLE-MODEL.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

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
