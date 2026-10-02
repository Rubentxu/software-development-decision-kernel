# SCOPE-CONTRACT — cl-cycle-enumeration

**Cycle:** `p-63676b11dc0ef88f/cycle-enumeration`
**Date:** 2026-10-02T18:40:00Z
**Authority:** [INC-DEBT-060](../../../debt/INC-DEBT-060-NO-SURFACE-ENUMERATES-CYCLES-97-OF-179-ARE-NAMED-BY-NO-COMMAND.md) (`high`/`P1`, `open`)
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)

---

## §0 — La medición que abre el lote

> ### ⚠️ Enmienda session-69d — §0 estaba mal y la tabla no se conserva como cierta
>
> La tabla de abajo es lo que se midió **al abrir** el lote, y su línea de
> «filas en la tabla `cycles`: 179» es la que el falsificador de R6 desmintió.
> Las dos primeras filas están mal; las demás son correctas pero están contadas
> sobre una población equivocada. Se conservan porque son la evidencia del
> punto de partida, y esta enmienda va delante para que no se lean sin ella.
>
> **179 no es el número de ciclos de este proyecto.** La tabla `cycles` de ese
> ledger contiene dos poblaciones:
>
> | `project_id` | filas | ids | `manifest_json` | eventos |
> |---|---|---|---|---|
> | `p-63676b11dc0ef88f` | **100** | `<project_id>/<slug>` | 98 legibles, 2 ilegibles | 77 con ≥1 evento, **23 sin ninguno** |
> | `__spine_import__` | **79** | slug desnudo | las 79 con `{}` | **0** |
>
> `__spine_import__` es una **fila real de la tabla `projects`** del mismo ledger
> (`display_name: "Spine Import Project"`, workspace `spine-import`), no un
> marcador. Medido en `04-spine-import.py`, read-only.
>
> **Consecuencia sobre este SCOPE:** las 81 filas ilegibles que D2 daba por
> «de este proyecto» son 2 de este proyecto + 79 de `__spine_import__`. D2
> **sigue siendo un defecto real** —`get_cycle` devuelve error, no registro, y
> hay 2 filas de este proyecto que no se pueden leer— pero es **79 veces menor**
> de lo que §0 afirmaba. §2.2 y STOP 1 no cambian: siguen sin tocarse esas filas.

Todo lo que sigue sale de leer el almacenamiento real de
`p-63676b11dc0ef88f` y ejecutar las superficies que el producto ofrece.

| Medición | Valor (al abrir el lote) |
|---|---|
| Filas en la tabla `cycles` | ~~**179**~~ → **100** de este proyecto (+ 79 de `__spine_import__`) |
| Ciclos que nombra alguna superficie del producto | ~~**82**~~ → **77** |
| Ciclos que no nombra ninguna | ~~**97**~~ → **23** (**17** con `status: OPEN`) |
| Ciclos cuyo `manifest_json` **no deserializa** en `CycleManifest` | ~~**81**~~ → **2** de este proyecto |
| — de ellos, con `manifest_json = {}` | ~~79~~ → 0 (las 79 con `{}` son de `__spine_import__`) |
| — con `manifest_json = {"reason", "notes"}` | 2 (`CLOSED`, docs-only, 2026-09-09) |
| Superficie de enumeración | **no existe** |

Los ~~81~~ **2** de D2 y los ~~97~~ **23** de D1 (ver enmienda arriba) son
**defectos distintos y ambos son de lectura**:

- **D1, enumeración.** No hay `list_cycles` en `crates/sddk-storage/src/lib.rs`
  (0 coincidencias) ni `sddk cycle list` en la CLI. Solo `get_cycle(cycle_id)`, que
  exige saber el id, y nada lo da. Un ciclo que el ledger afirma `OPEN` y que
  ningún comando nombra no es un ciclo abierto.
