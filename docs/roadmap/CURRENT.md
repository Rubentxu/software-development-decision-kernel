# CURRENT — puntero de reanudación de SDDK

**Estado:** C1 cerrado. Full profile verde sobre main@a1f0fa0. **Actualizado:** 2026-09-21T17:42:00Z. Este puntero NO acredita que se haya publicado la release (operador-side).

| Campo | Valor observado o pendiente |
| --- | --- |
| Fuente de la fotografía | `main@a1f0fa0` (post-manifest-regen) consultado 2026-09-21T17:42Z; **revalidar al comenzar cada sesión** |
| Workspace en esa fotografía | `1.169.134` (Cargo.toml) |
| Release pública comprobada en esa fotografía | `v1.169.122` (2026-09-20); **no asumir que sigue siendo la última** |
| Hito activo | `C1 CERRADO` — H01 (abca553), H02 (PR #9 cfe3766), H05+H06 (PR #10 1b3d7f0), cycle-c admisión v2 (PR #11 c4c7e0a) todos integrados en main. Full profile ejecutado sobre a1f0fa0: 4989 tests passed / 0 failed, fmt+clippy+manifest OK. Única acción post-merge: regeneración MANIFEST.sha256 (a1f0fa0) por inconsistencia preexistente de PR #8. |
| Estado PRs abiertos | Ninguno. Las 3 PR C1 están integradas y sus branches eliminados. |
| Reorganización docs/history | COMPLETA. `docs/history/` contiene paquetes legacy, handoffs, proposals, research, cycles, uat-plans y evolutivos. `docs/history/README.md` es el catálogo. Único roadmap ejecutable: `docs/roadmap/ROADMAP.md`. |
| Siguiente acción exacta | **Operator-side (NO jcode):** `bash scripts/release.sh` desde main@a1f0fa0 para publicar v1.169.134 (binario + bundle + GH release). Si quiere abrir C2, primero publicar C1; si no, mantener congelado. |
| Evidencia requerida para mover puntero | Recibo C0 firmado/aceptado, SHA nuevo, UAT T01/T02 observados, CURRENT y STATE reconciliados |
| Bloqueos y decisiones | Release/push bajo política del operador; binarios reales EXT y adaptador host necesarios para sus perfiles; J7/J8/J9/X08/R11 siguen diferidos; v1.169.123..134 no publicadas (operator-side). Deuda menor: PR #8 modificó assets/ sin regenerar manifest — amortizado en cierre C1 (a1f0fa0). |
| Próxima revisión | Al inicio de **cada** sesión y después de cada commit/release relevante |

## Recuperación sin adivinar

1. Confirmar qué rama contiene el nuevo plan, si el PR está integrado y cuál es la versión real de main. Si no está integrado, el puntero de main no ha cambiado.
2. Leer [ROADMAP.md](ROADMAP.md) y [CERTIFICATIONS.md](CERTIFICATIONS.md); localizar el último recibo **observado** del hito activo y los casos [UAT](UAT-MATRIX.md) no ejecutados.
3. Contrastar el último bloque de [SESSION-JOURNAL.md](SESSION-JOURNAL.md) con `git log -5` y el estado operativo; si difieren, registrar reconciliación como **nueva** entrada, sin editar el pasado.
4. Solo entonces abrir/continuar el próximo WorkItem. Un resumen de sesión, un commit de docs o un dry-run no sustituyen un recibo de certificación.
