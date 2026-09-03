# ADR-HX-002 — Human Interaction Plane

**Status:** proposed  
**Date:** 2026-08-28

## Context
Reporting, personalidad y HITL repartidos en prompts producirían duplicación.

## Decision
Crear un bounded context transversal de interacción y mantener phase agents como productores de facts/evidence.

## Consequences
Menos tokens y divergencia; requiere nuevos contratos estructurados.

## Alternatives considered
Copiar reglas a todos los agentes; o crear un 'chat agent' con autoridad propia. Ambas se rechazan.
