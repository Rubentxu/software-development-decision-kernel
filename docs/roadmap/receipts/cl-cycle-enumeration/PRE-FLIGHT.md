# cl-cycle-enumeration — PRE-FLIGHT

**Cycle:** `p-63676b11dc0ef88f/cycle-enumeration`
**Date:** 2026-10-02T18:45:00Z
**Status:** READY
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)
**HEAD:** `1f421ddd` (`docs(roadmap): session-69c 2a parte, INC-DEBT-060 y la premisa de INC-DEBT-049`), `HEAD == origin/main`, árbol limpio
**Authority:** INC-DEBT-060 (`high`/`P1`, `open`) · SCOPE-CONTRACT en este directorio

## Readiness: READY

| # | condición | estado |
|---|---|---|
| 1 | La deuda está **verificada vigente**, no heredada | OK — medida sobre el almacenamiento real en session-69c: 97 ciclos sin superficie, 81 con manifiesto ilegible |
| 2 | Deuda con criterio, severidad razonada y criterios de cierre falsables | OK — INC-DEBT-060, F60–F63 |
| 3 | SCOPE con objetivo falsable, no-objetivos y STOP conditions | OK — 4 objetivos, 5 no-objetivos, 5 STOP |
| 4 | Superficie mapeada y **leída**, no inferida | OK — abajo, con línea |
| 5 | Superficie acotada al lote | 2 ficheros de producción + 2 de test + el comando; ninguno fuera |
| 6 | Riesgo de datos | **cero por construcción**: el lote no escribe en el almacenamiento real, y R4 fija por escrito el comportamiento de lectura que no se va a cambiar en caliente |

## Superficie (leída, con línea)

| qué | dónde | por qué importa |
|---|---|---|
| `get_cycle` | `crates/sddk-storage/src/lib.rs:639` | **la única** lectura de `cycles` que expone el storage; selecciona `manifest_json, created_at, updated_at` |
| `cycle_from_row` | `lib.rs:1893` | deserializa `manifest_json` en `CycleManifest`; ahí nace D2 |
| `json_from_sql_error` | `lib.rs:1994` | convierte el fallo de serde en `FromSqlConversionFailure`, que `?` propaga |
| `CycleManifest` | `crates/sddk-domain/src/cycle.rs:182-210` | **11 campos obligatorios** sin `#[serde(default)]`; un `{}` falla |
| `CycleStatus` | `cycle.rs:12-13` | `SCREAMING_SNAKE_CASE` ⇒ `"PAUSED"` ↔ `Paused` **sí** round-tripa. Descartado como causa |
| `CycleRecord` | `crates/sddk-domain/src/models/identity.rs:25` | `{ manifest, created_at, updated_at }` — **no lleva `status` ni `phase`**, luego `cycle list` no puede reutilizar este tipo tal cual |
| `exists` sobre `cycles` | `lib.rs:508` | la otra consulta a la tabla, por id |
| `delete_cycle` | `lib.rs:930` | la escritura; **fuera de alcance** (§2.2) |
| `list_cycles` | **no existe** — 0 coincidencias | D1 |
| `cycle_from_row` en migraciones | `migrations.rs:412` | `INSERT INTO cycles_m21_widened SELECT * FROM cycles` — copia la tabla, luego **cualquier** filtro de enumeración debe acordarse con lo que esa migración preservó |
| `LedgerCommand::Events` | `crates/sddk-cli/src/ledger.rs:16-31` | el patrón de subcomando que se copia, `--limit` incluido |
| `LedgerEventsArgs.limit` | `ledger.rs:60+` | el truncamiento silencioso de F63: **no** se toca (§2.3) |
| `run_ledger_verify` | `ledger.rs:230` | el patrón de un comando de solo lectura: `RuntimeContext::open(..., false)` y `render_result` |
| `verify_streams` | `ledger.rs:286` | el patrón de enumerar sobre `event_store.list_streams()` **y** de no mentir sobre lo examinado |
| registro de comandos | `crates/sddk-cli/src/command_spec.rs:358-367` | declara **verbos raíz** (`cycle`, `ledger`), **no subcomandos**: `cycle list` **no** obliga a tocar el registro. Verificado leyendo el fichero, no supuesto |
| `command_spec_tests.rs` | tests | afirman forma del registro y comandos concretos (`change`, `verify`, `audit`, `config`); **no** cruzan subcomandos |

### Dos cosas que la lectura evitó que se dieran por buenas

1. **El registro de comandos.** `command_spec.rs` declara verbos raíz, así que
   añadir `cycle list` no lo toca. Es lo que se habría asumido al revés —«hay un
   `CommandRegistry`, luego hay que registrar»— y habría producido un SCOPE más
   ancho del necesario.
2. **`CycleStatus` vs el almacenamiento.** La suspicion de que `"PAUSED"` no
   deserializaría en `Paused` era **falsa**: el enum declara
   `SCREAMING_SNAKE_CASE`. Comprobado antes de escribirlo, y no consta como
   hallazgo.

## Lote de este apply

**Lote 1 — los seis tests RED y nada más** (R1–R6 del SCOPE). Si alguno no cae,
se para (STOP 2 / STOP 4).

El orden importa: **R4 va primero de los que tocan almacenamiento**, porque fija el
comportamiento actual de `get_cycle` ante un manifiesto `{}`. Si el arreglo de D1
tocara ese comportamiento sin que R4 lo hubiera fijado antes, el cambio sería una
deriva y no una decisión. Fijarlo antes es lo que lo convierte en STOP 1.

Un lote que solo escribe tests que caen es lo que permite que el siguiente sea
«hacerlos verdes» y no «comprobar a posteriori si algo se movió».

## Gates que deben seguir verdes al cerrar

`cargo test --workspace` **sin reescribir ningún test que ya fuera verde** ·
`cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D warnings` ·
`check_debt_index_coherence` · `test_adr_promotion_format` ·
`test_docs_script_contamination` · `test_gate_coverage` ·
`test_release_state_pointer` · falsificador propio que compruebe R6 contra el
almacenamiento real, en solo lectura, declarando su recuento.
