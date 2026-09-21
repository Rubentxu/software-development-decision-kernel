# CURRENT — puntero de reanudación de SDDK

**Estado:** IN_PROGRESS_C0_PENDING → C1 activo. **Actualizado:** 2026-09-21T15:13:00Z. Este puntero NO acredita que los gates hayan pasado ni que exista una release posterior.

| Campo | Valor observado o pendiente |
| --- | --- |
| Fuente de la fotografía | `main@544aa1186ad0894c1e50b222684330438cddf784` (Merge PR #8) consultado 2026-09-21T15:13Z; **revalidar al comenzar cada sesión** |
| Workspace en esa fotografía | `1.169.133` (Cargo.toml) |
| Release pública comprobada en esa fotografía | `v1.169.122` (2026-09-20); **no asumir que sigue siendo la última** |
| Hito activo | `C1` — H01 pusheado (74dfcc9+abca553 v1.169.133); cycle-c SCOPE-CONTRACT pusheado (68f6788); reorganización documental integrada vía PR #8 (merge commit 544aa11 = 8d785c4 + deaae8e + 28eb948). Pendiente: implementar ciclo-c (fix push/release) + recuperar H02 reflog + H05 + H06. |
| Estado PR #8 | INTEGRADO en `544aa11` (merge commit). PR #7 cerrado previo (13d4131 + 96f5366). |
| Reorganización docs/history | COMPLETA. `docs/history/` contiene paquetes legacy, handoffs, proposals, research, cycles, uat-plans y evolutivos. `docs/history/README.md` es el catálogo. Único roadmap ejecutable: `docs/roadmap/ROADMAP.md`. |
| Siguiente acción exacta | **Implementar cycle-c** (corrección del contrato de push/release en rama de trabajo). Plan: tests RED→GREEN del nuevo contrato (last_published_version + range-based allowlist), luego integrar vía PR o merge fast-forward. H02 se recupera en el reflog (commit 9d4c249) tras tener vía de integración limpia. NO bumpear, NO publicar, NO forzar push. |
| Evidencia requerida para mover puntero | Recibo C0 firmado/aceptado, SHA nuevo, UAT T01/T02 observados, CURRENT y STATE reconciliados |
| Bloqueos y decisiones | Release/push bajo política del operador; binarios reales EXT y adaptador host necesarios para sus perfiles; J7/J8/J9/X08/R11 siguen diferidos; v1.169.123..128 no publicadas (operator-side) |
| Próxima revisión | Al inicio de **cada** sesión y después de cada commit/release relevante |

## Recuperación sin adivinar

1. Confirmar qué rama contiene el nuevo plan, si el PR está integrado y cuál es la versión real de main. Si no está integrado, el puntero de main no ha cambiado.
2. Leer [ROADMAP.md](ROADMAP.md) y [CERTIFICATIONS.md](CERTIFICATIONS.md); localizar el último recibo **observado** del hito activo y los casos [UAT](UAT-MATRIX.md) no ejecutados.
3. Contrastar el último bloque de [SESSION-JOURNAL.md](SESSION-JOURNAL.md) con `git log -5` y el estado operativo; si difieren, registrar reconciliación como **nueva** entrada, sin editar el pasado.
4. Solo entonces abrir/continuar el próximo WorkItem. Un resumen de sesión, un commit de docs o un dry-run no sustituyen un recibo de certificación.
