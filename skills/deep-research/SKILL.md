---
name: deep-research
description: "Trigger: investigar tema a fondo, investigación profunda, evidencia rigurosa, fuentes primarias, corpus de conocimiento, agenda de investigación, triangulación, knowledge gaps, dinámica de sistemas, leverage points, feedback loops, stocks and flows, system archetypes, Donella Meadows, R0 R6, deep dive, evidence-based research. Skill MAESTRA de investigación profunda: define el sistema (R0), planifica, descubre fuentes, valida credibilidad, triangular, consolida corpus y extrae deliverables. Aplica el marco metodológico de Donella Meadows (Thinking in Systems 2008, Leverage Points 1997, Limits to Growth 1972, Dancing with Systems 2001) como lente transversal para investigar CUALQUIER tema. Esta skill es el índice de las 22 sub-skills especializadas en `sub/`; ver `references/index.md` para el mapa completo."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
  category: deep-research
  subcategory: master
  domain: deep-research
  methodology: donella-meadows-systems-thinking
  bundled: true
  type: master-skill
  sub_skills_count: 22
  references: [references/index.md, references/pipeline-r0-r6.md]
---

## Activation Contract

Úsala cuando el usuario pida investigar a fondo cualquier tema que pueda ser un capítulo de libro técnico, una decisión de arquitectura, un análisis de evidencia, o cualquier problema donde se necesite el rigor metodológico de Donella Meadows.

Esta es una **skill MAESTRA** (skill index pattern, LangChain "hierarchical skills"). Cuando se carga, expone 22 sub-skills en `sub/` que el agente puede invocar individualmente según la fase del pipeline R0–R6.

## Hard Rules

- **R0 obligatorio** antes de investigar: definir el sistema (propósito, elementos, interconexiones, feedback loops, leverage points, paradigmas, traps).
- **L1 floor para claims `critical`**: fuentes primarias (papers, libros, código fuente, docs oficiales).
- **Triangulación**: claims `critical` requieren ≥ 2 fuentes independientes.
- **Cita con página/sección**: nunca vaga.
- **Decaimiento**: tech caduca 1-2 años, AI/ML 6-12 meses, foundational (Meadows) no caduca.
- **No inventar cuantificaciones**: gap explícito > dato inventado.

## Pipeline R (R0-R6) con sub-skills

```
R0  Definir el sistema del tema (Meadows)         [obligatorio]
    Sub-skill relevante: ninguna específica (aplicar manualmente)
    → research/system-map/{topic}.yml

R1  Build agenda
    Sub-skill: deep-research/sub/deep-research-strategist
    → research/agenda.yml

R2  Discover sources
    Sub-skill: deep-research/sub/deep-source-discovery-specialist
    → research/candidate-pool.yml

R3  Evaluate credibility + Validate references (paralelo)
    Sub-skill a: deep-research/sub/deep-source-credibility-assessor
    Sub-skill b: deep-research/sub/deep-reference-validator
    → research/credibility/{source-id}.yml + research/reference-validation.jsonl

R4  Triangulate evidence
    Sub-skill: deep-research/sub/deep-evidence-triangulator
    → research/triangulation/{claim-id}.yml

R5  Consolidate corpus
    Sub-skill: deep-research/sub/deep-knowledge-corpus-curator
    → research/corpus.yml + research/corpus-snapshot-{date}.yml + research/gaps.yml

R6  Extract deliverables
    Sub-skill: deep-research/sub/deep-claim-extractor
    → research/evidence-cards/{topic}.yml      [LIBRO/DUAL]
    → research/blueprints/{component}.yml      [SOFTWARE/DUAL]
```

## Sub-pipelines activables (opcionales)

