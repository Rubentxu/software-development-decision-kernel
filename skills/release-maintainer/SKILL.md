---
name: release-maintainer
description: "Trigger: mantener libro, nueva edición, dependencias obsoletas, actualizar libro, mantenimiento, release del libro, segunda edición. Detecta dependencias obsoletas, breaking changes y prepara nuevas ediciones del libro manteniendo coherencia entre fuentes, ejemplos y versiones."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo de forma **periódica** (cada release del framework principal, o al menos trimestralmente) para mantener el libro vivo. Trabaja junto con `version-drift-detector`.

No la uses para crear contenido nuevo (`chapter-writer`).

## Hard Rules

- Toda actualización de versión debe propagarse a: `book-config.yml`, `examples/*/Cargo.toml`, `sources.yaml` (re-retrieved_at) y `claims.jsonl` (needs_recheck).
- Un cambio de versión que rompe ejemplos **bloquea la edición** hasta que `code-example-verifier` vuelve a verde.
- Cada edición del libro se registra en `CHANGELOG.adoc` y sube el `edition` en `book-config.yml`.
- Los breaking changes del framework se documentan (qué cambió, cómo migrar).

## Execution Steps

1. Ejecutar `version-drift-detector` para obtener el impacto.
2. Para cada dependencia con drift:
   - Actualizar `book-config.yml` y los manifests afectados.
   - Re-revisar las sources afectadas (`source-researcher` para nueva versión).
   - Marcar claims afectados como `needs_recheck` (`evidence-manager`).
3. Re-ejecutar `code-example-verifier` en todos los ejemplos.
4. Para los que fallan: delegar corrección a `code-example-generator`.
5. Re-ejecutar `technical-reviewer` + `hallucination-auditor` en capítulos afectados.
6. Renderizar nueva edición (`book-builder`).
7. Actualizar `CHANGELOG.adoc`.

## Output Contract

- `edition` incrementado en `book-config.yml`.
- `CHANGELOG.adoc` con los cambios de la edición.
- Todos los ejemplos `ALL_GREEN` tras la actualización.
- `build/release-report.yml` (qué cambió, qué capítulos se vieron afectados, severidad).

## References

- `references/maintenance-cadence.md` — cuándo y cómo ejecutar mantenimiento.
