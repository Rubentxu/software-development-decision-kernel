# cl-vault-declaration — PRE-FLIGHT

**Cycle:** `p-63676b11dc0ef88f/vault-declaration`
**Date:** 2026-10-02T21:10:00Z
**Status:** READY
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)
**HEAD:** `a61948a2` (`docs(roadmap): session-69f, auditoria de familia de F63 y correccion de ledger watch`), `HEAD == origin/main`, árbol limpio
**Authority:** misma clase que F63, medido en session-69f · SCOPE-CONTRACT en este directorio

## Readiness: READY

| # | condición | estado |
|---|---|---|
| 1 | La deuda está **verificada vigente**, no heredada | OK — **medida en session-69f**: 20 de 75 documentos, sin declaración; `--limit 0` devuelve `no hits`; JSON array |
| 2 | Criterios de cierre falsables | OK — O1–O4, y STOP 5 es explícito sobre truncar sin decir |
| 3 | SCOPE con objetivo, no-objetivos y STOP | OK — 4 objetivos, 5 no-objetivos, 5 STOP |
| 4 | Superficie mapeada y **leída**, con línea | OK — abajo |
| 5 | Radio de impacto **medido antes de cambiar** | OK — §4 del SCOPE: 8 tests de la función intactos, 1 consumidor de CLI |
| 6 | Riesgo de datos | **cero por construcción**: no se escribe en ningún índice real; `search_index` no cambia de firma |

## Superficie (leída, con línea)

| qué | dónde | por qué importa |
|---|---|---|
| `VaultSearchArgs.limit` | `crates/sddk-cli/src/vault_cmd.rs:104-106` | `default_value_t = 20`, y el doc no dice qué significa `0` |
| `run_vault_search` | `vault_cmd.rs:435-449` | devuelve `Vec<SearchHit>` y lo pasa a `render_result`: por eso el JSON es un array |
| `search_text` | `vault_cmd.rs:555-564` | una línea por hit; con lista vacía devuelve `"no hits\n"`, que **tampoco declara** |
| `search_index` | `crates/sddk-vault/src/search.rs:166-185` | **API pública**: `SELECT … MATCH ?1 ORDER BY rank LIMIT ?2`. El corte es en **SQL**, luego el total no viene en memoria |
| `FTS_TABLE` | `search.rs:173` | el nombre de la tabla FTS, reutilizable por el `COUNT` |
| `open_index` | `search.rs:188-196` | abre (o crea) el índice; **no** se toca |
| `index_has_rows` | `search.rs:199-205` | el precedente más cercano: ya hace un `COUNT(*)` sobre la tabla, luego un `COUNT` no es ajeno a este crate |
| re-export público | `crates/sddk-vault/src/lib.rs:29-30` | `search_index` está en la superficie pública del crate |
| tests de `search_index` | `search.rs:252-347` | **8 llamadas directas**, todas con `.len()` sobre el `Vec` |
| consumidor de CLI | `crates/sddk-cli/tests/cli.rs:8523-8541` | el único que parsea el JSON: `hits.as_array().unwrap().len() == 1` |
| `check_vault_capability` | `vault_cmd.rs:139-156` | el gate `vault.search`; se mantiene, no se toca |

### Tres cosas que la lectura evitó

1. **Que haría falta cambiar `search_index`.** Es API pública con 8 tests. El
   camino corto —devolver `(hits, total)` desde la misma función— rompía ocho
   tests y una API publicada. Añadir `count_matches` no rompe nada, y §3 del
   SCOPE demuestra que la cuenta sale **más barata** que la búsqueda que acompaña.
2. **Que el total fuese gratis.** En `ledger events` lo era; aquí no, porque el
   corte ocurre en SQL. Darlo por gratis sin medir habría producido un lote con
   un coste oculto, que es el mismo defecto con otro signo.
3. **Que `--limit 0` significara algo documentado.** No lo significa en ninguna
   parte: `LIMIT 0` en SQL no es caso especial, devuelve cero filas.

## Lote de este apply

**Lote 1 — los tests RED y nada más.**

| # | test | por qué cae hoy |
|---|---|---|
| R1 | la salida declara el total aunque **no** trunque | hoy no declara nunca; O1 literal |
| R2 | declara y dice `truncated` cuando trunca | O1 en el caso que F63 describe |
| R3 | el JSON lleva `total_hits`, `shown`, `truncated` | hoy es un array; O2 |
| R4 | `--limit 0` devuelve **todos** | hoy devuelve `no hits`; O3 |
| R5 | sin coincidencias declara **cero** | hoy devuelve `no hits`; O4 |

A nivel de CLI, y montando el índice en un temporal con `vault index`, para que
el fixture sea un vault de verdad y no una tabla escrita a mano.

R5 va aparte de R1 a propósito: R1 mide el camino con resultados, R5 el camino
sin ellos, y los dos se pueden romper por razones distintas.

## Gates que deben seguir verdes al cerrar

`cargo test --workspace` **sin reescribir ningún verde salvo `cli.rs:8541`** ·
`cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D warnings` ·
`check_debt_index_coherence` · `test_changelog_coverage` ·
`test_adr_promotion_format` · `test_docs_script_contamination` ·
`test_gate_coverage` · `test_release_state_pointer` · falsificador propio que
compruebe O1–O4 contra un índice real, en solo lectura.
