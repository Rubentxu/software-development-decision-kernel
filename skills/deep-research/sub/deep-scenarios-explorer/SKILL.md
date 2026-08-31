---
name: deep-scenarios-explorer
description: "Trigger: escenarios futuros, proyecciones, futuro de X, ¿qué pasa si Y?, escenarios alternativos, World3, simulación de escenarios. Construye escenarios alternativos sobre el futuro de un tema (tecnología, sistema, mercado, sociedad). Combina técnicas de Meadows (World3-style), scenario planning (Schwartz/Peter Drucker), y modelado de incertidumbres. Para LIBRO: secciones prospectivas; para SOFTWARE: motores de simulación de escenarios."
license: Apache-2.0
metadata:
  category: deep-research
  subcategory: domain-pipeline
  author: rubentxu
  version: "1.0"
  domain: deep-research, scenarios
  consumers: [book-orchestrator, orchestrator]
  based_on: "Donella Meadows (Limits to Growth), Peter Schwartz (The Art of the Long View), Kees van der Heijden (Scenarios: The Art of Strategic Conversation)"
---

## Activation Contract

Úsalo cuando el tema requiere **proyección a futuro**: ¿qué pasa si X? ¿qué escenarios son plausibles? Aplica el rigor metodológico de Meadows (modelos causales, World3-style) combinado con scenario planning.

No lo uses para: predicción puntual (eso es forecasting estadístico), análisis del pasado (`deep-historical-lineage-tracer`).

## Hard Rules

- **Múltiples escenarios (≥ 3)**: no proyectar UN futuro; proyectar 3-4 futuros plausibles y diferenciados.
- **Cada escenario tiene drivers explícitos**: qué variables lo determinan (paradigmas, reglas, tendencias).
- **Cada driver tiene fuente**: estado actual con L1-L2; proyección con L5 + disclaimer explícito.
- **Incertidumbre explícita**: ningún escenario se presenta como "lo que va a pasar"; todos son "lo que PODRÍA pasar si...".
- **Acción presente**: cada escenario termina con "¿qué podemos hacer HOY para prepararnos?".
- **Anti-Policy Resistance**: no quedarse en "el futuro será bueno/malo según hagamos X"; explorar caminos distintos con consecuencias distintas.

## Execution Steps

1. Activar pipeline R para el tema.
2. R0: definir el **sistema cuyo futuro se explora** (ver `deep-domain-modeler` + R0 del orquestador).
3. Identificar los **drivers clave** (incertidumbres críticas):
   - Fuerzas impulsoras (tendencias fuertes): ej. mejora de capacidad computacional.
   - Incertidumbres críticas: ej. regulación de IA, adopción por la sociedad.
4. Construir los **escenarios** (método 2x2 de Schwartz):
   - Selecciona 2 incertidumbres críticas independientes (no correlacionadas).
   - Construye 4 cuadrantes = 4 escenarios.
5. Para cada escenario:
   - **Nombre memorable** (ej: "Mundo Águila", "Mundo Lobo").
   - **Narrativa**: cómo se vive el 2030/2040/2050 en ese escenario.
   - **Drivers**: qué valores toman las incertidumbres.
   - **Consecuencias sistémicas**: efectos en stocks, loops, leverage points.
   - **Señales de alerta**: qué señales débiles hoy indican que este escenario se está materializando.
   - **Acciones preparatorias**: qué hacer HOY si este escenario es plausible.
6. Documentar las **implicaciones para el sistema actual** (Meadows): ¿qué leverage points cambian? ¿qué traps emergen?

## Esquema de escenarios

```yaml
scenarios:
  topic: "futuro de la IA generativa"
  horizon: "2035"
  drivers:
    - name: regulation
      description: "Grado de regulación gubernamental de los LLMs"
      current_evidence: [src-eu-ai-act, src-us-ai-executive-order]
      range: [strict-regulation, self-regulation]
    - name: capability
      description: "Velocidad de mejora de capacidad de los LLMs"
      current_evidence: [src-benchmark-frontier, src-paper-emergent-capabilities]
      range: [plateau, continuous-improvement, discontinuity]
  scenarios:
    - id: regulated-slowdown
      name: "Mundo Muros Altos"
      drivers: {regulation: strict-regulation, capability: plateau}
      narrative: |
        La regulación internacional estricta (estilo EU AI Act global) frena el desarrollo.
        Los LLMs se vuelven commodities. El valor se desplaza a la integración vertical.
      leverage_points:
        - {level: 5, point: "reglas internacionales de IA"}
        - {level: 4, point: "capacidad de self-organization de la industria"}
      warnings: [src-current-regulation-trend]
      actions_today:
        - "Construir capacidades de compliance"
        - "Invertir en integración vertical"
```

## Métodos de construcción de escenarios

- **2x2 de Schwartz**: 2 incertidumbres → 4 escenarios.
- **GSA (Global Scenarios for the Anthropocene): 5 escenarios** (Pezzoli et al.).
- **World3-style**: modelar stocks/flows con ecuaciones y simular (ver `deep-stocks-flows-diagrammer`).
- **Morphological analysis**: combinar muchas dimensiones para generar escenarios combinatorialmente.
- **Backcasting**: definir el futuro deseado y trabajar hacia atrás qué debe ocurrir.

## Output según modo

### Modo LIBRO
- `research/scenarios/{topic}.yml`.
- `research/drafts/{topic}-scenarios-section.md` (borrador AsciiDoc).
- `research/diagrams/{topic}-scenarios-2x2.mmd` (Mermaid quadrant).

### Modo SOFTWARE
- `research/scenarios/{topic}.yml`.
- `research/blueprints/scenario-engine.yml` (motor que ejecuta escenarios).
- `research/code-patterns/scenario-runner.py` (runner de escenarios).

## Decision Gates

| Situación | Acción |
|-----------|--------|
| Solo 1 escenario (futuro único) | STOP: añadir ≥ 2 más. Escenario único es policy resistance disfrazado. |
| Drivers sin fuente | Marcar `current_evidence: insufficient`; reducir horizonte |
| Escenario sin acciones preparatorias | STOP: todo escenario debe terminar en "qué hacer HOY" |
| Escenarios muy similares | Diferenciar: cambiar uno de los drivers |
| Narrativa pesimista sin contrapeso | Añadir escenario optimista (Schwartz) |

## Output Contract

- `research/scenarios/{topic}.yml` con ≥ 3 escenarios diferenciados.
- `research/drafts/{topic}-scenarios-section.md` (LIBRO).
- `research/blueprints/scenario-engine.yml` (SOFTWARE).

## References

- Peter Schwartz, *The Art of the Long View* (1991).
- Kees van der Heijden, *Scenarios: The Art of Strategic Conversation* (1996).
- Donella Meadows et al., *Limits to Growth* (1972) — método de escenarios cuantitativos.
- Ramirez & Wilkinson, *Strategic Reframing* (2016).
- `references/scenario-methods.md` — métodos detallados.
