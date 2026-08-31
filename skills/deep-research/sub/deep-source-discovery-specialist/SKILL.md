---
name: deep-source-discovery-specialist
description: "Trigger: descubrir fuentes, encontrar papers, localizar fuentes primarias, búsqueda de literatura, papers sobre X, dónde encuentro info sobre Y. Descubre fuentes multi-modales (papers, libros, código fuente, datos, blogs, videos, podcasts) para responder las preguntas del deep-research-strategist. Funciona para CUALQUIER dominio: tecnología, ciencia, sistemas, historia, etc. Aplica los principios de Meadows sobre cómo buscar evidencia sin caer en system traps (Policy Resistance, Shifting the Burden a fuentes secundarias)."
license: Apache-2.0
metadata:
  category: deep-research
  subcategory: r-pipeline
  author: rubentxu
  version: "1.0"
  domain: deep-research
  based_on: "source-discovery-specialist (rubentxu), generalizado"
---

## Activation Contract

Úsalo **después de `deep-research-strategist`**, una vez que existe `research/agenda.yml` con preguntas priorizadas. Recorre el pool de fuentes primarias y secundarias revisadas para construir un `candidate-pool.yml` que alimenta al `deep-source-credibility-assessor` (R3a) y `deep-reference-validator` (R3b).

No lo uses para: puntuar credibilidad (`deep-source-credibility-assessor`), validar referencias vivas (`deep-reference-validator`), triangular claims (`deep-evidence-triangulator`). Esta skill **descubre**; las demás **validan**.

## Hard Rules

- **Fuentes primarias primero.** Para todo concepto canónico de un campo, buscar el paper/libro original.
- **Multi-modal**: papers, libros, modelos, código fuente, tutoriales oficiales, archivos institucionales, datos primarios, podcasts/transcripciones cuando el autor es autoridad.
- **Cada candidato tiene metadatos completos**: título, autor, año, URL, tipo, idioma, claim_types que cubre, evidencia_level estimado.
- **Cobertura**: cada pregunta `risk: critical` de la agenda debe tener ≥ 2 candidatos de fuentes distintas.
- **No inventar URLs.** Si no se verifica la URL, marcar `url_status: unverified` y rechazar como candidato L1.
- **Idempotente**: una pregunta ya investigada (status `resolved`) no se re-descubre salvo que la agenda pida re-check.
- **Anti-Shifting the Burden**: no quedarse en fuentes L3-L5 cuando existen L1. Si una pregunta `critical` solo tiene fuentes secundarias, marcarla `escalate` y priorizar descubrir L1.

## Execution Steps

1. Lee `research/agenda.yml`. Filtra por `risk: critical` y `risk: normal` (descarta `low` salvo que el corpus esté vacío).
2. Para cada pregunta, identifica el `claim_type` y consulta la tabla de fuentes admisibles según el dominio (ver `references/source-types-by-domain.md`).
3. **Búsqueda por fuente canónica**:
   - Identifica los autores/Instituciones canónicos del campo (papers seminales, libros fundacionales).
   - Busca en sus sitios oficiales, repositorios (arXiv, JSTOR, PubMed, Google Scholar).
4. **Búsqueda por paper** (vía WebSearch):
   - Queries: `"{autor canónico}" {claim_type}`, `"{concepto}" {término técnico}`.
   - Prioriza Google Scholar, ResearchGate, repositorios institucionales.
   - Filtra por año (recientes para tech; foundational para science).
5. **Búsqueda por modelo/código fuente** (cuando el tema es tecnología):
   - Repos oficiales en GitHub/GitLab.
   - Registro de paquetes (crates.io, npm, PyPI).
   - Documentación oficial.
6. **Búsqueda por archivo institucional**:
   - Universidades (MIT OCW, Stanford Encyclopedia, etc.).
   - Sociedades científicas (ACM, IEEE, APA, AMA).
   - Gobiernos / organismos internacionales (INEGI, BLS, WHO, IPCC).
7. **Búsqueda por modelo predictivo/cuantitativo** (cuando aplica):
   - Datasets primarios (Kaggle, UCI ML, OpenNeuro).
   - Modelos pre-entrenados (HuggingFace, TensorFlow Hub).
   - Benchmarks publicados.
