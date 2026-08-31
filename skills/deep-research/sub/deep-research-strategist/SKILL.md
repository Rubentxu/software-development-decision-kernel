---
name: deep-research-strategist
description: "Trigger: agenda de investigación, plan de evidencia, qué investigar primero, priorización de preguntas, research plan, research roadmap, research agenda, gaps de conocimiento, estrategia de fuentes. Convierte el outline/tema en una agenda de investigación priorizada por riesgo y nivel de evidencia. Cerebro de R1 (pipeline R). Funciona para CUALQUIER dominio: tecnología, ciencia, historia, sistemas, etc. Aplicable en modo LIBRO (book-orchestrator) y SOFTWARE (orchestrator)."
license: Apache-2.0
metadata:
  category: deep-research
  subcategory: r-pipeline
  author: rubentxu
  version: "1.0"
  domain: deep-research
  based_on: "research-strategist (rubentxu), adaptado y generalizado"
---

## Activation Contract

Úsalo al **inicio del pipeline R**, después de Fase 1 (definir el sistema/tema) o cuando se añade un tema/capítulo/concepto nuevo. Convierte el outline + grafo curricular (o spec técnica) en una **agenda de investigación** priorizada.

No lo uses para: descubrir fuentes (`deep-source-discovery-specialist`), validar (`deep-reference-validator`), triangular (`deep-evidence-triangulator`), extraer claims (`deep-claim-extractor`). Esta skill **planifica**; las demás ejecutan.

## Hard Rules

- Toda afirmación se clasifica por **nivel de evidencia requerido** (no todas necesitan L1).
- La agenda es **priorizada por riesgo**: una afirmación sobre versión de API es `critical`; una anécdota histórica es `low`.
- Declarar explícitamente qué **tipos de fuente** son admisibles para cada `claim_type`.
- La agenda es **persistente**: `research/agenda.yml` evoluciona conforme se resuelven preguntas.
- **Genérica**: el mismo skill sirve para tecnología, ciencia, historia, sistemas, etc. La tabla de `claim_type` por dominio se carga dinámicamente.

## Execution Steps

1. Leer el contexto del proyecto:
   - Modo LIBRO: `planning/outline.yml`, `planning/curriculum-graph.yml`, `planning/audience-profile.yml`.
   - Modo SOFTWARE: `spec.md`, `docs/requirements.md`, o equivalente.
2. Inventariar **afirmaciones previsibles** que el producto final hará (capítulos, código, documentación).
3. Clasificar cada una por:
   - `claim_type`: varía por dominio (ver `references/claim-types.md`).
   - `evidence_level`: L1-L7 (ver `references/evidence-levels.md`).
   - `risk`: `critical` (afecta a corrección) | `normal` | `low`.
4. Formular **preguntas de investigación** concretas (una por afirmación crítica).
5. Definir la **estrategia de fuentes** para cada `claim_type`.
6. Detectar **gaps de conocimiento** del autor/equipo antes de investigar.
7. Generar `research/agenda.yml` (esquema en `assets/agenda.schema.yml`).
8. Alimentar a `deep-source-discovery-specialist` con las preguntas `risk: critical` primero.

## Tipos de claim por dominio

| Dominio | claim_types principales | Floor mínimo |
|---------|------------------------|--------------|
| Tecnología / Software | `api-existence`, `version`, `behavior`, `performance`, `security`, `best-practice`, `architectural-pattern` | L1 para versiones/APIs |
| IA / ML | `architecture`, `training-data`, `benchmark`, `limitation`, `safety`, `ethical-concern` | L1 para arquitectura; L2 para benchmarks |
| Systems Thinking (Meadows) | `concept-meadows`, `archetype-structure`, `leverage-rank`, `feedback-behavior`, `world3-model`, `historical-case` | L1 (texto primario) |
| Ciencia (biología/química/física) | `experimental-result`, `theory`, `mechanism`, `replication-status` | L1 (peer-reviewed) |
| Economía / Política | `dataset`, `policy-impact`, `historical-event`, `opinion-secondary` | L2 (datos oficiales) |
| Historia | `event-date`, `primary-source-quote`, `interpretation`, `revisionism` | L1 (fuente primaria) |
| Medicina | `clinical-trial`, `guideline`, `contraindication`, `mechanism` | L1 (peer-reviewed) |

Para el detalle completo por dominio, ver `references/claim-types.md`.

## Esquema de agenda (resumen)

```yaml
agenda:
  topic: "Bevy ECS scheduling"
  domain: technology
  questions:
    - id: RQ-scheduling-access-conflicts
      question: "¿Cómo detecta Bevy 0.19 conflictos de acceso entre sistemas?"
      claim_type: behavior
      evidence_level: L2
      risk: critical
      admissible_sources: [official-docs, source-code, release-notes]
      status: open
  gaps:
    - "Autor no conoce cambios de scheduling entre 0.18 y 0.19"
  strategy:
    default_authority_floor: L3
    hard_blockers: [L7]
```

## Decision Gates

| Necesidad | Acción |
|-----------|--------|
| Pregunta requiere experimentación | Marcar `evidence_level: L1-exp` (reproducible) |
| Afirmación de performance/benchmark | Exige benchmark reproducible, no opinión |
| Tema con poco material primario | Escalar: ¿reducir alcance o esperar? |
| claim de seguridad | Floor L1 (peer-reviewed o CVE oficial) |
| claim ético / filosófico | Floor L5 + disclaimer; nunca blocker |
| Concepto con fuerte reinterpretación moderna | Marcar conflicto; comparar con texto primario |

## Output Contract

- `research/agenda.yml` con preguntas priorizadas, niveles y estrategia.
- Lista de gaps de conocimiento del autor/equipo.
- `deep-source-discovery-specialist` recibe las preguntas `risk: critical` primero.

## References

- `references/evidence-levels.md` — definición de niveles L1-L7 universales.
- `references/claim-types.md` — tipos de claim por dominio.
- `assets/agenda.schema.yml` — esquema validable.
