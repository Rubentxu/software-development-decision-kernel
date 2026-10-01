---
id: INC-DEBT-055-UAT-VERDICT-READY-OVER-AN-EMPTY-SESSION
title: "El veredicto UAT salía READY sobre una sesión que no había ejecutado nada"
severity: high
priority: P1
status: resolved
opened: session-65f
resolved: session-65f
resolution: Ruta 1 (dejar de fabricar el veredicto, no bajar el gate)
component: sddk-domain, sddk-cli
surface: crates/sddk-domain/src/uat.rs, crates/sddk-cli/src/uat.rs, crates/sddk-cli/src/uat_serve.rs
---

# INC-DEBT-055: el veredicto UAT salía READY sobre una sesión vacía

> **RESUELTA (session-65f).** La regla del veredicto de sesión estaba escrita
> **tres veces**. Las dos copias que no tienen plan contra el que cruzar
> contaban `Fail`/`Blocked`/`NotRun` sobre `results`: con la lista vacía salían
> tres ceros y caían en el `else` → **`READY`**. Una sesión donde no se ejecutó
> nada se reportaba como lista.

## Criterio verificable

RED medido, antes del cambio, sobre los tests nuevos:

```
test uat_serve::tests::una_sesion_sin_escenarios_no_es_ready ... FAILED
  assertion `left == right` failed: cero escenarios ejecutados no puede producir un veredicto afirmativo
    left: "READY"
   right: "NOT_READY"

test uat_serve::tests::una_sesion_de_escenarios_parciales_no_es_ready ... FAILED
    left: "READY"
   right: "NOT_READY"
```

## Las tres copias, y por qué dos fallaban

| copia | sitio | cuenta `Partial` | `results` vacío |
|---|---|---|---|
| `aggregate_report` | `uat.rs:2380` | **sí** | n/d — cruza contra el plan |
| upsert del control plane | `uat.rs:1204` | no | **`READY`** |
| respuesta HTTP `/ingest` | `uat_serve.rs:269` | no | **`READY`** |

Las dos que fallan son exactamente las que **no tienen plan** contra el que
cruzar. El plan es lo que hacía el caso vacío inalcanzable en la copia buena:
`missing_scenario_is_not_run_and_never_ready` ya fijaba que un escenario
ausente es `NotRun` y nunca `Ready`. Sin plan, nada lo tapa.

## El alcance: el guard de integridad no cubría esta ruta

`process_session_for_ingest` ya rechaza una sesión `executor: human`
fabricada — exige `executed_by` + `finished_at` + evidencia o un estado no-PASS
(`uat.rs:1152`). Ese guard es **`if session.executor == UatExecutor::Human`**.
Una sesión `executor: fara` con `results: []` no entra en él, y por tanto:

1. se acepta la sesión,
2. se persiste como `READY` en el control plane,
3. y `total = session.results.len().max(1)` (línea 1200) **enmascara el vacío en
   el denominador de cobertura**: `coverage_pct` se calcula sobre 1 en lugar de
   sobre 0, así que la fila persistida dice `READY` con la cobertura de una
   sesión que no corrió nada.

La respuesta HTTP lo hace explícito sin querer: `"verdict":"READY","results":0`
en el mismo cuerpo.

## Por qué es peor que los otros dos del género

INC-DEBT-053 (`verify-chain` PASS sobre cero eventos) e INC-DEBT-054
(`doctor --strict` exit 0 sin medir) contestaban **`PASS` / exit 0** — un
veredicto de *integridad*. Este contesta **`READY`**, que es una afirmación
positiva de **aptitud para publicar**, y es lo que consume quien decide. Un
`PASS` sobre nada es un dato que falta; un `READY` sobre nada es una decisión
tomada con información que no existe.

## Resolución

Una sola autoridad, en el dominio, y las tres copias delegan:

- `UatVerdict::from_counts(failed, not_run, blocked, partial)` — **la regla que
  `aggregate_report` ya aplicaba**, que era la correcta de las tres porque-era
  la única que contaba `Partial`. Sin cambios de comportamiento donde ya se
  usaba.
- `UatVerdict::from_results(&[UatScenarioResult])` — delega en la anterior y
  añade **una sola** cosa: `results` vacío → `NotReady`. Es la traducción de la
  regla al caso sin plan, y es la misma negativa que el guard de `human`
  ya aplicaba, extendida a la ruta del agente.
