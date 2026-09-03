# ADR-HX-009 — Friction-Driven Adaptation

**Status:** proposed  
**Date:** 2026-08-28

## Context
SDDK ya tiene F3/control plane pero no mide confusión humana.

## Decision
Registrar friction signals y producir recomendaciones. Auto-tuning sólo en presentation no crítica.

## Consequences
UX mejora empíricamente sin convertir analytics en policy authority.

## Alternatives considered
Auto-modificar safety thresholds rechazado.
