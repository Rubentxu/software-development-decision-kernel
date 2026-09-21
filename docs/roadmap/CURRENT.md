# CURRENT — puntero de reanudación de SDDK

**Estado:** IN_PROGRESS_C0 (PR #7 integrado, baseline reconciliándose). **Actualizado:** 2026-09-21T11:56:00Z. Este puntero NO acredita que los gates hayan pasado ni que exista una release posterior.

| Campo | Valor observado o pendiente |
| --- | --- |
| Fuente de la fotografía | `main@68f6788fe9d2a8d10fb4b1ec5d10b86d44d8d9c5` consultado 2026-09-21; **revalidar al comenzar cada sesión** |
| Workspace en esa fotografía | `1.169.133` (Cargo.toml) |
| Release pública comprobada en esa fotografía | `v1.169.122` (2026-09-20); **no asumir que sigue siendo la última** |
| Hito activo | `C1` — H01 pusheado (74dfcc9+abca553 v1.169.133); cycle-c SCOPE-CONTRACT pusheado (68f6788); reorganización documental completada (docs/history poblado) |
| Estado PR #7 | INTEGRADO en `13d4131` (docs-only) + `96f5366` (AGENTS.md + bump 1.169.128); rama borrada vía `gh pr close --delete-branch` |
| Último hito histórico | A5-C Base v1.169.88 condicionado; closures A6/A7/A8/J/AIW por alcance documentado; NO declarar certificaciones enhanced/GA a partir de esto |
| Siguiente acción exacta | **Implementar cycle-c** (corrección del contrato de push/release en rama de trabajo). Plan: tests RED→GREEN del nuevo contrato (last_published_version + range-based allowlist), luego integrar vía PR o merge fast-forward. H02 se recupera en el reflog (commit 9d4c249) tras tener vía de integración limpia. NO bumpear, NO publicar, NO forzar push. |
| Evidencia requerida para mover puntero | Recibo C0 firmado/aceptado, SHA nuevo, UAT T01/T02 observados, CURRENT y STATE reconciliados |
| Bloqueos y decisiones | Release/push bajo política del operador; binarios reales EXT y adaptador host necesarios para sus perfiles; J7/J8/J9/X08/R11 siguen diferidos; v1.169.123..128 no publicadas (operator-side) |
| Próxima revisión | Al inicio de **cada** sesión y después de cada commit/release relevante |

## Recuperación sin adivinar

1. Confirmar qué rama contiene el nuevo plan, si el PR está integrado y cuál es la versión real de main. Si no está integrado, el puntero de main no ha cambiado.
2. Leer [ROADMAP.md](ROADMAP.md) y [CERTIFICATIONS.md](CERTIFICATIONS.md); localizar el último recibo **observado** del hito activo y los casos [UAT](UAT-MATRIX.md) no ejecutados.
3. Contrastar el último bloque de [SESSION-JOURNAL.md](SESSION-JOURNAL.md) con `git log -5` y el estado operativo; si difieren, registrar reconciliación como **nueva** entrada, sin editar el pasado.
4. Solo entonces abrir/continuar el próximo WorkItem. Un resumen de sesión, un commit de docs o un dry-run no sustituyen un recibo de certificación.
