---
name: curriculum-designer
description: "Trigger: currículo, grafo de conceptos, dependencias entre conceptos, roadmap de aprendizaje, curriculum. Convierte el tema + el perfil de lector en un grafo de objetivos de aprendizaje y dependencias antes de diseñar el índice."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo **después** de `audience-profiler` y **antes** de `book-outline-architect`. Sin grafo curricular, el índice será una colección de temas correctos pero pedagógicamente desordenados.

No lo uses si `planning/curriculum-graph.yml` ya existe y `outline.yml` no se ha generado aún.

## Hard Rules

- Cada concepto debe declarar sus **prerrequisitos** (otros conceptos del grafo).
- Cada concepto debe etiquetarse como `present` (se introduce), `practice` (se ejercita) o `consolidate` (se consolida) por capítulo.
- Lo que **no** pertenece al alcance debe listarse explícitamente en `out_of_scope`.
- Reutiliza ejemplos progresivamente: marca qué ejemplo base evoluciona a lo largo del libro.

## Execution Steps

1. Leer `planning/audience-profile.yml`.
2. Identificar el conjunto completo de conceptos a cubrir (desde lo enseñado, no lo asumido).
3. Construir el grafo de dependencias:
   - Para cada concepto, listar prerrequisitos.
   - Detectar ciclos (son un error de diseño pedagógico → romperlos).
4. Asignar profundidad según el nivel del lector.
5. Generar `planning/curriculum-graph.yml` (esquema en `assets/curriculum-graph.schema.yml`).
6. Generar una ordenación topológica sugerida (es la base del outline).
7. Devolver el grafo + la ordenación.

## Ejemplo de grafo

```yaml
concepts:
  - id: ecs-entity
    title: Entity
    prerequisites: []
    depth: core
  - id: ecs-component
    title: Component
    prerequisites: [ecs-entity]
    depth: core
  - id: ecs-system
    title: System
    prerequisites: [ecs-component, rust-borrowing]
    depth: core
  - id: ecs-scheduling
    title: Scheduling
    prerequisites: [ecs-system]
    depth: advanced
out_of_scope:
  - rendering pipeline interno
  - shaders
progressive_example: examples/arcade-game
```

## Output Contract

- `planning/curriculum-graph.yml`.
- Ordenación topológica (lista de ids).
- Lista de ciclos detectados (debe estar vacía) o advertencias de ruptura.
- Confirmación: `book-outline-architect` listo para consumir el grafo.

## References

- `assets/curriculum-graph.schema.yml` — esquema del grafo.
