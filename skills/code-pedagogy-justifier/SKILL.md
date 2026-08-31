---
name: code-pedagogy-justifier
description: "Trigger: justificar ejemplo, por qué este código, code card, decisión pedagógica del código, minimalismo del ejemplo, qué NO muestra el ejemplo. Articula formalmente la justificación pedagógica de cada ejemplo del libro en una code card: concepto demostrado, por qué este enfoque, qué minimalismo aplica y qué NO muestra."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo **después** de `code-integration-architect` (sabe qué ejemplo va en cada sección) y **junto con** `chapter-planner`. Cada ejemplo del libro recibe una **code card** que articula por qué existe.

No la uses para escribir el código (`code-example-generator`) ni para detectar errores (`code-prose-coherence-checker`).

## Hard Rules

- Toda afirmación "este código es la mejor manera de enseñar X" debe **poder defenderse** con una code card.
- La code card declara explícitamente **qué NO muestra** el ejemplo (lo que se omite por minimalismo o scope).
- Si una decisión pedagógica sacrifica fidelidad API (ej. enseñar conceptos testables en lugar de la API directa), la card lo **declara y justifica**.
- Las code cards se versionan con el ejemplo: si el código cambia, la card se revisa.

## Execution Steps

1. Leer `planning/code-map.yml` y el contrato del capítulo.
2. Para cada ejemplo (crate/región) referenciado, redactar una code card:
   - **concept**: concepto(s) que ilustra (del grafo curricular).
   - **approach**: por qué este enfoque y no otro.
   - **minimalism**: qué se ha reducido para no distraer.
   - **not_shown**: qué NO cubre el ejemplo (y dónde se cubre, si aplica).
   - **tradeoff**: si sacrifica fidelidad API por testabilidad/claridad, declararlo.
3. Persistir en `planning/code-cards/{crate}.yml`.
4. Alimentar a `chapter-planner` (el contrato cita las code cards) y a `code-prose-coherence-checker` (valida que la prosa no contradiga la card).

## Ejemplo de code card

```yaml
card:
  crate: chapters/chapter-12-scenes
  region: village-hierarchy
  concept: [ecs-hierarchy, childof-relationship]
  approach: >
    Construcción programática con ChildOf en lugar de bsn! directa.
    Razón: ChildOf es testeable headless (sin GPU); bsn! requiere
    contexto de render para validarse completamente.
  minimalism: >
    Solo Position/Name/Health como componentes; sin sprites, sin assets.
  not_shown: >
    No muestra la sintaxis declarativa de bsn! (se introduce en §12.7
    con includes de la fuente oficial). No cubre serialización de escenas.
  tradeoff: >
    Sacrifica mostrar la API ergonómica (bsn!) a cambio de tests CI sin GPU.
    El lector entiende el modelo subyacente que bsn! automatiza.
```

## Decision Gates

| Situación | Acción |
|-----------|--------|
| El ejemplo no se puede justificar | Eliminar o rediseñar |
| `tradeoff` sacrifica fidelidad y no se declara | Bloqueante hasta declarar |
| La prosa del libro contradice el `approach` | Devolver a `chapter-writer` |
| Ejemplo muestra más de lo que la card dice | `example-complexity-controller` |

## Output Contract

- `planning/code-cards/{crate}.yml` por cada ejemplo.
- Lista de ejemplos sin justificación (bloqueantes).
- `chapter-planner` y `code-prose-coherence-checker` consumen las cards.

## References

- `~/.zcode/skills/BOOK-REPO-CONTEXT.md` — principio "testable headless sobre API difícil de probar".
- `assets/code-card.schema.yml` — esquema de la code card.
