---
name: code-prose-coherence-checker
description: "Trigger: coherencia código-prosa, validar que prosa y código coinciden, drift código-libro, prosa dice una cosa código hace otra, include roto, tag desaparecido, código huérfano. Valida la coherencia bidireccional entre la prosa del libro y el código del workspace, y detecta drift cuando el código cambia. Modos: review (en revisión de capítulo) y drift (tras cambios de código)."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Tiene **dos modos** según el momento del pipeline:

- **Modo `review`**: durante la revisión de un capítulo (fase 10). Valida que cada afirmación sobre el código es fiel Y que cada región referenciada está explicada.
- **Modo `drift`**: tras un cambio en el workspace de ejemplos. Detecta qué capítulos quedan desactualizados.

No la uses para generar (`code-example-generator`) ni para compilar (`code-example-verifier`). Ella es **semántica**: ¿lo que dice el libro corresponde a lo que hace el código?

## Hard Rules

- **Bidireccional estricto**:
  - Cada afirmación de la prosa sobre comportamiento del código → verificable contra el código.
  - Cada región referenciada (include/tag) → explicada en la prosa (no código mudo).
- Una afirmación de la prosa que el código **no sostiene** es `DIVERGENCE` (critical).
- Un include a un `tag::` que ya no existe es `BROKEN_INCLUDE` (critical).
- Una región del repo que ninguna sección explica es `ORPHAN_CODE` (high).
- Un snippet pegado a mano que no aparece como región del repo es `MANUAL_COPY` (critical) — exactamente el fallo del libro Bevy.

## Modo `review` — Execution Steps

1. Leer el `.adoc`/HTML del capítulo y `planning/code-map.yml` + `planning/code-cards/`.
2. Para cada bloque de código mostrado:
   - ¿Proviene de una región `tag::` del workspace? Si no → `MANUAL_COPY`.
   - ¿El `include::` resuelve a un archivo/tag existente? Si no → `BROKEN_INCLUDE`.
3. Para cada afirmación de la prosa sobre el código ("este sistema recorre...", "al hacer X ocurre Y"):
   - Contrastar con el código real de la región referenciada.
   - Si no corresponde → `DIVERGENCE`.
4. Para cada región referenciada, ¿hay prosa que la explique? Si no → `ORPHAN_REGION_IN_BOOK`.
5. Contrastar prosa contra `code-cards`: la prosa no debe contradecir el `approach`/`not_shown`.
6. Emitir `build/reviews/{chapter-id}.coherence.yml`.

## Modo `drift` — Execution Steps

1. Recibir el diff del workspace de ejemplos (qué crates/regiones cambiaron).
2. Cruzar contra `planning/code-map.yml`:
   - Regiones modificadas → capítulos que las referencian.
   - Tags renombrados/eliminados → includes rotos.
   - Crates eliminados → secciones huérfanas.
3. Para cada capítulo afectado, listar:
   - `BROKEN_INCLUDE`: tag/archivo desaparecido.
   - `CONTENT_DRIFT`: región existe pero cambió contenido (prosa puede estar desactualizada).
   - `ORPHAN_SECTION`: sección citaba un crate eliminado.
4. Emitir `build/drift-code-report.yml` con capítulos afectados y acciones.
5. `release-maintainer` consume este informe para priorizar la re-revisión.

## Categorías de hallazgo

| Categoría | Severidad | Cuándo |
|-----------|-----------|--------|
| `MANUAL_COPY` | critical | Snippet en el libro no es región del repo |
| `BROKEN_INCLUDE` | critical | include:: apunta a tag/archivo inexistente |
| `DIVERGENCE` | critical | Prosa afirma algo que el código no sostiene |
| `ORPHAN_REGION_IN_BOOK` | high | Región referenciada sin explicación |
| `ORPHAN_CODE` | high | Crate del repo sin sección que lo cite |
| `CONTENT_DRIFT` | med | Región cambió; prosa puede estar desfasada |

## Output Contract

- Modo `review`: `build/reviews/{chapter-id}.coherence.yml` + `verdict` (`PASS`/`BLOCKED`).
- Modo `drift`: `build/drift-code-report.yml` con capítulos afectados y acciones.
- Cualquier `critical` → `BLOCKED`; el capítulo vuelve a `chapter-writer`.

## References

- `~/.zcode/skills/BOOK-REPO-CONTEXT.md` — convenciones del workspace.
- `references/divergence-examples.md` — ejemplos reales de divergencia (incluido el caso `bsn!` del piloto).
