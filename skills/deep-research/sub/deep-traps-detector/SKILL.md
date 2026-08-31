---
name: deep-traps-detector
description: "Trigger: anti-patrones del dominio, errores comunes, trampas conocidas, pitfalls, cosas que evitar, what not to do. Detecta y documenta las trampas (system traps) y anti-patrones conocidos del dominio que se está investigando. Produce una guía de errores comunes con sus síntomas, causas, consecuencias y escapes. Aplica marco Meadows: los 8 system archetypes + policy resistance + rule beating + seeking the wrong goal."
license: Apache-2.0
metadata:
  category: deep-research
  subcategory: systems-thinking
  author: rubentxu
  version: "1.0"
  domain: deep-research, systems-thinking
  consumers: [book-orchestrator]
  based_on: "Meadows (Thinking in Systems 2008 cap. 5), Senge (Fifth Discipline 1990), Kim & Anderson (Systems Archetypes Basics)"
---

## Activation Contract

Úsalo cuando el tema tiene **trampas conocidas** que deben documentarse para que el lector/programador las evite. Produce un catálogo de errores comunes con su estructura sistémica.

No lo uses para: diagnóstico de un sistema concreto (eso es `deep-system-archetypes-mapper`), clasificación de intervenciones (eso es `deep-leverage-points-analyst`).

## Hard Rules (Meadows)

- **Las trampas son sistémicas, no personales**: "policy resistance... happens because different actors have different mental models of the system" (Meadows). Documentar la estructura, no el "error humano".
- **Cada trampa tiene un escape documentado**: si no hay escape conocido, marcarlo explícitamente.
- **Síntomas observables**: ¿cómo se reconoce la trampa? (sin esto, no es accionable).
- **Causas estructurales**: ¿qué configuración del sistema la produce?
- **Consecuencias sistémicas**: ¿qué empeora con el fix equivocado?

## Execution Steps

1. Activar pipeline R para el dominio.
2. Identificar las **trampas sistémicas** del dominio:
   - ¿Qué errores cometen los profesionales del campo repetidamente?
   - ¿Qué políticas/herramientas no funcionan?
   - ¿Hay debates famosos sobre fallos recurrentes?
3. Para cada trampa:
   - **Nombre**: memorable.
   - **Síntomas**: observables (qué se ve).
   - **Causa estructural**: por qué ocurre (estructura del sistema).
   - **Arquetipo subyacente**: si corresponde a uno de los 8 (Fixes That Fail, Shifting the Burden, etc.).
   - **Consecuencias**: qué empeora con el fix equivocado.
   - **Escape**: cómo salir.
   - **Caso real**: ejemplo documentado.

## Catálogo base de Meadows (Thinking in Systems cap. 5)

| # | Trampa | Estructura | Síntomas | Escape |
|---|--------|-----------|----------|--------|
| 1 | **Policy Resistance** | Múltiples actores con goals distintos; cada uno tira en su dirección | Mucha energía gastada, sistema no se mueve | Alinear metas (overarching goal); LET GO de policies inefectivas |
| 2 | **Tragedy of the Commons** | Recurso compartido + actor racional | Recurso se degrada | Privatización + regulación; gobernanza comunitaria (Ostrom) |
| 3 | **Drift to Low Performance** | Meta se ajusta a la baja cuando hay gap | Deterioro gradual, estándares cada vez más bajos | Estándares absolutos externos |
| 4 | **Escalation** | Dos R opuestos (A → B → A) | Carrera sin fin, todos pierden | Meta-arbitro; transparencia; acuerdo asimétrico |
| 5 | **Success to the Successful** | Dos R paralelos compitiendo | Winner takes all | Separar recursos; reglas de equidad |
| 6 | **Shifting the Burden / Addiction** | Solución sintomática atrofia fundamental | Dependencia creciente | Fortalecer fundamental ANTES de retirar sintomática |
| 7 | **Rule Beating** | Cumplir letra violando espíritu | Métricas mejoran, realidad empeora | Rediseñar reglas con entendimiento sistémico |
| 8 | **Seeking the Wrong Goal** | Medir lo fácil, no lo importante | Optimización para la métrica equivocada | Encontrar el goal real (a veces requiere cuestionar el paradigma) |

Para trampas específicas de un dominio (e.g., "shifting burden en RLHF"), combinar con el catálogo base.

## Esquema de trampa

```yaml
traps:
  - id: trap-rlhf-addiction
    domain: ai-alignment
    name: "RLHF Addiction"
    archetype: "Shifting the Burden"
    symptoms:
      - "El equipo confía en RLHF como solución completa"
      - "Los problemas de alignment se 'resuelven' con más RLHF"
      - "La capacidad de oversight humano se atrofia"
    structural_cause:
      - "RLHF es rápido y produce métricas visibles"
      - "No hay inversión en oversight humano robusto"
      - "El feedback loop premia el fix sintomático"
    consequences:
      - "Modelos que reward-hack"
      - "Pérdida de capacidad de evaluar alignment realmente"
    escape:
      - "Invertir en oversight humano ANTES de retirar RLHF"
      - "Diseñar métricas que no se puedan reward-hack"
    real_case: "GPT-4 reward hacking en producción (2023)"
    sources: [src-casper-2023, src-meadows-2008-cap5]
```

## Output según modo

### Modo LIBRO
- `research/traps/{topic}.yml`.
- `research/drafts/{topic}-traps-section.md` (borrador AsciiDoc con tabla de trampas).
- `research/diagrams/{topic}-traps.mmd` (Mermaid por trampa).

### Modo SOFTWARE
- `research/traps/{topic}.yml`.
- `research/blueprints/trap-detector.yml` (analiza código/config para detectar trampas).

## Decision Gates

| Situación | Acción |
|-----------|--------|
| Trampa sin escape documentado | Marcar `escape: unknown`; consultar literatura |
| Síntomas vagos | Refinar: ¿qué se OBSERVA, no qué se "siente"? |
| Trampa sin caso real | Documentar pero marcar `evidence_level: L3` (puede ser solo teoría) |
| Conflicto entre trampas (una recomienda X, otra recomienda NO X) | Documentar el trade-off; no resolver |

## Output Contract

- `research/traps/{topic}.yml`.
- `research/drafts/{topic}-traps-section.md` (LIBRO).
- Actualizar `research/corpus.yml`.

## References

- **Meadows**, *Thinking in Systems* (2008) cap. 5: "System Traps... and Opportunities".
- **Senge**, *The Fifth Discipline* (1990) cap. 6 + Appendix 2.
- **Kim & Anderson**, *Systems Archetypes Basics* (Pegasus WB002E).
- `references/trap-catalog.md` — trampas por dominio (RLHF addiction, microservices sprawl, etc.).
