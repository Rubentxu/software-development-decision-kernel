---
name: deep-leverage-points-analyst
description: "Trigger: analizar leverage points, dónde intervenir en un sistema, clasificar intervenciones, jerarquía de los 12 puntos de apalancamiento, paradigma, meta, estructura, reglas, transcender paradigmas. Analiza y clasifica intervenciones propuestas sobre un sistema siguiendo los 12 leverage points de Donella Meadows. Núcleo metodológico de la Fase 4. Detecta cuándo una intervención está mal clasificada (parámetro vs. paradigma), cuándo será absorbida por Policy Resistance, y propone intervenciones de alto nivel con su resistencia esperada."
license: Apache-2.0
metadata:
  category: deep-research
  subcategory: systems-thinking
  author: rubentxu
  version: "1.0"
  domain: deep-research, systems-thinking
  based_on: "Donella Meadows (Leverage Points 1997, Thinking in Systems 2008 cap. 6)"
  consumers: [book-orchestrator, orchestrator]
---

## Activation Contract

Úsalo en **Fase 4 del análisis sistémico**, después de haber mapeado el sistema (R0/Fase 1), modelado los loops (Fase 2), diagnosticado el arquetipo y la trampa (Fase 3). Recibe una intervención propuesta y la clasifica en uno de los 12 leverage points; o recibe un sistema y propone intervenciones priorizadas por leverage.

No lo uses para: modelar feedback loops (`deep-feedback-loops-analyzer`), mapear arquetipos (`deep-system-archetypes-mapper`), explorar paradigmas a fondo (`deep-paradigms-explorer`).

## Hard Rules (Meadows)

- **Orden de los 12 leverage points (de mayor a menor leverage)**:
  1. **Power to transcend paradigms** — capacidad de cuestionar TODOS los paradigmas.
  2. **The mindset or paradigm** — la fuente de la que el sistema surge.
  3. **The goals of the system** — el propósito u objetivo.
  4. **Power to add/change/evolve/self-organize structure** — capacidad de reorganización.
  5. **The rules of the system** — incentivos, castigos, constraints.
  6. **Structure of information flows** — quién tiene acceso a qué información.
  7. **Gain around driving positive feedback loops** — amplificar o frenar R loops.
  8. **Strength of negative feedback loops** — fortalecer el termostato.
  9. **Lengths of delays** — acelerar o introducir delays deliberadamente.
  10. **Structure of material stocks-and-flows** — nodos físicos, estructura de transporte.
  11. **Sizes of buffers and stabilizing stocks** — reservas, capacidad ociosa.
  12. **Constants, parameters, numbers** — subsidios, impuestos, estándares.

- **Cita textual obligatoria** (Meadows, *Leverage Points* 1997, *Thinking in Systems* cap. 6).
- **Toda intervención se clasifica por su leverage_level (1-12)**, no por su impacto aparente.
- **Toda intervención lleva `risk_of_resistance`**: los leverage points altos (1-4) tienen mayor resistencia.
- **No se propone "más educación/comunicación" como solución universal**: nivel 6 cuando el problema es nivel 2 es Policy Resistance.
- **Anti-patrón crítico**: cambiar parámetros (nivel 12) con un problema de paradigma (nivel 2) — la intervención será absorbida por el sistema.

## Execution Steps

### Modo A: Clasificar una intervención propuesta

1. Lee la intervención (en texto libre).
2. Identifica sobre **qué estructura del sistema** actúa:
   - ¿Cambia cómo la gente piensa? → Paradigma (2) o trascender paradigmas (1).
   - ¿Cambia el propósito del sistema? → Goals (3).
   - ¿Cambia la capacidad de reorganizar? → Self-organization (4).
   - ¿Cambia reglas/incentivos/castigos? → Rules (5).
   - ¿Cambia quién sabe qué? → Information flows (6).
   - ¿Cambia la fuerza de un R loop? → Gain around R (7).
   - ¿Cambia la fuerza de un B loop? → Strength of B (8).
   - ¿Cambia la velocidad del feedback? → Delays (9).
   - ¿Cambia la estructura física/material? → Stocks-and-flows structure (10).
   - ¿Cambia el tamaño de un buffer? → Buffers (11).
   - ¿Cambia un número? → Parameters (12).
3. Asigna `leverage_level` y `risk_of_resistance`:
   - Niveles 1-4: `risk_of_resistance: very-high` (cambian la fuente del sistema).
   - Niveles 5-6: `risk_of_resistance: high` (cambian reglas/información).
   - Niveles 7-9: `risk_of_resistance: medium` (ajustan la dinámica).
   - Niveles 10-12: `risk_of_resistance: low` (ajustes materiales/parámetros).
4. Si el sistema diagnosticado tiene un arquetipo (`archetype-match.yml`), verifica que la intervención **escapa** del arquetipo, no que lo refuerza.
5. Documenta en `research/leverage-points/{topic}.yml` con `claim_ids` que la respalden.

### Modo B: Proponer intervenciones para un sistema

