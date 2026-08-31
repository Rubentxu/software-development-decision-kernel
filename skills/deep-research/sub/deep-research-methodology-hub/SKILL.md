---
name: deep-research-methodology-hub
description: "Trigger: metodología deep-research, marco Meadows, R0-R6 pipeline, leverage points methodology, systems thinking framework. Hub metodológico de investigación profunda. Aplica el marco de Donella Meadows (Thinking in Systems 2008, Leverage Points 1997, Limits to Growth 1972, Dancing with Systems 2001) como lente transversal para investigar CUALQUIER tema de capítulo de libro técnico o pieza de software. Define el pipeline R0-R6: definir el sistema del tema (R0), construir agenda, descubrir fuentes, evaluar credibilidad, triangular, consolidar corpus, extraer deliverables. Cargado por el agente `deep-research-orchestrator`."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
  category: deep-research
  subcategory: methodology-hub
  domain: deep-research
  methodology: donella-meadows-systems-thinking
  consumers: [deep-research-orchestrator agent]
  output: claims-yml, blueprints-yml, code-patterns, knowledge-graph
  based_on: "Donella Meadows (Thinking in Systems, Leverage Points), Jay Forrester (System Dynamics), Peter Senge (Fifth Discipline), Daniel Kim (Systems Archetypes Basics)"
  bundled: true
  note: "Renamed from deep-research-orchestrator to avoid collision with the orchestrator-side gate skill of the same name."
---

## Activation Contract

Cargado por el agente `deep-research-orchestrator` (NO por el orchestrator directamente).

**Aplica el marco metodológico de Donella Meadows como lente transversal:**

- **Definir el sistema** (elementos, interconexiones, propósito) — antes de buscar datos.
- **Modelar los feedback loops** (balancing/reinforcing, delays) — para entender dinámica, no eventos.
- **Identificar leverage points** (12 niveles: paradigmas → goals → estructura → reglas → info → loops → delays → stocks → buffers → parámetros).
- **Detectar system traps** (Policy Resistance, Tragedy of the Commons, Shifting the Burden, etc.) — para no caer en anti-patrones conocidos.
- **Persistir el corpus** con decaimiento y gaps — la evidencia caduca, hay que re-verificar.

## Hard Rules (metodológicas)

- R0 es **obligatorio**: aplicar antes de investigar.
- L1 floor para claims `critical`: fuentes primarias (papers, libros, código fuente, docs oficiales).
- Triangulación: claims `critical` requieren ≥ 2 fuentes independientes.
- Cita con página/sección: nunca vaga.
- Sin alucinaciones: si falta dato, documentar el gap.
- Decaimiento temporal: tech caduca 1-2 años, AI/ML 6-12 meses, foundational no caduca.
- Anti-patrón: cambiar parámetros cuando el problema es de paradigma (nivel 12 vs nivel 2).

## Pipeline R (R0-R6)

```
R0  Definir el sistema del tema (Meadows)            [obligatorio]
    → research/system-map/{topic}.yml

R1  Build agenda (deep-research-strategist)
R2  Discover sources (deep-source-discovery-specialist)
R3  Evaluate credibility (deep-source-credibility-assessor)
    Validate references (deep-reference-validator)        ┐
                                                       ┘ en paralelo
R4  Triangulate evidence (deep-evidence-triangulator)
R5  Consolidate corpus (deep-knowledge-corpus-curator)
R6  Extract deliverables (deep-claim-extractor)
    → research/evidence-cards/{topic}.yml    [LIBRO]
    → research/blueprints/{component}.yml    [SOFTWARE]
```

## Sub-pipelines (activación condicional)

| Sub-pipeline | Skill activadora | Cuándo |
|--------------|------------------|--------|
| Software research | `deep-software-research` | Tecnología/framework |
| Pattern extraction | `deep-pattern-extractor` | Patrones de implementación |
| Domain modeling | `deep-domain-modeler` | Entidades/relaciones |
| Knowledge graph | `deep-knowledge-graph-builder` | Mapa de relaciones |
| Historical lineage | `deep-historical-lineage-tracer` | Evolución temporal |
| Scenarios | `deep-scenarios-explorer` | Proyección/futuros |
| Paradigms | `deep-paradigms-explorer` | Modelos mentales |
| Traps | `deep-traps-detector` | Errores comunes |
| **Systems Thinking** | `deep-coach-systems-thinking` | Donella Meadows / System Dynamics |

## Anti-patrones (etiquetados por Meadows)

| Anti-patrón | Etiqueta |
|-------------|----------|
| Recolectar datos sin definir el sistema | "collecting data without a lens" |
| Confundir L3 (Wikipedia) con L1 (paper original) | Shifting the Burden |
| Single-source para claims `critical` | Insufficient triangulation |
| Inventar cuantificaciones | Seeking the Wrong Goal |
| Citar sin página/sección | Drift to Low Performance |
| Muchas voces sin alinear goals | Policy Resistance |
| Re-citar secundarias sin leer primarias | Shifting the Burden |
| Invertir en parámetros cuando el problema es de paradigma | Seeking the Wrong Goal |
| Ignorar decay_date | Eroding Goals |

## Output Contract

- `research/system-map/{topic}.yml` (R0).
- `research/agenda.yml` con preguntas priorizadas.
- `research/evidence-cards/{topic}.yml` listo para `chapter-writer` (modo LIBRO).
- `research/blueprints/{component}.yml` listo para code generation (modo SOFTWARE).
- `research/LEDGER.md` siempre actualizado.
