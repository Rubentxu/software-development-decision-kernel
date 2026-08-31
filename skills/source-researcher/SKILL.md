---
name: source-researcher
description: "Trigger: buscar fuentes, documentación oficial, evidence cards, extraer afirmaciones verificables del corpus, RFC, papers, primary sources, recopilar referencias. Extrae evidence cards verificables del corpus de conocimiento (o, si no existe, hace investigación focalizada) para que chapter-writer solo cite afirmaciones con respaldo. En la Macro-fase R es consumidor del corpus, no descubridor amplio."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.1"
---

## Activation Contract

Úsalo **antes** de que `chapter-writer` redacte, y como investigación previa a `chapter-planner`. Ninguna afirmación técnica del libro debe apoyarse en resultados de búsqueda no verificados.

**Dos modos según el contexto**:

- **Modo `corpus-driven` (preferente)**: si existe `research/corpus.yml` (producido por `research-knowledge-curator` tras la Macro-fase R), extrae evidence cards de las claims `verified` del corpus. No redescubre fuentes — el corpus ya las trianguló.
- **Modo `focal` (fallback)**: si no hay corpus, hace investigación focalizada para el capítulo concreto (flujo original). Útil para capítulos sueltos sin Macro-fase R previa.

No lo uses para descubrimiento amplio (`source-discovery-specialist`), ni para triangular (`evidence-cross-validator`), ni para validar URLs (`reference-validator`). Es el **extractor** que convierte conocimiento verificado en cards citables.

## Hard Rules

- Priorizar **siempre** fuentes primarias: documentación oficial, RFC, código fuente, specs, papers.
- **Registrar la versión** de cada tecnología en el momento de la consulta (`retrieved_at` + `version`).
- Separar **hechos**, **interpretaciones** y **recomendaciones**. Nunca mezclarlos.
- Guardar **extractos pequeños** junto con su procedencia exacta (URL + versión + fecha + selector si aplica).
- `chapter-writer` solo puede citar evidence cards verificadas; no resultados de búsqueda en crudo.

## Execution Steps

### Modo `corpus-driven` (cuando existe `research/corpus.yml`)
1. Recibir el scope de investigación (conceptos del capítulo).
2. Consultar el corpus: localizar claims `verified` que cubran esos conceptos.
3. Para cada claim verificada, generar una evidence card con su `quote`, `source_id` y `confidence_score` del corpus.
4. Si un concepto del capítulo **no** tiene claim en el corpus → marcar gap y escalar (la Macro-fase R debe cubrirlo, o hace falta investigación focal).
5. Persistir las cards en `research/evidence-cards/{concept}.yml`.

### Modo `focal` (cuando no hay corpus)
1. Recibir el scope de investigación (conceptos del contrato de capítulo).
2. Para cada concepto, localizar candidatos de fuente por orden de autoridad (L1→L7).
3. Para cada fuente, extraer **evidence cards** (1 afirmación verificable por card).
4. Persistir en `research/sources.yaml` (inventario) y `research/evidence-cards/` (cards).
5. Recomendar ejecutar la Macro-fase R completa si el libro es serio (el modo focal es menos robusto que la triangulación del corpus).

## Esquema de fuente (sources.yaml)

```yaml
source:
  id: rust-book-ownership
  type: official-documentation
  title: "Understanding Ownership"
  technology: Rust
  version: "1.95"
  retrieved_at: "2026-07-22"
  authority: primary
  url: https://doc.rust-lang.org/book/ch04-01-what-is-ownership.html
  chapters: [ownership, borrowing]
```

## Esquema de evidence card

```yaml
card:
  id: ev-borrowing-exclusive-mut
  source_id: rust-book-ownership
  claim: "El préstamo mutable debe ser exclusivo durante su uso."
  type: fact            # fact | interpretation | recommendation
  quote: "At any given time, you can have either one mutable reference..."
  quote_locator: "§4.2"
  verified_at: "2026-07-22"
  status: verified      # verified | unverified | disputed
```

## Output Contract

- `research/sources.yaml` actualizado.
- Evidence cards en `research/evidence-cards/{concept}.yml`.
- Índice de cobertura: qué conceptos tienen evidencia y cuáles siguen sin ella.
- Marcar conceptos sin fuente primaria como **riesgo** (bloquea publicación si persisten).

## References

- `assets/evidence-card.schema.yml` — esquema de la card.
- `references/authority-ranking.md` — orden de autoridad de fuentes.
