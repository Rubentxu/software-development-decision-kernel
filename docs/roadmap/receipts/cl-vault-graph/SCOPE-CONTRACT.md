# SCOPE-CONTRACT — cl-vault-graph

**Cycle:** `p-63676b11dc0ef88f/vault-graph`
**Date:** 2026-10-02T21:40:00Z
**Origin:** los dos slices anteriores dejaron `vault graph` y `vault show` como
**NO MEDIDOS**, y esa distinción entre «no medido» y «correcto» es la que faltó
con `ledger watch`
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)

---

## §0 — La medición, que es lo que faltaba

`/var/home/rubentxu/vault/03-medir-gs.py` y `04-medir-ciclos.py`. La primera
pasada dio **dos resultados distintos**, y por eso el veredicto no es «los dos
están mal»:

### `vault graph` — DEFECTO REAL

Con un vault **acíclico** de 30 nodos, `vault graph` parece correcto:
`node_count: 30` igual al real, `topological_order` con los 30. **Ese es el caso
fácil, y quedarse solo con él es como se habría cerrado el asunto.**

Con un vault de **6 nodos y dos ciclos distintos** (A→B→C→A y X→Y→X):

```
node_count: 6        edge_count: 5        cyclic: true
sample_cycle: CYC-Y -> CYC-X -> CYC-Y
topological_order: <ausente>
```

Dos cosas que no dice:

1. **`sample_cycle` es UNO de dos ciclos y no declara cuántos hay.** El código
   lo confirma: `find_sample_cycle` (`graph.rs:88-94`) devuelve el primer ciclo
   que encuentra y **para**; `GraphView` no tiene ningún campo de recuento. Un
   grafo con un ciclo y otro con doscientos producen la misma salida.
2. **`topological_order` desaparece sin decir por qué.** `graph.rs:81` lo pone a
   `None` cuando hay ciclos, y el texto simplemente no imprime la línea. Quien
   lee ve `cyclic: true` y una línea que falta, sin conexión entre ambas.

### `vault show` — NO es de esta clase

`backlinks` **no tiene cota**: buscando `take`, `limit` y truncamientos en
`crates/sddk-vault/src/` no aparece ninguno. El array es el conjunto completo, así
que no hay omisión silenciosa que declarar. El texto sí dice `backlinks (1): …` y
el JSON lleva el array entero; que el JSON no repita el recuento es una asimetría
menor, no un defecto de truncamiento. **Queda medido y no se toca.**

## §1 — La decisión de diseño, y por qué NO es un recuento

La solución obvia —añadir `cycle_count` exacto— se midió antes de prometerla, y
**la medición la refutó**:

```
n      tipo      ciclos reales   cuenta ingenua     ms
50     cadena          1              50          1,34
200    cadena          1             200         20,57
1000   cadena          1            1000        558,59
50     bouquet        25             100          0,12
200    bouquet       100             400          0,48
1000   bouquet       500            1000          1,17
```

Un recuento ingenuo de ciclos es **incorrecto**, no solo lento: la cadena de 1000
nodos tiene **un** ciclo y la cuenta devuelve 1000, porque cada rotación del
mismo ciclo es un ciclo distinto; y el bouquet de 500 cuenta 1000, porque cada
ciclo se recorre en las dos direcciones. Y es lento justo en el caso común: una
cadena es el grafo más trivial que existe y tarda medio segundo.

**Un `cycle_count` exacto habría sido un número falso y, en el caso de la cadena,
medio segundo de retardo.** Es la forma más cara de mentir: un campo que parece
una verdad y no lo es.

La forma correcta es **saturada**, y es barata: tras encontrar el ciclo de
muestra, se quitan sus aristas y se busca **otro**. Si aparece, hay al menos dos.
Son dos pasadas de la función que ya existe, del mismo orden de complejidad, sin
contador que pueda equivocarse.

## §2 — Objetivo, falsable

1. **O1.** La salida declara **cuántos ciclos hay** cuando hay más de uno, con una
   forma que no pueda mentir: un entero para 0 y 1, y `null` con un campo
   booleano para «2 o más».
2. **O2.** La ausencia de `topological_order` **explica su causa** en texto y en
   JSON, en vez de desaparecer sin más.
3. **O3.** El caso **acíclico** no cambia: sigue declarando `cyclic: false`, el
   orden topológico completo y ningún recuento de ciclos.

## §3 — No-objetivos

1. **NO** se cambia `build_graph`, ni el criterio de qué es un ciclo, ni la
   detección de aciclicidad. Este lote cambia **lo que se declara**, no lo que se
   calcula del grafo.
2. **NO** se toca `vault show`. Medido en §0 y fuera de esta clase.
3. **NO** se toca `vault index`, `vault validate`, `vault search` ni el HTML de
   `vault export`. Este último replica `sample_cycle` (`export.rs:24-25`), y por
   eso §5 dice qué se hace con él en vez de dejar que se descubra después.
4. **NO** se escribe en ningún vault real.
5. **NO** se promete un recuento exacto. §1 midió por qué no es honesto.

## §4 — Superficie

| qué | dónde |
|---|---|
| `GraphView` | `crates/sddk-vault/src/graph.rs:24-33` — 4 campos, ninguno de recuento |
| `graph_view` | `graph.rs:55-86` — dos ramas: acíclico con orden, cíclico con `sample_cycle` |
| `find_sample_cycle` | `graph.rs:88-94` — **devuelve el primero y para** |
| `dfs_cycle` | `graph.rs:96+` — la DFS que la envuelve |
| `graph_text` | `crates/sddk-cli/src/vault_cmd.rs:620-632` — imprime la orden solo si existe |
| consumidor de CLI | `crates/sddk-cli/tests/cli.rs:8552-8556` — `graph["node_count"] == 2`, `graph[...]` |
| réplica en HTML | `crates/sddk-vault/src/export.rs:24-25` y `:41-42` |

## §5 — STOP conditions

1. **Si declarar el recuento obliga a enumerar ciclos**, se para: §1 midió que
   esa cuenta es incorrecta y lenta, y un campo de recuento mal calculada es peor
   que ningún campo.
2. **Si el caso acíclico cambia de salida**, se para: O3 es que no cambia, y si
   cambia es un defecto de este arreglo.
3. **Si algún test verde hay que reescribirlo**, se para y se escribe por qué.
   **La excepción prevista es vacía:** el único consumidor
   (`crates/sddk-cli/tests/cli.rs:8579-8582`) afirma `node_count`, `edge_count`,
   `cyclic` y que `sample_cycle` es un array, y **añadir campos es aditivo**: los
   cuatro siguen verdad sin tocar el test. Es la diferencia con `ledger events`
   (envoltura por array) y con `vault search` (ídem), donde sí hubo que
   reescribir. Aquí el cambio no es de forma sino de contenido.
4. **Si `vault export` deja de cuadrar con `vault graph`**, se para: son dos
   proyecciones del mismo `GraphView`, y divergir entre ellas es un defecto que
   este lote no crea pero tampoco debe dejar pasar.
5. **Si el recuento declarado no cuadra con lo que el grafo tiene**, se para y se
   investiga antes de ajustar nada.

## §6 — Gates al cerrar

`cargo test --workspace` sin reescribir ningún verde salvo el declarado ·
`cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D warnings` ·
`check_debt_index_coherence` · `test_changelog_coverage` ·
`test_adr_promotion_format` · `test_docs_script_contamination` ·
`test_gate_coverage` · `test_release_state_pointer` · falsificador que mida O1–O3
sobre un vault con **dos ciclos** y otro **acíclico**, declarando su recuento.