8. Por cada candidato encontrado, anota:
   - `title`, `author(s)`, `year`, `publisher`/`journal`, `url`, `url_status` (live/dead/unverified).
   - `source_type`: `book-primary` | `paper-peer-reviewed` | `paper-grey` | `institutional-white-paper` | `model-code` | `tutorial` | `encyclopedia` | `archive` | `blog` | `video` | `podcast` | `dataset`.
   - `evidence_level` estimado: L1-L7.
   - `claim_types_covered`: lista.
   - `independent_from`: otros candidatos (para evaluar independencia en R4).
9. Genera `research/candidate-pool.yml` (esquema en `assets/candidate-pool.schema.yml`).
10. Marca gaps: si una pregunta `critical` queda con < 2 candidatos, márcala `coverage: insufficient`.

## Tipos de fuente por dominio (resumen)

| Dominio | Fuentes primarias | Fuentes secundarias válidas |
|---------|-------------------|----------------------------|
| Tecnología / Software | Código fuente, RFC, release notes, docs oficiales | MDN, Stack Overflow (sólo L3), blogs oficiales |
| IA / ML | Papers originales (arXiv + código), datasets originales | Papers peer-reviewed en venues (NeurIPS, ICML) |
| Systems Thinking | Meadows/Forrester/Senge originales, MIT OCW, Academy for Systems Change | Wikipedia, The Systems Thinker |
| Ciencia | Papers peer-reviewed, datasets primarios, Society statements | Review papers, Stanford Encyclopedia |
| Historia | Archivos primarios, documentos de época, autobiografías | Historiografía peer-reviewed |
| Medicina | Clinical trials registrados, papers peer-reviewed | Society guidelines, FDA approvals |
| Economía | Datos oficiales, papers peer-reviewed | Reports de instituciones (IMF, WB) |

Para el detalle completo, ver `references/source-types-by-domain.md`.

## Esquema de candidate-pool (resumen)

```yaml
candidate_pool:
  generated_at: "2026-08-16"
  system_id: "bevy-ecs-scheduling"
  questions_covered: 5
  sources_found: 12
  candidates:
    - id: src-bevy-source-schedule
      title: "Bevy Schedule source code"
      authors: ["Carter Anderson"]
      year: 2026
      url: "https://github.com/bevyengine/bevy/blob/main/crates/bevy_ecs/src/schedule/"
      url_status: live
      source_type: source-code
      evidence_level: L1
      claim_types_covered: [api-existence, behavior, version]
      independent_from: [src-bevy-docs, src-bevy-rfcs]
    - id: src-bevy-rfcs
      title: "Bevy RFCs repository"
      authors: ["Bevy contributors"]
      year: 2024
      url: "https://github.com/bevyengine/rfcs"
      url_status: live
      source_type: institutional-white-paper
      evidence_level: L1
      claim_types_covered: [architectural-pattern, design-decision]
      independent_from: [src-bevy-source-schedule]
  coverage:
    RQ-scheduling-access-conflicts: {candidates: 2, status: sufficient}
    RQ-system-trap-detection: {candidates: 1, status: insufficient}
```

## Decision Gates

| Situación | Acción |
|-----------|--------|
| Pregunta `critical` con 0 candidatos | Marcar `coverage: insufficient` y escalar al autor |
| Solo se encuentran blogs (L5) para un claim_type que exige L1 | Marcar `coverage: escalate` (no se puede resolver al nivel requerido) |
| URL no verificada | `url_status: unverified` + warning; no usar como L1 |
| Fuente encontrada pero es una reinterpretación moderna | Marcar `evidence_level: L3` (no L1) |
| Concepto canónico citado en un blog que NO coincide con el original | Marcar `conflict: detected` y comparar con texto primario |
| Tema de Systems Thinking | Usar `references/canonical-urls-systems-thinking.md` para URLs verificadas |
| **Caer en Shifting the Burden**: descubrir muchas fuentes L3-L5 pero pocas L1 | STOP: re-priorizar la búsqueda hacia L1 (papers, libros originales, código fuente) |

## Output Contract

- `research/candidate-pool.yml` con metadatos completos de cada candidato.
- `coverage: insufficient` o `coverage: escalate` marcado para preguntas sin cobertura.
- `deep-source-credibility-assessor` y `deep-reference-validator` reciben el candidate-pool.

## References

- `assets/candidate-pool.schema.yml` — esquema validable.
- `references/source-types-by-domain.md` — tabla detallada por dominio.
- `references/canonical-urls-systems-thinking.md` — URLs verificadas para el dominio Systems Thinking (Meadows).
- `references/canonical-urls-technology.md` — URLs verificadas para tecnología.
