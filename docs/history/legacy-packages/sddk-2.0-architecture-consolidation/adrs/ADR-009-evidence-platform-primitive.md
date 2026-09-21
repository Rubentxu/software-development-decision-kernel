# ADR-009 — Evidence as a Platform Primitive

**Status:** Proposed  
**Date:** 2026-08-11

## Context

UAT has rich evidence kinds that are equally useful for architecture, security, release and agent evaluation.

## Decision

Extract a reusable evidence/provenance model and let UAT reference it.

## Consequences

### Positive

- Consistent assurance model.
- Enables graph provenance and stale propagation.

### Trade-offs / risks

- Requires careful redaction/storage rules.

## Implementation notes

Define EvidenceItem, Claim/Observation, Oracle, Verdict and receipt links. Migrate UAT without changing user-visible evidence semantics.

## Revisit trigger

Revisit storage representation after evidence volume and binary-artifact patterns are measured.
