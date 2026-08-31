# ADR-005 — Extract UAT as a Bounded Context and First Pack

**Status:** Proposed  
**Date:** 2026-08-11

## Context

UAT has become feature-rich and is concentrated in very large CLI/domain modules.

## Decision

Treat UAT as a bounded context with domain/application/adapters/web boundaries and use it as the first serious pack extraction.

## Consequences

### Positive

- Reduces God modules.
- Validates pack architecture on a real complex domain.
- Preserves guided runner investment.

### Trade-offs / risks

- Migration risk around CLI compatibility and schemas.

## Implementation notes

Keep current commands as facades while moving use cases. Extract universal evidence/identity dependencies rather than duplicating them inside UAT.

## Revisit trigger

Revisit crate granularity if compilation/developer overhead exceeds modularity benefits; bounded-context boundary remains.
