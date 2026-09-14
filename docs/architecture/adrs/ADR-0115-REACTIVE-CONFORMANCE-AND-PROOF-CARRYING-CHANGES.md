---
id: ADR-0115-REACTIVE-CONFORMANCE-AND-PROOF-CARRYING-CHANGES
status: proposed
supersedes_history: false
proposed_at: 2026-09-14
accepted_at: null
accepted_by_cycle: null
implementation_evidence: []
superseded_by: []
related_adrs:
  - "ADR-0094-ONE-CANONICAL-FACT-LOG"
  - "ADR-0098-ONE-SEMANTIC-GRAPH"
  - "ADR-0112-TYPED-ARCHITECTURAL-CONTRACTS"
stale_after: 2027-03-14
---

# ADR-0115 — Reactive Conformance and Proof-Carrying Changes

## Context

JCode/reactive workspace integration will produce bounded change events. Re-running full architecture audits after every change is noisy and expensive.

## Decision

Use KMT/graph impact to map material change sets to affected architectural contracts, then run delta-scoped Verify. The result is an `ArchitectureConformanceDelta` that may be delivered to agents as advisory context.

A structured change MAY carry a `ChangeConformanceEnvelope` referencing the verification receipt/evidence for affected contracts. This envelope does not authorize merge or release by itself.

`turn_done` or provider events are debounce/input boundaries, not lifecycle authority.

## Consequences

- architecture feedback becomes incremental and relevant;
- JCode host adapters do not reimplement Verify/Alignment;
- Governance may explicitly require conformance receipts for selected operations while preserving separation of authority;
- routine reads/grep/list events remain ephemeral.

## Replaces

This replaces full-repository conformance reruns as the default response to every material edit.
