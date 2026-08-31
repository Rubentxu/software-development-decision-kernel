---
name: repository-knowledge-extractor
description: "Trigger: extraer conocimiento de repositorio, analizar base de código, mapa de módulos, APIs públicas, flujos de ejecución, entender código fuente. Analiza una base de código (con AST/índices de símbolos antes que búsqueda semántica) y genera mapa de módulos, APIs públicas, flujos y candidatos a ejemplo."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo cuando el libro debe explicar un framework/repositorio concreto (Rust/Bevy, Kubernetes, etc.) y necesitas un **mapa estructural fiable** del código antes de escribir. Produce insumo para `source-researcher` y `chapter-planner`.

No la uses para búsquedas puntuales de un símbolo (lectura directa).

## Hard Rules

- Priorizar **herramientas del lenguaje** (rust-analyzer, AST, `cargo doc`, tree-sitter) antes que búsqueda semántica difusa.
- Distinguir **API pública** de internos.
- Versionar el análisis (`extracted_at` + commit/versión del repo analizado).
- La salida es **estructura**, no prosa: mapas, listas, flujos.

## Execution Steps

1. Clonar/ubicar el repositorio objetivo en la versión declarada.
2. Ejecutar indexación del lenguaje (rust-analyzer / `cargo metadata` / tree-sitter).
3. Generar:
   - **Mapa de módulos** (árbol de crates/módulos).
   - **API pública** (items `pub` con firmas).
   - **Flujos de ejecución** principales (entry points → call graph de alto nivel).
   - **Decisiones de diseño** desde CHANGELOG/commit history relevante.
   - **Ejemplos candidatos** (archivos en `examples/` del propio repo).
   - **Elementos que requieren explicación** (macros complejas, unsafe, traits clave).
4. Persistir en `research/repo-analysis/{repo}.yml`.
5. Alimentar a `source-researcher` (fuente nivel 3) y `chapter-planner`.

## Output Contract

- `research/repo-analysis/{repo}.yml` (mapa, API, flujos, candidatos).
- `extracted_at` + `commit`/`version` del repo.
- Lista de "elementos a explicar" priorizada para el outline.

## References

- `references/extraction-tools.md` — herramientas por lenguaje.
