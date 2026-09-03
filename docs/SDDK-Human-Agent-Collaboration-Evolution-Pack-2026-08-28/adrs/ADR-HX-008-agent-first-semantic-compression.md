# ADR-HX-008 — Agent-First Semantic Compression

**Status:** proposed  
**Date:** 2026-08-28

## Context
Los agentes sufrían secuencias CLI largas; la baseline ya introduce facade e Instruction Contract Matrix.

## Decision
Consolidar facade/goal surface como interfaz de intención. Low-level CLI sigue canónico para debug/contract tests. Parity obligatoria.

## Consequences
Menos llamadas/tokens sin perder evidencia.

## Alternatives considered
Eliminar low-level CLI o simplificar omitiendo receipts/gates rechazado.
