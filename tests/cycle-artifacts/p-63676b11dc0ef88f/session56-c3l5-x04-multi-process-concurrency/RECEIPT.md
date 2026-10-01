# RECEIPT — C3l.5 X04: concurrencia real multi-proceso

**Slice:** `session56-c3l5-x04-multi-process-concurrency`
**Fecha:** 2026-10-01 · **Baseline:** `ccf7ada9` (7 commits sin publicar en el momento del cierre)
**Workflow:** `A-lite` · **Commit de la slice:** `6abcf052`
**SCOPE-CONTRACT:** [`SCOPE-CONTRACT.md`](SCOPE-CONTRACT.md)
**boundary_class:** `PROCESS / SQLITE_DURABLE` (antes: `IN_PROCESS`)

---

## §1 El defecto, y su causa raíz

El exit gate del paquete —*"al menos dos PIDs distintos y SQLite durable compartido"*— era **inalcanzable por construcción**, no por accidente:

```
$ grep -rn 'impl LeaseStore' --include=*.rs crates/
crates/sddk-engine/src/agent_host.rs:114:impl LeaseStore for InMemoryLeaseStore {
```

**Una** implementación, en memoria. Y el doc del propio trait (agent_host.rs:89) declaraba una intención inexistente: *"Real implementations forward to `sddk_storage::Ledger`"*.

**Causa raíz:** `LeaseStore` era un **puerto que vivía en `sddk-engine`**, mientras el patrón canónico del repo es puerto en `sddk-domain/src/ports.rs` + implementación en `sddk-storage` — exactamente lo que hace `Ledger`. Un puerto en el engine con una única impl en memoria es la razón estructural por la que X04 no podía verificarse. Por eso la solución no fue "añadir un test más fuerte" sino **mover el puerto** (§2.7: una autoridad canónica por concepto).

## §2 GREEN: los 8 puntos del exit gate

| Gate | Test | Resultado |
|---|---|---|
| G1 winner único | `g1_two_real_processes_race_exactly_one_wins_and_loser_does_not_mutate` | ok |
| G1 PID real | `g1_child_is_genuinely_a_second_pid` | ok |
| G2 fencing monotónico | `g2_fencing_is_monotonic_across_reopen` | ok |
| G3 loser no muta / release CAS | `g3_stale_token_cannot_release_the_current_owners_lease` | ok |
| G4 crash del winner | `g4_crashed_winner_holds_the_lease_until_expiry` | ok |
| G5 expiry/reacquire | `g5_expired_lease_can_be_reacquired_with_a_greater_token` | ok |
| G7 sin lock leaking | `g7_concurrent_processes_never_surface_database_locked` (6 procesos) | ok |
| G8 sin divergencia | `g8_durable_state_is_identical_to_what_the_winner_believes` | ok |

**8 passed / 0 failed / 1 ignored.** El ignored es el cuerpo del hijo, que solo se ejecuta cuando se lanza explícitamente. El reloj es **real**: `MockClock` haría G5 vacuo. El PID se **afirma**, no se asume: si el hijo compartiera PID, `g1_child_is_genuinely_a_second_pid` falla.

## §3 Defecto real encontrado por el test nuevo

**El test encontró un bug de fencing que el test antiguo era estructuralmente incapaz de encontrar.**

`SqliteLeaseStore::release` hacía `DELETE FROM agent_leases`, destruyendo el contador. Tras una release, el siguiente `acquire` reemitía **token 1** — que ese ciclo ya había usado. Un holder obsoleto de token 1 habría pasado por vigente: exactamente el agujero que fencing debe cerrar.

```
panicked at: a new owner must get a strictly greater token (2 -> 1)
```

**Corrección:** `release` pasa a `UPDATE owner='', expires_at_ms=0` conservando fila y token. El fencing es monotono **por ciclo**, no por lease. `g3` pinea ambas caras: la release limpia el holder **y** el contador sobrevive.

## §4 Falsificadores OBSERVED

| F | Mutación | Resultado | Veredicto |
|---|---|---|---|
| F6 | `release` ignora el `fencing_token` | **1 FAIL** (G3) | load-bearing |
| F8 | `acquire` acepta un holder vivo | **5 FAIL** (G1,G2,G4,G7,G8) | load-bearing |
| **F9** | **cada conexión abre una BD en memoria** | **8 FAIL (todas)** | **decisivo: sin estado durable, toda afirmación multi-proceso colapsa** |
| F7 | sin escalera de retry (`MAX_RETRIES = 0`) | **0 FAIL** | **NO load-bearing — ver §5** |

Restaurado tras cada mutación; `git diff` vacío = byte-idéntico al commit.

## §5 Límite declarado: F7 no mordió

Quitar la escalera de retry **no rompe ningún test**. Con 6 procesos concurrentes en esta máquina, el `busy_timeout` de SQLite (5 s) absorbe la contención por sí solo, así que **G7 no aísla la contribución del retry**.

Lo que G7 **sí** demuestra: a 6 vías, exactamente uno gana y `database is locked` no llega al llamante. Lo que **no** demuestra: que el retry sea necesario a ese nivel.

La escalera está heredada de INC-DEBT-029, cuyo test la hace load-bearing forzando un lock de 7 s (superior al `busy_timeout`). Replicar ese patrón aquí lo haría falsable; **no se hizo en esta slice** y queda declarado, no disfrazado de verde.

## §6 Errores propios

1. **Premisa mala en G2, primera redacción:** afirmaba que un owner distinto podía tomar la lease mientras A seguía vivo. El store la refusó correctamente y el test falló — sobre su propia premisa, no sobre un defecto. Reescrito como: A viva bloquea a B; tras release, B entra con token mayor.
2. **Dos errores de compilación** (`Option<&str>.to_string()`, uso tras `move`) y un `unused_mut`. Ninguno del store.

## §7 Lo que este slice NO hace

1. **Verifica X04; no certifica "dos CLIs de SDDK".** Los procesos son el binario de test re-ejecutado, no `sddk` como dos invocaciones de producción. Es la diferencia entre `PROCESS` y `PRODUCTION_CLI`.
2. **`AIW-S8` no pasa a VERIFIED.** X07 (segundo binario real) sigue abierto y es C3l.6.
3. **Mover el puerto toca la firma pública de `sddk-engine`.** La re-export preserva compatibilidad; la suite completa del workspace es la que verifica que nada roto.
4. **La tabla `agent_leases` es nueva y propia.** No reutiliza `cycle_leases` (que pertenece al ciclo y tiene FK a `cycles`): son dos ciclo de vida distintos — un lease de agente puede existir para un ciclo que aún no está en el ledger.

## §8 Siguiente paso

**C3l.6** — X07: segundo binario real. Después **C3l.7** (architecture gate), que cierra la vía C3l y desbloquea C3n.
