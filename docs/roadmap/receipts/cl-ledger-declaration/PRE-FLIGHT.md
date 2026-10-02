# cl-ledger-declaration — PRE-FLIGHT

**Cycle:** `p-63676b11dc0ef88f/ledger-declaration`
**Date:** 2026-10-02T20:45:00Z
**Status:** READY
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)
**HEAD:** `90038756` (`docs(roadmap): session-69d, lote 2 entregado y magnitud de INC-DEBT-060 corregida`), `HEAD == origin/main`, árbol limpio
**Authority:** INC-DEBT-060 (`high`/`P1`, `open`), falsificador F63 · SCOPE-CONTRACT en este directorio

## Readiness: READY

| # | condición | estado |
|---|---|---|
| 1 | La deuda está **verificada vigente**, no heredada | OK — **medida de nuevo en esta sesión**, no heredada de INC-DEBT-060: 50 de 590, 19 de 114 ciclos, sin declaración, exit 0 |
| 2 | Deuda con criterio, severidad razonada y criterios de cierre falsables | OK — F63, enunciado en INC-DEBT-060 y **endurecido** por la medición (§0 del SCOPE) |
| 3 | SCOPE con objetivo falsable, no-objetivos y STOP conditions | OK — 4 objetivos, 5 no-objetivos, 5 STOP |
| 4 | Superficie mapeada y **leída**, no inferida | OK — abajo, con línea |
| 5 | Superficie acotada al lote | 1 fichero de producción (la rama `events` de `ledger.rs`) + tests. **Storage no se toca**, y eso es STOP 1 |
| 6 | Riesgo de datos | **cero por construcción**: el lote no escribe en el almacenamiento real; los tests van en sandbox con `env_remove("SDDK_DATA_DIR")` |

## Superficie (leída, con línea)

| qué | dónde | por qué importa |
|---|---|---|
| `LedgerEventsArgs.limit` | `crates/sddk-cli/src/ledger.rs:74-78` | `#[arg(long, default_value_t = 50)]` — el 50 de F63 |
| `run_ledger_events` | `ledger.rs:398-424` | `.rev().take(args.limit).rev()` — **toma los 50 más recientes**, luego los reordena |
| `ledger_events_text` | `ledger.rs:594-608` | una línea por evento; con lista vacía devuelve `"no events\n"`, que **tampoco declara nada** |
| `LedgerEventOutput` | `ledger.rs` | el tipo serializado; un `Vec<LedgerEventOutput>` es lo que se emite hoy, y por eso el JSON es un array |
| `Storage::list_events` | `crates/sddk-storage/src/lib.rs:974` | llama a `canonical_events`, que recorre **todos** los streams con `u32::MAX`: **el total ya está cargado** |
| `canonical_events` | `lib.rs:983-992` | `store.list_streams()` + `load_stream(&stream, None, u32::MAX)` — sin límite |
| `LedgerExportArgs.limit` | `ledger.rs:85-90` | **`0` = todos**, tratado explícitamente en `ledger.rs:442-445`. La convención que `events` contradice |
| `run_ledger_export` | `ledger.rs:436-447` | el patrón del que copiar la semántica de `0` |
| `LedgerWatchArgs.max_events` | `ledger.rs:142` | `0` = ilimitado también. **Mismo defecto, otra superficie** — §2.3 lo excluye |
| `render_result` | `crates/sddk-cli/src/lib.rs:2418` | `fn(&T) -> String` sobre un `T: Serialize`; cambiar `T` de `Vec<…>` a una envoltura **no** obliga a tocarlo |
| consumidor del JSON | `crates/sddk-cli/tests/cli.rs:1443` | `events_json.as_array().unwrap().len() == 6` — el único, y por eso §3 del SCOPE |
| registro de comandos | `crates/sddk-cli/src/command_spec.rs:358-367` | verbos raíz: `ledger` ya está, **no se añade subcomando**, luego no se toca el registro |

### Tres cosas que la lectura evitó que se dieran por buenas

1. **Que hacer falta una consulta de recuento.** No hace falta, y es lo que
   convierte el arreglo en pequeño. `list_events` ya devuelve el vector entero;
   el número a declarar es `events.len()` **antes** del `.take`. Si se hubiera
   añadido un `count_events()` al storage, el lote habría crecido y STOP 1
   habría saltado sin necesidad.
2. **Que `--limit 0` fuera un detalle.** Es una inconsistencia con `export` en el
   mismo binario, y produce una lista vacía con exit 0. Es el mismo defecto que
   F63 —declarar lo que se está mirando— en otra forma.
3. **Que el JSON fuera un array por descuido.** Es una **elección de forma**, y la
   elección es la que impide declarar el total. Por eso §3 del SCOPE la cambia de
   forma explícita y declara el radio de impacto medido, en vez de que se
   descubra al romper el test.

## Lote de este apply

**Lote 1 — los tests RED y nada más** (R1–R4 del lote, abajo). Si alguno no cae,
se para (STOP 2 / STOP 5).

| # | test | por qué cae hoy |
|---|---|---|
| R1 | la salida de texto declara el total aunque **no** trunque | hoy no declara nunca; O1 es literal |
| R2 | la salida de texto declara el total **cuando trunca**, y lo dice como truncamiento | hoy no dice nada; es el caso de F63 |
| R3 | el JSON lleva `total_events`, `shown` y `truncated` | hoy es un array sin nada de eso; O2 |
| R4 | `--limit 0` devuelve **todos**, y `--limit N` devuelve N con el total declarado | hoy `0` devuelve 0; O3 |

Van a **nivel de CLI** por el motivo ya escrito en el SCOPE de
`cl-cycle-enumeration`: un test que llama a una función que todavía no existe no
compila, y un RED comprado rompiendo el build tumba el crate entero.

R1 va antes que R2 a propósito: R1 es el que puede **no** caer, porque es el caso
en el que la salida ya es correcta por accidente. Si R1 pasara hoy, la premisa
de O1 estaría mal y hay que volver a medir antes de escribir nada.

## Gates que deben seguir verdes al cerrar

`cargo test --workspace` **sin reescribir ningún test verde salvo el declarado en
§4.2 del SCOPE** · `cargo fmt --check` · `cargo clippy --workspace --all-targets
-- -D warnings` · `check_debt_index_coherence` · `test_changelog_coverage` ·
`test_adr_promotion_format` · `test_docs_script_contamination` ·
`test_gate_coverage` · `test_release_state_pointer` · falsificador propio que
compruebe O1–O4 contra el almacenamiento real, en **solo lectura**, declarando su
recuento.
