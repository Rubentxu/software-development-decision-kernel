---
name: analogy-auditor
description: "Trigger: revisar analogías, metáforas técnicas, analogía que engaña, validar analogía, analogía inexacta. Revisa analogías y metáforas para detectar cuándo dejan de representar correctamente el comportamiento técnico y pueden inducir a error."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo como sub-pase de `pedagogical-reviewer` cuando un capítulo contiene analogías/metáforas para explicar conceptos técnicos. Las analogías son útiles pero **peligrosas** cuando dejan de mapear.

No la uses para revisar prosa literal (`editorial-reviewer`).

## Hard Rules

- Toda analogía debe declarar **dónde se rompe** (qué aspectos del concepto NO representa).
- Una analogía que induce un modelo mental erróneo es `MISLEADING` (bloqueante).
- Las analogías no sustituyen a la explicación técnica; la complementan.

## Execution Steps

1. Identificar todas las analogías/metáforas del capítulo.
2. Para cada una:
   - ¿Qué concepto representa?
   - ¿Qué aspectos del concepto mapea bien?
   - ¿Dónde se rompe el mapeo?
   - ¿Podría inducir un modelo mental erróneo?
3. Clasificar:
   - `OK` — útil y honesta (declara sus límites).
   - `INCOMPLETE` — no declara dónde se rompe (pedir disclaimer).
   - `MISLEADING` — induce error (bloqueante, reescribir).
4. Emitir `build/reviews/{chapter-id}.analogy.yml`.

## Ejemplo

Analogía: "Ownership de Rust es como tener el coche: solo una persona puede ser dueña."
- Mapea bien: exclusividad del ownership.
- Se rompe: el *move* transfiere propiedad sin transacción; el *borrow* no transfiere nada.
- Si el capítulo no aclara el *borrow* tras la analogía → `INCOMPLETE`.

## Output Contract

- `build/reviews/{chapter-id}.analogy.yml`.
- `verdict`: `PASS` | `PASS_WITH_DISCLAIMERS` | `MISLEADING`.
- Si `MISLEADING`, devolver a `chapter-writer` para reescribir o eliminar la analogía.

## References

- `references/analogy-templates.md` — plantilla para declarar límites de analogías.
