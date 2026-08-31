---
name: book-project-initializer
description: "Trigger: nuevo libro, inicializar libro, crear repositorio de libro, book project setup, bootstrap book. Crea el repositorio, estructura de capítulos, configuración de publicación (AsciiDoc/Asciidoctor), convenciones y CI para un libro técnico."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo cuando el usuario quiera **empezar un libro técnico nuevo** desde cero (no un capítulo suelto). Crea la estructura de repositorio completa, reproducible y versionable.

No lo uses para añadir un capítulo a un libro ya inicializado (eso es `book-outline-architect` + `chapter-planner`).

## Hard Rules

- Stack por defecto: **AsciiDoc + Asciidoctor** (ver `references/stack-asciidoc.md`).
- Todo artefacto generado debe ser **determinista**: el mismo `book-config.yml` produce la misma estructura.
- Los ejemplos de código viven en **proyectos ejecutables reales**, no como fragmentos sueltos en el texto.
- El repositorio debe construirarse y pasar CI desde el primer commit.

## Estructura canónica del repositorio

```
book-project/
├── book-config.yml              # Metadatos del libro (título, autor, stack, versiones)
├── README.md
├── justfile                     # Comandos: build, test, lint, render
├── .github/workflows/ci.yml     # Compilar + testear + renderizar en cada push
├── src/                         # Fuente AsciiDoc del libro
│   ├── book.adoc                # Master que incluye partes y capítulos
│   ├── parts/
│   │   ├── part-1.adoc
│   │   └── part-2.adoc
│   └── chapters/
│       ├── _chapter-template.adoc
│       └── ch01-introduccion.adoc
├── examples/                    # Proyectos ejecutables (un Cargo.toml/proyecto por ejemplo)
│   └── ch01-hello/
│       └── ... (proyecto real)
├── diagrams/                    # Diagramas como código (.mmd, .puml, .dot)
├── evidence/                    # sources.yaml, claims.jsonl, references.bib
│   ├── sources.yaml
│   └── claims.jsonl
├── research/                    # Inventario de fuentes y evidence cards
├── planning/                    # audience-profile.yml, curriculum-graph.yml, outline.yml
└── build/                       # Salidas renderizadas (HTML/PDF/EPUB) — gitignored
```

## Execution Steps

1. Leer `book-config.yml` del usuario; si no existe, pedir: título, autor, tecnología principal, versión objetivo, idioma (castellano por defecto), nivel del lector.
2. Validar config contra `assets/book-config.schema.yml`.
3. Generar la estructura de directorios anterior.
4. Crear `book.adoc` (master), `_chapter-template.adoc`, `justfile`, `ci.yml`.
5. Inicializar git, primer commit con `.gitignore` (ignora `build/`).
6. Ejecutar `just build` para verificar que el esqueleto compila a HTML.
7. Devolver la ruta del proyecto y los próximos pasos sugeridos (audience → curriculum → outline).

## Decision Gates

| Necesidad | Acción |
|-----------|--------|
| Stack distinto (mdBook/Quarto) | Generar la variante correspondiente desde `references/` |
| Libro Rust/Bevy | Precargar `justfile` con `cargo fmt/clippy/test` + `asciidoctor` |
| Libro multi-lenguaje | Un directorio `examples/` por capítulo con su propio toolchain |

## Output Contract

Devuelve:
- Ruta absoluta del proyecto creado.
- Árbol de archivos generados.
- Resultado de `just build` (debe ser verde).
- Próximos skills recomendados: `audience-profiler` → `curriculum-designer` → `book-outline-architect`.

## References

- `references/stack-asciidoc.md` — plantillas de book.adoc, justfile y ci.yml para Asciidoctor.
