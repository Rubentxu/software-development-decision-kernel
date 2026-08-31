# ADR-0008 — LadybugDB aplazada como proyección analítica opcional

**Estado:** aceptada
**Fecha:** 2026-08-03

## Contexto

LadybugDB sería útil para grafos masivos, Cypher, algoritmos y GraphRAG, pero el conocimiento canónico ya se ha definido como un vault de ficheros.

## Decisión

No incorporar LadybugDB en el núcleo de v3.6.

La arquitectura expondrá una interfaz de consulta de grafo implementada inicialmente mediante SQLite y `petgraph`. LadybugDB podrá añadirse como proyector reconstruible cuando existan métricas que lo justifiquen.

## Disparadores de reevaluación

Reevaluar cuando se cumplan al menos dos condiciones:

- Más de 100 000 nodos.
- Más de 500 000 relaciones.
- Consultas frecuentes de cuatro o más saltos.
- Algoritmos de comunidades o centralidad recurrentes.
- GraphRAG híbrido como requisito real.
- Cuello de botella medido en SQLite/petgraph.

## Consecuencias

Se evita complejidad prematura sin cerrar la evolución futura.