- `UatVerdict::as_str()` — una sola grafía para `READY` / `READY_WITH_RISKS` /
  `NOT_READY`, la misma que publica `agents/uat-reporter.md`.

## Lo que NO se decidió aquí: la clase de `Partial`

`Partial` es de primera clase — `uat.rs` lo trata en cinco sitios y
`agents/uat-reporter.md` lo exige en el `summary` — pero **ADR-012 §6, la
definición que `uat-reporter.md` cita, no menciona partial**: define
`READY_WITH_RISKS` como *"blockers only (not failed), or failures in P1/P2 with
documented workarounds"*. `Partial` no es un blocker ni un fallo.

Las tres copias discrepaban y **ninguna tenía el contrato detrás**:

- `aggregate_report` (la que produce el `uat-report.yaml` publicado) lo trataba
  como **riesgo** → `READY_WITH_RISKS`.
- las dos copias sin plan ni lo contaban → `READY`.

El arreglo **no elige**: adopta la clasificación de la autoridad previa
(riesgo) y elimina la tercera respuesta, que fue la que introduje por primera vez
al escribir el test — un `NotReady` que ninguna autoridad previa sostenía y que
habría hecho divergir el reporte publicado respecto a la fila del control plane
en el caso opuesto. **Si `Partial` debe ser riesgo o bloqueo es una decisión de
contrato** y queda registrada, no resuelta por el agente que la encontró.

## Consecuencia aceptada

Una sesión `executor: fara` sin resultados ya no se persiste como `READY`: se
persiste `NOT_READY`. Cualquier pipeline que emitiera sesiones vacías para
"adelantar" el estado de un release aparece como bloqueado, que es lo correcto.

## Falsificadores

Cuatro mutaciones, una por regla, ejecutadas sobre el árbol y revertidas. Un
test que sobrevive a la eliminación de la regla que dice comprobar no está
midiendo.

| # | mutación | dientes que mueren | dientes que sobreviven |
|---|---|---|---|
| M1 | quitar el `results.is_empty() → NotReady` | `sin_escenarios_no_es_ready` | los otros 3 del dominio |
| M2 | quitar `partial` de la clase riesgo | `la_precedencia_por_contadores_se_conserva`, `from_results_delega_en_from_counts` | vacío, grafía |
| M3 | quitar `not_run` de la clase bloqueo | `la_precedencia_por_contadores_se_conserva`, `from_results_delega_en_from_counts` | vacío, grafía |
| M4 | `from_results` devuelve siempre `NotReady` | `from_results_delega_en_from_counts` y, en el CLI, **`una_sesion_con_todo_pass_sigue_siendo_ready`**, **`una_sesion_bloqueada_sigue_siendo_ready_with_risks`**, **`una_sesion_de_escenarios_parciales_es_ready_with_risks`** | `sin_escenarios_no_es_ready`, grafía |

M4 es la que importa: mata los **tres** dientes positivos del CLI, luego el
arreglo no se puede cerrar degradando los casos buenos. Sin ellos, un
`return Self::NotReady` al principio de `from_results` habría pasado la
verificación.

**El falsificador falló su propia primera ejecución, y es la cuarta vez que
ocurre.** La sonda del script hacía `sed 's/ \.\.\..*//'`, que **borraba el
sufijo `... ok` / `... FAILED`**: imprimía la lista de nombres de test y nada
más. Con las cuatro mutaciones aplicadas, la salida era idéntica a la del árbol
sano, y «los ocho tests aparecen» se lee como «los ocho tests pasaron» si no se
mira el código de la sonda. **Es el mismo género que esta INC, en el
instrumento que la verifica**: una comprobación que reporta sin examinar el
dato que dice examinar. Se detectó porque el resultado fue *demasiado
bonito* — cuatro mutaciones y cero dientes muertos es un resultado que exige
duda antes que crédito. Corregida la sonda para no recortar el estado, y
re-ejecutada desde cero; los resultados de la tabla son los de la segunda
ejecución.

Precedentes del mismo patrón: el guard de `test_debt_index_coherence.sh`
encontró dos bugs en sí mismo en session-43; el falsificador F53 de
INC-DEBT-050 estaba mal diseñado y se sustituyó **antes** de ejecutarlo
(`to_lowercase` → `to_ascii_lowercase` no cambia el resultado en ASCII, luego el
pin no habría fallado y la prueba habría sido inútil); y el caso de fail-closed
de `test_push_prevention_hook.sh` medía otro repositorio porque la invocación
vivía en un `if (...)` ya cerrado.
