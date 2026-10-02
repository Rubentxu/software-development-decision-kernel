# SCOPE-CONTRACT — cl-vault-html-replica

**Cycle:** `p-63676b11dc0ef88f/vault-html-replica`
**Date:** 2026-10-02T23:20:00Z
**Origin:** el cierre de `cl-vault-graph` dejó escrito, como riesgo 3 de su
recibo, que la réplica HTML «no se contradice con el grafo» pero **no se había
verificado** si es intencional. Aquí se mide, y la respuesta es que **no es
intencional**: es una tercera superficie de la misma clase de defecto.
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)

---

## §0 — La medición

`/var/home/rubentxu/vault/07-medir-html.py`, sobre el **mismo vault de dos ciclos
disjuntos** que dispara el defecto en `vault graph`. Compara el JSON que el
comando incrusta en el HTML (`window.__vault_graph__`) contra el JSON que
`vault graph` emite.

```
vault graph  : cycle_count=None multiple_cycles=True absent_because='cyclic'
html replica : claves=['cyclic', 'sample_cycle', 'topological_order']

OK  : html declara cyclic igual que el grafo -- html=True grafo=True
GAP : html declara cuantos ciclos hay
GAP : html declara que hay mas de un ciclo
GAP : html explica la ausencia del orden topologico
SUPERFICIES EN DEFECTO: 3/4
```

## §1 — Por qué STOP 4 estaba redactado demasiado flojo

El STOP 4 de `cl-vault-graph` decía «si `vault export` deja de cuadrar con
`vault graph`, se para: son dos proyecciones del mismo `GraphView`». Se interpretó
como **«no deben contradecirse»**, y el falsificador F6 lo pasó porque solo
comprobaba que `vault export` **siguiera funcionando** y contuviera `cyclic`.

La medición muestra que «no contradecirse» es una exigencia **débil**: el HTML no
contradice al grafo, **simplemente no declara lo mismo**. Lo que sí declara —
`sample_cycle` con un ciclo y sin decir cuántos hay, `topological_order` ausente
sin decir por qué— es **exactamente el defecto que `cl-vault-graph` acaba de
cerrar**, intacto.

La causa está en el código y es estructural: `GraphExport` (`export.rs:90-95`) no
es una vista parcial de `GraphView`, es una **estructura distinta** con tres
campos, escrita a mano. Dos declaraciones del mismo grafo, dos listas de campos
que se pueden separar sin que nada lo note.

**Consecuencia que se acepta:** STOP 4 era un guard débil, y se registra como
tal. La forma correcta de esa condición es «`vault export` declara **lo mismo**
que `vault graph`», que es lo que este ciclo implementa.

## §2 — Objetivo, falsable

1. **O1.** El JSON incrustado en el HTML lleva `cycle_count` y
   `multiple_cycles`, con la **misma forma saturada** que el grafo.
2. **O2.** El JSON incrustado lleva `topological_order_absent_because` cuando el
   orden falta, por la misma causa que el grafo declara.
3. **O3.** Las dos declaraciones **coinciden campo a campo** con las de
   `vault graph` para el mismo vault. Esto es lo que convierte O1 y O2 en una
   forma que no puede separarse otra vez.

## §3 — No-objetivos

1. **NO** se cambia el HTML visible (la tabla de nodos, los estilos, el título).
   Lo que se arregla es el **dato incrustado**, que es donde está el defecto.
2. **NO** se cambia `GraphView`. Este lote no añade campos: consume los que
   `cl-vault-graph` ya añadió.
3. **NO** se toca `export_node`, ni la forma de `window.__vault_nodes__`.
4. **NO** se escribe en ningún vault real.
5. **NO** se reescribe el test existente de `export.rs` (`renders_self_contained_inspector`):
   sus siete aserciones siguen verdad, porque **añadir campos al JSON incrustado
   no cambia ninguna**. Es la comprobación de que el cambio es aditivo.

## §4 — Superficie

| qué | dónde |
|---|---|
| `GraphExport` | `crates/sddk-vault/src/export.rs:90-95` — **3 campos**, ninguno de recuento |
| incrustación | `export.rs:23-27` — construye `GraphExport` desde `&GraphView` |
| test existente | `export.rs:118-138` — 7 aserciones, **no se reescribe** |
| consumidor del HTML | el JavaScript de la página, embebido; **no hay consumidor en el repo** de `__vault_graph__` más allá del test |

## §5 — STOP conditions

1. **Si añadir campos al JSON incrustado rompe el test existente**, se para: ese
   test afirma que el HTML se renderiza entero y autocontenido, y nada de lo que
   se añade debería afectarlo.
2. **Si el HTML no puede llevar los campos sin cambiar su forma pública**
   (`window.__vault_graph__` como objeto), se para y se investiga antes de
   cambiar la clave.
3. **Si O3 no se puede comprobar** —es decir, si no hay forma de extraer el JSON
   del HTML y compararlo con el del grafo—, se para: un objetivo que no se puede
   falsar no es un objetivo.

## §6 — Gates al cerrar

`cargo test --workspace` sin reescribir ningún verde salvo el declarado ·
`cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D warnings` ·
`test_changelog_coverage` · `test_debt_index_coherence` ·
`test_release_state_pointer` · el script de §0 pasa de **3/4 en GAP** a **0**.
