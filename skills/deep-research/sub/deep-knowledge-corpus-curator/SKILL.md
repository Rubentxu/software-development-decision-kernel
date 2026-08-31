---
name: deep-knowledge-corpus-curator
description: "Trigger: consolidar corpus, deduplicar fuentes, knowledge base del libro, decaimiento de evidencia, caducidad de fuentes, gaps de conocimiento, knowledge graph del libro. Mantiene el corpus de conocimiento del libro/proyecto como entidad persistente: deduplica fuentes, detecta gaps, gestiona decaimiento, snapshot por edición. Es el 'sistema nervioso' de la Macro-fase R."
license: Apache-2.0
metadata:
  category: deep-research
  subcategory: r-pipeline
  author: rubentxu
  version: "1.0"
  domain: deep-research
  based_on: "research-knowledge-curator (rubentxu), generalizado"
---

## Activation Contract

Úsalo **transversal y recurrentemente**: tras cada ronda del pipeline R, y como puente entre investigación y redacción/consumo (capítulos o software). El corpus es **persistente entre ediciones** — evita investigar dos veces lo mismo y conserva el conocimiento verificado.

No lo uses para: investigar (`deep-source-discovery-specialist`), puntuar credibilidad (`deep-source-credibility-assessor`), triangular (`deep-evidence-triangulator`). Esta skill **consolida y mantiene** lo que los demás producen.

## Hard Rules

- El corpus es la **única fuente de verdad** para `deep-claim-extractor` (R6) y los consumidores finales (book-orchestrator, orchestrator): nada se cita sin pasar por aquí.
- **Deduplicación**: misma afirmación en varias fuentes se consolida en una sola claim con múltiples sources.
- **Gaps explícitos**: lo que NO sabemos se trackea en `research/gaps.yml` (priorizado por `risk`).
- **Decaimiento**: cada claim tiene `decay_date`. Claims vencidas → `needs_recheck` automáticamente.
- **Snapshot por edición**: cada revisión mayor del corpus deja `research/corpus-snapshot-{date}.yml` para auditoría.
- **Coherencia con el sistema del tema (R0)**: el corpus debe mantener viva la conexión entre claims y el `system-map` (Meadows: no perder la lente sistémica).

## Execution Steps

1. Recoger salidas del pipeline R:
   - `research/system-map/{topic}.yml` (R0).
   - `research/agenda.yml` (R1).
   - `research/candidate-pool.yml` (R2).
   - `research/credibility/*.yml` (R3a).
   - `research/reference-validation.jsonl` (R3b).
   - `research/triangulation/*.yml` (R4).
2. Consolidar en el corpus `research/corpus.yml` (esquema en `assets/corpus.schema.yml`):
   - **Sources**: deduplicadas (mismo DOI/URL/ISBN = misma fuente), con `evidence_level` y `decay_date`.
   - **Claims**: afirmaciones verificadas con `confidence_score` y `status`.
   - **Gaps**: preguntas abiertas sin resolver, priorizadas.
   - **System map**: el propósito, elementos, loops, leverage points (R0) referenciados por claims.
   - **Index**: mapeo tema → claims → sources.
3. Calcular `decay_date` por claim según la velocidad de cambio del dominio (ver `references/decay-rates.md`):
   - Tech (frameworks, APIs): 1-2 años.
   - AI/ML (state-of-the-art): 6-12 meses.
   - Science: 3-5 años salvo refutación.
   - Foundational (Meadows, Knuth): no caduca; revisar si hay reinterpretación.
   - Historia: no caduca.
4. Detectar duplicados (mismo contenido, URLs distintas) y consolidar.
5. Detectar gaps: mover claims `resolved` fuera, añadir nuevos descubiertos.
6. Emitir `research/corpus-snapshot-{date}.yml` para auditoría.
7. Alimentar a `deep-claim-extractor` (R6) que genera evidence cards desde claims `verified`.

## Esquema del corpus (resumen)

```yaml
corpus:
  version: "2026-08-16-01"
  generated_at: "2026-08-16"
  topics:
    - id: ecs-scheduling
      system_map_ref: research/system-map/bevy-ecs-scheduling.yml
      claims:
        - id: cl-bevy-access-conflict-detection
          text: "Bevy detecta conflictos de acceso entre sistemas en tiempo de compilación"
          status: verified
          confidence_score: 0.92
          sources: [src-bevy-source-schedule, src-bevy-rfcs]
          decay_date: "2027-08-16"
        - id: cl-avian-not-019
          text: "avian2d 0.7 NO es compatible con Bevy 0.19"
          status: verified
          confidence_score: 0.9
          sources: [src-crates-io-avian]
          decay_date: "2026-10-23"
      gaps:
        - id: gap-alternative-fork
          question: "¿Hay fork alternativo de avian compatible con Bevy 0.19?"
          priority: normal
```

## Gestión del decaimiento

- `decay_date` vencida → la claim pasa a `status: needs_recheck`.
- `deep-reference-validator` detecta `version_drift` → marca la claim como `needs_recheck`.
- Un capítulo no puede citar claims `needs_recheck` sin re-verificar.
- `version-drift-detector` (en book-orchestrator) dispara re-checks cuando cambia una versión.

## Decisión Gates

| Situación | Acción |
|-----------|--------|
| Claim duplicada en varias fuentes | Consolidar en una con todas las sources listadas |
| Claim `verified` con `decay_date` vencida | Re-marcar `needs_recheck`; re-disparar R4 |
| Gap descubierto durante redacción/codificación | Añadir al corpus; el curator agenda re-R |
| Corpus sin snapshot en esta edición | Generar snapshot antes de publicar/desplegar |
| Claim contradice el `system-map` (R0) | STOP: re-evaluar la lente sistémica. ¿La claim es correcta pero el mapa está mal? ¿O la claim es un artefacto? |

## Output Contract

- `research/corpus.yml` (corpus vivo y consultable).
- `research/corpus-snapshot-{date}.yml` (audit trail por edición/despliegue).
- `research/gaps.yml` (conocimiento faltante, priorizado).
- `deep-claim-extractor` (R6) consume claims `verified`.

## References

- `references/decay-rates.md` — tasas de decaimiento por dominio.
- `assets/corpus.schema.yml` — esquema del corpus.
