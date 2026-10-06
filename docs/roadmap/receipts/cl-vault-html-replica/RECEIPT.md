# RECEIPT — cl-vault-html-replica

**Bloque:** `cl-vault-html-replica` — bloque de roadmap, **no** es un ciclo del ledger
**Ciclo SDDK:** ninguno. El ledger de `p-63676b11dc0ef88f` da **0 filas** para
este nombre (medido 2026-10-06). El encabezado declaraba antes un `cycle_id`
completo que la autoridad nunca emitió; corregido por INC-DEBT-063.
**Closed:** 2026-10-02T23:55:00Z
**Workspace:** 2.5.3 (declarada, **no publicada**; último tag remoto `v2.5.2`)
**Baseline:** `ce09a855` (`docs(roadmap): session-69h…`)

---

## §1 — Qué se cambió

| commit | concernia | contenido |
|---|---|---|
| `d181b489` | `docs(roadmap)` | SCOPE-CONTRACT y PRE-FLIGHT (`Readiness: READY`) |
| `47d8b169` | `test(vault)` | 3 RED + `serde_json` en dev-dependencies (árbol **rojo a propósito**) |
| `d47e8508` | `fix(vault)` | `impl From<&GraphView> for GraphExport`; R1 **corregido** |
| `3f7efb99` | `chore(changelog)` | entradas `fix(vault)`, `test(vault)` |
| este | `docs(roadmap)` | este recibo, RECONCILIATION en el recibo anterior, punteros |

**Producción tocada:** `crates/sddk-vault/src/export.rs` — **un solo fichero**, y
dentro de él solo la construcción de `GraphExport` y su `From`. El HTML visible
—tabla, estilos, título— no se toca.

## §2 — Comandos ejecutados y resultados

| comando | resultado |
|---|---|
| `cargo test --workspace --no-fail-fast` | **5391 passed, 0 failed**, 280 binarios. Baseline 5388 → **+3**, los tests nuevos |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `python3 ce/06-scan.py` | **CLEAN** |
| `vault/07-medir-html.py` | **0/4 en GAP** (antes 3/4) |
| `vault/06-falsify-graph.py` | **PASS=6 FAIL=0**, sin regresión del ciclo anterior |
| `tests/test_docs_script_contamination.py` | **PASS** — 6 preexistentes en allowlist, 0 nuevas |
| `tests/test_gate_coverage.py` | **PASS** — 41 tests, 36 con runner, 5 excepcionados, **0 huérfanos** |
| `test_changelog_coverage` · `test_debt_index_coherence` · `test_vault_adr_mirror_coverage` · `test_release_state_pointer` · `test_adr_promotion_format` | exit 0 |

**Contexto real vs. sintético:** el script de medición construye un vault de
markdown en un árbol temporal con `XDG_*` propio y `SDDK_DATA_DIR` eliminado, y
extrae el JSON del HTML **del artefacto que produce el binario**, no de una
estructura en memoria. **Ninguna medición de este ciclo se hizo contra el vault
real**, y el ciclo no escribe en él.

## §3 — Objetivos y STOP

| objetivo | veredicto | evidencia |
|---|---|---|
| **O1** el JSON incrustado lleva `cycle_count` y `multiple_cycles` | **CUMPLIDO** | R1, y el script: `multiple_cycles=True` |
| **O2** lleva `topological_order_absent_because` | **CUMPLIDO** | R2: `"cyclic"`; script: `porque='cyclic'` |
| **O3** coinciden campo a campo con `vault graph` | **CUMPLIDO** | R3 compara los cinco campos por valor |

| STOP | veredicto |
|---|---|
| 1 — añadir campos no rompe el test existente | **no se cruzó**: sus 7 aserciones siguen verdad, sin reescribir |
| 2 — el HTML conserva su forma pública | **no se cruzó**: `window.__vault_graph__` sigue siendo el objeto, con la misma clave |
| 3 — O3 tiene que ser comprobable | **no se cruzó**: R3 existe precisamente para eso, y se escribe por qué en el propio test |

## §4 — El guard que estaba mal, no el producto

**El primer R1 afirmaba que la clave `cycle_count` tiene que estar presente, y
falló contra una implementación correcta.** El producto estaba bien.

La forma saturada **omite** `cycle_count` cuando la respuesta es «2 o más»:
`None` *es* la codificación de «2 o más», y `multiple_cycles: true` es lo que la
hace distinguible de «ausente». Exigir la clave habría reintroducido exactamente
la ambigüedad que el ciclo quita, y habría forzado un `cycle_count: null` que
`vault graph` **no emite** — una divergencia de signo contrario.

**El script de medición arrastraba la misma aserción equivocada**, así que primero
falló R1 y después salió un `GAP` residual en el script. Se corrigió **el guard**
en los dos casos: la propiedad comprobada es la **forma** —entero exacto cuando es
0 o 1, `multiple_cycles: true` sin recuento cuando es 2 o más—, no la presencia.

## §5 — RECONCILIATION sobre el ciclo anterior

El §6 del recibo de `cl-vault-graph` afirma que `test_docs_script_contamination` y
`test_gate_coverage` no existen. **Es falso y el error es mío.** Los dos existen
y son `.py`, no `.sh`; están cableados en `release.sh:272,274`. El comprobador
que usé fue `[ -x tests/$t.sh ]`, luego busqué un `.sh` donde hay un `.py`, y en
lugar de dudar del instrumento escribí que no existían y ejecuté otros gates.
Los dos reales se han ejecutado y **pasan**. La corrección está en el recibo
anterior como §9, **conservando §6 sin tocar**.

## §6 — Lo que este ciclo NO resuelve

1. **La familia no está auditada de forma exhaustiva.** Este ciclo llegó a otra
   superficie desde un defecto concreto, no desde un criterio. Puede haber una
   cuarta declaración del mismo grafo que nadie ha mirado. **No medido.**
2. `export_node` y `window.__vault_nodes__` **no se han auditado** contra la
   clase. Fuera del SCOPE por no-objetivo 3, y por tanto **no medido**.
3. La página HTML **no tiene consumidor en el repo** más allá del test: no hay
   JavaScript que lea `__vault_graph__`, luego «el consumidor puede recuperar los
   campos» está probado **contra el artefacto**, no contra una página en
   funcionamiento. **No verificado en navegador.**

## §7 — UAT

No hay ejecución UAT: no hay superficie de usuario final y no hay página en
funcionamiento que recorrer. **No se declara PASS de UAT.** No se ha tocado
ninguna fila de `docs/roadmap/UAT-MATRIX.md`.
