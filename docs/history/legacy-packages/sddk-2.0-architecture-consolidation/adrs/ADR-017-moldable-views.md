# ADR-017 — Moldable Views over a Single Ledger/Graph

**Status:** Proposed  
**Date:** 2026-08-11

## Context

Architecture, UAT, release and agent debugging need different representations of the same underlying facts.

## Decision

Build view descriptors and multiple projections rather than one fixed graph UI. C4 drill-down, timeline and assurance views share identities.

## Consequences

### Positive

- Task-specific UX.
- Avoids duplicate diagram sources.

### Trade-offs / risks

- Rendering stack can become complex.

## Implementation notes

Start with read-only descriptors and graph/timeline views. Spike high-performance graph and canvas adapters separately.

## Revisit trigger

Revisit renderer choices based on measured graph sizes and editing requirements.
