# ADR-003 — Separate Workflow Authority from Reactive World Modeling

**Status:** Proposed  
**Date:** 2026-08-11

## Context

Reactive graph coordination is attractive but SDDK already has explicit phase/gate semantics that provide deterministic control.

## Decision

Keep the workflow FSM authoritative for lifecycle/control flow. Use reactive behaviors for pattern detection, derived knowledge and proposals only.

## Consequences

### Positive

- Preserves deterministic gates.
- Gains reactive extensibility without surrendering control.

### Trade-offs / risks

- Two models must be clearly explained.
- Cross-boundary events need careful naming.

## Implementation notes

Define workflow events in CEP. Behaviors can observe workflow transitions but cannot advance protected phases except by proposal handled by application services.

## Revisit trigger

Revisit only if a concrete workflow proves impossible to model without reactive transitions; require an ADR for any exception.
