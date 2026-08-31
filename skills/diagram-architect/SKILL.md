---
name: diagram-architect
description: "Trigger: crear diagrama, diagrama de arquitectura, diagrama UML, diagrama de secuencia, mermaid, plantuml, C4, graphviz, diagrama como código. Genera diagramas (C4, UML, secuencia, flujo, arquitectura) como código versionable y valida que sus relaciones coinciden con el código o modelo explicado."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo cuando `chapter-planner` lista un `diagram` en el contrato, o cuando una explicación necesita soporte visual. Los diagramas viven como **texto versionable**, no como imágenes binarias.

No la uses para decidir *si* hace falta un diagrama (eso es `chapter-planner`).

## Hard Rules

- El diagrama debe almacenarse como **texto** (`.mmd`, `.puml`, `.dot`).
- Elección de notación por propósito (ver tabla de Decision Gates).
- El diagrama no es decorativo: sus **relaciones deben corresponder** con el código o modelo explicado.
- Cada diagrama declara `claims` (qué afirma) y `validated_against` (contra qué se valida).
- Si el código cambia, el diagrama debe revisarse.

## Execution Steps

1. Recibir el propósito del diagrama desde el contrato del capítulo.
2. Elegir notación (ver tabla).
3. Generar el fuente del diagrama en `diagrams/{id}.{ext}`.
4. Declarar `diagrams/index.yml` con `claims` y `validated_against`.
5. Renderizar a SVG/PNG y referenciarlo en el `.adoc` vía `image::`.
6. Validar que las relaciones del diagrama coinciden con el ejemplo/evidence citado.

## Decision Gates — notación por propósito

| Propósito | Notación | Extensión |
|-----------|----------|-----------|
| Flujo, secuencia, estados, dependencias sencillas | Mermaid | `.mmd` |
| UML formal (clases, componentes, actividades) | PlantUML | `.puml` |
| Arquitectura (contexto, contenedores, componentes) | C4-PlantUML / Structurizr DSL | `.puml`/`.dsl` |
| Grafos complejos | Graphviz/DOT | `.dot` |

## Esquema de metadatos

```yaml
diagram:
  id: ecs-scheduler
  purpose: Mostrar los conflictos de acceso entre sistemas
  notation: mermaid
  source: diagrams/ecs-scheduler.mmd
  claims:
    - "System A y System C pueden ejecutarse en paralelo"
    - "System B requiere acceso exclusivo"
  validated_against:
    - examples/scheduling/conflicts
```

## Output Contract

- `diagrams/{id}.{ext}` (fuente textual).
- Render SVG/PNG en `build/diagrams/`.
- `diagrams/index.yml` actualizado con claims y validación.
- Placeholder `// diagram: {id}` del `.adoc` reemplazado por `image::`.

## References

- `references/notation-guide.md` — cuándo usar cada notación, con ejemplos.
