---
name: deep-system-archetypes-mapper
description: "Trigger: identificar arquetipo, qué patrón sistémico es, system archetype, Shifting the Burden, Tragedy of the Commons, Limits to Success, Escalation, Fixes that Fail, Eroding Goals, Success to the Successful, Growth and Underinvestment. Mapea un sistema a uno de los 8 arquetipos de Peter Senge / Daniel Kim, con justificación estructural y conductual. Produce evidencia textual (LIBRO) y blueprint de detector (SOFTWARE)."
license: Apache-2.0
metadata:
  category: deep-research
  subcategory: systems-thinking
  author: rubentxu
  version: "1.0"
  domain: deep-research, systems-thinking
  based_on: "Peter Senge (Fifth Discipline 1990), Daniel Kim & Virginia Anderson (Systems Archetypes Basics, Pegasus WB002E)"
  consumers: [book-orchestrator, orchestrator]
---

## Activation Contract

Úsalo en **Fase 3 del análisis sistémico**, después de Fase 2 (CLD + stocks-and-flows). Recibe un sistema mapeado y modelado, y devuelve el arquetipo (o compuesto) que mejor encaja, con justificación estructural y conductual.

Modos:
- **LIBRO**: produce `research/archetype-match/{topic}.yml` + `research/drafts/archetypes-section.md` (borrador AsciiDoc) + claims con citas de Senge/Kim.
- **SOFTWARE**: produce `research/blueprints/archetype-detector.yml` (API/función para detectar arquetipos en un CLD) + `research/code-patterns/archetype-detector.{py,rs}`.
- **DUAL**: ambos.

No lo uses para: detectar traps específicos (eso es `deep-traps-detector`), clasificar leverage points (`deep-leverage-points-analyst`), ni modelar feedback loops (`deep-feedback-loops-analyzer`).

## Hard Rules

- **Los 8 arquetipos canónicos** (Senge cap. 6 + Appendix 2; Kim/Anderson):
  1. **Fixes That Fail** — solución corto plazo empeora el síntoma.
  2. **Shifting the Burden** — solución sintomática atrofia la fundamental.
  3. **Limits to Success / Limits to Growth** — R(crecimiento) choca con B(límite).
  4. **Eroding Goals / Drift to Low Performance** — meta se ajusta a la baja.
  5. **Growth and Underinvestment** — capacidad insuficiente frena crecimiento.
  6. **Success to the Successful** — dos R compitiendo, el ganador se lleva más.
  7. **Escalation** — dos R opuestos, carrera competitiva.
  8. **Tragedy of the Commons** — múltiples R + recurso compartido limitado.

- **Match estructural + conductual**: ≥ 2 elementos estructurales (de 5) Y ≥ 1 indicador conductual (de 3) = match confirmado.
- **Arquetipos compuestos**: documentar el primario y el secundario; ambos deben tener justificación.
- **Cita textual obligatoria** (Senge 1990; Kim/Anderson WB002E).
- **No inventar arquetipos**. Si no encaja, decir "ninguno canónico" y documentar por qué.

## Execution Steps

### Modo A: Mapear un sistema a un arquetipo

1. Lee `research/system-map/{topic}.yml`, `research/causal-loop-diagram.md`, `research/stock-flow-diagram.md`.
2. Identifica los **loops principales** (B1, B2, R1, R2...) con polaridad.
3. Identifica los **delays** entre causa y efecto (marcados `||`).
4. Para cada arquetipo canónico, evalúa:
   - **Estructura mínima** (presencia de loops con la polaridad y conexiones correctas).
   - **Indicadores conductuales** (¿el sistema crece/decae/oscila/se estabiliza/decolapsa de forma característica?).
5. Calcula `structural_match` (0-5) y `behavioral_match` (0-3) por arquetipo.
6. Si el mejor match es `≥ 2 AND ≥ 1` → arquetipo confirmado.
7. Documenta `dominant_loop` (cuál loop explica más comportamiento).
8. Genera `research/archetype-match/{topic}.yml`.

### Modo LIBRO: producir evidencia textual

9. Por cada arquetipo confirmado, genera un `claim` con cita textual:
   - `research/evidence-cards/cl-{arquetipo}-definition.yml`
   - `research/evidence-cards/cl-{arquetipo}-structure.yml`
   - `research/evidence-cards/cl-{arquetipo}-escape.yml`
10. Genera `research/drafts/archetypes-section.md` (borrador AsciiDoc con tabla + Mermaid).

### Modo SOFTWARE: producir blueprint de detector

11. Diseña la interfaz del detector:
    - Input: `CausalLoopDiagram` con nodos, aristas (con polaridad) y delays.
    - Output: `List[ArchetypeMatch]` con score y justificación.
12. Genera `research/blueprints/archetype-detector.yml`.
13. Genera `research/code-patterns/archetype-detector.py` (implementación de referencia).
14. Genera `research/test-fixtures/{arquetipo}-example.cld` (CLDs de prueba para cada arquetipo).

## Esquema de match (resumen)

```yaml
archetype:
  primary: "Shifting the Burden"
  secondary: "Policy Resistance"
  structural_match: 4
  behavioral_match: 3
  confidence: high
  loops_identified: [B1, R1]
  delays_present: true
  dominant_loop: R1-atrofia
  evidence:
    claim_ids: [cl-shifting-burden-senge, cl-policy-resistance-meadows]
    page_references: ["Senge 1990, Appendix 2", "Meadows 2008, cap. 5"]
  escape_options:
    - "Fortalecer la fundamental antes de retirar la sintomática"
```

## Decision Gates

| Situación | Acción |
|-----------|--------|
| Match < 2 estructural + < 1 conductual | "Ninguno canónico"; documentar gap; volver a R0 si sistema mal mapeado |
| Match alto en 2 arquetipos diferentes | Marcar como `archetype_compound`; ambos deben justificarse |
| Sistema no tiene loops R (todo B) | Inconsistente; revisar Fase 2 |
| Sistema tiene R loops pero ningún B | Inconsistente; todo sistema tiene homeostasis |

## Output Contract

### LIBRO
- `research/archetype-match/{topic}.yml` + claims en evidence-cards.
- `research/drafts/archetypes-section.md`.

### SOFTWARE
- `research/blueprints/archetype-detector.yml`.
- `research/code-patterns/archetype-detector.py`.
- `research/test-fixtures/{arquetipo}-example.cld`.

### DUAL
Ambos.

## References

- `references/eight-archetypes.md` — descripción detallada con diagramas Mermaid.
- `assets/archetype-match.schema.yml`.
- Padre: `deep-coach-systems-thinking` (o `deep-research-orchestrator`).
