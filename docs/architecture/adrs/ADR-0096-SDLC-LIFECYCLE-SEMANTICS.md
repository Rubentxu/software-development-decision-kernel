---
id: ADR-0096-SDLC-LIFECYCLE-SEMANTICS
package_local_id: ADR-003
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-003-SDLC-LIFECYCLE-SEMANTICS.md
status: accepted
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
accepted_at: 2026-09-12
accepted_by_cycle: "p-63676b11dc0ef88f/adr-promotion-m0-meta-and-batch-1"
implementation_evidence:
  - "crates/sddk-engine/src/planning/ — Goal/WorkItem types per chain"
  - "crates/sddk-engine/src/workflow_runtime.rs — WorkflowDefinition + Run"
  - "crates/sddk-engine/src/execution_frontier.rs — ExecutablePlan"
  - "docs/architecture/specs/arch-spec-002-lifecycle-model.md — spec"
superseded_by: []
related_adrs:
  - "ADR-0001-ADR-PROMOTION-PROCESS"
  - "ADR-0101-AGENT-OUTCOME-CONTRIBUTION-SYNTHESIS"
stale_after: 2027-09-12
---

# ADR-0096 — SDLC-LIFECYCLE-SEMANTICS

> **Mirror of package ADR `ADR-003`.** Repository-native numbering is `ADR-0096-SDLC-LIFECYCLE-SEMANTICS` (range ADR-0094..0110). Per SUPERSESSION.md, the package's original ADR ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

> **Status:** accepted (2026-09-12) per cycle `p-63676b11dc0ef88f/adr-promotion-m0-meta-and-batch-1`. Lifecycle defined by ADR-0001 §3.1.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `ADR-003` |
| Repository native | `ADR-0096-SDLC-LIFECYCLE-SEMANTICS` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-003-SDLC-LIFECYCLE-SEMANTICS.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# ADR-003 — Separate intent, planning, delivery and execution lifecycles

**Status:** Proposed

## Decision

Use the canonical chain `Goal → WorkItem → WorkflowDefinition → ExecutablePlan → Run`.

`Cycle` is a delivery/change container. It MUST NOT encode transient runtime states such as waiting for a specific gate, retry or agent attempt. Those belong to Run facts/projections.

Execution Spine is a desired planning manifest/import format; Planning Ledger is the operational planning authority after reconciliation.

## Consequences

Cycle status can shrink over migrations. Planning and runtime can evolve independently. A single goal may produce multiple work items/runs without semantic contortions.
