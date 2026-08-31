---
name: citation-manager
description: "Trigger: citas, bibliografía, references.bib, sources.yaml, claims.jsonl, referencias bibliográficas, BibTeX, CSL, Pandoc citations. Genera y mantiene el sistema de citas (references.bib, sources.yaml, claims.jsonl) y produce bibliografía formateada vía Pandoc/CSL."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo para mantener la **maquinaria de citas** del libro de forma coherente. `source-researcher` crea las fuentes; esta skill las formatea y mantiene exportables a BibTeX/CSL para Pandoc/Asciidoctor.

No la uses para buscar fuentes nuevas (`source-researcher`).

## Hard Rules

- Tres artefactos coherentes: `evidence/sources.yaml` (inventario), `evidence/references.bib` (BibTeX), `evidence/claims.jsonl` (claims).
- Cada entrada BibTeX se sincroniza con su `source_id`.
- Las citas en el `.adoc` usan claves estables (`<<rust-book-ownership>>`).
- El estilo de bibliografía se define una vez (CSL) y se aplica a todo el libro.

## Execution Steps

1. Leer `evidence/sources.yaml` y `evidence/claims.jsonl`.
2. Generar/regenerar `evidence/references.bib` (una entrada por fuente).
3. Mapear claves de cita `source_id` ↔ `bibkey`.
4. Validar que toda cita del `.adoc` resuelve en `.bib`.
5. Configurar estilo CSL (APA, IEEE, etc.) en `build/csl/`.
6. Integrar con Asciidoctor/Pandoc para renderizar la bibliografía al final del libro.
7. Emitir `evidence/citations-report.yml` (citas huérfanas, entradas sin uso).

## Artefactos mantenidos

```
evidence/
├── sources.yaml        # inventario (source-researcher)
├── references.bib      # BibTeX (esta skill)
├── claims.jsonl        # claims (evidence-manager)
└── csl/
    └── style.csl       # estilo bibliográfico
```

## Output Contract

- `evidence/references.bib` regenerado y coherente con `sources.yaml`.
- `evidence/citations-report.yml`.
- Bibliografía renderizada al final del libro (vía book-builder).

## References

- `references/csl-styles.md` — dónde obtener estilos CSL.
