# ADR-003 — Separate intent, planning, delivery and execution lifecycles

**Status:** Proposed

## Decision

Use the canonical chain `Goal → WorkItem → WorkflowDefinition → ExecutablePlan → Run`.

`Cycle` is a delivery/change container. It MUST NOT encode transient runtime states such as waiting for a specific gate, retry or agent attempt. Those belong to Run facts/projections.

Execution Spine is a desired planning manifest/import format; Planning Ledger is the operational planning authority after reconciliation.

## Consequences

Cycle status can shrink over migrations. Planning and runtime can evolve independently. A single goal may produce multiple work items/runs without semantic contortions.
