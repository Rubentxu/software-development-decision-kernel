---
id: ADR-0112-TYPED-ARCHITECTURAL-CONTRACTS
status: proposed
supersedes_history: false
proposed_at: 2026-09-14
accepted_at: null
accepted_by_cycle: null
implementation_evidence: []
superseded_by: []
related_adrs:
  - "ADR-0097-COMMON-REVISION-SUBSTRATE"
  - "ADR-0100-UNIVERSAL-EVIDENCE"
  - "ADR-0102-UNIFIED-AUTHORITY-ENGINE"
stale_after: 2027-03-14
---

# ADR-0112 — Typed Architectural Contracts

## Context

SDDK stores decisions/specifications and can verify code, but architecture intent is still mostly prose. Conformance audits therefore reconstruct claims manually from ADRs and tests.

## Decision

Introduce a small typed `ArchitecturalContract` model owned by Decision/Knowledge interfaces. An accepted decision may publish contracts such as single-authority, unique-owner, forbidden-dependency, projection-only, bounded-compatibility and provider-boundary constraints.

The contract is the machine-verifiable claim. The ADR remains the rationale.

Contracts SHALL be revisioned, basis-addressed and traceable to their Decision/Spec source. Markdown MAY be an ingestion/authoring surface but SHALL NOT become runtime authority after validated semantic objects exist.

## Consequences

- Verify can compute affected claims rather than rerun a vague repository audit.
- receipts can say exactly which accepted contract is proven/unknown/contradicted;
- architecture fitness becomes derivable from decisions instead of a disconnected lint registry;
- not every ADR needs a contract; only regression-relevant, verifiable claims should become executable.

## Replaces

This decision replaces ad-hoc prose-only architectural constraints that require agents to rediscover their executable meaning on every audit.
