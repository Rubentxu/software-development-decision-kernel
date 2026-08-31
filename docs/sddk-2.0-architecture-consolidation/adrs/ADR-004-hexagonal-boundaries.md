# ADR-004 — Enforce Hexagonal Boundaries Structurally

**Status:** Proposed  
**Date:** 2026-08-11

## Context

The current engine directly depends on storage and the CLI has infrastructure responsibilities, weakening the intended architecture.

## Decision

Create/clarify an application layer owning ports and use cases. Concrete adapters depend inward. Enforce dependency rules with architecture lint.

## Consequences

### Positive

- Better testability and replaceability.
- Smaller CLI.
- Graph/event backends can evolve independently.

### Trade-offs / risks

- Refactor touches many constructors and tests.
- Temporary facades will add short-lived duplication.

## Implementation notes

First break engine->storage with a port. Then move persistence orchestration out of CLI. Add ARCH001-ARCH005 ratchets.

## Revisit trigger

Revisit individual exceptions only with measurable cost and a time-bounded ADR.
