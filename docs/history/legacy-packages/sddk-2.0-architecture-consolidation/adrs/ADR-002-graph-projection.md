# ADR-002 — Graph Is a Projection, Not Authority

**Status:** Proposed  
**Date:** 2026-08-11

## Context

A graph is ideal for relationships, impact and pattern detection, but making a graph database authoritative would create another source of truth and backend lock-in.

## Decision

Build the knowledge/evidence graph deterministically from ledger events. GraphStore remains replaceable.

## Consequences

### Positive

- Local-first rebuildability.
- Backend flexibility.
- Clear audit lineage.

### Trade-offs / risks

- Some graph writes require translating intent into domain events first.
- Rebuild cost must be managed with snapshots.

## Implementation notes

Create GraphProjection + GraphStore ports, rebuild command and checkpointing. No domain API may mutate GraphStore directly as authoritative state.

## Revisit trigger

Revisit backend choice when graph size/query latency measurements exceed the in-process implementation envelope.
