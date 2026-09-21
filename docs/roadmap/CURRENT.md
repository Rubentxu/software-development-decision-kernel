# CURRENT — puntero de reanudación de SDDK

**Estado:** C1 H01/H02/H05/H06 entregados vía PR (H01 ya en main, H02 y H05+H06 esperando merge). **Actualizado:** 2026-09-21T15:52:00Z. Este puntero NO acredita que los gates hayan pasado ni que exista una release posterior.

| Campo | Valor observado o pendiente |
| --- | --- |
| Fuente de la fotografía | `main@6aac99e` (post-PR#8 merge 544aa11) consultado 2026-09-21T15:52Z; **revalidar al comenzar cada sesión** |
| Workspace en esa fotografía | `1.169.133` (Cargo.toml); bump a 1.169.134 vive en ramas de trabajo (H02, H05+H06) sin mergear |
| Release pública comprobada en esa fotografía | `v1.169.122` (2026-09-20); **no asumir que sigue siendo la última** |
| Hito activo | `C1` — H01 pusheado (74dfcc9+abca553 v1.169.133); cycle-c SCOPE-CONTRACT pusheado (68f6788); reorganización documental integrada vía PR #8 (544aa11+6aac99e). H02 recovery: PR #9 abierto en `c1-h02-recovery` con cherry-pick (f794c1d) + bump 1.169.134 (8e467c3). H05+H06: PR #10 abierto en `c1-h05-seam-isolation` (5309742 + fe84b47 + bump 1.169.134 fc418a7). |
| Estado PRs abiertos | #9 (H02) y #10 (H05+H06), ambos mergeables sin conflictos. Ambos bumps a 1.169.134, mismo parent main@6aac99e. |
| Reorganización docs/history | COMPLETA. `docs/history/` contiene paquetes legacy, handoffs, proposals, research, cycles, uat-plans y evolutivos. `docs/history/README.md` es el catálogo. Único roadmap ejecutable: `docs/roadmap/ROADMAP.md`. |
| Siguiente acción exacta | **Operator-side:** merge PR #9 y PR #10 (el orden no afecta a la versión final porque ambos bumps son a la misma versión). Tras merge, el operator-side reconcilia CURRENT/STATE/JOURNAL con el nuevo SHA. Tras eso, implementar cycle-c (corrección del contrato de push/release en `githooks/pre-push` y `scripts/lib/release_admission.sh`) usando el SCOPE-CONTRACT 68f6788 como diseño aprobado. NO bumpear más hasta `bash scripts/release.sh` desde HEAD post-merge. |
| Evidencia requerida para mover puntero | Recibo C0 firmado/aceptado, SHA nuevo, UAT T01/T02 observados, CURRENT y STATE reconciliados |
| Bloqueos y decisiones | Release/push bajo política del operador; binarios reales EXT y adaptador host necesarios para sus perfiles; J7/J8/J9/X08/R11 siguen diferidos; v1.169.123..128 no publicadas (operator-side) |
| Próxima revisión | Al inicio de **cada** sesión y después de cada commit/release relevante |

## Recuperación sin adivinar

1. Confirmar qué rama contiene el nuevo plan, si el PR está integrado y cuál es la versión real de main. Si no está integrado, el puntero de main no ha cambiado.
2. Leer [ROADMAP.md](ROADMAP.md) y [CERTIFICATIONS.md](CERTIFICATIONS.md); localizar el último recibo **observado** del hito activo y los casos [UAT](UAT-MATRIX.md) no ejecutados.
3. Contrastar el último bloque de [SESSION-JOURNAL.md](SESSION-JOURNAL.md) con `git log -5` y el estado operativo; si difieren, registrar reconciliación como **nueva** entrada, sin editar el pasado.
4. Solo entonces abrir/continuar el próximo WorkItem. Un resumen de sesión, un commit de docs o un dry-run no sustituyen un recibo de certificación.
