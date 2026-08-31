---
name: deep-source-credibility-assessor
description: "Trigger: evaluar credibilidad de fuente, ranking L1-L7, sesgo de fuente, conflicto de interés, freshness, qué tan confiable es X. Puntúa cada candidato del candidate-pool con niveles L1-L7, evaluando sesgo, conflicto de interés, frescura, autoridad del autor, metodología. Produce ranking admitidas/rechazadas para alimentar al triangulator. Aplica el principio de Meadows: las fuentes tienen diferentes leverage (L1 tiene mucho más leverage que L5)."
license: Apache-2.0
metadata:
  category: deep-research
  subcategory: r-pipeline
  author: rubentxu
  version: "1.0"
  domain: deep-research
  based_on: "source-credibility-assessor (rubentxu), generalizado"
---

## Activation Contract

Úsalo en **R3a del pipeline R**, después de R2 (`deep-source-discovery-specialist`) y en paralelo con R3b (`deep-reference-validator`). Recibe `research/candidate-pool.yml` y produce `research/credibility/{source-id}.yml` con ranking final.

No lo uses para: descubrir fuentes (R2), validar URLs (R3b), triangular claims (R4). Esta skill **puntúa credibilidad**; las demás validan.

## Hard Rules

- **Criterios evaluados** (5 dimensiones):
  1. **Autoridad**: ¿el autor es autoridad reconocida en el campo? (papers seminales, posición institucional).
  2. **Metodología**: ¿el paper usa métodos rigurosos? (peer review, replicación, dataset abierto).
  3. **Independencia**: ¿la fuente tiene conflictos de interés? (sponsor comercial, advocacy).
  4. **Frescura**: ¿la evidencia está vigente? (tech: ≤2 años; science: ≤5 años salvo foundational).
  5. **Trazabilidad**: ¿se puede verificar la cita? (URL live, DOI, página exacta).
- **Niveles L1-L7** (ver `references/evidence-levels.md`):
  - L1: fuente primaria, autor canónico, metodología impecable.
  - L2: oficial autoritativa (institución, white paper).
  - L3: secundaria revisada (enciclopedia, peer-reviewed que cita L1).
  - L4: periodística especializada.
  - L5: terciaria (blog, video).
  - L6: anécdota.
  - L7: sin fuente.
- **Sesgo explícito**: documentar el posible sesgo de la fuente (ej: "Microsoft Research tiene interés comercial en Azure").
- **Conflicto de interés (COI)**: marcar cualquier sponsor comercial, advocacy, financiación sesgada.
- **Independencia entre fuentes**: marcar `independent_from: [other-id]` para evaluar en triangulación.

## Execution Steps

1. Lee `research/candidate-pool.yml`.
2. Para cada candidato:
   a. **Evaluar autoridad del autor(es)**: h-index, posición institucional, papers seminales anteriores, premios.
   b. **Evaluar metodología**: peer review, replicabilidad, dataset abierto, código fuente disponible.
   c. **Evaluar independencia**: COI explícitos, sponsors, advocacy. Si hay sesgo, marcar `bias: detected` con descripción.
   d. **Evaluar frescura**: fecha de publicación vs. campo. Tech: caduca rápido; foundational: no caduca.
   e. **Evaluar trazabilidad**: URL live, DOI registrado, página exacta verificable.
3. Asigna `evidence_level` final (L1-L7) y `credibility_score` (0.0-1.0).
4. Marca `admitted: true/false` (admisible como soporte de claims).
5. Marca `independent_from: [lista]` (otros candidatos del pool).
6. Genera `research/credibility/{source-id}.yml` con la evaluación completa.

## Criterios detallados

### Autoridad
- **L1**: autor es creador/originador del concepto (Meadows para leverage points, Hoare para Quicksort).
- **L2**: autor es autoridad institucional (researcher en institución reconocida).
- **L3**: autor es académico con publicaciones pero no autoridad directa en el tema.
- **L5**: blogger, comentarista, sin credenciales verificables.

### Metodología
- **Excelente** (L1): peer review, datos primarios, replicación, código fuente.
- **Buena** (L2): peer review sin datos primarios; o white paper institucional riguroso.
- **Aceptable** (L3): peer review de segunda mano (review papers); o documentación oficial.
- **Limitada** (L5): sin peer review, sin datos primarios.

### Independencia / COI
- **Independiente**: universidad pública, journal sin sponsor comercial, autor sin afiliación.
- **Posible COI**: industria con interés comercial (ej: paper sobre cloud por vendor cloud).
- **COI fuerte**: advocacy group, lobby, financiado por actor con interés directo.

### Frescura
- **Tech**: ≤2 años para state-of-the-art; foundational puede ser 10+ años (Knuth, Hoare).
- **Science**: ≤5 años para resultados experimentales; foundational puede ser 50+ años (Darwin, Newton).
- **Systems Thinking**: foundational (Meadows 2008, Forrester 1961) no caduca; re-tests sí.

### Trazabilidad
- **Excelente**: URL live, DOI, página exacta, cita textual disponible.
- **Buena**: URL live pero sin página exacta.
- **Limitada**: URL muerta, sin DOI, sin paginación.

## Esquema de credibility/{source-id}.yml

```yaml
credibility:
  source_id: src-bevy-source-schedule
  evaluated_at: "2026-08-16"
  authority:
    level: high  # high | medium | low
    rationale: "Carter Anderson es creador de Bevy; código fuente es fuente primaria"
  methodology:
    level: high
    rationale: "Código fuente ejecutable; verificable"
  independence:
    bias_detected: false
    coi: "Open source mantenido por Bevy Foundation"
  freshness:
    assessment: current
    rationale: "Repositorio activo en 2026"
  traceability:
    url_status: live
    doi: null
    page_exact_available: true  # para papers; código se cita por path
  evidence_level: L1
  credibility_score: 0.98
  admitted: true
  independent_from: [src-bevy-rfcs, src-bevy-docs]
  notes: "Fuente primaria por excelencia para comportamiento del scheduler"
```

## Decision Gates

| Situación | Acción |
|-----------|--------|
| evidence_level < L3 para claim_type que exige L1-L2 | Marcar `admitted: false` con razón; sugerir búsqueda de L1-L2 |
| COI fuerte detectado | `admitted: false` para claims `critical`; puede admitirse para L5 con disclaimer |
| Frescura insuficiente (tech > 2 años sin re-test) | Marcar `decay_date` acelerada; recomendar re-check |
| Fuente con sesgo pero única disponible | Admitir con `bias_warning: "..."` obligatorio en la cita |
| Múltiples fuentes L1-L2 independientes | Ideal para triangulación (R4) |

## Output Contract

- `research/credibility/{source-id}.yml` para cada candidato del pool.
- `admitted: true/false` por fuente.
- Lista de fuentes admitidas lista para `deep-evidence-triangulator` (R4).

## References

- `references/evidence-levels.md` — niveles L1-L7.
- `references/bias-detection.md` — heurísticas para detectar sesgo.
- `assets/credibility.schema.yml` — esquema validable.
