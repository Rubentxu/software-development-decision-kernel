---
name: deep-coach-systems-thinking
description: "Trigger: enseñar dinámica de sistemas, libro sobre Donella Meadows, enseñar System Dynamics, leverage points, World3, sistema complejo a fondo. Sub-pipeline de dominio para enseñar/aplicar el marco completo de Donella Meadows. Activa deep-leverage-points-analyst, deep-system-archetypes-mapper, deep-feedback-loops-analyzer, deep-stocks-flows-diagrammer. Es la skill de dominio canónica cuando el capítulo es específicamente sobre systems thinking."
license: Apache-2.0
metadata:
  category: deep-research
  subcategory: systems-thinking
  author: rubentxu
  version: "1.0"
  domain: deep-research, systems-thinking
  consumers: [book-orchestrator, orchestrator]
  based_on: "Donella Meadows (Thinking in Systems 2008, Leverage Points 1997, Limits to Growth 1972, Dancing with Systems 2001), Jay Forrester (System Dynamics), Peter Senge (Fifth Discipline 1990), Daniel Kim (Systems Archetypes Basics)"
---

## Activation Contract

Úsalo cuando el tema del capítulo o la pieza de software es **explícitamente sobre Systems Thinking / Dinámica de Sistemas / Donella Meadows / System Dynamics**. Activa el sub-pipeline completo del dominio.

Si el tema es tecnología, historia u otro campo pero quiere APLICAR el marco de Meadows, también se activa (como metodología transversal — eso lo hace el orquestador genérico en R0).

No lo uses para: análisis técnico genérico (`deep-software-research`), conceptos no sistémicos.

## Hard Rules (Meadows)

- **El sistema es la causa de su propio comportamiento** (Meadows, Thinking in Systems, Introduction).
- **Estructura > eventos** (Meadows, cap. 4).
- **Stocks y flows son las únicas dos clases de variables** en cualquier sistema (Forrester, 1971; Meadows cap. 2).
- **Los 12 leverage points están ordenados por efectividad** (Meadows, Leverage Points 1997; Thinking in Systems cap. 6).
- **Higher leverage points resisten más** (Meadows: "societies often rub out truly enlightened beings").
- **No se controla, se baila** (Meadows, Dancing with Systems 2001).
- **Cita textual obligatoria** (Meadows, Forrester, Senge, Kim originales).

## Sub-pipeline que activa

```
deep-coach-systems-thinking (este skill)
├─ deep-leverage-points-analyst    (Fase 4 — clasifica intervenciones por los 12 puntos)
├─ deep-system-archetypes-mapper   (Fase 3 — mapea a los 8 arquetipos de Senge/Kim)
├─ deep-feedback-loops-analyzer    (Fase 2 — modela causal-loop diagrams)
└─ deep-stocks-flows-diagrammer    (Fase 2 — modela stocks-and-flows + ecuaciones)
```

Estas 4 skills operan sobre el `system-map/{topic}.yml` producido en R0.

## Execution Steps

### Cuando el capítulo ES sobre Systems Thinking

1. Activar pipeline R completo.
2. **R0**: definir el sistema del tema (ej: "World3 model", "Shifting the Burden en sistemas educativos").
3. **R1-R6**: estándar.
4. **Activar las 4 skills del sub-pipeline** según necesidad:
   - ¿Hay que clasificar intervenciones? → `deep-leverage-points-analyst`.
   - ¿Hay que mapear patrones conocidos? → `deep-system-archetypes-mapper`.
   - ¿Hay que modelar causalidad? → `deep-feedback-loops-analyzer`.
   - ¿Hay cuantificación / simulación? → `deep-stocks-flows-diagrammer`.
5. Generar:
   - **Modo LIBRO**: capítulo completo con ejemplos canónicos (Meadows: Population loop, World3, Shifting the Burden de los antibióticos).
   - **Modo SOFTWARE**: librería de simulación con modelo World3, detector de arquetipos, clasificador de leverage points.

### Cuando el capítulo APlica Systems Thinking a otro dominio

1. El orquestador genérico (`deep-research-orchestrator`) ya aplicó R0 (marco Meadows) en el `system-map`.
2. Activar las 4 skills para profundizar en los aspectos sistémicos del tema (ej: leverage points del mercado de LLMs).

## Output Contract

- Modo LIBRO: capítulo/s sobre Systems Thinking con citas textuales de Meadows/Forrester/Senge.
- Modo SOFTWARE: librería Python/Rust con:
  - Implementación de World3 simplificado.
  - Detector de arquetipos en un CLD.
  - Clasificador de leverage points en una intervención propuesta.
  - Validador contra World3 standard run.

## References

- **Donella Meadows**:
  - *Thinking in Systems: A Primer* (Chelsea Green, 2008) — texto canónico.
  - "Leverage Points: Places to Intervene in a System" (Whole Earth Review 1997; donellameadows.org).
  - "Dancing with Systems" (Whole Earth Review 2001; donellameadows.org).
  - *The Limits to Growth* (Universe Books, 1972, con Dennis Meadows, Jørgen Randers, William Behrens).
- **Jay W. Forrester**:
  - *Industrial Dynamics* (MIT Press, 1961) — texto fundacional.
  - *Urban Dynamics* (MIT Press, 1969).
  - *World Dynamics* (Productivity Press, 1971) — predecesor de World3.
- **Peter Senge**: *The Fifth Discipline* (Doubleday, 1990).
- **Daniel Kim & Virginia Anderson**: *Systems Archetypes Basics* (Pegasus WB002E).
- **Sustainability Institute / Academy for Systems Change**: `https://www.academyforchange.org/`.
- **MIT OCW 15.988 — System Dynamics**: `https://ocw.mit.edu/courses/15-988-system-dynamics-self-study-fall-1998-spring-1999/`.
- **donellameadows.org**: archivo oficial.
- Ver `references/meadows-canon.md` en orquestador principal.
- Ver `references/canonical-urls-systems-thinking.md` en `deep-source-discovery-specialist`.
