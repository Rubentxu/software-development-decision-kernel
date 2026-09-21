# CURRENT — puntero de reanudación de SDDK

**Estado:** C1 H01/H02/H05/H06/cycle-c todos integrados en main. **Actualizado:** 2026-09-21T17:12:00Z. Pendiente: full profile C1 como gate de cierre. Este puntero NO acredita que los gates hayan pasado.

| Campo | Valor observado o pendiente |
| --- | --- |
| Fuente de la fotografía | `main@c4c7e0a` (post-PR#9+#10+#11) consultado 2026-09-21T17:12Z; **revalidar al comenzar cada sesión** |
| Workspace en esa fotografía | `1.169.134` (Cargo.toml) |
| Release pública comprobada en esa fotografía | `v1.169.122` (2026-09-20); **no asumir que sigue siendo la última** |
| Hito activo | `C1` — H01 pusheado (74dfcc9+abca553); cycle-c SCOPE-CONTRACT pusheado (68f6788); reorganización documental integrada vía PR #8; H02 vía PR #9 (cfe3766); H05+H06 vía PR #10 (1b3d7f0); cycle-c admisión v2 vía PR #11 (c4c7e0a). Las 3 PR C1 mergeadas sin bumps ceremoniales. |
| Estado PRs abiertos | Ninguno. Las 3 PR C1 están integradas y sus branches eliminados (`--delete-branch`). |
| Reorganización docs/history | COMPLETA. `docs/history/` contiene paquetes legacy, handoffs, proposals, research, cycles, uat-plans y evolutivos. `docs/history/README.md` es el catálogo. Único roadmap ejecutable: `docs/roadmap/ROADMAP.md`. |
| Siguiente acción exacta | **Jcode (AUTO):** ejecutar full profile C1 sobre main@c4c7e0a — `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, shellcheck sobre tests/*.sh scripts/*.sh tests-e2e/tui/run.sh. Si verde, reconciliar punteros y declarar C1 cerrado (sin release — operador decidirá `bash scripts/release.sh` aparte). |
| Evidencia requerida para mover puntero | Recibo C0 firmado/aceptado, SHA nuevo, UAT T01/T02 observados, CURRENT y STATE reconciliados |
| Bloqueos y decisiones | Release/push bajo política del operador; binarios reales EXT y adaptador host necesarios para sus perfiles; J7/J8/J9/X08/R11 siguen diferidos; v1.169.123..134 no publicadas (operator-side) |
| Próxima revisión | Al inicio de **cada** sesión y después de cada commit/release relevante |

## Recuperación sin adivinar

1. Confirmar qué rama contiene el nuevo plan, si el PR está integrado y cuál es la versión real de main. Si no está integrado, el puntero de main no ha cambiado.
2. Leer [ROADMAP.md](ROADMAP.md) y [CERTIFICATIONS.md](CERTIFICATIONS.md); localizar el último recibo **observado** del hito activo y los casos [UAT](UAT-MATRIX.md) no ejecutados.
3. Contrastar el último bloque de [SESSION-JOURNAL.md](SESSION-JOURNAL.md) con `git log -5` y el estado operativo; si difieren, registrar reconciliación como **nueva** entrada, sin editar el pasado.
4. Solo entonces abrir/continuar el próximo WorkItem. Un resumen de sesión, un commit de docs o un dry-run no sustituyen un recibo de certificación.
