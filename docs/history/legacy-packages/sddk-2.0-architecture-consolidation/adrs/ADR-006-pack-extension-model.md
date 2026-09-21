# ADR-006 — Packs as Primary Extension Model

**Status:** Proposed  
**Date:** 2026-08-11

## Context

SDDK capability surface spans SDD, testing, UAT, research, UI and other domains, risking core sprawl.

## Decision

Promote PackManifest into a real runtime extension contract with requires/integrates_with/provides/conflicts semantics.

## Consequences

### Positive

- Small stable kernel.
- Optional capabilities.
- Independent evolution and distribution.

### Trade-offs / risks

- Dependency resolution/versioning complexity.
- Requires strong conformance checks.

## Implementation notes

Implement manifest v2 validation, pack registry/lifecycle, fixtures and first-party UAT pack before migrating every domain.

## Revisit trigger

Revisit packaging/distribution mechanism after at least three independently versioned packs exist.
