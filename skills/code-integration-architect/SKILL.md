---
name: code-integration-architect
description: "Trigger: integrar código en el libro, mapear código-capítulo, estrategia de includes, regiones tag, qué ejemplo para cada sección, mapeo bidireccional libro-workspace. Define la estrategia de integración entre la prosa del libro y el workspace de ejemplos, manteniendo un mapa bidireccional capítulo↔código."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo **después** de `book-outline-architect` (necesita el outline) y **antes** de `chapter-planner` (alimenta el contrato). Esta skill es el **puente** entre la estructura del libro y el repo de ejemplos: decide qué código ilustra cada sección y cómo se enlaza.

No la uses para escribir el código (`code-example-generator`) ni la prosa (`chapter-writer`).

## Hard Rules

- Toda afirmación "el código hace X" del libro debe apuntar a una **región real y estable** del workspace (`include::` con `tag::`).
- Mantén un **mapa bidireccional**: sección del libro → crate/región del workspace, y crate → secciones que lo usan.
- Un crate sin secciones que lo referencien es **código huérfano** (alerta).
- Una sección que cita código sin región mapeada es **prosa sin respaldo** (alerta).
- Lee `~/.zcode/skills/BOOK-REPO-CONTEXT.md` antes de tocar nada.

## Execution Steps

1. Leer `BOOK-REPO-CONTEXT.md` (convenciones del workspace) y `planning/outline.yml`.
2. Para cada capítulo, decidir:
   - ¿Qué **conceptos** del outline se ilustran con código?
   - ¿Qué **crate** del workspace los contiene? (`chapters/chapter-{NN}-{slug}/`)
   - ¿Qué **regiones etiquetadas** (`tag::`/`end::`) se exponen al libro?
   - ¿Qué secciones del libro consumen cada región?
3. Definir la estrategia de **includes**: prefieren `include::` desde el repo real (código probado = código mostrado) sobre copiar snippets.
4. Construir `planning/code-map.yml` (mapa bidireccional, esquema en `assets/code-map.schema.yml`).
5. Detectar huérfanos y gaps antes de pasar a `chapter-planner`.

## Mapa bidireccional (resumen del esquema)

```yaml
# Dirección libro → código
sections:
  - chapter: cap-12
    section: "12.3 Jerarquías"
    crate: chapters/chapter-12-scenes
    region: village-hierarchy      # tag:: en el .rs
    include_path: "src/lib.rs[tag=village-hierarchy]"

# Dirección código → libro
crates:
  - crate: chapters/chapter-12-scenes
    referenced_by: [cap-12 §12.3, cap-13 §13.1]
    orphan: false
```

## Decision Gates

| Situación | Acción |
|-----------|--------|
| Concepto sin crate que lo ilustre | Marcar gap → `code-example-generator` |
| Crate sin sección que lo cite | Marcar huérfano → decidir si eliminar o añadir sección |
| Concepto difícil de testear headless | Decisión pedagógica → `code-pedagogy-justifier` |
| Región compartida por varios capítulos | Confirmar que el include es estable (no se rompe al evolucionar) |

## Output Contract

- `planning/code-map.yml` (mapa bidireccional completo).
- Lista de gaps (secciones sin código) y huérfanos (código sin sección).
- Estrategia de includes por capítulo.
- `chapter-planner` consume el mapa al construir el contrato.

## References

- `~/.zcode/skills/BOOK-REPO-CONTEXT.md` — convenciones del workspace (fuente de verdad).
- `assets/code-map.schema.yml` — esquema del mapa bidireccional.
- `references/include-strategies.md` — patrones de include por formato (AsciiDoc/Hugo/HTML).