- **D2, legibilidad.** `get_cycle` deserializa `manifest_json` en `CycleManifest`
  (`cycle_from_row`, `lib.rs:1893`), y `CycleManifest` exige once campos sin
  `#[serde(default)]` (`crates/sddk-domain/src/cycle.rs:182-210`). Un `{}` falla
  con *missing field `schema_version`*, y `json_from_sql_error` (`lib.rs:1994`) lo
  convierte en `FromSqlConversionFailure`, que `?` propaga. **Para esas filas la
  API de storage devuelve un error, no un registro** — medido: **2 filas de este
  proyecto**, más 79 de `__spine_import__` que §2.2 también prohíbe tocar.

D2 es más grave que D1 y no estaba escrito en INC-DEBT-060: no es que no se puedan
nombrar, es que **no se pueden leer**. Es la razón por la que el lote 1 empieza
por probarlo y no por implementarlo.

Comprobado y **descartado** por el camino, para no arrastrar una sospecha:

- `CycleStatus` usa `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]`
  (`cycle.rs:13`), luego `"PAUSED"` ↔ `Paused` **sí** round-tripa. El almacenamiento
  y el dominio no discrepan ahí.
- `sddk cycle status` **no** es un falso negativo: infiere el ciclo actual por
  lease vivo (`cycle.rs:341`) y los 30 leases del proyecto están caducados.

## §1 — Objetivo, falsable

1. **O1.** Existe `Storage::list_cycles(project_id, filtro_de_estado_opcional)` que
   devuelve **todas** las filas de `cycles` de ese proyecto, incluidas las que no
   tienen eventos, y **sin** dejar ninguna fuera por no deserializar su manifiesto.
2. **O2.** `sddk cycle list` expone esa enumeración y su recuento **coincide** con
   el de la tabla. Coincidir, no aproximarse.
3. **O3.** La salida **declara** cuántas filas son de manifiesto ilegible, en lugar
   de ocultarlas o de abortar la lista entera.
4. **O4.** Un ciclo que no tiene eventos **aparece** en la lista. La enumeración no
   se construye sobre `events_v1`, que es exactamente el error que produce los 97.

## §2 — No-objetivos

Se escriben porque un alcance que solo dice lo que hace es un alcance que se
amplia en caliente.

1. **NO** se cambia el contrato de `get_cycle`. Sigue devolviendo error ante un
   manifiesto ilegible: eso es una decisión de otro SCOPE (§4, STOP 1).
2. **NO** se limpia, migra ni borra ninguna fila de `cycles`: ni las 100 de este
   proyecto ni las 79 de `__spine_import__`. Qué son las ilegibles es
   **decisión del operador** y está bloqueada desde la apertura de INC-DEBT-060.
   (Redactado con las cifras originales, 179 y 81; véase la enmienda de §0.)
3. **NO** se toca `sddk ledger events` ni su truncamiento por defecto (F63). Es
   otra superficie y otro defecto; meterlo aquí haría el lote más grande y sus
   falsificadores menos claros. Queda como slice propio.
4. **NO** se repara la deuda de procedencia de los 79 imports. Este lote solo hace
   que sean **visibles**.
5. **NO** se escribe en el almacenamiento real de esta máquina. Ni una fila.

## §3 — Lote 1: qué se escribe antes de tocar producción

> **Enmienda del lote 1, escrita antes de escribir un solo test.** El borrador de
> esta sección decía R4 "RED", y era un error: **un test que afirma el
> comportamiento actual pasa hoy**, y llamarlo RED no lo hace caer. R4 es un test
> de **caracterización**. Además, R3 no se puede escribir todavía, y la razón es
> técnica y no de gusto: llama a `list_cycles`, que no existe, luego el fichero no
> compila y **se cairían también los tests que ya estaban verdes**. Es exactamente
> STOP 2. R3 se escribe en el lote 2, el mismo día que exista la función.

