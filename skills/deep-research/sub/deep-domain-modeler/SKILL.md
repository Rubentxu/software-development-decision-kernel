---
name: deep-domain-modeler
description: "Trigger: modelar dominio, mapear entidades, ¿cómo se relacionan los conceptos?, modelo conceptual, ontología del dominio, términos del campo. Modela conceptualmente un dominio: entidades, relaciones, propiedades, cardinalidad, invariantes. Produce modelos para libros (diagramas UML/Mermaid/ER) y para software (tipos, schemas, contratos). Aplica marco Meadows: el modelo es la 'definición del sistema' (R0) hecha explícita."
license: Apache-2.0
metadata:
  category: deep-research
  subcategory: domain-pipeline
  author: rubentxu
  version: "1.0"
  domain: deep-research
  consumers: [book-orchestrator, orchestrator]
---

## Activation Contract

Úsalo cuando el tema requiere **mapear conceptualmente un dominio**: qué entidades existen, cómo se relacionan, qué propiedades tienen, qué reglas las gobiernan. Es la **R0 explícita** (definir el sistema) hecha modelo formal.

No lo uses para: implementar el modelo en código (`deep-pattern-extractor` + `deep-software-research`),的历史 de los conceptos (`deep-historical-lineage-tracer`).

## Hard Rules

- **Modelo explícito**: entidades con nombre, propiedades con tipo, relaciones con cardinalidad.
- **Invariantes documentados**: reglas que el modelo debe cumplir siempre.
- **Trazabilidad con fuentes**: cada entidad/propiedad/relación tiene al menos una fuente (paper seminal, RFC, doc oficial).
- **Alineación con R0**: el modelo debe responder al propósito identificado en el `system-map`.
- **Diagramas como código**: Mermaid, PlantUML, ER, o JSON Schema; nunca imágenes sueltas.

## Execution Steps

1. Activar pipeline R para el dominio:
   - R0: definir el propósito del dominio.
   - R1: agenda con preguntas sobre entidades, relaciones, propiedades.
   - R2-R4: descubrir fuentes canónicas (papers seminales, RFCs, docs oficiales).
   - R5: consolidar.
2. Identificar **entidades**:
   - ¿Qué "cosas" existen en el dominio? (sustantivos).
   - ¿Cuáles son abstractas vs. concretas? (e.g., "Usuario" vs. "Sesión").
3. Identificar **propiedades**:
   - Para cada entidad, ¿qué atributos tiene? (tipo, rango, unidades).
4. Identificar **relaciones**:
   - ¿Cómo se conectan las entidades? (verbos).
   - ¿Cuál es la cardinalidad? (1:1, 1:N, N:M).
5. Identificar **invariantes**:
   - Reglas que el modelo debe cumplir siempre (e.g., "todo Pedido tiene al menos un Item").
6. Generar el modelo:
   - **Modo LIBRO**: diagramas Mermaid/PlantUML + tablas en AsciiDoc.
   - **Modo SOFTWARE**: tipos en TypeScript/Python, JSON Schema, OpenAPI, GraphQL SDL.
7. Validar:
   - ¿El modelo responde al propósito del R0?
   - ¿Hay entidades huérfanas (sin relaciones)?
   - ¿Hay reglas implícitas no documentadas?

## Esquema del modelo (formato agnóstico)

```yaml
domain_model:
  topic: "e-commerce"
  purpose: "Permitir a usuarios comprar productos"
  sources: [src-rfc-ecommerce, src-book-domain-driven-design]
  entities:
    - name: User
      properties:
        - {name: id, type: UUID, required: true}
        - {name: email, type: string, required: true, unique: true}
      invariants:
        - "email debe ser único"
    - name: Order
      properties:
        - {name: id, type: UUID, required: true}
        - {name: user_id, type: UUID, required: true}
        - {name: status, type: enum, values: [pending, paid, shipped, delivered]}
      invariants:
        - "Una Order pertenece a exactamente 1 User"
  relations:
    - {from: User, to: Order, type: has_many, cardinality: 1:N}
    - {from: Order, to: Product, type: contains, cardinality: N:M}
  derived_views:
    - "Cart = Order temporal con status=pending"
```

## Output según modo

### Modo LIBRO
- `research/domain-models/{topic}.yml` (modelo formal).
- `research/diagrams/{topic}-er.mmd` (diagrama Mermaid).
- `research/drafts/{topic}-domain-section.md` (borrador AsciiDoc con tablas y diagramas).

### Modo SOFTWARE
- `research/domain-models/{topic}.yml`.
- `research/code-patterns/{topic}-types.{ts,py}` (tipos generados).
- `research/blueprints/{topic}-validation.yml` (validador del modelo).

## Decision Gates

| Situación | Acción |
|-----------|--------|
| Entidad sin fuente | STOP: investigar origen (R2) |
| Relación con cardinalidad ambigua | Preguntar al autor antes de publicar |
| Modelo no responde al propósito (R0) | Re-evaluar R0; el modelo debe servir al propósito |
| Modelo con > 30 entidades | Considerar dividir en bounded contexts (DDD) |
| Sin invariantes | Sospechoso: todo modelo tiene reglas implícitas; documentarlas |

## Output Contract

- `research/domain-models/{topic}.yml`.
- `research/diagrams/{topic}-er.mmd` (LIBRO).
- `research/code-patterns/{topic}-types.{ts,py}` (SOFTWARE).
- Actualizar `research/corpus.yml` con el modelo como entry.

## References

- Skills relacionadas: `deep-software-research`, `deep-pattern-extractor`.
- Fuentes de modelado:
  - Eric Evans, *Domain-Driven Design* (2003).
  - Martin Fowler, *Patterns of Enterprise Application Architecture* (2002).
  - UML 2.5 specification (OMG).
  - JSON Schema Draft 2020-12.
- `references/modeling-notations.md` — Mermaid, PlantUML, ER.
