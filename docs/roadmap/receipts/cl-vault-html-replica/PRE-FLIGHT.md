# cl-vault-html-replica — PRE-FLIGHT

**Cycle:** `p-63676b11dc0ef88f/vault-html-replica`
**Date:** 2026-10-02T23:25:00Z
**Status:** READY
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)
**HEAD:** `ce09a855` (`docs(roadmap): session-69h, vault graph cerrado y un gate de release desbloqueado`), `HEAD == origin/main`, árbol limpio
**Authority:** el riesgo 3 del recibo de `cl-vault-graph`, que decía «no se ha
verificado si es intencional» · SCOPE-CONTRACT en este directorio

## Readiness: READY

| # | condición | estado |
|---|---|---|
| 1 | El defecto está **medido**, no supuesto | OK — `07-medir-html.py`, 3 de 4 declaraciones ausentes |
| 2 | Criterios de cierre falsables | OK — O1, O2, O3; STOP 3 exige que O3 sea comprobable o se para |
| 3 | SCOPE con objetivo, no-objetivos y STOP | OK — 3 objetivos, 5 no-objetivos, 3 STOP |
| 4 | Superficie mapeada y **leída**, con línea | OK — §4 del SCOPE |
| 5 | Radio de impacto medido | OK — **1** consumidor (`export.rs:118-138`), y el cambio es **aditivo** sobre un JSON incrustado |
| 6 | Riesgo de datos | **cero por construcción**: no se escribe en ningún vault real |

## Superficie (leída, con línea)

| qué | dónde | por qué importa |
|---|---|---|
| `GraphExport` | `crates/sddk-vault/src/export.rs:90-95` | estructura **propia**, tres campos; ninguno de recuento ni de ausencia |
| construcción | `export.rs:23-27` | mapea `&GraphView` → `GraphExport` **a mano**, campo a campo |
| incrustación | `export.rs:70-74` | `window.__vault_graph__={…}`, que es lo que el falsificador extrae |
| test existente | `export.rs:118-138` | 7 aserciones sobre el HTML renderizado; **no se reescribe** |
| `GraphView` | `crates/sddk-vault/src/graph.rs` | ya tiene los tres campos que faltan; **este lote no lo toca** |

## Lo que la medición evitó

1. **Dar el riesgo por cerrado porque «no se contradicen».** El STOP 4 de
   `cl-vault-graph` se satisface literalmente hoy —el HTML no afirma nada falso
   sobre el grafo— y aun así el defecto está entero. **Una condición que se
   puede cumplir con el defecto presente no es un guard**, y este la cumplía.
2. **Contar superficies por los comandos que las exponen.** `vault graph` y
   `vault export` son dos comandos, luego son «dos superficies» —y el HTML
   incrustado es una tercera declaración del mismo hecho que ninguno de los dos
   enumera por separado. El defecto de `cl-vault-graph` estaba cerrado en **dos
   de tres** declaraciones, no en dos de dos.

## Lote de este apply

**Lote 1 — el test RED y nada más.**

| # | test | por qué cae hoy |
|---|---|---|
| R1 | el JSON incrustado lleva `cycle_count` y `multiple_cycles` | `GraphExport` no los tiene |
| R2 | el JSON incrustado lleva `topological_order_absent_because` | `GraphExport` no lo tiene |
| R3 | las dos declaraciones **coinciden campo a campo** con las de `vault graph` | es lo que impide que vuelvan a separarse; sin él, R1 y R2 solo fijan presencia |

R3 es el que tiene el peso: **R1 y R2 sin R3 son dos pruebas de presencia**, que
pasan en cuanto se añade el campo, aunque el valor sea inventado. R3 compara
**valores**, y es el que convierte la arreglo en una forma.

## Gates que deben seguir verdes al cerrar

`cargo test --workspace` **sin reescribir ningún verde** ·
`cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D warnings` ·
`test_changelog_coverage` · `test_debt_index_coherence` ·
`test_release_state_pointer` · `07-medir-html.py` pasa de **3/4 en GAP a 0**.