| # | Test | Tipo | Por qué |
|---|---|---|---|
| **R4** | `get_cycle` sobre un ciclo con `manifest_json = {}` ⇒ **error**, y el mensaje nombra el campo que falta | **caracterización: pasa hoy** | fija D2 **antes** de que la enumeración lo esquive; si alguien lo cambia sin STOP 1, este test lo delata |
| **R1** | `sddk cycle list` sobre un storage con 3 ciclos ⇒ declara **3** | RED | el subcomando no existe: sale con «unrecognized subcommand» |
| **R2** | un ciclo **superseded** sigue apareciendo, con su estado | RED | falla si la enumeración filtra por estado; la enumeración es de todos |
| **R5** | un ciclo de manifiesto ilegible **no aborta** la lista y su recuento lo declara | RED | una fila ilegible no puede hacer fallar la lista entera ni desaparecer |
| **R6** | el recuento de `cycle list` **iguala** el número de filas de `cycles` | RED | O2 literal; el que cierra O2 |
| **R3** | un ciclo **sin eventos** aparece | **lote 2** | necesita que `list_cycles` exista para compilar; si no, el RED sería un error de compilación que tumba el crate entero |

R1, R2, R5 y R6 se escriben **a nivel de CLI**, invocando el binario, y por eso
caen en tiempo de ejecución sin romper la compilación del crate. Esa es la razón
de que el lote 1 sea viable y de que R3 no lo sea: un RED que se compra rompiendo
el build no es un RED, es un apagón.

R4 va **primero de los que tocan almacenamiento**, porque fija el comportamiento
que este lote no va a cambiar. Si D1 se arreglara sin R4 escrito antes, el cambio
de ese comportamiento sería una deriva y no una decisión.

## §4 — STOP conditions

1. **Si el arreglo exige tocar `get_cycle`,** se para y se abre SCOPE aparte.
   Cambiar el contrato de lectura de un registro durable no se amplía en caliente.
2. **Si algún test que ya estaba verde hay que reescribirlo para que pase,** se
   para. El criterio es que la prueba era verde y significaba algo.
3. **Si la enumeración necesita una migración de esquema,** se para: eso tocaría
   las filas de `cycles` y §2.2 lo prohíbe.
4. **Si R4 no cae,** se para. Si `get_cycle` devolviera un registro para `{}`,
   la premisa de D2 es falsa y este SCOPE está construido sobre una suposición
   falsa: hay que volver a medir antes de escribir una línea.
5. **Si el recuento de R6 no cuadra,** se para y se investiga antes de ajustar el
   test para que pase. Un test que se ajusta al código deja de medir.

## §5 — Riesgo

1. **Volumen.** El storage tiene **333** directorios de proyecto. El más grande
   medido tiene 100 ciclos propios (179 filas en total, contando las 79 de
   `__spine_import__`). `list_cycles` sin paginación devuelve 100 filas hoy, y
   eso es aceptable; si mañana un proyecto llega a miles, hará falta un límite, y
   ese límite tiene que **declarar** que trunca. Mismo principio que F63.
2. **Lectura, no escritura.** Este lote no escribe en el almacenamiento real. Los
   tests usan sandboxes (`CliSandbox` con `env_remove("SDDK_DATA_DIR")`, como ya hace
   el resto del repo) porque un `CliSandbox` que hereda el entorno escribiría en el
   storage de verdad.
3. **El riesgo de este lote es de alcance, no de datos:** la tentación es arreglar
   D2 de paso, porque D1 y D2 conviven en el mismo fichero. STOP 1 existe para
   eso.

## §6 — Gates que deben seguir verdes al cerrar

`cargo test --workspace` **sin reescribir ningún test que ya fuera verde** ·
`cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D warnings` ·
`check_debt_index_coherence` · `test_adr_promotion_format` ·
`test_docs_script_contamination` · `test_gate_coverage` ·
`test_release_state_pointer` · falsificador propio que compruebe R6 contra el
almacenamiento real, en **solo lectura**, y declare su recuento.
