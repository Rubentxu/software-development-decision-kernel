---
name: deep-evidence-triangulator
description: "Trigger: triangular evidencia, cruzar fuentes, encontrar conflictos, calcular confidence_score, claim verificada, fuentes en conflicto. Combina los resultados de credibilidad (R3a) y validación (R3b) para evaluar cada claim del corpus: cuántas fuentes independientes la respaldan, qué nivel de confianza asignar, si hay conflictos. Núcleo de R4. Aplica principio Meadows: triangulación es el equivalente a 'multiple feedback loops' confirmando la dirección."
license: Apache-2.0
metadata:
  category: deep-research
  subcategory: r-pipeline
  author: rubentxu
  version: "1.0"
  domain: deep-research
  based_on: "evidence-cross-validator (rubentxu), generalizado"
---

## Activation Contract

Úsalo en **R4 del pipeline R**, después de R3 (credibilidad + validación). Recibe las claims en borrador con sus fuentes evaluadas y produce `research/triangulation/{claim-id}.yml` con `confidence_score` y status final.

No lo uses para: descubrir (R2), evaluar credibilidad individual (R3a), validar URLs (R3b), consolidar corpus (R5). Esta skill **combina**; las demás producen datos.

## Hard Rules

- **Triangulación obligatoria** para claims `risk: critical`:
  - ≥ 2 fuentes independientes (L1 o L2).
  - Si solo 1 fuente → `confidence_score ≤ 0.7` y requiere disclaimer.
  - Si 0 fuentes → `status: unverified`, bloquea para `chapter-writer`.
- **Independencia importa**: 2 papers del mismo autor en la misma revista NO son independientes. Buscar fuentes de grupos distintos.
- **Conflictos explícitos**: si 2+ fuentes de calidad dicen cosas distintas, marcar `status: disputed` con `conflict_summary`.
- **Consenso aparente sospechoso**: si muchas fuentes dicen lo mismo, verificar que NO vienen todas de la misma fuente primaria (cascada de citas).
- **confidence_score** calculado, no inventado:
  - `0.0-0.5`: muy baja (1 fuente, baja credibilidad).
  - `0.5-0.7`: baja (1 fuente creíble, o varias no independientes).
  - `0.7-0.85`: media (2+ fuentes independientes, ≥ 1 L1).
  - `0.85-0.95`: alta (3+ fuentes independientes, ≥ 2 L1).
  - `0.95-1.0`: muy alta (consenso amplio de L1 independientes).

## Execution Steps

1. Lee el corpus en borrador: claims con sus `source_ids`.
2. Lee `research/credibility/*.yml` para evaluar cada fuente.
3. Lee `research/reference-validation.jsonl` para verificar que las fuentes están vivas.
4. Para cada claim:
   a. Identifica las fuentes que la respaldan.
   b. Filtra: solo fuentes con `admitted: true` y `validation_status: live`.
   c. Evalúa independencia: ¿vienen de grupos/autores distintos?
   d. Detecta cascada de citas: si varias fuentes citan a la misma fuente primaria, contarlas como 1.
   e. Detecta conflictos: ¿alguna fuente admitida contradice la claim?
   f. Calcula `confidence_score` (algoritmo en `references/confidence-algorithm.md`).
   g. Asigna `status`: `verified` | `disputed` | `needs_recheck` | `deprecated`.
5. Genera `research/triangulation/{claim-id}.yml` con el análisis completo.

## Algoritmo de confidence_score

```
score = 0
+ (0.3 * número_de_fuentes_L1_independientes, max 0.9)
+ (0.15 * número_de_fuentes_L2_independientes, max 0.45)
+ (0.05 * número_de_fuentes_L3_independientes, max 0.15)
- 0.2 si hay conflicto con fuente admitida
- 0.1 si todas las fuentes vienen del mismo grupo/autor
- 0.1 si la fuente primaria es de hace >5 años (tech) sin re-test
clamp(score, 0.0, 1.0)
```

## Estados posibles de una claim

| Status | Cuándo |
|--------|--------|
| `verified` | `confidence_score ≥ 0.85` y sin conflictos |
| `verified-with-disclaimer` | `confidence_score 0.7-0.85` o 1 fuente única pero creíble |
| `disputed` | `confidence_score ≥ 0.5` pero con conflicto explícito |
| `needs_recheck` | `decay_date` vencida o `version_drift` detectado |
| `unverified` | `confidence_score < 0.5` o 0 fuentes admitidas |
| `deprecated` | refutada por evidencia posterior (con paper de refutación) |

## Esquema de triangulation/{claim-id}.yml

```yaml
triangulation:
  claim_id: cl-bevy-access-conflict-detection
  claim_text: "Bevy detecta conflictos de acceso entre sistemas en tiempo de compilación"
  evaluated_at: "2026-08-16"
  sources_supporting:
    - source_id: src-bevy-source-schedule
      evidence_level: L1
      admitted: true
      validation_status: live
      independent: true
      page_reference: "schedule.rs: scheduler::conflicting_access"
    - source_id: src-bevy-rfcs
      evidence_level: L1
      admitted: true
      validation_status: live
      independent: true
      page_reference: "RFC #18: Scheduler design"
  sources_contradicting: []
  cascade_detected: false
  conflicts: []
  confidence_score: 0.92
  status: verified
  decay_date: "2027-08-16"  # revisar en 1 año (tech)
  notes: "Triangulación fuerte con código fuente y RFC"
```

## Decision Gates

| Situación | Acción |
|-----------|--------|
| `confidence_score < 0.7` para claim `critical` | Bloquear; no puede ir al libro sin disclaimer |
| Conflicto entre fuentes L1 | Marcar `disputed`; el chapter-writer debe presentar ambas posiciones |
| Cascada de citas detectada | Re-evaluar; el `confidence_score` se reduce |
| Claim refutada por paper reciente | Marcar `deprecated` con link al paper de refutación |
| `decay_date` vencida durante la evaluación | Marcar `needs_recheck`; el curator agenda re-check |

## Output Contract

- `research/triangulation/{claim-id}.yml` para cada claim evaluada.
- `research/triangulation/_summary.yml` con métricas globales (claims por status, conflictos abiertos, etc.).
- `deep-knowledge-corpus-curator` (R5) recibe el resultado para consolidar.

## References

- `references/confidence-algorithm.md` — detalles del cálculo.
- `references/conflict-detection.md` — heurísticas para detectar conflictos sutiles.
- `assets/triangulation.schema.yml` — esquema.
