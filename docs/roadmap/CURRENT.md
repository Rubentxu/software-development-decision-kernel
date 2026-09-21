# CURRENT — puntero de reanudación de SDDK

**Estado:** READY_C0 (propuesta de documentación pendiente de integración). **Actualizado:** 2026-09-21. Este puntero NO acredita que los gates hayan pasado ni que exista una release posterior.

| Campo | Valor observado o pendiente |
| --- | --- |
| Fuente de la fotografía | `main@2ffff3127e7179b5f3c3104c471c8ad2c7d920ff` consultado 2026-09-21; **revalidar al comenzar cada sesión** |
| Workspace en esa fotografía | `1.169.127` (Cargo.toml) |
| Release pública comprobada en esa fotografía | `v1.169.122` (2026-09-20); **no asumir que sigue siendo la última** |
| Hito activo propuesto | `C0` — reconciliar SHA/tag/release/receipts y congelar casos UAT |
| Último hito histórico | A5-C Base v1.169.88 condicionado; closures A6/A7/A8/J/AIW por alcance documentado; NO declarar certificaciones enhanced/GA a partir de esto |
| Siguiente acción exacta | Leer AGENTS §«Recuperación», `git fetch`, `git status -sb`, `git rev-parse HEAD`, `git tag`, consultar GitHub Releases y revisar recibos; después emitir SCOPE y EVIDENCE para C0 |
| Evidencia requerida para mover puntero | Recibo C0 firmado/aceptado, SHA nuevo, UAT T01/T02 observados, CURRENT y STATE reconciliados |
| Bloqueos y decisiones | Release/push bajo política del operador; binarios reales EXT y adaptador host necesarios para sus perfiles; J7/J8/J9/X08/R11 siguen diferidos |
| Próxima revisión | Al inicio de **cada** sesión y después de cada commit/release relevante |

## Recuperación sin adivinar

1. Confirmar qué rama contiene el nuevo plan, si el PR está integrado y cuál es la versión real de main. Si no está integrado, el puntero de main no ha cambiado.
2. Leer [ROADMAP.md](ROADMAP.md) y [CERTIFICATIONS.md](CERTIFICATIONS.md); localizar el último recibo **observado** del hito activo y los casos [UAT](UAT-MATRIX.md) no ejecutados.
3. Contrastar el último bloque de [SESSION-JOURNAL.md](SESSION-JOURNAL.md) con `git log -5` y el estado operativo; si difieren, registrar reconciliación como **nueva** entrada, sin editar el pasado.
4. Solo entonces abrir/continuar el próximo WorkItem. Un resumen de sesión, un commit de docs o un dry-run no sustituyen un recibo de certificación.
