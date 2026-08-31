---
name: book-outline-architect
description: "Trigger: índice del libro, outline, tabla de contenidos, estructura de partes, planificar capítulos, TOC. Diseña partes, capítulos y secciones con progresión pedagógica a partir del grafo curricular y el perfil de lector."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo **después** de `curriculum-designer` (necesita el grafo) y **antes** de `chapter-planner`. Convierte la ordenación topológica de conceptos en una estructura de libro (partes → capítulos → secciones).

No lo uses si `planning/outline.yml` ya existe y está aprobado.

## Hard Rules

- Cada capítulo debe tener un **objetivo de aprendizaje explícito** derivado del grafo curricular.
- Cada sección dentro de un capítulo debe mapear a ≥1 concepto del grafo.
- La progresión debe respetar las dependencias del grafo (nunca usar un concepto antes de presentarlo).
- Balancear carga cognitiva: ningún capítulo introduce demasiados conceptos nuevos de golpe.
- Declarar qué ejemplos progresivos aparecen en cada capítulo.

## Execution Steps

1. Leer `planning/curriculum-graph.yml` y `planning/audience-profile.yml`.
2. Agrupar conceptos en **partes** (bloques temáticos coherentes).
3. Dentro de cada parte, asignar **capítulos** (1 capítulo ≈ 1-3 conceptos nuevos).
4. Para cada capítulo, redactar:
   - `id`, `title`, `part`, `order`.
   - `concepts` presentados / practicados / consolidados.
   - `sections` (título + concepto mapeado).
   - `progressive_example_step` (si aplica).
   - `estimated_pages`.
5. Validar cobertura: ¿todo concepto del grafo aparece en ≥1 capítulo?
6. Generar `planning/outline.yml` (esquema en `assets/outline.schema.yml`).
7. Generar una vista previa del índice en AsciiDoc (`src/book.adoc` outline).
8. Pedir aprobación al autor antes de pasar a `chapter-planner`.

## Validaciones automáticas

- Ningún concepto usado antes de su presentación (check contra grafo).
- Ninguna parte vacía ni capítulo sin secciones.
- Distribución de páginas razonable (advertir capítulos >150% de la media).

## Output Contract

- `planning/outline.yml`.
- `src/book.adoc` actualizado con el esqueleto de partes/capítulos (includes sin contenido aún).
- Informe de cobertura (conceptos cubiertos vs. gap).
- Lista de capítulos listos para `chapter-planner`.

## References

- `assets/outline.schema.yml` — esquema del outline.
