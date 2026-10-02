# cl-ledger-watch-total — PRE-FLIGHT

**Cycle:** `p-63676b11dc0ef88f/ledger-watch-total` (real, `OPEN/explore`)
**Date:** 2026-10-02T21:12:36Z
**Status:** READY
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)
**HEAD:** `b0153dc0` (`chore(roadmap): reconcilia el puntero de estado tras el lote 2`), `HEAD == origin/main`, árbol limpio
**Authority:** la auditoría por criterio que `cl-vault-node-projection` dejó
abierta, aplicada a las superficies de lectura del ledger
**SCOPE-CONTRACT y EXPLORATION-REPORT en este directorio**

## Readiness: READY

| # | condición | estado |
|---|---|---|
| 1 | El defecto está **medido**, no supuesto | OK — `03-medir-watch.py`: declara `[5]`, hay **591**; texto, JSON y el detalle de `export` |
| 2 | Criterios de cierre falsables | OK — O1–O4; STOP 1 y 2 prohíben la segunda cuenta |
| 3 | SCOPE con objetivo, no-objetivos y STOP | OK — 4 objetivos, 7 no-objetivos, 5 STOP |
| 4 | Superficie mapeada y **leída**, con línea | OK — §5 del SCOPE, siete filas |
| 5 | Radio de impacto medido | OK — **1** función (`run_ledger_watch`) y su pie; el JSON de `watch` **solo** lo emite esa línea de resumen, luego el cambio es aditivo |
| 6 | Riesgo de datos | **cero por construcción**: la medición corre contra copia y el mtime del original se comprueba |

## Superficie (leída, con línea)

| qué | dónde | por qué importa |
|---|---|---|
| `run_ledger_watch` | `ledger.rs:689-799` | el bucle y su pie |
| el corte por tope | `ledger.rs:764-766` | `return Ok(emitted)` sale **sin** decir qué queda |
| el pie | `ledger.rs:788` | `[watch] emitted {count} events, exiting` — solo lo emitido |
| el resumen JSON | `ledger.rs:785` | `{"__watch_complete":true,"emitted":N}` — un objeto, y dice una sola cosa |
| los filtros | `ledger.rs:727-732` | `retain` en Rust: el total tiene que contar **esto**, no el ledger entero |
| el cursor inicial | `ledger.rs:707-713` | `--from-tail` lo fija antes del bucle |
| el largo ya descartado | `sddk-storage/src/lib.rs:1026` | `.take(limit)`: el total se tira en cada poll |
| precedente | `ledger.rs:437-472`, `:651-657` | la forma que `ledger events` ya tiene y hay que replicar |

## Lo que la medición evitó

1. **Confundir «no declara» con «no trunca».** `watch` con `--max-events 5`
   sobre 591 **sí** sale y **sí** dice cuántos emitió. Declarar eso está bien.
   Lo que falta es el otro número, y es exactamente el de F63.
2. **Elegir el remedio por precio sin medir su verdad.** `COUNT(*)` es
   **51,8× más barato** (0,077 ms vs 4,015 ms, medido) y aun así se descartó:
   con `--cycle`/`--frame` habría que **probar** que el predicado SQL coincide
   con el `retain` en Rust en tres formas, y esa prueba no está hecha. Un
   `COUNT` que no coincide no es una declaración más barata: es una
   declaración **falsa** con mejor rendimiento. La objeción contraria —«A es
   caro»— tampoco se sostiene: es **una iteración más** de un trabajo que el
   bucle ya hace cada 500 ms.
3. **Un detector que puede pasar con el defecto presente.** El primero buscaba
   `of`/`total`/`pending` en toda la salida, y el payload de los eventos
   emitidos contiene esas palabras. El de `ledger export` contaba además los
   dígitos del `tmpdir` (`[0, 5, 9]`). Los dos miden lo que tienen al lado en
   vez de lo que buscan. Todos los FAIL de esta medición fueron **del
   detector** y se corrigieron **en el detector**, nunca en el producto.

## Lote de este apply

**Lote 1 — los tests RED y nada más.**

| # | test | por qué cae hoy |
|---|---|---|
| R1 | el texto **declara** el total, incluso sin nada que dejar fuera | el pie solo dice emitidos |
| R2 | el JSON lleva `total_events` y `pending` | el resumen es un objeto de un campo |
| R3 | `emitted + pending == total_events` | no hay `pending`; y si lo hubiera a mano, esta es la que lo detecta |
| R4 | con `--cycle X` el total es **el de ese ciclo**, no el del ledger | el guard que evita la solución de §3.2 |
| R5 | con `--from-sequence N` el total cuenta solo lo posterior a N | el guard que evita contar el cursor equivocado |

R4 y R5 son los dos guards que valen el ciclo: R1 y R2 se pueden satisfacer
declarando un número, y R4/R5 son los que impiden **declarar un número que no
es el de esta consulta**.

## Cobertura objetivo → guard

Esta tabla no es documentación: es lo que comprueba
`/var/home/rubentxu/f63/04-req-testable.py`, que es el instrumento cuyo
`output_digest` sostiene el gate `requirements-testable`. La primera vez que
corrió **falló con 6 problemas** porque el mapa no existía —los guards estaban
descritos en prosa y nada decía cuál cubría cuál—. Un objetivo sin guard
nombrado es una nota, y una nota no es un requisito.

| objetivo | guard | por qué ese guard puede fallar con el defecto presente |
|---|---|---|
| **O1** declara el total en texto y JSON | R1, R2 | hoy el pie dice solo `[5]` y el JSON solo `emitted`; ninguno de los dos puede ver el total |
| **O2** `pending` derivado, aritmética cerrada | R3 | hoy no existe `pending`; y si se escribiera a mano, `emitted + pending != total_events` es exactamente lo que R3 mide |
| **O3** mismo predicado y mismo cursor que el emisor | R4, R5 | un `COUNT(*)` sin filtro pasa R1 y R2, y **falla** R4; un total contado desde el cursor equivocado pasa todo y **falla** R5 |
| **O4** declara también cuando no queda nada fuera | R1 | con `pending == 0` un producto que solo declara al truncar no dice nada, y R1 exige la frase con un ledger que cabe entero |

## Gates que deben seguir verdes al cerrar

`cargo test --workspace` **sin reescribir ningún verde** ·
`cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D warnings` ·
`test_changelog_coverage` · `test_debt_index_coherence` ·
`test_release_state_pointer` · `03-medir-watch.py` con **0/3** GAP en las
comprobaciones de `ledger watch`, y con el GAP de `ledger export` **nombrado**,
no oculto.