1. Lee el `system-map.yml`, `archetype-match.yml`, `traps-report.md`.
2. Identifica el **archetype_addressed** (qué trampa está reproduciendo el sistema).
3. Para cada nivel de leverage (1-12), pregúntate: **¿qué intervención aquí escaparía al arquetipo?**
4. Prioriza las de nivel 4-6 (donde Meadows dice que hay "más leverage de lo que la gente cree" y son accesibles).
5. Marca cada propuesta con su `expected_impact` y `time_to_effect`.
6. Si todas las intervenciones realistas están en niveles 10-12, **marca el sistema como "paradigm-locked"** y sugiere activar `deep-paradigms-explorer`.

### Modo C: Auditar una intervención ya implementada

1. Clasifica la intervención (Modo A).
2. Pregunta: **¿el sistema la absorbió?** Si el problema persistió sin cambios estructurales → la intervención estaba en un nivel demasiado bajo.
3. Sugiere el nivel de leverage que SÍ funcionaría.

## Esquema de intervención

```yaml
interventions:
  - id: int-001
    description: "Cambiar el subsidio por estudiante a una métrica de logro"
    leverage_level: 5          # rules
    archetype_addressed: "Shifting the Burden"
    trap_escaped: "Shifting the Burden to the Intervenor"
    risk_of_resistance: high
    expected_impact: medium
    time_to_effect: "3-5 years"
    evidence:
      claim_ids: [cl-rules-leverage, cl-shifting-burden-escape]
    notes: "Cambia incentivos; riesgo de gaming (medir lo fácil)"
```

## Anti-patrones de intervención

| Intervención | Por qué falla | Leverage correcto |
|--------------|---------------|-------------------|
| "Más educación/comunicación" | Nivel 6 cuando el problema es de paradigma (2) | Escalar a paradigma: cuestionar "¿qué significa 'éxito' aquí?" |
| "Cambiar la cantidad de dinero" | Parámetro (12) cuando el problema es estructural | Escalar a reglas (5) o self-organization (4) |
| "Nombrar un comité" | No cambia estructura | Self-organization (4): dar capacidad real de cambio |
| "Establecer una nueva regla sin enforcement" | Regla sin feedback loop | Combinar con information flows (6) + feedback (7-8) |
| "Confiar en que el mercado autorregulará" | B loop existente pero con resistencia | Strengthen B loop (8) o cambiar goal (3) |

## Decálogo de Meadows sobre leverage points

1. "The higher the leverage point, the more the system will resist changing it."
2. "Magical leverage points are not easily accessible, even if we know where they are and which direction to push on them."
3. "There are no cheap tickets to mastery."
4. "You have to work hard at it, whether that means rigorously analyzing a system or rigorously casting off your own paradigms and throwing yourself into the humility of not-knowing."
5. "Mastery has less to do with pushing leverage points than it does with strategically, profoundly, madly, letting go and dancing with the system."
6. "Go quiet. Go still. Let it settle."
7. "Aim to enhance total systems properties, such as creativity, stability, diversity, resilience, and sustainability."
8. "Make feedback policies for feedback systems."
9. "Pay attention to what is important, not just what is quantifiable."
10. "Hold the vision of what you want — clearly, persistently, confidently — and let the system figure out how to get there."

## Decision Gates

| Situación | Acción |
|-----------|--------|
| Intervención en nivel 10-12 con problema de nivel 2 | Escalar: el sistema absorberá la intervención |
| Intervención en nivel 1-4 sin capacidad de sostener la resistencia | Marcar `time_to_effect: >10 years` y `risk_of_resistance: very-high`; considerar si vale la pena |
| Múltiples intervenciones propuestas en el mismo nivel | Consolidar: una intervención por nivel es más legible |
| Intervención "más transparencia" sin especificar QUÉ información a QUIÉN | Detallar: information flows (6) requiere actor + dato + flujo |
| Intervención "cambiar el paradigma" sin cómo | No aprobar: pasar a `deep-paradigms-explorer` |

## Output Contract

### Modo LIBRO
- `research/leverage-points/{topic}.yml` con intervenciones clasificadas.
- `research/drafts/{topic}-leverage-points-section.md` (borrador AsciiDoc con tabla de los 12 puntos).
- Claims con citas de Meadows (`research/evidence-cards/leverage-points.yml`).

### Modo SOFTWARE
- `research/leverage-points/{topic}.yml`.
- `research/blueprints/leverage-classifier.yml` (API/función para clasificar intervenciones).
- `research/code-patterns/leverage-points.py` (implementación con enum de los 12 niveles).

## References

- **Meadows, D. H.** (1997). "Leverage Points: Places to Intervene in a System". *Whole Earth Review*. URL: `https://donellameadows.org/archives/leverage-points-places-to-intervene-in-a-system/`.
- **Meadows, D. H.** (2008). *Thinking in Systems: A Primer*. Chelsea Green. Cap. 6, pp. 145-165.
- `references/twelve-leverage-points.md` (en orquestador).
- `references/leverage-cases.md` (en orquestador).
- `assets/leverage-points.schema.yml`.
