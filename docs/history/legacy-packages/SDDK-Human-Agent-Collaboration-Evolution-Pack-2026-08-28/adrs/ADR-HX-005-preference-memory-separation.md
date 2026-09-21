# ADR-HX-005 — Separate User Preference Memory

**Status:** proposed  
**Date:** 2026-08-28

## Context
Operational memory y project knowledge tienen autoridades y cadencias distintas a preferencias.

## Decision
Namespace/store lógico separado bajo XDG con provenance, confidence y controles del usuario.

## Consequences
Mejora privacidad, rollback y semántica.

## Alternatives considered
Guardar preferencias en vault o ledger operativo fue rechazado.
