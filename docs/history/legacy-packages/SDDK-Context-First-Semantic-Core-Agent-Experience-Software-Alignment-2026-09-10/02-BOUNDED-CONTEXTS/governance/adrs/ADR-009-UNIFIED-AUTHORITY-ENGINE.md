# ADR-009 — Unified authority/admission engine

**Status:** Proposed

## Decision

All governed side effects pass through one logical Authority Engine:

```text
ActionProposal + Actor + Facts + PolicySnapshot
        ↓
AdmissionDecision
  = Allow | Deny(reason) | RequireApproval(requirement)
```

Permission checks, risk policy, gates, role contracts and approvals are inputs/rules/facts. They do not independently execute effects.

Capabilities execute only after Allow and emit receipts/evidence/postconditions.

## Consequences

A single audit path answers “why was this action allowed?”. Packs add policy rules through bounded extension points.

## 2026-09-10 amendment

Software Alignment is advisory and does not call effects directly. A project policy may choose to turn an explicit ContractViolation into a gate, but the admission decision remains AuthorityEngine responsibility.
