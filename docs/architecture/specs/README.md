# Architecture Specifications Index

This directory contains repository-native architecture specifications.

## Normative succession

Single documented succession path (`PR-GAP-010`):

```text
2026-09-09 semantic-core + agent-experience baseline
  -> C0..C7 conformance closeout (A1 receipt)
  -> 2026-09-10 context-first roadmap (adopted after the C7 receipt)
  -> 2026-09-14 production-readiness convergence / provider / agentic layers
  -> per-profile readiness receipts
```

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

## Spec status crosswalk (`PR-GAP-010`)

Every native spec is currently `proposed`. That is deliberate, not an omission: a spec's status is **evidence**, not intent, so it only advances when the named receipt exists at a specific commit. The reason each spec remains `proposed` is recorded here so no spec is `proposed` "without reason".

### 2026-09-09 baseline specs (SPEC-001..018)

Implementation is known to exist in the codebase, but conformance is **not yet certified**. These stay `proposed` until the 09/09 C7 closeout (roadmap A1) produces `09-09-CONFORMANCE-RECEIPT.md` marking each `PASS`/`PASS_WITH_COMPAT` at a named commit. Flipping them to a green status before that receipt would fabricate evidence.

| Spec | Package id | Status | Reason it is `proposed` |
|---|---|---|---|
| `arch-spec-001` | SPEC-001 | proposed | Awaiting C7 conformance receipt (A1) |
| `arch-spec-002` | SPEC-002 | proposed | Awaiting C7 conformance receipt (A1) |
| `arch-spec-003` | SPEC-003 | proposed | Awaiting C7 conformance receipt (A1) |
| `arch-spec-004` | SPEC-004 | proposed | Awaiting C7 conformance receipt (A1) |
| `arch-spec-005` | SPEC-005 | proposed | Awaiting C7 conformance receipt (A1) |
| `arch-spec-006` | SPEC-006 | proposed | Awaiting C7 conformance receipt (A1) |
| `arch-spec-007` | SPEC-007 | proposed | Awaiting C7 conformance receipt (A1) |
| `arch-spec-008` | SPEC-008 | proposed | Awaiting C7 conformance receipt (A1) |
| `arch-spec-009` | SPEC-009 | proposed | Awaiting C7 conformance receipt (A1) |
| `arch-spec-010` | SPEC-010 | proposed | Awaiting C7 conformance receipt (A1) |
| `arch-spec-011` | SPEC-011 | proposed | Awaiting C7 conformance receipt (A1) |
| `arch-spec-012` | SPEC-012 | proposed | Awaiting C7 conformance receipt (A1) |
| `arch-spec-013` | SPEC-013 | proposed | Awaiting C7 conformance receipt (A1) |
| `arch-spec-014` | SPEC-014 | proposed | Awaiting C7 conformance receipt (A1) |
| `arch-spec-015` | SPEC-015 | proposed | Awaiting C7 conformance receipt (A1) |
| `arch-spec-016` | SPEC-016 | proposed | Awaiting C7 conformance receipt (A1) |
| `arch-spec-017` | SPEC-017 | proposed | Awaiting C7 conformance receipt (A1) |
| `arch-spec-018` | SPEC-018 | proposed | Awaiting C7 conformance receipt (A1) |

### 2026-09-14 convergence / Agentic specs (arch-spec-019..031)

These were added by the production-readiness package and are `proposed` because their implementation milestones and receipts have not closed yet.

| Spec | Status | Reason it is `proposed` |
|---|---|---|
| `arch-spec-019` production-readiness-convergence | proposed | Convergence program in progress (A0..A5) |
| `arch-spec-020` storage-schema-ownership | proposed | Awaiting convergence receipts; PR-GAP-005 code evidence exists but the spec gate closes with Base |
| `arch-spec-021` intelligence-provider-boundary | proposed | Awaiting A6/A7 provider tracks |
| `arch-spec-022` agentic-workspace-boundary | proposed | Awaiting J1/J2 track |
| `arch-spec-023` host-capability-negotiation | proposed | Awaiting J3/J8 track |
| `arch-spec-024` agentic-session-binding | proposed | Awaiting J3 track |
| `arch-spec-025` reactive-host-event-bridge | proposed | Awaiting J5 track |
| `arch-spec-026` context-delta-delivery | proposed | Awaiting J4 track |
| `arch-spec-027` structured-agent-work | proposed | Awaiting J6 track |
| `arch-spec-028` permission-interruption-bridge | proposed | Awaiting J8 track |
| `arch-spec-029` workspace-locality | proposed | Awaiting J3 track |
| `arch-spec-030` agentic-integration-api-sdk | proposed | Awaiting J1/J9 track |
| `arch-spec-031` jcode-anti-corruption-layer | proposed | Awaiting J2 track |

Historical/superseded packages remain listed in `docs/architecture/README.md` and do not claim competing current authority.
