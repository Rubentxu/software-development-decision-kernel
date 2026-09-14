---
id: arch-spec-037-reactive-conformance-loop
status: proposed
proposed_at: 2026-09-14
source: docs/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/
supersedes_history: false
---

# arch-spec-037 — Reactive Conformance Loop

## Intent

Material changes and new observations SHALL be able to trigger bounded conformance re-evaluation without replacing explicit Workflow/Run lifecycle.

## Requirements

- AC-037-001: material change sets are coalesced before contract impact analysis.
- AC-037-002: ephemeral host/read events do not become canonical facts by default.
- AC-037-003: reactive verifiers consume behavior-local graph/evidence slices.
- AC-037-004: KMT/freshness identifies affected units/contracts.
- AC-037-005: JCode adapters deliver useful ArchitectureConformanceDelta through generic Agentic API, not host-specific domain types.
- AC-037-006: normal Alignment tension is advisory/contextual unless explicit Governance policy says otherwise.
- AC-037-007: provider events enter through SDDK-owned ports and become evidence, never direct truth mutation.

## Acceptance

AC-UAT-020..023, 030..033.
