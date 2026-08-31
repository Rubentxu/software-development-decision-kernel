---
name: research-strategist
description: "Trigger: plan de investigación, agenda de investigación, qué investigar del libro, estrategia de fuentes, research plan, research roadmap, preguntas de investigación. Define la agenda de investigación de un libro o tema: qué afirmaciones deben evidenciarse, con qué nivel de rigor, qué gaps existen y qué estrategia de fuentes se seguirá. Es el cerebro de la Macro-fase R."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo al **inicio de la Macro-fase R** (antes que cualquier otro skill de investigación) o cuando se añade un tema/capítulo nuevo. Convierte el outline + grafo curricular en una **agenda de investigación** priorizada.

No lo uses para buscar fuentes (`source-discovery-specialist`), validar (`reference-validator`) ni extraer afirmaciones (`source-researcher`).

## Hard Rules

- Toda afirmación técnica del libro se clasifica por **nivel de evidencia requerido** (no todas necesitan la misma profundidad).
- La agenda es **priorizada por riesgo**: una afirmación sobre versión de API es más crítica que una anécdota histórica.
- Declarar explícitamente qué **tipos de fuente** son admisibles para cada tipo de afirmación.
- La agenda es **persistente**: `research/agenda.yml` evoluciona conforme se resuelven preguntas.

## Execution Steps

1. Leer `planning/outline.yml`, `planning/curriculum-graph.yml` y `planning/audience-profile.yml`.
2. Inventariar todas las **afirmaciones previsibles** que el libro hará, por capítulo/concepto.
3. Clasificar cada una por:
   - `evidence_level`: qué rigor exige (ver `references/evidence-levels.md`).
   - `claim_type`: `api-existence` | `version` | `behavior` | `performance` | `history` | `opinion` | `best-practice`.
   - `risk`: `critical` (afecta a corrección/código) | `normal` | `low`.
4. Formular **preguntas de investigación** concretas (una por afirmación crítica).
5. Definir la **estrategia de fuentes**: para cada `claim_type`, qué tipos de fuente son admisibles y cuáles no.
6. Detectar **gaps de conocimiento** del autor antes de investigar (¿hay temas que nadie en el equipo domina?).
7. Generar `research/agenda.yml` (esquema en `assets/agenda.schema.yml`).
8. Alimentar a `source-discovery-specialist` con las preguntas priorizadas.

## Esquema de agenda (resumen)

```yaml
agenda:
  topic: "Bevy ECS scheduling"
  questions:
    - id: RQ-scheduling-access-conflicts
      question: "¿Cómo detecta Bevy 0.19 conflictos de acceso entre sistemas?"
      claim_type: behavior
      evidence_level: L2           # requiere doc oficial + código fuente
      risk: critical
      admissible_sources: [official-docs, source-code, release-notes]
      status: open                 # open | investigating | resolved
  gaps:
    - "Autor no conoce cambios de scheduling entre 0.18 y 0.19"
  strategy:
    default_authority_floor: L3    # mínimo nivel aceptable
    hard_blockers: [L7]            # fuentes que nunca se citan solas
```

## Decision Gates

| Necesidad | Acción |
|-----------|--------|
| Pregunta que requiere experimentación | Marcar `evidence_level: L1-exp` (reproducible) |
| Afirmación de performance | Exige benchmark reproducible, no opinión |
| Tema con poco material primario | Escalar al autor: ¿reducimos alcance? |

## Output Contract

- `research/agenda.yml` con preguntas priorizadas, niveles y estrategia.
- Lista de gaps de conocimiento del autor.
- `source-discovery-specialist` recibe las preguntas `risk: critical` primero.

## References

- `references/evidence-levels.md` — definición de niveles L1–L7 y a qué tipo de afirmación aplica cada uno.
- `assets/agenda.schema.yml` — esquema validable de la agenda.
