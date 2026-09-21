# ADR-014 — Signed Local Gate Receipts with Remote Verification

**Status:** Proposed  
**Date:** 2026-08-11

## Context

SDDK intentionally values local validation, but protected shared workflows still need independent verification.

## Decision

Generate signed local gate receipts and allow CI/release infrastructure to verify commit binding, required gates and provenance.

## Consequences

### Positive

- Preserves local-first execution.
- Adds independent trust boundary.

### Trade-offs / risks

- Signing identity and freshness policy need definition.

## Implementation notes

Extend current release attestation/cosign path rather than inventing a parallel format. Prototype `sddk dev check --attest`.

## Revisit trigger

Revisit trust policy when multi-user/team adoption requirements are clearer.
