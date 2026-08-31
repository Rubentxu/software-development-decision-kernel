---
name: pedagogical-reviewer
description: "Trigger: revisión pedagógica, salto conceptual, conocimiento no introducido, explicación circular, progresión didáctica, revisar didáctica. Detecta saltos conceptuales, explicaciones circulares y conocimientos asumidos sin introducir, cruzando contra el grafo curricular."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo **después** de `chapter-writer` y junto con `technical-reviewer`. Mientras `technical-reviewer` mira *corrección*, esta skill mira *comprensibilidad y progresión*.

No la uses para prosa/estilo (`editorial-reviewer`).

## Hard Rules

- La validación se hace **contra el grafo curricular** (`planning/curriculum-graph.yml`).
- Todo término técnico usado debe haber sido **introducido antes** (present) o en el propio capítulo.
- No se permiten explicaciones circulares (A definido por B, B definido por A).
- Cada objetivo del contrato debe quedar **cubierto** al final del capítulo.

## Execution Steps

1. Leer el contrato del capítulo y el grafo curricular.
2. Para cada sección del `.adoc`:
   - Listar términos técnicos usados.
   - Verificar que cada uno fue introducido antes o lo es en esta sección.
   - Marcar **saltos conceptuales** (uso de término sin introducir).
3. Detectar **explicaciones circulares** (grafo de definiciones con ciclos).
4. Comprobar cobertura: ¿cada `learning_objective` del contrato se aborda?
5. Comprobar carga cognitiva: ¿demasiados conceptos nuevos por sección?
6. Emitir `build/reviews/{chapter-id}.pedagogy.yml`.

## Categorías de hallazgo

| Categoría | Significado |
|-----------|-------------|
| `missing_prerequisite` | Se usa un concepto sin introducir |
| `circular_definition` | A↔B sin base externa |
| `uncovered_objective` | Objetivo del contrato no tratado |
| `cognitive_overload` | Demasiadas ideas nuevas por sección |
| `orphan_concept` | Concepto presentado que no se usa después |

## Output Contract

- `build/reviews/{chapter-id}.pedagogy.yml` con hallazgos categorizados y severidad.
- `verdict`: `PASS` | `PASS_WITH_REMEDIATION` | `BLOCKED`.
- Si `BLOCKED`, lista de saltos conceptuales que devuelven el capítulo a `chapter-writer`.

## References

- `references/pedagogy-heuristics.md` — heurísticas de detección.
