# Architecture Specifications Index

This directory contains repository-native architecture specifications.

## Normative succession

The currently adopted 2026-09-09 canonical architecture remains the baseline until its C7 closeout/reconciliation is complete. The 2026-09-14 production-readiness package does not silently replace that baseline; it adds an explicit convergence/evolution layer and repository-native specifications that become active according to their status and acceptance evidence.

Current convergence package:

`docs/SDDK-Production-Readiness-Alignment-2026-09-14/`

Execution order and priorities:

`docs/SDDK-Production-Readiness-Alignment-2026-09-14/02-MINI-ROADMAP.md`

Proposal/track disposition:

`docs/SDDK-Production-Readiness-Alignment-2026-09-14/06-PROPOSAL-DISPOSITION-REGISTER.md`

## 2026-09-14 repository-native specifications

### Production convergence / providers

- `arch-spec-019-production-readiness-convergence.md`
- `arch-spec-020-storage-schema-ownership.md`
- `arch-spec-021-intelligence-provider-boundary.md`

### Agentic Workspace / host integration

- `arch-spec-022-agentic-workspace-boundary.md`
- `arch-spec-023-host-capability-negotiation.md`
- `arch-spec-024-agentic-session-binding.md`
- `arch-spec-025-reactive-host-event-bridge.md`
- `arch-spec-026-context-delta-delivery.md`
- `arch-spec-027-structured-agent-work.md`
- `arch-spec-028-permission-interruption-bridge.md`
- `arch-spec-029-workspace-locality.md`
- `arch-spec-030-agentic-integration-api-sdk.md`
- `arch-spec-031-jcode-anti-corruption-layer.md`

## Status rule

A specification marked `proposed` is not evidence that its implementation exists. Its roadmap/UAT/receipt must close before the corresponding capability or GA/readiness declaration can be made.

Do not infer implementation status from document presence alone.
