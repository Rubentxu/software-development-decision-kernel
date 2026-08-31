---
name: source-discovery-specialist
description: "Trigger: descubrir fuentes, buscar bibliografía, encontrar libros técnicos, manuales, RFC, papers, blogs, posts, documentación oficial, ejemplos de código, recopilar material. Descubrimiento multi-modal y amplio de fuentes candidatas (docs oficiales, RFC, papers, código, libros técnicos, manuales, blogs y posts de calidad) para alimentar el corpus de investigación."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo **después** de `research-strategist` (necesita la agenda priorizada) y **antes** de `source-researcher` (que extrae evidence cards de las fuentes que esta descubre). Su trabajo es **descubrimiento amplio**: cuantas más fuentes candidatas relevantes, mejor; el filtrado lo hacen `source-credibility-assessor` y `evidence-cross-validator`.

No la uses para extraer afirmaciones (`source-researcher`) ni para validar que una URL existe (`reference-validator`).

## Hard Rules

- Descubrir en **todas las modalidades admisibles** para cada pregunta de la agenda (ver tabla de modalidades).
- **Literatura técnica secundaria** (libros, manuales, blogs, posts) es válida y deseable — pero se etiqueta como `secondary` y nunca se cita sola para una afirmación `critical`.
- Toda fuente candidata se registra con `retrieved_at` + `version` (si aplica) + `modalidad`.
- No descartar en esta fase: se registra todo lo relevante; el filtrado es posterior.
- Buscar **diversidad**: no conformarse con la primera fuente; el cross-validator necesita varias para triangular.

## Modalidades de descubrimiento

| Modalidad | Qué buscar | Nivel típico |
|-----------|-----------|--------------|
| `official-docs` | docs.rs, doc.rust-lang.org, bevyengine.org/learn, MDN | L2 |
| `spec-standard` | RFC (IETF, Rust RFC), W3C, ISO, ECMAScript spec | L1 |
| `source-code` | repo oficial, módulos, tests, release notes, changelog | L3 |
| `academic-paper` | arXiv, ACM, IEEE, DOI verificable | L4 |
| `maintainer-post` | blog oficial, discourse, charlas del equipo core con fecha | L5 |
| `canonical-book` | libros técnicos de referencia (O'Reilly, No Starch, Manning, oficial del lenguaje) | L6 |
| `technical-manual` | manuales de referencia (man pages, handbook oficial de FreeBSD, JDK docs extendidos) | L6 |
| `community-blog` | blogs técnicos de autores reconocidos con trayectoria verificable | L7 |
| `community-post` | SO aceptada, Reddit de la comunidad, articles | L7 |
| `companion-articles` | artículos y tutoriales largos que cubren el tema en profundidad | L7 |

## Execution Steps

1. Leer `research/agenda.yml` (preguntas priorizadas y `admissible_sources`).
2. Para cada pregunta `risk: critical` primero, luego las `normal`:
   - Lanzar descubrimiento en cada modalidad admisible.
   - Para libros/manuales: buscar en catálogos (publisher sites, Google Books, Library Genesis solo como índice, nunca como fuente final), identificar autor + edición + año.
   - Para blogs/posts: identificar autor con trayectoria, fecha, si es citado por la comunidad.
3. Registrar cada candidata en `research/candidate-pool.yml` (esquema en `assets/candidate-pool.schema.yml`).
4. Evitar duplicados (mismo contenido en dos URLs).
5. Marcar `coverage` por pregunta: ¿tenemos al menos 2 fuentes independientes para triangular?

## Esquema de candidate pool (resumen)

```yaml
candidate:
  id: cand-brown-bsn-post
  agenda_question: RQ-bsn-syntax
  modalidad: community-blog
  url: https://example.com/bevy-bsn-walkthrough
  title: "A practical guide to bsn!"
  author: "Jane Doe"            # verificar trayectoria
  published: "2026-07-01"
  retrieved_at: "2026-07-23"
  relevance: high
  level_estimate: L7
  notes: "Cubre sintaxis; parece derivar de la doc oficial — útil como contexto."
```

## Decision Gates

| Situación | Acción |
|-----------|--------|
| Pregunta sin ≥2 fuentes independientes | Marcar `coverage: thin` → más descubrimiento |
| Solo fuentes L7 (blogs) para afirmación critical | `source-credibility-assessor` + buscar L2/L3 |
| Fuente secundaria (libro/blog) que contradice la oficial | Flag para `evidence-cross-validator` |
| Libro sin autor verificable o fecha | Descartar (no es fuente, es ruido) |

## Output Contract

- `research/candidate-pool.yml` con todas las candidatas.
- Mapa de cobertura: qué preguntas tienen ≥2 fuentes y cuáles están `thin`.
- `source-researcher` extraerá evidence cards de las candidatas que pasen `source-credibility-assessor`.

## References

- `references/discovery-search-strategy.md` — estrategias de query por modalidad.
- `assets/candidate-pool.schema.yml` — esquema del pool de candidatas.
