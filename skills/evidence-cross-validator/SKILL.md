---
name: evidence-cross-validator
description: "Trigger: triangulación de fuentes, conflicto entre fuentes, cross-check, validar afirmación contra varias fuentes, puntuación de confianza, consensus, contradiction. Triangula cada afirmación clave entre múltiples fuentes independientes, resuelve conflictos cuando las fuentes se contradicen y asigna una puntuación de confianza que determina si la afirmación es publicable."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo **después** de `source-credibility-assessor` (que ha rankeado las candidatas) y **antes** de que `source-researcher` produzca evidence cards definitivas. Es la **capa de calidad epistémica**: una afirmación no es fiable porque una fuente la diga, sino porque varias independientes convergen (o el conflicto se resuelve explícitamente).

No lo uses para descubrir fuentes (`source-discovery-specialist`) ni para validar que una URL existe (`reference-validator`).

## Hard Rules

- Una afirmación `critical` necesita **≥2 fuentes independientes** convergiendo, o **1 fuente L1/L2** de autoridad indiscutible.
- "Independientes" significa que no se citan mutuamente (dos blogs que copian de la doc oficial no son independientes entre sí; ambas dependen de la oficial).
- Todo conflicto entre fuentes se **resuelve explícitamente** y se documenta (no se ignora).
- La puntuación de confianza determina el `status` final: `verified` (publicable) / `disputed` (no publicable sin disclaimer) / `unverified`.
- La resolución de conflictos **favorece la fuente de mayor autoridad**, salvo que la menor presente evidencia reproducible (L1-exp) que la refute.

## Execution Steps

1. Leer `research/candidate-pool.yml` (fuentes rankeadas) y `research/agenda.yml`.
2. Agrupar fuentes por `agenda_question`.
3. Para cada pregunta/afirmación clave:
   - Extraer la postura de cada fuente sobre esa afirmación.
   - Clasificar la relación entre fuentes: `consensus` | `partial-agreement` | `contradiction` | `silence`.
   - Verificar independencia (¿se citan mutuas?).
4. Resolver conflictos:
   - Si `contradiction`: identificar qué fuente tiene mayor autoridad o evidencia reproducible.
   - Documentar la resolución en `research/triangulation/{question}.yml`.
5. Asignar `confidence_score` (0.0–1.0) según convergencia, autoridad e independencia (ver `references/confidence-scoring.md`).
6. Mapear score a `status`: ≥0.8 `verified`, 0.5–0.79 `disputed` (publicable con disclaimer), <0.5 `unverified` (bloqueante).
7. Emitir `research/triangulation-report.yml`.

## Esquema de triangulación (resumen)

```yaml
triangulation:
  question: RQ-bsn-syntax
  claim: "bsn! usa sintaxis { Component { field } } y Children [] para jerarquías"
  sources:
    - id: bevy-019-news          # L2
      stance: confirms
    - id: bevy-scene-docsrs      # L2
      stance: confirms
    - id: blog-tutorial-bsn      # L7
      stance: confirms
      independent_of: [bevy-019-news]   # deriva de la oficial → NO independiente
  relation: consensus
  independent_count: 2           # oficial + docs (la L7 depende de la oficial)
  confidence_score: 0.95
  status: verified
```

## Resolución de conflictos (ejemplo)

```yaml
triangulation:
  question: RQ-avian-version
  claim: "avian2d es compatible con Bevy 0.19"
  sources:
    - id: avian-readme           # L3, dice "0.19"
      stance: confirms
    - id: crates-io-avian        # L2, dice "compatible 0.18 max"
      stance: refutes
      has_reproducible_evidence: true   # el manifest del crate
  relation: contradiction
  resolution: >
    crates.io (L2, evidencia reproducible del manifest) prevalece sobre el README
    (L3, posiblemente desactualizado). avian2d NO es compatible con 0.19 a día de hoy.
  confidence_score: 0.9
  status: verified   # la afirmación CORREGIDA sí está verificada
```

## Decision Gates

| Situación | Acción |
|-----------|--------|
| Solo 1 fuente para afirmación critical | `coverage: thin` → más descubrimiento |
| Fuentes L7 "independientes" que derivan de la misma oficial | No cuentan como independientes |
| Conflicto sin resolución clara | `status: disputed`, bloqueante |
| Conflicto resuelto a favor de menor autoridad con L1-exp | Documentar reasoning |

## Output Contract

- `research/triangulation/{question}.yml` por pregunta crítica.
- `research/triangulation-report.yml` con scores y status por afirmación.
- `source-researcher` produce evidence cards solo para afirmaciones `verified`.
- Afirmaciones `disputed`/`unverified` bloquean su uso en capítulos.

## References

- `references/confidence-scoring.md` — fórmula de puntuación de confianza.
- `references/conflict-resolution.md` — protocolo de resolución por autoridad y evidencia.
