---
name: deep-claim-extractor
description: "Trigger: extraer evidence cards, preparar claims para capítulo, evidence card para chapter-writer, claim listo para citar. Extrae evidence cards desde el corpus verified para que el chapter-writer (modo LIBRO) o el agente de code generation (modo SOFTWARE) las use directamente. Es el puente entre el corpus y el consumidor final. Núcleo de R6."
license: Apache-2.0
metadata:
  category: deep-research
  subcategory: r-pipeline
  author: rubentxu
  version: "1.0"
  domain: deep-research
---

## Activation Contract

Úsalo en **R6 del pipeline R**, después de R5 (`deep-knowledge-corpus-curator`). Recibe el corpus y produce:
- Modo LIBRO: `research/evidence-cards/{topic}.yml` consumible por `chapter-writer`.
- Modo SOFTWARE: `research/blueprints/{component}.yml` consumible por code generation.

No lo uses para: investigar (R0-R2), evaluar (R3-R4), consolidar (R5). Esta skill **empaqueta** para el consumidor final.

## Hard Rules

- **Solo claims `verified`** (o `verified-with-disclaimer` con el disclaimer incluido) pueden extraerse a evidence cards.
- **Cada evidence card** lleva:
  - `claim_id` (único en el corpus).
  - `text` (la afirmación exacta que aparecerá en el libro/código).
  - `sources[]` con `excerpt` textual cuando esté disponible.
  - `quote_style` recomendado (inline / block / paraphrase).
  - `confidence_score`.
  - `decay_date`.
- **Sin excerpt, no es L1/L2 utilizable**: si la fuente es L1 pero no tenemos la cita textual, marcar `quote_style: paraphrase` y degradar a L3 para la cita.
- **Trazabilidad**: cada evidence card referencia al menos un `corpus.claim.id`.
- **Agrupación por capítulo/componente**: las evidence cards se agrupan por el tema que el `chapter-writer` o `code generation` necesita.

## Execution Steps

1. Lee `research/corpus.yml`.
2. Filtra: solo claims con `status: verified` o `verified-with-disclaimer`.
3. Agrupa por `topic` o por `chapter_id` (LIBRO) o `component_id` (SOFTWARE).
4. Para cada grupo:
   - Modo LIBRO: genera `research/evidence-cards/{topic}.yml` con todas las claims del tema, cada una con excerpt listo para incluir en AsciiDoc.
   - Modo SOFTWARE: genera `research/blueprints/{component}.yml` con la interfaz, algoritmo, referencias y test_acceptance (ver `references/blueprint-template.md`).
5. Si alguna claim `verified-with-disclaimer`, asegurar que el disclaimer esté en la card.
6. Si alguna claim tiene `decay_date` próxima (< 30 días), marcar `recheck_soon: true` en la card.

## Esquema de evidence-cards/{topic}.yml (modo LIBRO)

```yaml
evidence_cards:
  topic: "leverage-points"
  chapter: "cap-06-leverage-points"
  generated_at: "2026-08-16"
  corpus_version: "2026-08-16-01"
  cards:
    - claim_id: cl-leverage-points-list
      text: "Donella Meadows identificó 12 lugares para intervenir en un sistema, ordenados de menor a mayor efectividad."
      quote_style: block
      excerpt: "I'm starting with that list, in increasing order of effectiveness..."
      source:
        id: src-meadows-2008
        title: "Thinking in Systems: A Primer"
        author: "Donella H. Meadows"
        publication: "Chelsea Green Publishing, 2008"
        page_reference: "Chapter 6, pp. 145-165"
        url: "https://research.fit.edu/.../Meadows-2008.-Thinking-in-Systems.pdf"
      confidence_score: 0.98
      status: verified
      decay_date: "2031-08-16"
      recheck_soon: false
      asciidoc_hint: |
        === The 12 Leverage Points
        
        [%quote]
        ____
        I'm starting with that list, in increasing order of effectiveness...
        ____
        
        -- {author}, {publication}, {page_reference}
    - claim_id: cl-leverage-points-resistance
      text: "Los leverage points altos son resistidos por el sistema."
      quote_style: inline
      excerpt: "The higher the leverage point, the more the system will resist changing it — that's why societies often rub out truly enlightened beings."
      source:
        id: src-meadows-2008
        page_reference: "Chapter 6"
```

## Esquema de blueprints/{component}.yml (modo SOFTWARE)

```yaml
blueprint:
  id: feedback-loop-detector
  name: "Feedback Loop Detector"
  purpose: "Detecta si un CLD contiene loops balancing/reinforcing y reporta polaridad."
  interface:
    language: python
    inputs: [...]
    outputs: [...]
    errors: [...]
  algorithm:
    steps: [...]
    complexity: "O(n²)"
    dependencies: [...]
  references:
    - claim_id: cl-balancing-feedback-loop-definition
      citation: "Meadows 2008, Chapter 2"
      how_to_use: "..."
  test_acceptance:
    - name: "..."
      expected_behavior: "..."
      reference_value: "..."
  corpus_version: "2026-08-16-01"
  generated_at: "2026-08-16"
```

## Decision Gates

| Situación | Acción |
|-----------|--------|
| Claim `verified` sin excerpt textual | Marcar `quote_style: paraphrase` y degradar a L3 en el uso de la cita |
| Claim con `decay_date` < 30 días | `recheck_soon: true`; el curator agenda re-check |
| Claim con `disputed` | NO extraer como evidence card; el chapter-writer debe presentar el debate |
| Capítulo requiere claim que NO está en el corpus | STOP: el chapter-writer no debe inventar; el curator debe investigar primero |
| Modo DUAL | Generar ambos artefactos y verificar coherencia cifra↔test |

## Output Contract

- `research/evidence-cards/{topic}.yml` (modo LIBRO).
- `research/blueprints/{component}.yml` (modo SOFTWARE).
- `research/code-patterns/{pattern}.{md,py,rs}` (SOFTWARE, opcional, cuando la blueprint incluye snippets).
- `research/LEDGER.md` actualizado con la lista de cards generadas.

## References

- `references/evidence-card-template.md` — plantilla detallada.
- `references/blueprint-template.md` — plantilla para blueprints.
- `assets/evidence-card.schema.yml`, `assets/blueprint.schema.yml`.
