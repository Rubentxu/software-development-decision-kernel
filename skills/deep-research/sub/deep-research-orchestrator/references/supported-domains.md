# Dominios soportados

Las 21 skills de deep-research aplican a cualquier dominio. Esta tabla documenta los dominios canónicos y las skills que típicamente se activan.

## Tecnología / Software

| Aspecto | Detalle |
|---------|---------|
| claim_types | `api-existence`, `version`, `behavior`, `performance`, `security`, `best-practice`, `architectural-pattern`, `dependency` |
| Floor típico | L1 (código fuente, RFC, release notes) |
| Decaimiento típico | 1-2 años |
| Sub-pipelines | `deep-software-research`, `deep-pattern-extractor`, `deep-domain-modeler` |
| Fuentes primarias | Código fuente oficial, RFC, release notes, docs oficiales, crates.io/npm/PyPI |

## AI / Machine Learning

| Aspecto | Detalle |
|---------|---------|
| claim_types | `architecture`, `training-data`, `benchmark`, `limitation`, `safety`, `ethical-concern`, `cost`, `capability` |
| Floor típico | L1 (paper arXiv con código), L2 (peer-reviewed venue) |
| Decaimiento típico | 6-12 meses (state-of-the-art) |
| Sub-pipelines | `deep-software-research`, `deep-pattern-extractor`, `deep-traps-detector`, `deep-paradigms-explorer` |
| Fuentes primarias | Papers arXiv, HuggingFace, Papers with Code, conference proceedings |

## Systems Thinking (Donella Meadows)

| Aspecto | Detalle |
|---------|---------|
| claim_types | `concept-meadows`, `archetype-structure`, `leverage-rank`, `feedback-behavior`, `world3-model`, `historical-case`, `policy-resistance`, `paradigm-claim` |
| Floor típico | L1 (Meadows original, Forrester, Senge, Kim) |
| Decaimiento típico | Foundational no caduca; re-tests sí |
| Sub-pipelines | `deep-coach-systems-thinking` (núcleo), `deep-leverage-points-analyst`, `deep-system-archetypes-mapper`, `deep-feedback-loops-analyzer`, `deep-stocks-flows-diagrammer`, `deep-paradigms-explorer`, `deep-traps-detector`, `deep-historical-lineage-tracer` |
| Fuentes primarias | `donellameadows.org`, MIT OCW 15.988, Academy for Systems Change, libros canónicos |

## Ciencia (biología, química, física)

| Aspecto | Detalle |
|---------|---------|
| claim_types | `experimental-result`, `theory`, `mechanism`, `replication-status`, `consensus-position` |
| Floor típico | L1 (peer-reviewed) |
| Decaimiento típico | 3-5 años salvo refutación |
| Sub-pipelines | `deep-domain-modeler`, `deep-knowledge-graph-builder`, `deep-historical-lineage-tracer` |
| Fuentes primarias | Papers peer-reviewed, PubMed, arXiv, society statements |

## Medicina / Salud

| Aspecto | Detalle |
|---------|---------|
| claim_types | `clinical-trial`, `guideline`, `contraindication`, `mechanism`, `side-effect` |
| Floor típico | L1 (ClinicalTrials.gov, peer-reviewed) |
| Decaimiento típico | 2-5 años (guías pueden actualizarse) |
| Sub-pipelines | `deep-domain-modeler`, `deep-traps-detector`, `deep-historical-lineage-tracer` |
| Fuentes primarias | ClinicalTrials.gov, PubMed, FDA/EMA approvals, society guidelines |

## Economía / Política

| Aspecto | Detalle |
|---------|---------|
| claim_types | `dataset`, `policy-impact`, `historical-event`, `opinion-secondary`, `forecast` |
| Floor típico | L2 (datos oficiales: BLS, INE, Banco Mundial) |
| Decaimiento típico | Datos: 1-2 años; análisis: 3-5 años |
| Sub-pipelines | `deep-historical-lineage-tracer`, `deep-paradigms-explorer`, `deep-scenarios-explorer` |
| Fuentes primarias | Datos oficiales, papers peer-reviewed, IMF/WB reports |

## Historia

| Aspecto | Detalle |
|---------|---------|
| claim_types | `event-date`, `primary-source-quote`, `interpretation`, `revisionism`, `counterfactual` |
| Floor típico | L1 (archivo, documento de época) |
| Decaimiento típico | No caduca (contexto estable) |
| Sub-pipelines | `deep-historical-lineage-tracer`, `deep-knowledge-graph-builder` |
| Fuentes primarias | Archivos, autobiografías, historiografía peer-reviewed |

## Filosofía / Ética

| Aspecto | Detalle |
|---------|---------|
| claim_types | `argument`, `school-of-thought`, `ethical-stance` |
| Floor típico | L1 (texto del filósofo), L2 (Stanford Encyclopedia) |
| Decaimiento típico | No caduca |
| Sub-pipelines | `deep-paradigms-explorer`, `deep-historical-lineage-tracer` |
| Fuentes primarias | Textos originales, Stanford Encyclopedia, peer-reviewed |

## Otros dominios

Cualquier dominio puede mapearse siguiendo el patrón:
1. Identificar `claim_types` (ver `references/claim-types.md` para guía).
2. Determinar `floor` por claim_type.
3. Identificar fuentes primarias canónicas.
4. Elegir sub-pipelines relevantes.
5. Aplicar R0 (marco Meadows) para definir el sistema del tema.

---

## Sub-pipelines disponibles (resumen)

| Sub-pipeline | Skill activadora | Cuándo aplicar |
|--------------|------------------|----------------|
| `software-research` | `deep-software-research` | Tecnología concreta |
| `pattern-extraction` | `deep-pattern-extractor` | Implementación verificable |
| `domain-modeling` | `deep-domain-modeler` | Entidades/relaciones |
| `knowledge-graph` | `deep-knowledge-graph-builder` | Muchas entidades relacionadas |
| `historical-lineage` | `deep-historical-lineage-tracer` | Dimensión temporal |
| `scenarios` | `deep-scenarios-explorer` | Proyección/futuros |
| `paradigms` | `deep-paradigms-explorer` | Modelos mentales/cultura |
| `traps` | `deep-traps-detector` | Anti-patrones del dominio |
| `systems-thinking` | `deep-coach-systems-thinking` | Donella Meadows / System Dynamics |
