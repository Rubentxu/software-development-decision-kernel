---
id: arch-spec-027-structured-agent-work
status: proposed
proposed_at: 2026-09-14
source: docs/history/legacy-packages/SDDK-Production-Readiness-Alignment-2026-09-14/
supersedes_history: false
---

# arch-spec-027 — Structured Agent Work

## Intent

Orchestrated agent work SHALL use typed request/return contracts where the host supports structured execution, avoiding markdown parsing as an architectural protocol.

## Requirements

### SAW-001 — Typed request

SDDK SHALL express orchestrated work as an `AgentWorkRequest` containing task/context/policy/return-contract references without embedding host-specific request types in domain contracts.

### SAW-002 — Schema-constrained result

A capable host SHOULD execute against an explicit return schema and produce a typed `ContributionV2` or equivalent SDDK-owned ADT.

### SAW-003 — Invalid output is visible

Schema failure, cancellation, timeout or incomplete host output SHALL remain distinguishable. SDDK SHALL NOT fabricate a successful Contribution from malformed output.

### SAW-004 — Same adapter, two modes

Companion and orchestrated usage SHALL share the same semantic adapter/boundary. A second integration stack solely for orchestrated mode is prohibited.

### SAW-005 — Contribution is not authority

A host/agent Contribution is an input to Synthesis/Decision/Evidence. It SHALL NOT mutate Decision Memory, Verification truth or Governance authority directly.

### SAW-006 — Receipt

Structured execution SHALL contribute to an AgentExecutionReceipt containing the relevant task/context/instruction/host compatibility basis and result provenance.

## Acceptance

`AW-UAT-060..063`.
