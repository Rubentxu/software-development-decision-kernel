# Estrategia de búsqueda por modalidad

Queries y tácticas concretas para descubrir en cada modalidad. El objetivo es **amplitud**, no filtrado.

## official-docs (L2)
- `site:docs.rs {crate}` — API pública.
- `site:doc.rust-lang.org {concepto}` — libro oficial del lenguaje.
- `{framework} docs {concepto} version {X.Y}` — docs versionadas.
- Navegar el índice del módulo en docs.rs (no solo buscar).

## spec-standard (L1)
- `{tecnología} RFC {tema}` — ej. "Rust RFC ownership".
- `site:datatracker.ietf.org {keyword}`.
- Para web: `site:w3.org {spec}` o `site:tc39.es`.

## source-code (L3)
- `site:github.com/{org}/{repo}` + buscar en Issues/PRs/Discussions.
- `git log -S "{símbolo}"` para cuándo se introdujo algo.
- `CHANGELOG.md` y `release notes` entre versiones.
- Tests del propio repo: demuestran comportamiento.

## academic-paper (L4)
- `site:arxiv.org {tema}` o Google Scholar.
- Verificar DOI en `doi.org/{doi}`.

## maintainer-post (L5)
- Blog oficial del proyecto (`{framework}.org/blog`).
- Discourse/foro oficial.
- Charlas de GDC/RustConf del equipo core.

## canonical-book (L6)
- Catálogos: O'Reilly Learning, Manning, No Starch, Pragmatic Bookshelf, editoriales de la comunidad (Rust Book oficial).
- Google Books para metadatos (autor, ISBN, año, edición).
- `library genesis` SOLO como índice de descubrimiento; la fuente citable es el libro físico/ebook legal.
- Criterios: autor verificable, editorial reputada, edición reciente, citado por la comunidad.

## technical-manual (L6)
- Handbooks oficiales (FreeBSD Handbook, JDK docs extendidos, Linux man-pages).
- Reference guides del proyecto (`{framework} reference`).

## community-blog (L7)
- Blogs de autores con trayectoria (verificar: bio, otros posts, si son contributors del proyecto).
- Agregadores: Hacker News, lobste.rs — para descubrir, no para citar.
- Fecha obligatoria.

## community-post (L7)
- SO: respuesta aceptada con score alto + fecha.
- Reddit de la comunidad con upvotes.
- Medium/Dev.to con autor verificable.

## companion-articles (L7)
- Tutoriales largos y profundos (taintedcoders, books online no oficiales).
- Útiles para contexto y para ver cómo otros explican el tema.

## Regla de diversidad
Para cada pregunta crítica, buscar al menos una fuente de **cada** modalidad admisible relevante. Dos blogs que dicen lo mismo no son triangulación; un blog + la doc oficial + el código sí.
