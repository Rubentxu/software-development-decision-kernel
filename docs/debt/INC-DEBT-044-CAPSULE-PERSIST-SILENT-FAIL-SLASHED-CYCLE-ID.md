---
id: INC-DEBT-044
title: la capsule de un ciclo real (id con barra) nunca se escribe a disco y el bootstrap lo reporta como compiled
severity: high
priority: P1
status: resolved
detected_at: 2026-09-30
detected_in_session: session-50
closed_in_session: session-50
component: engine
surface: crates/sddk-engine/src/durable_capsule_store.rs
references:
  - crates/sddk-cli/src/context_cmd.rs
  - docs/architecture/adrs/ADR-0147-FRONTIER-SEMANTICS-AND-CYCLE-CAPSULE-INPUTS.md
  - docs/debt/INC-DEBT-042-CONTEXT-BOOTSTRAP-COMPLETE-WITHOUT-CAPSULE.md
  - tests/uat_ctx_007_context_expand.sh
---

# INC-DEBT-044: `persist` tragaba en silencio la escritura de la capsule de todo ciclo real

## Criterio verificable (y se sostenía antes del fix)

Los ids de ciclo del runtime son SIEMPRE `p-<hex>/<name>` — llevan barra. El
store durable de capsules construye el nombre de fichero como
`<workflow_run>:<node>:<attempt>.json` **sin codificar**, así que para un
ciclo real el nombre era `cycle-p-<hex>/<name>:bootstrap:cold-start.json`: la
barra convierte la ruta en un subdirectorio inexistente, `fs::write` falla, y
`persist` **traga el error y retorna** (los tres fallos posibles hacen
`return` sin reportar nada).

Observado end-to-end contra el binario release, en ledger aislado (session-50):

```console
$ sddk context bootstrap --root "$WT" --session dbg2 --cycle "$CID" --format json
context_source = "compiled"          # afirma haber compilado
basis_revision = "cycle-p-…:bootstrap:cold-start"   # apunta a la capsule

$ find "$XDG_DATA_HOME/sddk/projects/$PID/context/capsules" -type f
(ningún fichero — el directorio está VACÍO)
```

**La salida afirma una persistencia que no ocurrió.** Es el defecto de la
familia INC-DEBT-039/042/043 («la salida dice algo que no se hizo»), esta vez
en la capa de almacenamiento: `compiled` era verdad en memoria y falso en
disco.

## Por qué nadie lo había visto

**Todos los tests de capsule usaban ids de ciclo sin barra**
(`c-ctx-cycle-under-test`, `cycle-explicit`, `run-1`): con esos ids el nombre
de fichero es plano y el round-trip funciona. El primer test que usó la forma
real del id fue el UAT end-to-end de `context expand` — y lo destapó en la
primera corrida. Es la misma lección de INC-DEBT-043: **un fixture que no usa
la forma real del dato no puede distinguir éxito de ficción**.

## Alcance real del defecto

- Todo `context bootstrap` sobre un ciclo real recompilaba la capsule en cada
  invocación y **nunca la persistía**: `context_source: compiled` honesto por
  casualidad (recompilar era lo que hacía), pero `basis_revision` apuntaba a
  una capsule inexistente y **CTX-UAT-006** (proceso A compila, proceso B
  recupera la misma basis desde storage) era imposible de cumplir con ids
  reales.
- `context expand` (C3j objetivo 4) no podía funcionar: su paso 2 lee la
  capsule del store.
- Los deltas no estaban afectados (leen el binding, no el store de capsules).

## Resolución

`file_name_for` ahora **percent-encodea** los tres componentes
(`%`→`%25`, `/`→`%2F`, `:`→`%3A`) antes de unirlos; `last_capsule` y
`last_capsule_for_node` comparan codificado contra codificado, así que el
round-trip es exacto y el nombre en disco queda libre de separadores
ambiguos. Los nombres sin barra conservan su representación exacta — **no hay
migración**: los nombres defectuosos con barra jamás llegaron a disco porque
la escritura fallaba.

## Evidencia

- **RED observado primero** (test nuevo
  `capsule_persists_and_recovers_with_slashed_runtime_run`): persist + reopen
  con `workflow_run = "cycle-p-c3ade1d7c35774cc/dbg-cycle"` →
  `last_capsule` devolvía `None` (panicked en el expect).
- **Verde después**: `cargo test -p sddk-engine --lib durable_capsule_store`
  **5/5**; `cargo test -p sddk-engine --lib` **1352 passed / 0 failed**;
  fmt y clippy `-D warnings` limpios.
- **End-to-end**: `tests/uat_ctx_007_context_expand.sh` pasa de 6 FAIL (capsule
  inexistente para el ciclo real) a PASS tras el fix.

## Límite residual declarado

`CapsuleStore::persist` sigue siendo fire-and-forget (la firma del trait no
devuelve `Result`), así que un fallo de escritura por OTRAS causas (permisos,
disco lleno) seguiría sin reportarse en el bootstrap. Cerrar eso exige tocar
el trait y sus dos implementaciones — trabajo propio, que queda fuera de este
cierre por alcance. El modo de fallo conocido y realista (ids con barra) está
eliminado de raíz; el resto queda declarado, no oculto.
