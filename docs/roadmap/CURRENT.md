# CURRENT — puntero de reanudación de SDDK

**Estado:** C1 H02 integrado en main. H05+H06 y cycle-c pendientes de merge (PR #10, PR #11 mergeables). **Actualizado:** 2026-09-21T17:03:00Z. Este puntero NO acredita que los gates hayan pasado ni que exista una release posterior.

| Campo | Valor observado o pendiente |
| --- | --- |
| Fuente de la fotografía | `main@cfe3766` (post-PR#9 merge) consultado 2026-09-21T17:03Z; **revalidar al comenzar cada sesión** |
| Workspace en esa fotografía | `1.169.134` (Cargo.toml); mismo bump en PR #10 (aún mergeable) que debe deduplicarse en rebase |
| Release pública comprobada en esa fotografía | `v1.169.122` (2026-09-20); **no asumir que sigue siendo la última** |
| Hito activo | `C1` — H01 pusheado (74dfcc9+abca553); cycle-c SCOPE-CONTRACT pusheado (68f6788); reorganización documental integrada vía PR #8; **H02 integrado vía PR #9 (merge cfe3766)**. PR #10 (H05+H06) y PR #11 (cycle-c admisión v2) mergeables. |
| Estado PRs abiertos | #10 (H05+H06) y #11 (cycle-c admisión v2), ambos mergeables. PR #10 con bump 1.169.134 que hay que deduplicar en rebase contra main@cfe3766. |
| Reorganización docs/history | COMPLETA. `docs/history/` contiene paquetes legacy, handoffs, proposals, research, cycles, uat-plans y evolutivos. `docs/history/README.md` es el catálogo. Único roadmap ejecutable: `docs/roadmap/ROADMAP.md`. |
| Siguiente acción exacta | **Operator-side/jcode (con preautorización operador):** rebase PR #10 sobre main@cfe3766 (resolver conflicto de bump 1.169.134 eliminando el commit fc418a7 porque el bump ya está en main), merge PR #10, rebase PR #11 sobre el nuevo main, merge PR #11, reconciliar CURRENT/STATE/JOURNAL con SHA final. Tras eso, ejecutar full profile C1 (cargo fmt --check, cargo clippy --workspace --all-targets -- -D warnings, cargo test --workspace) y reconciliar. |
| Evidencia requerida para mover puntero | Recibo C0 firmado/aceptado, SHA nuevo, UAT T01/T02 observados, CURRENT y STATE reconciliados |
| Bloqueos y decisiones | Release/push bajo política del operador; binarios reales EXT y adaptador host necesarios para sus perfiles; J7/J8/J9/X08/R11 siguen diferidos; v1.169.123..134 no publicadas (operator-side) |
| Próxima revisión | Al inicio de **cada** sesión y después de cada commit/release relevante |

## Recuperación sin adivinar

1. Confirmar qué rama contiene el nuevo plan, si el PR está integrado y cuál es la versión real de main. Si no está integrado, el puntero de main no ha cambiado.
2. Leer [ROADMAP.md](ROADMAP.md) y [CERTIFICATIONS.md](CERTIFICATIONS.md); localizar el último recibo **observado** del hito activo y los casos [UAT](UAT-MATRIX.md) no ejecutados.
3. Contrastar el último bloque de [SESSION-JOURNAL.md](SESSION-JOURNAL.md) con `git log -5` y el estado operativo; si difieren, registrar reconciliación como **nueva** entrada, sin editar el pasado.
4. Solo entonces abrir/continuar el próximo WorkItem. Un resumen de sesión, un commit de docs o un dry-run no sustituyen un recibo de certificación.
