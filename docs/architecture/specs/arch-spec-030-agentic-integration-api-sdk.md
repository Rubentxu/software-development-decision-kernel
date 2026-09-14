---
id: arch-spec-030-agentic-integration-api-sdk
status: proposed
proposed_at: 2026-09-14
source: docs/SDDK-Production-Readiness-Alignment-2026-09-14/
supersedes_history: false
---

# arch-spec-030 — Agentic Integration API/SDK

## Intent

SDDK SHALL expose a stable integration boundary for agent hosts so external adapters do not import SDDK internals or depend on storage/layout implementation details.

## Requirements

### AIS-001 — Public semantic boundary

The integration API/SDK SHALL expose SDDK-owned contracts for sessions/bindings, context delivery, host events/actions, structured work, capability negotiation, permission mediation and receipts.

### AIS-002 — Internal isolation

External adapters SHALL NOT depend on private `sddk-domain`, engine/storage internals, migration types or concrete persistence implementation.

### AIS-003 — Transport neutrality

The public semantic contract SHALL be independent of transport. Local socket, stdio child or a future remote transport are implementation options.

### AIS-004 — On-demand activation

A permanently running SDDK daemon SHALL NOT be required by the architecture. A host integration MAY spawn/connect to an on-demand integration endpoint.

### AIS-005 — Packaging discipline

Candidate crates such as `sddk-agentic-api` and `sddk-agentic-sdk` SHALL exist only where the public release/dependency boundary justifies them. Internal bounded-context crate proliferation remains evidence-driven.

### AIS-006 — Compatibility

API/SDK SemVer SHALL be independent of SDDK product release and host product release. Compatibility receipts SHALL record the exact tested tuple.

### AIS-007 — Fake host conformance

The boundary SHALL ship transport/client/server/fake-host tests so host adapters can be developed without importing a real host runtime.

### AIS-008 — Stability gate

The generic API SHALL remain pre-1.0/experimental until at least one second-host validation demonstrates that JCode-specific assumptions did not leak into the generic contract.

## Acceptance

`AW-UAT-010..014`, `AW-UAT-090..092`.
