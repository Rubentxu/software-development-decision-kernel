---
name: example-complexity-controller
description: "Trigger: ejemplo demasiado complejo, simplificar ejemplo, complejidad de ejemplo, ejemplo que distrae, demasiadas ideas en un ejemplo. Comprueba que cada ejemplo introduce pocas ideas nuevas, evitando ejemplos LLM típicamente sobredimensionados que distraen del concepto principal."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo **después** de `code-example-generator` y **antes** de `chapter-writer`/`code-example-verifier`. Es el filtro de carga cognitiva del código.

No la uses para verificar corrección (`code-example-verifier`).

## Hard Rules

- Un ejemplo ilustra **un** concepto principal (máx. 2 secundarios explícitos).
- Si un ejemplo necesita más, se **divide** en varios atómicos.
- El boilerplate (setup, imports) no cuenta como "idea", pero debe minimizarse.
- Rechazar ejemplos que introducen conceptos fuera del contrato del capítulo.

## Execution Steps

1. Leer el contrato del capítulo y el concepto que el ejemplo debe ilustrar.
2. Analizar el ejemplo generado:
   - Contar conceptos técnicos no triviales que aparecen.
   - Identificar cuáles son el foco vs. distracción.
3. Si conceptos-no-foco > 2 → `OVERCOMPLEX`, devolver a `code-example-generator` para dividir/simplificar.
4. Si hay APIs/feature flags fuera del contrato → `SCOPE_CREEP`.
5. Si el boilerplate oculta el concepto → `NOISE`, pedir etiquetado con `tag::` más ajustado.
6. Emitir `build/reviews/{example-id}.complexity.yml`.

## Output Contract

- `build/reviews/{example-id}.complexity.yml`.
- `verdict`: `PASS` | `OVERCOMPLEX` | `SCOPE_CREEP` | `NOISE`.
- Si no PASS, indicar cómo dividir/simplificar.

## References

- `references/complexity-heuristics.md` — heurísticas de conteo de ideas.
