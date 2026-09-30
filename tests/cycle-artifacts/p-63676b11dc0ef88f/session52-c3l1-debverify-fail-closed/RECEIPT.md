# RECEIPT — session-52: C3l.1 DebVerify fail-closed (AT-UAT-002/003)

- **Fecha (UTC):** 2026-09-30T22:49Z → 2026-10-01T00:15Z
- **Baseline:** `origin/main = 4ad3429b` (v2.4.0 publicada en session-50)
- **Slice:** C3l.1 del paquete acceptance-truthfulness (primera slice de código de la vía C3l)
- **Ledger del operador:** intacto (slice IN_PROCESS; sin sandbox necesario salvo los propios tests)

## Defecto (verificado en código antes de tocar)

`DebVerifyKernel::reconcile` (debverify_kernel/mod.rs): `Err(_) => {}` — errores de estrategia tragados; `strategies_run = applicable.len()` contaba aplicables. Un pass podía terminar `ConfirmedBaseline { strategies_run }` con estrategias fallidas. Exactamente el defecto declarado en C3l.1.

## Resolución (`22459708`, tipada, sin scores ni booleanos ambiguos)

1. `StrategyFailure { strategy_id, reason }` — Serialize; `MissingInput(d)` → `"missing input: {d}"`.
2. `ReconciliationSummary::Incomplete { failures }` — variante nueva del enum cerrado.
3. `ConfirmedBaseline` **y** `AcceptedDebt` inalcanzables con fallos (aceptar deuda con una estrategia fallida ocultaría la deuda que encontraría).
4. Señales reales dominan: Contradiction/EvidenceGap/Staleness salen si se encontraron.
5. `strategies_run` = outcomes Ok (completadas).
6. Consumidores extendidos: `intelligence_loop::reconciliation_digest` (content-addressed sobre fallas en orden canónico) e `intelligence_advisory::reconciliation_tag` (`"incomplete"`). Grep previo: no había matches exhaustivos del summary en producción; los 2 no-exhaustivos fallaron a compile time (fail-loud correcto).

## Evidencia

| Qué | Resultado |
|---|---|
| RED antes del fix | los 6 tests de `c3l1_falsifiers` no compilaban (tipos ausentes: `StrategyFailure`/`Incomplete` — el RED honesto para una feature tipada) |
| GREEN después | `debverify_kernel` **34/34** (28 previos + 6 falsificadores) |
| Suite engine | `sddk-engine --lib` **1358/0** |
| Perfil completo | `cargo test --workspace` **5194/0/19** (+6 exactos), TEST_EXIT=0 |
| fmt / clippy | limpios (`-p sddk-engine --all-targets -D warnings`) |

Los 5 falsificadores declarados por el paquete, todos en `debverify_kernel/tests.rs::c3l1_falsifiers`: (1) una falla ⇒ `Incomplete` con la falla tipada; (2) todas fallan ⇒ `Incomplete` ×3; (3) falla+limpia ⇒ `Incomplete`; (4) falla+contradicción ⇒ `Contradiction` (señal real domina, jamás ConfirmedBaseline); (5) conteo de completadas (2 limpias ⇒ `ConfirmedBaseline { strategies_run: 2 }`; falla ⇒ jamás ConfirmedBaseline).

## Matriz / trazabilidad

- **R6** en ACCEPTANCE-TRUTHFULNESS-MATRIX: `IMPLEMENTED → re-verificable`; el estado definitivo lo fija C3n.2 re-ejecutando los falsificadores (ya viven en el suite: la re-certificación es correr `cargo test -p sddk-engine --lib debverify_kernel` sobre el SHA nuevo).
- AT-UAT-002/003: PASS en UAT-MATRIX con referencia al módulo de tests y al commit.

## Límites declarados

- `ChallengeError` hoy solo tiene `MissingInput`; el match de `reconcile` es exhaustivo, así que un variante nuevo sin tratamiento falla a compile time (correcto por diseño).
- Los señales-dominan (contradicción etc.) no incluyen las `StrategyFailure` en su payload: la falla queda visible en el digest del intelligence_loop y en cualquier receipt que serialice el summary solo si el outcome final es `Incomplete`. Si C3n.2 exige fallas visibles en TODOS los outcomes, eso es un refinamiento del payload (no bloqueante para el invariante C3l.1, que solo exige ≠ ConfirmedBaseline).
- Release fix→PATCH **2.4.1** tras perfil completo (addendum al cierre).

## Commits

- `22459708` fix(engine): DebVerify reconcile respeta ChallengeError
- _(docs + bump: se consignan al cerrar)_

## ADDENDUM — release v2.4.1 PUBLICADA (misma sesión)

- **SemVer derivado:** fix → PATCH → **2.4.1** (release-bump.sh). Perfil completo antes del commit de docs: **5194/0/19** (+6 exactos = los falsificadores).
- **Flujo local 0–8b OK** (ACCEPT 2.4.0→2.4.1, binario musl static-pie verificado BuildID 90bdfbe8, manifest 377 ficheros, bundle+unified+sbom, vault mirror 51 skipped/0 created). Parada en 8c **por diseño** (INC-DEBT-024). Nota de incidente menor: la primera corrida del flujo perdió su log en /tmp (exit 1 ambiguo); la re-ejecución en foreground con tee mostró que el estado era el 8c esperado — segunda vez que un artefacto de medición desaparece bajo esta sesión; el control aplicado fue re-ejecutar observando en vivo.
- **CI:** run **36778476542 completed success** (13/13 jobs). Tag anotado objeto `2292991a`, peel `c7cef2e7` == origin/main (push → sync → tag → CI).
- **Release:** isDraft=false, isPrerelease=false, publishedAt 2026-09-30T21:25:13Z, **27 assets**.
- **9b OBSERVED:** 27/27 HTTP 200; gate PASS=13 FAIL=0; anclaje ls-remote verificado.
- **9c OBSERVED:** sha CDN `99657fa551e92833…` == declarado; cosign **Verified OK** identity `release.yml@refs/tags/v2.4.1`.
- **10–12 OBSERVED:** install.sh exit 0; `sddk 2.4.1`; current → 2.4.1; doctor **all_present: true**; prune removed 2.4.0 kept 2.4.1.