| Sub-pipeline | Sub-skill | Cuándo |
|--------------|-----------|--------|
| Software research | `deep-research/sub/deep-software-research` | Tecnología/framework |
| Pattern extraction | `deep-research/sub/deep-pattern-extractor` | Code patterns verificables |
| Domain modeling | `deep-research/sub/deep-domain-modeler` | Entidades/relaciones |
| Knowledge graph | `deep-research/sub/deep-knowledge-graph-builder` | Mapa de relaciones |
| Historical lineage | `deep-research/sub/deep-historical-lineage-tracer` | Evolución temporal |
| Scenarios | `deep-research/sub/deep-scenarios-explorer` | Proyección/futuros |
| Paradigms | `deep-research/sub/deep-paradigms-explorer` | Modelos mentales |
| Traps | `deep-research/sub/deep-traps-detector` | Errores comunes |
| **Systems Thinking** | `deep-research/sub/deep-coach-systems-thinking` | Donella Meadows / System Dynamics |

Y 6 skills subordinadas adicionales de Systems Thinking (ver `references/index.md`).

## Anti-patrones (Meadows labels)

- Saltarse R0 = "collecting data without a lens"
- Confundir L3 (Wikipedia) con L1 (paper original) = Shifting the Burden
- Single-source para claims `critical` = Insufficient triangulation
- Inventar cuantificaciones = Seeking the Wrong Goal
- Cambiar parámetros cuando el problema es de paradigma = también Seeking the Wrong Goal
- Muchas voces sin alinear goals = Policy Resistance
- Re-citar secundarias sin leer primarias = Shifting the Burden
- Citar sin página/sección = Drift to Low Performance

## Output Contract

- `research/system-map/{topic}.yml` (R0)
- `research/agenda.yml` (R1)
- `research/candidate-pool.yml` (R2)
- `research/credibility/{source-id}.yml` (R3a)
- `research/reference-validation.jsonl` (R3b)
- `research/triangulation/{claim-id}.yml` (R4)
- `research/corpus.yml` + `corpus-snapshot-{date}.yml` + `gaps.yml` (R5)
- `research/evidence-cards/{topic}.yml` (R6, LIBRO/DUAL)
- `research/blueprints/{component}.yml` (R6, SOFTWARE/DUAL)
- `research/{topic}-research-report.md` (deliverable principal)

## Cómo descubre el orchestrator las sub-skills

Cuando esta skill se carga, el SKILL.md instruye al agente a leer `references/index.md`, que lista las 22 sub-skills con sus nombres y descripciones. El agente puede entonces invocar cualquiera por su nombre (cargando `sub/<name>/SKILL.md`).

**Nota de compatibilidad con el CLI actual** (SDDK 1.13.0): el CLI actual no escanea subdirectorios en `skills/` (asume 1 nivel). Hasta que se implemente SDDK2-411 (modificar CLI para recursar 1 nivel), el orquestador necesita descubrir las sub-skills manualmente leyendo `references/index.md` después de cargar la maestra. Ver `docs/sddk-2.0-architecture-consolidation/adrs/ADR-019-workflow-self-discovery.md` (Opción B propuesta para sddk-2.0).

## References

- `references/index.md` — mapa de las 22 sub-skills con descripciones y triggers.
- `references/pipeline-r0-r6.md` — guía detallada del pipeline R0-R6.
- `sub/*/SKILL.md` — las 22 sub-skills especializadas.
- `../../../.zcode/skills/deep-research-orchestrator/SKILL.md` — agente ejecutor (fuera del bundle).
- `../../DEEP-RESEARCH-INDEX.md` — catálogo legacy (en raíz de skills/, deprecado tras esta consolidación).

## Provenance

- **Author**: rubentxu
- **Bundled with**: sddk-framework v1.14.0
- **Based on**: Donella Meadows (Thinking in Systems 2008, Leverage Points 1997), Jay Forrester (System Dynamics), Peter Senge (Fifth Discipline), Daniel Kim (Systems Archetypes Basics).
- **Pattern**: Master + sub-skills (LangChain "hierarchical skills", RFC-318 Collection-based namespacing).
- **ADR**: ADR-019 (workflow self-discovery) y ADR-0016 (skill categorization).
