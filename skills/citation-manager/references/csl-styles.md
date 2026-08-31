# Estilos CSL

El repositorio canónico de estilos CSL es https://github.com/citation-style-language/styles.

Estilos comunes para libros técnicos:
- `ieee.csl` — IEEE (numérico, habitual en informática).
- `acm-siggraph.csl` — ACM.
- `apa.csl` — APA (autor-año).
- `chicago-author-date.csl` — Chicago.

## Uso con Asciidoctor
Asciidoctor no trae BibTeX nativo, pero se integra con Pandoc para el paso de bibliografía, o se usan extensiones (`asciidoctor-bibtex`). La salida recomendada es renderizar las referencias con Pandoc desde el `.bib` + `.csl` e incluirlas como sección final.

## Reglas
- Un estilo por libro (definir en `book-config.yml`).
- Las claves de cita son estables y derivan del `source_id`.
- Nunca editar el `.bib` a mano si `sources.yaml` es la fuente de verdad: regenerar.
