---
name: book-builder
description: "Trigger: renderizar libro, generar PDF, generar EPUB, generar HTML, build del libro, compilar libro, publicar libro, asciidoctor build. Genera HTML, PDF y EPUB publicables a partir de las fuentes AsciiDoc, tras verificar que todos los ejemplos compilan y los capítulos pasan revisión."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo como **fase final del pipeline**, cuando los capítulos han pasado `technical-reviewer` y los ejemplos están `ALL_GREEN`.

No lo uses para arreglar contenido: si el build falla por contenido roto, devolver al skill responsable.

## Hard Rules

- Stack por defecto: **AsciiDoc + Asciidoctor** (`asciidoctor`, `asciidoctor-pdf`, `asciidoctor-epub3`).
- El build **solo se ejecuta** si `code-example-verifier` está en `ALL_GREEN` y ningún capítulo está `BLOCKED`.
- Todos los formatos se generan desde la **misma fuente** (`src/book.adoc`).
- Salidas a `build/` (gitignored); nunca se commitean binarios.

## Execution Steps

1. **Preflight** — comprobar:
   - `build/verify-report.jsonl` → todos los ejemplos `ALL_GREEN`.
   - Ningún `build/reviews/*.review.yml` en `BLOCKED`.
   - `evidence/claims.jsonl` sin claims `unverified` críticos.
   - Si el libro usa `book-template/` (modo compartido),
     `book-template/scripts/sync-to-blog.sh --apply` se ejecuta
     para asegurar que `libros/shared/` está al día.
2. Si el preflight falla → `BUILD_REFUSED` con lista de bloqueos.
3. Ejecutar cadena de render (ver `assets/build-asciidoc.sh`):
   ```bash
   asciidoctor -D build/html src/book.adoc
   asciidoctor-pdf -D build/pdf src/book.adoc
   asciidoctor-epub3 -D build/epub src/book.adoc
   ```
4. Validar que los `include::` resuelven y que los diagramas embebidos existen.
5. Validar que `book-template/scripts/verify-book.sh` no detecta
   archivos huérfanos ni numeración no consecutiva.
6. Comprobar TOC, índice y numeración coherentes entre formatos.
7. Si el libro es multi-página, ejecutar
   `book-template/scripts/build-chapters.py --clean`
   para regenerar la salida web multi-página.
8. Emitir `build/manifest.json` con rutas, tamaños y checksums.
9. Si CI: publicar artefactos (GitHub Pages, release, etc.).

## Decision Gates

| Estado | Acción |
|--------|--------|
| Ejemplo `BLOCKED` | Rechazar build → `code-example-verifier` |
| Capítulo `BLOCKED` | Rechazar build → `technical-reviewer` |
| Claim `unverified` crítico | Rechazar → `hallucination-auditor` |
| Build roto por sintaxis | Devolver a `chapter-writer` |

## Output Contract

- `build/html/`, `build/pdf/`, `build/epub/` con los artefactos.
- `build/manifest.json` (ruta, tamaño, sha256 por artefacto).
- Estado: `BUILT` o `BUILD_REFUSED` (con bloqueos).

## References

- `assets/build-asciidoc.sh` — script de render multi-formato.
