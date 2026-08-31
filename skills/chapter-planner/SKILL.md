---
name: chapter-planner
description: "Trigger: contrato de capítulo, planificar capítulo, chapter contract, antes de redactar, preparar capítulo. Crea el contrato verificable de un capítulo (prerrequisitos, objetivos, evidencia, ejemplos, diagramas, ejercicios, prohibiciones) antes de que chapter-writer redacte una sola línea."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo **antes** de `chapter-writer` para cada capítulo. El contrato es la **puerta de validación**: `chapter-writer` no puede escribir hasta que el contrato pasa la validación.

No lo uses si el contrato del capítulo ya existe en `planning/chapters/{id}.yml` y está aprobado.

## Hard Rules

- El contrato se deriva de `outline.yml` (capítulo) + `curriculum-graph.yml` (conceptos) + `research/` (evidencia).
- Todo `learning_objective` debe ser **medible** (arranca con verbo: explicar, diseñar, diagnosticar).
- Todo `evidence` listado debe existir en `research/sources.yaml`.
- Los `forbidden` son obligatorios: listan qué NO debe aparecer (APIs experimentales, benchmarks sin metodología, etc.).

## Execution Steps

1. Leer la entrada del capítulo en `planning/outline.yml`.
2. Resolver prerrequisitos desde el grafo curricular.
3. Listar objetivos de aprendizaje medibles.
4. Vincular evidence cards existentes; si faltan, encargar a `source-researcher` (bloqueante).
5. Definir ejemplos ejecutables necesarios (delegar a `code-example-generator`).
6. Definir diagramas necesarios (delegar a `diagram-architect`).
7. Definir ejercicios necesarios (delegar a `exercise-designer`).
8. Redactar `forbidden`.
9. Validar contrato contra `assets/chapter-contract.schema.yml`.
10. Si pasa, habilitar `chapter-writer`; si no, devolver bloqueos.

## Esquema de contrato

```yaml
chapter:
  id: ch05-ecs-scheduling
  title: Planificación de sistemas
  prerequisites: [components, queries, rust-borrowing]
  learning_objectives:
    - Explicar cómo se detectan conflictos de acceso
    - Diseñar sistemas ejecutables en paralelo
    - Diagnosticar una planificación secuencial accidental
  evidence:
    - bevy-schedules-docs
    - bevy-source-schedule-module
  executable_examples:
    - examples/scheduling/basic
    - examples/scheduling/conflicts
  diagrams:
    - system-access-graph
  exercises:
    - scheduling-analysis
  forbidden:
    - APIs experimentales no explicadas
    - benchmarks sin metodología
```

## Validaciones automáticas

- Todo `evidence` resuelve en `sources.yaml`.
- Todo `prerequisite` existe en el grafo y se presenta antes.
- Todo `learning_objective` arranca con verbo medible.
- `forbidden` no está vacío.

## Output Contract

- `planning/chapters/{chapter-id}.yml`.
- Lista de bloqueos pendientes (evidencia faltante, ejemplos pendientes) o estado `READY_FOR_WRITER`.
- `chapter-writer` puede consumir el contrato.

## References

- `assets/chapter-contract.schema.yml` — esquema del contrato.
