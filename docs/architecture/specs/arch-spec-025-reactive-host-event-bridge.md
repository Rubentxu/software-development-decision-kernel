---
id: arch-spec-025-reactive-host-event-bridge
status: proposed
proposed_at: 2026-09-14
source: docs/SDDK-Production-Readiness-Alignment-2026-09-14/
supersedes_history: false
---

# arch-spec-025 — Reactive Host Event Bridge

## Intent

Agentic host integration SHALL be event-driven and materially selective. Host event volume MUST NOT create a second canonical event stream or trigger unbounded verification work.

## Requirements

### RHB-001 — Event normalization

Host-native events SHALL terminate at an adapter and be normalized into SDDK-owned semantic host events/change sets before entering SDDK workflows.

### RHB-002 — Materiality classification

Events SHALL be classified by semantic relevance. Routine read/search/list/housekeeping is normally EPHEMERAL; source mutation, relevant build/test result, commit/change-set, permission outcome affecting feasible execution, turn completion or structured contribution MAY be MATERIAL.

### RHB-003 — No Canonical Event Log flood

Host telemetry/ephemeral events SHALL NOT be mirrored mechanically into the Canonical Event Log.

### RHB-004 — Semantic debounce

Bursts of related edits SHALL coalesce into a bounded `WorkspaceChangeSet` before expensive verification. `turn_done` SHOULD be used as the normal interactive coalescing boundary when available.

### RHB-005 — Incremental verification

Material change sets SHALL update KMT/freshness and invoke delta-scoped Verify/VerifyPreflight. They SHALL NOT imply unconditional whole-repository scans.

### RHB-006 — Provider deepening is optional

CogniCode/Chronos MAY deepen evidence based on risk/uncertainty/Task requirement. Their absence SHALL remain `NOT_EVALUATED`/EvidenceGap and SHALL NOT break Base host integration.

### RHB-007 — Existing watches are observability surfaces

`ledger watch` and `diff-watch` MAY aid debugging/operator workflows but SHALL NOT become the host integration transport or semantic authority.

### RHB-008 — Attention levels

Reactive output SHALL distinguish `SILENT`, `CONTEXTUAL`, `ATTENTION`, `INTERRUPT`. Normal Alignment tension SHALL NOT automatically become `INTERRUPT`.

## Acceptance

`AW-UAT-040..053`.
