# ADR-0002 — Vault Markdown como fuente canónica del conocimiento

**Estado:** aceptada
**Fecha:** 2026-08-03

## Contexto

SDDK necesita conservar decisiones, requisitos, arquitectura, riesgos, ciclos y conocimiento de código de forma accesible para personas y agentes.

## Decisión

El conocimiento canónico se almacenará en un vault Markdown compatible con Obsidian, usando:

- Frontmatter estructurado.
- IDs estables independientes del nombre del fichero.
- Relaciones tipadas.
- Wikilinks para navegación humana.
- Git opcional para versionado y sincronización.

SQLite mantendrá un índice reconstruible, no una segunda copia canónica del contenido.

## Consecuencias positivas

- Portabilidad y ausencia de lock-in.
- Edición directa con herramientas comunes.
- Diffs y revisión Git.
- Recuperación sin depender del CLI.

## Consecuencias negativas

- Debe imponerse un schema estricto para evitar Markdown caótico.
- Los cambios manuales requieren indexación y validación.

## Decisiones relacionadas

- ADR-0003: SQLite operativo e índice.
- ADR-0008: aplazamiento de LadybugDB.
