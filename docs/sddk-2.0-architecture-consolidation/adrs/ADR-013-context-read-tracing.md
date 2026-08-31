# ADR-013 — Optional Context-Read Tracing

**Status:** Proposed  
**Date:** 2026-08-11

## Context

Users need to understand what evidence/context an agent used without storing private reasoning traces.

## Decision

Record bounded context.read bookkeeping events linking executions to artifacts/graph objects read.

## Consequences

### Positive

- Causal explainability.
- Enables stale decision detection.

### Trade-offs / risks

- Trace volume/privacy concerns.

## Implementation notes

Opt-in initially; IDs/hashes only, bounded count, non-triggering bookkeeping events.

## Revisit trigger

Revisit default sampling after storage/utility measurements.
