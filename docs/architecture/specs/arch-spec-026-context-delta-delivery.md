---
id: arch-spec-026-context-delta-delivery
status: proposed
proposed_at: 2026-09-14
source: docs/SDDK-Production-Readiness-Alignment-2026-09-14/
supersedes_history: false
---

# arch-spec-026 — Context Delta Delivery

## Intent

Agentic sessions SHALL receive compact, attributable context updates without repeatedly replacing full context or allowing advisory information to become executable instruction authority.

## Requirements

### CDD-001 — Bootstrap once per binding basis

A newly established/re-established SessionBinding SHALL receive a compact `SessionBootstrapContext` compiled from the current ContextBasis.

### CDD-002 — Delta after bootstrap

Subsequent material changes SHOULD deliver `ContextDelta` rather than a full ContextCapsule. A delta SHALL identify prior/new basis, provenance and relevance reason.

### CDD-003 — KMT relevance filter

Changes that do not affect the active binding/context requirements SHALL produce no host injection.

### CDD-004 — Advisory boundary

Alignment content SHALL appear only as advisory context. Delivery through a host message API MUST NOT make it an `InstructionSource` or change the EffectiveInstructionSet hash.

### CDD-005 — Duplicate/order safety

Duplicate or stale deltas SHALL be idempotently ignored or explicitly rejected according to sequence/basis semantics. Silent out-of-order context mutation is prohibited.

### CDD-006 — Cache/noise discipline

The integration SHALL avoid full-context resend loops that cause prompt-cache churn, duplicate context and provenance ambiguity.

## Acceptance

`AW-UAT-030..034` plus the Base advisory-hash UAT.
