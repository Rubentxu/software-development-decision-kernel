# ADR-HX-003 — Interaction Events as Projection

**Status:** proposed  
**Date:** 2026-08-28

## Context
El ledger ya es event-sourced; crear un segundo event log de lifecycle sería doble autoridad.

## Decision
Derivar InteractionEvents de ledger/artifacts cuando sea posible. Persistir sólo eventos humanos no reconstruibles y telemetría mínima.

## Consequences
Evita inconsistencia; algunas vistas requieren composición en runtime.

## Alternatives considered
Nuevo event store completo rechazado.
