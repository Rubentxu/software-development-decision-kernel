# cl-vault-graph — PRE-FLIGHT

**Cycle:** `p-63676b11dc0ef88f/vault-graph`
**Date:** 2026-10-02T21:45:00Z
**Status:** READY
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)
**HEAD:** `f1659281` (`docs(roadmap): session-69g, vault search cerrado y el coste del total medido`), `HEAD == origin/main`, árbol limpio
**Authority:** cierra el «NO MEDIDO» que dejaron session-69f y session-69g · SCOPE-CONTRACT en este directorio

## Readiness: READY

| # | condición | estado |
|---|---|---|
| 1 | El defecto está **medido**, no supuesto | OK — dos vaults sintéticos, con y sin ciclos, y `vault show` medido aparte |
| 2 | Criterios de cierre falsables | OK — O1–O3, STOP 1 impide la solución ingenua |
| 3 | SCOPE con objetivo, no-objetivos y STOP | OK — 3 objetivos, 5 no-objetivos, 5 STOP |
| 4 | Superficie mapeada y **leída**, con línea | OK — §4 del SCOPE |
| 5 | Radio de impacto medido | OK — **1** consumidor, y el cambio es **aditivo**: STOP 3 queda vacío |
| 6 | Riesgo de datos | **cero por construcción**: no se escribe en ningún vault real |

## Superficie (leída, con línea)

| qué | dónde | por qué importa |
|---|---|---|
| `GraphView` | `crates/sddk-vault/src/graph.rs:24-33` | cuatro campos; **ninguno** de recuento de ciclos |
| `graph_view` | `graph.rs:55-86` | dos ramas: acíclico → orden topológico; cíclico → `sample_cycle` y `topological_order: None` |
| `find_sample_cycle` | `graph.rs:88-94` | **devuelve el primer ciclo y para**: el origen del defecto |
| `dfs_cycle` | `graph.rs:96+` | la DFS que envuelve; es reutilizable tal cual |
| `build_graph` | `graph.rs` | **no se toca**: este lote cambia lo que se declara, no lo que se calcula |
| `graph_text` | `crates/sddk-cli/src/vault_cmd.rs:620-632` | imprime el orden solo si existe, y no explica su ausencia |
| consumidor de CLI | `crates/sddk-cli/tests/cli.rs:8579-8582` | afirma `node_count`, `edge_count`, `cyclic`, `sample_cycle.is_array()`; **aditivo, no se reescribe** |
| réplica en HTML | `crates/sddk-vault/src/export.rs:24-25`, `:41-42` | `vault export` proyecta el mismo `GraphView`; STOP 4 |

### Dos cosas que la medición evitó

1. **Cerrar el asunto con el caso fácil.** El primer fixture era una cadena
   acíclica de 30 nodos y `vault graph` parecía correcto: `node_count` cuadraba y
   el orden topológico tenía los 30. **`sample_cycle` solo aparece cuando hay
   ciclos**, que es justo donde la salida miente. Un solo fixture, el que no
   dispara el defecto, habría producido un «no hay problema» con evidencia.
2. **Prometer un `cycle_count` exacto.** Se midió antes de proponerlo, y el
   recuento ingenuo resultó **incorrecto**, no solo lento: la cadena de 1000
   nodos tiene un ciclo y la cuenta devuelve 1000 rotaciones; el bouquet de 500
   cuenta 1000 por dirección. Además tardó 558 ms en el caso más trivial que
   existe. Un campo de recuento que parece verdad y no lo es es peor que no
   tenerlo.

## Lote de este apply

**Lote 1 — los tests RED y nada más.**

| # | test | por qué cae hoy |
|---|---|---|
| R1 | con **dos ciclos**, la salida declara que hay más de uno | `sample_cycle` no dice nada; O1 |
| R2 | con **un** ciclo, declara que hay exactamente uno | el campo no existe; O1 |
| R3 | `topological_order` ausente **explica su causa** | hoy la línea simplemente no aparece; O2 |
| R4 | el caso **acíclico** no cambia: orden completo, `cyclic: false` | caracterización: pasa hoy, y fija que el arreglo no lo rompe |

R4 es de caracterización y **pasa hoy**, y se declara como tal: llamarlo RED sería
falso, igual que el guard de `vault search`.

Los tres RED necesitan un vault **con ciclos**, que es el caso que el primer
fixture no tenía.

## Gates que deben seguir verdes al cerrar

`cargo test --workspace` **sin reescribir ningún verde** ·
`cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D warnings` ·
`check_debt_index_coherence` · `test_changelog_coverage` ·
`test_adr_promotion_format` · `test_docs_script_contamination` ·
`test_gate_coverage` · `test_release_state_pointer` · falsificador que mida O1–O3
sobre un vault con **dos ciclos** y otro **acíclico**.
