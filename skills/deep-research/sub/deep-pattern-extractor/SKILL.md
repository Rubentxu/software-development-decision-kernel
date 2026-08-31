---
name: deep-pattern-extractor
description: "Trigger: extraer patrón de implementación, identificar patrón arquitectónico, code pattern, best practice de implementación, ¿cómo se implementa X en producción?. Extrae patrones de implementación de papers, libros, código fuente de referencia. Produce snippets verificables con tests. Para SOFTWARE (orchestrator) es el núcleo de la implementación; para LIBRO (book-orchestrator) son ejemplos de código incluidos vía `include::`."
license: Apache-2.0
metadata:
  category: deep-research
  subcategory: domain-pipeline
  author: rubentxu
  version: "1.0"
  domain: deep-research, software
  consumers: [orchestrator, book-orchestrator]
---

## Activation Contract

Úsalo cuando el tema requiere **patrones de implementación concretos**: cómo se implementa X en código, qué decisiones de diseño son canónicas, qué tradeoffs existen. Trabaja en conjunto con `deep-software-research` y `deep-claim-extractor`.

No lo uses para: análisis arquitectónico abstracto (`deep-domain-modeler`), comparativa de frameworks (`deep-knowledge-graph-builder`).

## Hard Rules

- **Cada patrón tiene fuente verificable**: paper seminal, libro canónico (GoF, Clean Code, etc.), código fuente de un proyecto de referencia.
- **Cada snippet compila y pasa tests**: la implementación debe ser ejecutable, no pseudocódigo.
- **Tradeoffs explícitos**: "este patrón optimiza X a costa de Y" — no solo beneficios.
- **Anti-patrones asociados**: documentar qué errores evita y qué errores introduce.
- **Conexión con el sistema (R0)**: ¿este patrón es leverage point (nivel 4-6) o ajuste de parámetro (nivel 12)? Sin esto, es solo "qué hacer" sin "por qué".

## Execution Steps

1. Activar pipeline R para el patrón:
   - R0: definir el sistema donde aplica el patrón.
   - R1: agenda con preguntas sobre el patrón (origen, evolución, casos de uso, anti-patrones).
   - R2-R4: descubrir fuentes seminales (papers, libros, código).
   - R5: consolidar en corpus.
2. Identificar el patrón canónico:
   - ¿Tiene nombre? (Singleton, Observer, ECS, Repository, etc.).
   - ¿Quién lo introdujo? (GoF, Fowler, etc.).
   - ¿Cómo ha evolucionado? (mejoras, críticas, alternativas).
3. Documentar:
   - **Nombre**: técnico + aliases.
   - **Problema**: qué dolor resuelve.
   - **Solución**: estructura (clases, funciones, módulos).
   - **Consecuencias**: tradeoffs (positivos y negativos).
   - **Implementación de referencia**: snippet ejecutable.
   - **Anti-patrones asociados**: cuándo NO usarlo.
4. Generar `research/code-patterns/{pattern}.{md,py,rs}` con:
   - Documentación markdown.
   - Snippet Python ejecutable con tests.
   - (Opcional) Snippet Rust.
5. Validar: ejecutar los tests; si fallan, el patrón NO se publica.

## Esquema de code-patterns/{pattern}.md

```markdown
# Pattern: {Nombre}

## Provenance
- **Origin**: {autor/año}
- **Source**: {paper/libro}
- **Status**: {canonical | emerging | deprecated}

## Problem
{Qué problema resuelve, en términos del sistema del tema}

## Solution
{Estructura: clases/funciones/módulos}

## Tradeoffs
### Pros
- {tradeoff positivo 1}

### Cons
- {tradeoff negativo 1}

## Implementation

```python
# {snippet ejecutable}
```

## Tests
{Qué tests verifican el patrón}

## Related Patterns
- {patrones que lo combinan, lo extienden, o son alternativas}

## Anti-patterns
- {errores comunes al aplicar este patrón}

## System Map (R0)
- **Leverage point**: {nivel 1-12 de Meadows}
- **Trap evitado**: {qué system trap se evita con este patrón}
- **Trap introducido**: {qué system trap puede introducir si se aplica mal}
```

## Decision Gates

| Situación | Acción |
|-----------|--------|
| Patrón sin fuente seminal clara | Marcar `origin: community` y verificar consenso; si es solo 1 fuente, `evidence_level: L3` |
| Snippet no compila o falla tests | NO publicar; volver a R2 para descubrir patrón correcto |
| Anti-patrón conocido no documentado | STOP: añadir antes de publicar |
| Sin tradeoffs negativos | Sospechoso: todo patrón tiene costos; preguntar al autor |
| Patrón obsoleto (reemplazado por otro) | Marcar `status: deprecated` y enlazar al sucesor |

## Output Contract

- `research/code-patterns/{pattern}.md` con documentación completa.
- `research/code-patterns/{pattern}.py` con snippet ejecutable + tests.
- (Opcional) `research/code-patterns/{pattern}.rs`.
- Actualizar `research/corpus.yml` con el patrón como entry.

## References

- Skills relacionadas: `deep-software-research`, `deep-domain-modeler`.
- Catálogo de patrones:
  - GoF (Gang of Four): `references/gof-catalog.md`.
  - POSA (Pattern-Oriented Software Architecture): distributed patterns.
  - Cloud Design Patterns (Microsoft): `https://learn.microsoft.com/en-us/azure/architecture/patterns/`.
  - Martin Fowler: `https://martinfowler.com/eaaCatalog/`.
