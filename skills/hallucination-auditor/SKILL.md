---
name: hallucination-auditor
description: "Trigger: auditoría de alucinaciones, APIs inventadas, referencias inexistentes, afirmaciones sin verificar, detectar fabricaciones, hallucination check. Busca afirmaciones no demostradas, APIs inventadas, crates inexistentes y referencias falsas comparando el texto contra el índice de evidencia verificable."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo **después** de `chapter-writer`, en paralelo con `technical-reviewer`. Es el detector sistemático de fabricaciones típicas de LLM: APIs que no existen, crates con versiones rotas, resultados inventados, referencias falsas.

Esta skill existe precisamente por lo que detectó tu auditoría del libro Bevy (bsn!, App Settings, BRP REST, crates con versiones inventadas).

No la uses para corrección de estilo (`editorial-reviewer`).

## Hard Rules

- **Cero tolerancia** con APIs/crates/referencias no verificables.
- Toda afirmación técnica sin `claim_id` con `status: verified` se trata como **fabricación presunta** hasta probar lo contrario.
- El auditor puede **bloquear la publicación**.
- Compara contra: `research/sources.yaml`, `evidence/claims.jsonl`, y el código real en `examples/`.

## Execution Steps

1. Extraer del `.adoc` toda afirmación marcada con `// evidence:` y toda afirmación técnica sin marcar.
2. Para cada afirmación:
   - ¿Tiene `claim_id` que resuelve en `claims.jsonl`?
   - ¿El claim está `verified` con fuente de autoridad ≤6?
   - Si no → `UNVERIFIED_CLAIM`.
3. Para cada API/crate nombrado:
   - ¿Existe en la versión declarada? (cruzar con `sources.yaml` y `examples/*/Cargo.toml`).
   - Si no aparece en documentación oficial ni en el código → `INVENTED_API`.
4. Para cada referencia externa (URL, paper, RFC):
   - ¿Resuelve? ¿Existe? → si no, `FAKE_REFERENCE`.
5. Para cada resultado/salida mostrada:
   - ¿Está verificado por `code-example-verifier` o evidence card? → si no, `INVENTED_RESULT`.
6. Emitir `build/reviews/{chapter-id}.hallucination.yml`.

## Categorías de hallazgo

| Categoría | Severidad | Ejemplo |
|-----------|-----------|---------|
| `INVENTED_API` | critical | `bsn!`, `AppSettings` derive que no existen |
| `INVENTED_CRATE` | critical | `bevy_navigation` que no está en crates.io |
| `VERSION_DRIFT` | critical | crate "0.18" cuando real es 0.7 |
| `FAKE_REFERENCE` | high | URL o RFC que no resuelve |
| `INVENTED_RESULT` | high | salida de comando fabricada |
| `UNVERIFIED_CLAIM` | med | afirmación sin evidence card |

## Output Contract

- `build/reviews/{chapter-id}.hallucination.yml` con todos los hallazgos.
- `verdict`: `PASS` | `BLOCKED`. Cualquier `critical` → `BLOCKED`.
- Si `BLOCKED`, lista exacta para devolver a `chapter-writer` + `source-researcher`.

## References

- `references/red-flags.md` — patrones típicos de fabricación LLM en libros técnicos.
