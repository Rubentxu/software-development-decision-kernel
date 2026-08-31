---
name: deep-paradigms-explorer
description: "Trigger: paradigma, modelo mental, mindset, visión del mundo, cambio de paradigma, supuestos implícitos, ¿por qué la gente cree X?, transcendencia de paradigmas. Explora los paradigmas (level 2 de leverage points) y la capacidad de trascenderlos (level 1) detrás de un tema. Produce análisis del modelo mental compartido, sus orígenes, su influencia en el comportamiento del sistema, y cómo podría evolucionar. Aplica marco Meadows directamente: 'paradigms are the sources of systems'."
license: Apache-2.0
metadata:
  category: deep-research
  subcategory: systems-thinking
  author: rubentxu
  version: "1.0"
  domain: deep-research, systems-thinking
  consumers: [book-orchestrator]
  based_on: "Donella Meadows (Leverage Points 1997), Thomas Kuhn (Structure of Scientific Revolutions 1962)"
---

## Activation Contract

Úsalo cuando el tema tiene **dimensión cultural, ideológica o de modelos mentales**: ¿por qué la gente piensa así? ¿qué paradigma sostiene este campo? ¿cómo podría cambiar? Produce análisis del paradigma compartido y opciones para trascenderlo.

No lo uses para: análisis técnico puro (eso es `deep-software-research`), comparativa de opciones (eso es `deep-knowledge-graph-builder`).

## Hard Rules (Meadows)

- **Paradigma = fuente del sistema**: de los 12 leverage points, el paradigma es el #2 (después de "trascender paradigmas"). Es el más poderoso y el más difícil de cambiar.
- **Paradigmas son sociales**: rara vez son individuales. "Shared social agreements about the nature of reality" (Meadows).
- **Sin evidencia factual los paradigmas son inmunes a la evidencia**: Kuhn mostró que los paradigmas solo cambian cuando hay crisis + alternativa viable.
- **Trascender paradigmas** (level 1) requiere humildad epistémica: capacidad de cuestionar TODOS los paradigmas, incluido el propio.
- **Documentar la hegemonía**: ¿quién refuerza el paradigma? ¿qué instituciones lo sostienen?

## Execution Steps

1. Activar pipeline R para el tema.
2. Identificar el **paradigma actual dominante**:
   - ¿Qué supuestos compartidos hay?
   - ¿Cómo se expresan? (frases hechas, métricas, prácticas).
   - ¿Quién los sostiene? (instituciones, autores, medios).
3. Identificar los **paradigmas competidores o históricos**:
   - ¿Hubo paradigmas previos en este campo?
   - ¿Hay alternativas activas hoy?
4. Documentar la **influencia del paradigma en el sistema**:
   - ¿Cómo determina las reglas?
   - ¿Cómo determina los goals?
   - ¿Cómo determina qué se considera "leverage point" legítimo?
5. Explorar la **trascendencia** (level 1):
   - ¿Qué requeriría cuestionar este paradigma?
   - ¿Hay señales de crisis del paradigma?
   - ¿Hay alternativas viables?
6. Conectar con leverage points: ¿qué intervenciones en niveles 5-12 serían posibles si el paradigma cambiara?

## Esquema de paradigma

```yaml
paradigm:
  topic: "AI alignment"
  current_paradigm:
    name: "Scalable oversight via RLHF"
    shared_assumptions:
      - "Los LLM son optimizables para alinearse con valores humanos via reward modeling"
      - "Más capacidad + más datos = mejor alineación"
      - "El problema técnico es encontrar el reward signal correcto"
    sources_supporting: [src-christiano-2017, src-ouyang-2022]
    critics:
      - {claim: "RLHF solo optimiza para aprobación, no para valores reales", source: src-casper-2023}
      - {claim: "El problema es estructural, no de reward signal", source: src-bostrom-2014}
    institutions_reinforcing: [OpenAI, Anthropic, DeepMind]
  historical_paradigms:
    - {name: "AI safety via provability", period: "1990s", sources: [src-russell-norvig]}
    - {name: "AI safety via corrigibility", period: "2010s", sources: [src-soares-2015]}
  transcendence:
    questions:
      - "¿Y si el problema NO es técnico?"
      - "¿Y si la 'alineación' asume una definición de 'valores humanos' que no existe?"
    signals_of_crisis:
      - "Modelos que reward-hack de formas sutiles"
      - "Dificultad creciente para evaluar alignment"
    alternative_visions:
      - {name: "Constitutional AI via deliberación", source: src-bai-2022}
      - {name: "Democratized alignment via pluralismo", source: src-davidson-2024}
  leverage_point_implications:
    - {if_paradigm_changes: "Aceptación de que la regulación es inevitable", new_leverage_points: [5, 6]}
```

## Output según modo

### Modo LIBRO
- `research/paradigms/{topic}.yml`.
- `research/drafts/{topic}-paradigm-section.md` (borrador AsciiDoc con cita de Meadows/Kuhn).
- `research/diagrams/{topic}-paradigm-shift.mmd` (Mermaid).

### Modo SOFTWARE
- `research/paradigms/{topic}.yml` para documentar supuestos en `blueprints`.
- `research/blueprints/paradigm-detector.yml` (analiza texto para detectar paradigma subyacente).

## Decision Gates

| Situación | Acción |
|-----------|--------|
| Tema sin paradigma identificable | Puede que sea muy técnico; considerar si realmente aplica |
| Paradigma sin críticos documentados | Sospechoso: ¿estás en una cámara de eco? Buscar voces disidentes. |
| Solo 1 alternativa al paradigma | Sesgo de confirmación; buscar al menos 2 alternativas |
| Sin señales de crisis | El paradigma puede estar fuerte; pero documentar igual |
| Trascendencia sin evidencia | Es especulación; marcar `evidence_level: L5` |

## Output Contract

- `research/paradigms/{topic}.yml`.
- `research/drafts/{topic}-paradigm-section.md` (LIBRO).
- Actualizar `research/corpus.yml`.

## References

- **Donella Meadows**, "Leverage Points: Places to Intervene in a System" (1997). El paradigma es level 2; trascender paradigmas es level 1.
- **Thomas Kuhn**, *The Structure of Scientific Revolutions* (1962). Citado por Meadows en *Leverage Points* nota 7.
- **Imre Lakatos**, *The Methodology of Scientific Research Programmes* (1978). Para programas de investigación.
- **Daniel Dennett**, *From Bacteria to Bach and Back* (2017). Para evolución de competencias.
- `references/paradigm-detection.md` — heurísticas para identificar paradigmas en un texto.
