---
name: research-knowledge-curator
description: "Trigger: corpus de conocimiento, base de conocimiento del libro, mantener fuentes, deduplicar fuentes, knowledge base, gaps de conocimiento, decaimiento de evidencia, caducidad de fuentes, consolidar investigación. Mantiene el corpus de conocimiento del libro como entidad persistente: deduplica fuentes, detecta gaps, gestiona el decaimiento de la evidencia y consolida todo para que capítulos y ediciones reutilicen el mismo conocimiento verificado."
license: Apache-2.0
metadata:
  author: rubentxu
  version: "1.0"
---

## Activation Contract

Úsalo de forma **transversal y recurrente**: tras cada ronda de la Macro-fase R, y como puente entre la investigación y la redacción. El corpus es **persistente entre capítulos y ediciones** — evita investigar lo mismo dos veces y conserva el conocimiento verificado.

No lo uses para investigar (`source-discovery-specialist`), ni para puntuar fuentes (`source-credibility-assessor`). Esta skill **consolida y mantiene** lo que los demás producen.

## Hard Rules

- El corpus es la **única fuente de verdad** para `source-researcher` y `chapter-writer`: nada se cita sin pasar por aquí.
- Toda entrada tiene `decay_date`: la evidencia tecnológica caduca; lo que era válido hace 2 años puede no serlo.
- Detección de duplicados: misma afirmación en varias fuentes se consolida (no se repite).
- Los gaps de conocimiento se trackean explícitamente (¿qué no sabemos todavía?).
- Versionado: cada revisión del corpus deja un snapshot (para auditoría entre ediciones).

## Execution Steps

1. Recoger salidas de la Macro-fase R:
   - `research/agenda.yml` (preguntas y status)
   - `research/credibility/` (fuentes admitidas)
   - `research/triangulation/` (afirmaciones verificadas)
   - `research/reference-validation.jsonl`
2. Consolidar en el corpus `research/corpus.yml` (esquema en `assets/corpus.schema.yml`):
   - **Sources**: deduplicadas, con nivel y `decay_date`.
   - **Claims**: afirmaciones verificadas con `confidence_score` y `status`.
   - **Gaps**: preguntas abiertas sin resolver.
   - **Index**: mapeo tema → claims → sources.
3. Calcular `decay_date` por claim según velocidad de cambio del tema (ver `references/decay-rates.md`).
4. Detectar duplicados (mismo contenido, URLs distintas) y consolidar.
5. Actualizar gaps: mover `resolved` fuera, añadir nuevos descubiertos.
6. Emitir `research/corpus-snapshot-{date}.yml` para auditoría.
7. Alimentar a `source-researcher` (que genera evidence cards desde claims `verified`).

## Esquema del corpus (resumen)

```yaml
corpus:
  version: "2026-07-23-01"
  generated_at: "2026-07-23"
  topics:
    - id: ecs-scheduling
      claims:
        - id: cl-sched-access-conflict
          text: "Bevy detecta conflictos de acceso entre sistemas en tiempo de compilación"
          status: verified
          confidence_score: 0.95
          sources: [bevy-019-news, bevy-source-schedule]
          decay_date: "2027-07-23"   # revisar en 1 año
      gaps: []
    - id: avian-compat
      claims:
        - id: cl-avian-not-019
          text: "avian2d 0.7 NO es compatible con Bevy 0.19 (depende de 0.18)"
          status: verified
          confidence_score: 0.9
          sources: [crates-io-avian]
          decay_date: "2026-10-23"   # revisar pronto (puede sacar update)
      gaps: ["¿hay fork alternativo para 0.19?"]
```

## Gestión del decaimiento

La evidencia tecnológica no es eterna:
- `decay_date` vencida → la claim pasa a `needs_recheck`.
- `version-drift-detector` (Macro-fase D) dispara re-checks cuando cambia una versión.
- Un capítulo no puede citar claims `needs_recheck` sin re-verificar.

## Decision Gates

| Situación | Acción |
|-----------|--------|
| Claim duplicada en varias fuentes | Consolidar en una con todas las sources |
| Claim `verified` con `decay_date` vencida | Re-marcar `needs_recheck` → `evidence-cross-validator` |
| Gap descubierto durante redacción | Añadir al corpus → `research-strategist` agenda |
| Corpus sin snapshot en esta edición | Generar snapshot antes de publicar |

## Output Contract

- `research/corpus.yml` (corpus vivo y consultable).
- `research/corpus-snapshot-{date}.yml` (audit trail por edición).
- `research/gaps.yml` (conocimiento faltante, priorizado).
- `source-researcher` consume claims `verified`; `version-drift-detector` consume `decay_date`.

## References

- `references/decay-rates.md` — tasas de decaimiento por tipo de tema.
- `assets/corpus.schema.yml` — esquema del corpus.
