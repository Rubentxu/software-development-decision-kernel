# Deep Research — Mapa de las 22 sub-skills

Esta skill maestra (`skills/deep-research/`) expone 22 sub-skills especializadas. Cada una es una skill autónoma que el agente puede invocar independientemente.

## Estructura del bundle

```
skills/deep-research/
├── SKILL.md                  ← esta maestra (índice)
├── references/
│   ├── index.md              ← este archivo
│   └── pipeline-r0-r6.md    ← guía detallada del pipeline
└── sub/
    ├── deep-research-orchestrator/                (gate)
    ├── deep-research-methodology-hub/             (methodology)
    ├── deep-research-strategist/                  (R1)
    ├── deep-source-discovery-specialist/           (R2)
    ├── deep-source-credibility-assessor/          (R3a)
    ├── deep-reference-validator/                  (R3b)
    ├── deep-evidence-triangulator/                (R4)
    ├── deep-knowledge-corpus-curator/             (R5)
    ├── deep-claim-extractor/                      (R6)
    ├── deep-software-research/                   (sub-pipeline)
    ├── deep-pattern-extractor/                    (sub-pipeline)
    ├── deep-domain-modeler/                       (sub-pipeline)
    ├── deep-knowledge-graph-builder/              (sub-pipeline)
    ├── deep-historical-lineage-tracer/            (sub-pipeline)
    ├── deep-scenarios-explorer/                   (sub-pipeline)
    ├── deep-paradigms-explorer/                   (sub-pipeline)
    ├── deep-traps-detector/                       (sub-pipeline)
    ├── deep-coach-systems-thinking/               (Systems Thinking maestro)
    ├── deep-leverage-points-analyst/              (Systems Thinking)
    ├── deep-system-archetypes-mapper/             (Systems Thinking)
    ├── deep-feedback-loops-analyzer/              (Systems Thinking)
    ├── deep-stocks-flows-diagrammer/              (Systems Thinking)
    └── deep-paradigms-explorer/                   (Systems Thinking)
```

## Catálogo por fase del pipeline

### Gate (orchestrator-side, dispara al agente)

| Sub-skill | Función | Trigger |
|-----------|---------|---------|
| `sub/deep-research-orchestrator` | SKILL con gate; el orchestrator la carga y delega al agente ejecutor | "Investigar tema X" |

### Methodology Hub

| Sub-skill | Función |
|-----------|---------|
| `sub/deep-research-methodology-hub` | Hub metodológico (renombrado para evitar colisión); el agente la carga |

### R-Pipeline core (R1-R6)

| Sub-skill | Fase | Función | Trigger típico |
|-----------|------|---------|----------------|
| `sub/deep-research-strategist` | R1 | Agenda priorizada por riesgo + nivel de evidencia | "Plan de investigación", "qué investigar primero" |
| `sub/deep-source-discovery-specialist` | R2 | Descubrimiento multi-modal (papers, docs, código, libros, datos, blogs) | "Encontrar fuentes", "papers sobre X" |
| `sub/deep-source-credibility-assessor` | R3a | Ranking L1-L7, sesgo, COI, frescura | "Evaluar credibilidad" |
| `sub/deep-reference-validator` | R3b | Validación viva (URL HEAD, DOI, version drift) | "¿La URL sigue viva?", "link rot" |
| `sub/deep-evidence-triangulator` | R4 | Cruza fuentes independientes, calcula confidence_score | "Triangular evidencia", "fuentes en conflicto" |
| `sub/deep-knowledge-corpus-curator` | R5 | Consolida corpus, deduplica, detecta gaps, gestiona decay | "Consolidar corpus" |
| `sub/deep-claim-extractor` | R6 | Genera evidence cards + blueprints | "Extraer evidence cards" |

### Sub-pipelines activables (opcionales)

| Sub-skill | Cuándo | Modo de salida |
|-----------|--------|----------------|
| `sub/deep-software-research` | Tecnología/framework | LIBRO + SOFTWARE |
| `sub/deep-pattern-extractor` | Patrones de implementación verificables | SOFTWARE |
| `sub/deep-domain-modeler` | Modelo conceptual (entidades, relaciones) | LIBRO + SOFTWARE |
| `sub/deep-knowledge-graph-builder` | Mapa de relaciones (autores, papers, conceptos) | LIBRO + SOFTWARE |
| `sub/deep-historical-lineage-tracer` | Evolución temporal de un campo | LIBRO |
| `sub/deep-scenarios-explorer` | Proyección / futuros alternativos | LIBRO + SOFTWARE |
| `sub/deep-paradigms-explorer` | Modelos mentales, cultura, paradigmas | LIBRO |
| `sub/deep-traps-detector` | Anti-patrones / errores comunes del dominio | LIBRO |

### Systems Thinking (Donella Meadows) — 7 sub-skills

| Sub-skill | Función |
|-----------|---------|
| `sub/deep-coach-systems-thinking` | Maestro del sub-pipeline Systems Thinking; activa las 6 siguientes |
| `sub/deep-leverage-points-analyst` | Aplicar los 12 leverage points de Meadows |
| `sub/deep-system-archetypes-mapper` | Mapear a los 8 arquetipos de Senge/Kim |
| `sub/deep-feedback-loops-analyzer` | Modelar causal-loop diagrams |
| `sub/deep-stocks-flows-diagrammer` | Modelar stocks-and-flows (Forrester) con simulación |
| `sub/deep-paradigms-explorer` | Descubre paradigmas (level 2 de leverage) |
| `sub/deep-traps-detector` | Detecta system traps (Policy Resistance, etc.) |

Nota: `sub/deep-traps-detector` y `sub/deep-paradigms-explorer` aparecen dos veces en el catálogo (una como sub-pipeline, otra como Systems Thinking). Es el mismo skill, no duplicado.

## Cómo invocar una sub-skill

Cuando el agente carga la skill maestra, lee `SKILL.md`, que contiene la tabla de sub-pipelines. El agente entonces carga `sub/<name>/SKILL.md` para la sub-skill específica según la fase del pipeline o el sub-pipeline activado.

```python
# Pseudocódigo del flujo del agente
master = load_skill("skills/deep-research/SKILL.md")
sub_index = load_skill("skills/deep-research/references/index.md")
# Agente identifica que R1 es necesario
sub = load_skill("skills/deep-research/sub/deep-research-strategist/SKILL.md")
```

## Estado

- **Versión**: 1.0
- **Fecha**: 2026-08-16
- **Patrón**: Master + sub-skills (LangChain "hierarchical skills", RFC-318 collection-based namespacing)
- **Alineado con**: ADR-019 (workflow self-discovery), ADR-0016 (skill categorization)
