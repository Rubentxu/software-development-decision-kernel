---
id: INC-DEBT-029-ATOM-PER-ROW-DATABUSY-UNHANDLED
title: "Los write sites ATOM-PER-ROW propagan DatabaseBusy tras agotar busy_timeout, incumpliendo el contrato de concurrencia"
status: resolved
resolution: execute-with-busy-retry
resolved: 2026-09-28
severity: high
priority: P1
created: 2026-09-28
discovered_by: session-30 (OBSERVED, fallo real del gate de release en el paso 1/14)
resolved_by: session-30 (OBSERVED, mutacion que prueba que el gate puede fallar)
cluster_id: CL-SUBSTRATE
related: [INC-DEBT-021, planning_substrate_contract]
fingerprint: "atom_per_row_sites_propagate_databasebusy_after_busy_timeout"
---

## Qué es

`crates/sddk-storage/src/planning_substrate_contract.md` promete, para los
sitios clasificados **ATOM-PER-ROW**, que "concurrent inserts with distinct ids
each succeed". La implementación no honraba esa promesa.

`Storage::open` fija `busy_timeout(5s)`, y ese valor es un presupuesto **por
intento**, no total. Cuando un escritor competidor retiene el lock de
escritura más de 5s, la conexión perdedora recibe `DatabaseBusy` y el error
crudo se propagaba tal cual.

## Cómo se observó (no inferido)

No fue una lectura de código: fue el **gate de release rejecting el publish**.
`scripts/release.sh` paso 1/14 (`cargo test --workspace`) falló:

```text
test result: FAILED. 3 passed; 2 failed; 0 ignored
    concurrent_insert_decision_records_distinct_ids_all_persist
    concurrent_insert_work_items_same_cycle_serializes_and_count_matches
called `Result::unwrap()` on an `Err` value:
  Database(SqliteFailure(Error { code: DatabaseBusy, extended_code: 5 },
                         Some("database is locked")))
```

Lo que lo hace una deuda y no un evento aislado: **pasa 10/10 en aislamiento**
y falla bajo carga del workspace completo. Es decir, el gate era no
determinista, y un publish podía fallar o pasar según la carga de la máquina.

## Medición que aisló la causa

Sondas temporales (eliminadas tras el diagnóstico) con los mismos ajustes que
`Storage::open`:

```text
PROBE persisted journal_mode = wal
PROBE write while held: ERR after 5.006s: database is locked
PROBE journal_mode pragma while held: None after 39.173µs
PROBE Immediate tx while held: Some("database is locked") after 5.004s
PROBE2 competitor after 11.718ms: OK     # hold corto: busy_timeout lo absorbe
```

Esto descarta dos hipótesis que parecían obvias y eran falsas:

- **No** era que faltara `busy_timeout` (ya estaba, y sí espera).
- **No** era que faltara WAL (el modo persistido era `wal`).
- El hold corto (11.7ms) lo absorbe `busy_timeout`; el hold largo (>5s) lo
  agota. El umbral es el presupuesto, no la configuración.

## Resolución

Nuevo helper `Storage::execute_with_busy_retry` (session-30), hermano de
`with_busy_retry` pero para la clase ATOM-PER-ROW:

- 10 intentos, backoff 20–400ms exponencial **más jitter determinista**
  (`attempt * 37 % 25` ms).
- El jitter es **load-bearing**: los tests de concurrencia sincronizan sus
  escritores con `Barrier`, así que un backoff sin jitter hace que ambos hilos
  choquen otra vez en el mismo calendario.
- Reintenta **solo** en `DatabaseBusy`; cualquier otro error se propaga.
- Aplicado a los 4 sitios ATOM-PER-ROW: `insert_work_item`,
  `insert_decision_record`, `insert_dependency_edge`, `backfill_spine_columns`.

`with_busy_retry` no cubría estos casos: es `&mut self` y envuelve una
transacción IMMEDIATE, mientras que estos sitios son un `execute` suelto con
lock adquirido implícitamente por autocommit.

## Evidencia de mutación (el gate puede fallar)

`tests/planning_atomic_per_row_busy_retry.rs` fuerza la contención de forma
determinista en vez de depender del timing de la suite. Con el presupuesto de
retry forzado a cero:

```text
MUTATION-PROBE decision insert under lock FAILED: database is locked
MUTATION-PROBE work item insert under lock FAILED: database is locked
test result: FAILED. 0 passed; 2 failed
```

Restaurado el retry, ambos verdes. La mutación se revirtió y se verificó por
`diff` contra el fichero previo: idéntico.

### Dos trampas del propio harness (documentadas en el test)

1. Un `UPDATE` que no casa con ninguna fila reporta 0 filas y **nunca** toma el
   lock de escritura: la primera versión del harness era vacua y pasaba en
   verde sin demostrar nada. Se asserta `rows == 1`.
2. `HOLD_MS` debe **superar** `busy_timeout`, o el sujeto simplemente espera
   dentro de SQLite y nunca se lanza `DatabaseBusy`.

## Impacto

- El gate de release (paso 1/14) dejó de ser no determinista en este punto.
- Sin tag, sin release, sin assets parciales: el fallo ocurrió en el paso 1
  de 14, antes de cualquier publicación.

## Deuda que queda (no resuelta aquí)

`update_work_item_status` sigue fuera del retry. No se ha observado que falle
y no se ha medido su contención, así que no se toca a ciegas: un cambio sin
evidencia sería exactamente el error que este documento corrige.
