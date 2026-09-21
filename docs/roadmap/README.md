# SDDK — punto único de entrada operativo

**Vigente desde:** 2026-09-21. **Estado:** propuesta de continuación; no certifica cambios futuros ni modifica las certificaciones históricas.

**Arranque obligatorio de cada sesión:** leer [CURRENT.md](CURRENT.md) → [ROADMAP.md](ROADMAP.md) → [CERTIFICATIONS.md](CERTIFICATIONS.md) → [UAT-MATRIX.md](UAT-MATRIX.md) → último bloque de [SESSION-JOURNAL.md](SESSION-JOURNAL.md), contrastar con `git status -sb`, `git rev-parse HEAD`, último tag/release y recibos reales. Las reglas están en [AGENTS.md](../../AGENTS.md).

| Documento | Autoridad |
| --- | --- |
| [CURRENT.md](CURRENT.md) | Puntero humano **actual**: commit comprobado, hito activo, siguiente acción, decisiones y bloqueos. No constituye evidencia por sí solo. |
| [STATE.yaml](STATE.yaml) | Espejo legible por máquina de CURRENT; si difieren, STOP y reconciliar, no elegir arbitrariamente. |
| [ROADMAP.md](ROADMAP.md) | **Único roadmap ejecutable nuevo**, orden de dependencia y criterios de salida. |
| [CERTIFICATIONS.md](CERTIFICATIONS.md) | Contratos de certificación, promociones, validez, expiración y recibos. |
| [UAT-MATRIX.md](UAT-MATRIX.md) | Escenarios falsables que deben observarse antes de certificar. |
| [SESSION-JOURNAL.md](SESSION-JOURNAL.md) | Diario cronológico append-only: qué pasó realmente y cómo recuperar. |
| [../history/README.md](../history/README.md) | Catálogo de archivados y documentos históricos congelados. |

## Orden de precedencia y conservación

1. El código, commit/tag, artefactos y resultados ejecutados son evidencia factual; un texto `CLOSED` sin prueba vinculada no equivale a PASS.
2. ADRs aceptados en [../architecture/adrs/](../architecture/adrs/), specs vigentes y contratos de seguridad/release existentes gobiernan el **cómo**; este roadmap gobierna **qué sigue**. Ante contradicción: STOP, abrir decisión de reconciliación y no relajar un gate.
3. [14/09 mini-roadmap](../SDDK-Production-Readiness-Alignment-2026-09-14/02-MINI-ROADMAP.md), [A5 snapshot](../architecture/a5/A5-CURRENT-ROADMAP.md) y [AIW state](../proposals/2026-09-19-adaptive-inputs-workflows/STATE-OF-AIW.md) son trazabilidad histórica de su fecha, no backlogs concurrentes.
4. No reescribir recibos antiguos, certificar retroactivamente, borrar incidencias ni reinterpretar `DEFERRED` como `DONE`. Un nuevo ciclo puede aportar evidencia nueva y referenciar la anterior.

## Regla de mantenimiento

Al abrir/cerrar un slice: actualizar CURRENT + STATE + diario y enlazar SCOPE, UAT, RECEIPT y SHA. No avanzar el puntero a `DELIVERED` antes de observar la aceptación. No alterar `main` o publicar una release sin seguir el contrato local-first de AGENTS/release.sh.
