# RECEIPT — cl-vault-graph

**Cycle:** `p-63676b11dc0ef88f/vault-graph`
**Closed:** 2026-10-02T22:40:00Z
**Workspace:** 2.5.3 (declarada, **no publicada**; último tag remoto `v2.5.2`)
**Baseline:** `f1659281` (`docs(roadmap): session-69g, vault search cerrado…`)

---

## §1 — Qué se cambió

| commit | concernia | contenido |
|---|---|---|
| `7a6f6b79` | `docs(roadmap)` | SCOPE-CONTRACT y PRE-FLIGHT (`Readiness: READY`) |
| `fe927da9` | `test(cli)` | 3 RED + R4 de caracterización (árbol **rojo a propósito**) |
| `895f1129` | `fix(vault-graph)` | `GraphView` + `graph_text`; **solución** |
| `93c80e38` | `fix(release)` | remedio del gate de espejo de ADR (concernia **separada**, ver §5) |
| `7f2cb04e` | `chore(changelog)` | entradas `fix(vault-graph)`, `fix(release)`, `test(cli)` |

**Producción tocada:** `crates/sddk-vault/src/graph.rs`,
`crates/sddk-cli/src/vault_cmd.rs`. **Tests nuevos:**
`crates/sddk-cli/tests/vault_graph_declaration.rs`.

## §2 — Comandos ejecutados y resultados

| comando | resultado |
|---|---|
| `cargo test --workspace --no-fail-fast` | **5388 passed, 0 failed**, 279 binarios. Baseline 5384 → **+4**, que son los tests nuevos |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `python3 /var/home/rubentxu/ce/06-scan.py` | **CLEAN** |
| falsificador `vault/06-falsify-graph.py` | **PASS=6 FAIL=0** |
| falsificador `ce/07-falsify-mirror.py` | **PASS=5 FAIL=0** |
| `tests/test_changelog_coverage.sh` | **PASS=54 FAIL=0** |
| `tests/test_vault_adr_mirror_coverage.sh` | exit 0 — **rojo antes**, verde después (ver §5) |
| `tests/test_debt_index_coherence.sh` | PASS=12 FAIL=0 |
| `tests/test_release_state_pointer.sh` | PASS |
| `tests/test_adr_promotion_format.sh` | PASS |
| `tests/test_deny_lint_zero_hits.sh` | exit 0 |
| `tests/test_advisory_lint_explanations.sh` | exit 0 |

**Contexto real vs. sintético:** los falsadores construyen vaults de markdown en
un árbol temporal con `XDG_*` propio y `SDDK_DATA_DIR` eliminado. **Ninguna
medición de este ciclo se hizo contra el vault real**, y el ciclo no escribe en él.
La única escritura fuera del repo fue el espejo de ADR de §5, que es **aditivo**
(3 creados, 54 saltados, 0 sobrescritos).

## §3 — Objetivos y STOP

| objetivo | veredicto | evidencia |
|---|---|---|
| **O1** declarar cuántos ciclos hay, con forma que no pueda mentir | **CUMPLIDO** | F3 (`cycle_count: 1`), F4 (`multiple_cycles: true` + `cycle_count: null`) |
| **O2** la ausencia de `topological_order` explica su causa | **CUMPLIDO** | F5: texto con `topological_order: absent (cyclic)` y `cycle_count: 2 or more` |
| **O3** el caso acíclico no cambia | **CUMPLIDO** | F1 y F2: `node_count` 3, orden de 3, sin recuento |

| STOP | veredicto |
|---|---|
| 1 — declarar el recuento no puede obligar a enumerar ciclos | **no se cruzó**: la forma satura; §1 del SCOPE ya lo había medido |
| 2 — el caso acíclico no cambia | **no se cruzó** (F1, F2, y R4 lo fija como caracterización) |
| 3 — ningún verde reescrito | **VACÍO**, como estaba previsto: el cambio es aditivo y `cli.rs:8579` no se tocó |
| 4 — `vault export` no puede contradecir a `vault graph` | **no se cruzó** (F6) |
| 5 — el recuento declarado tiene que cuadrar | **no se cruzó**: 0/1/`None` es derivación, no recuento |

## §4 — Bugs del propio falsificador

**Dos, y los dos fallaban «por la razón equivocada»** — que es un FAIL que no mide
nada y además tapa el defecto con un mensaje que no lo describe:

1. **F6 escribía fuera del árbol XDG.** `vault export` tiene un guard fail-closed
   (ADR-0082, `writer.rs:76`) que **canonicaliza** el directorio de datos del
   proyecto y **rechaza si no existe**. El arnés escribía en la raíz temporal y
   recibía `STORAGE_WRITER_XDG_VIOLATION`, que se lee como un fallo del producto y
   es el guard funcionando.
2. **F6 buscaba el `project_id` en el árbol equivocado.** Leía
   `XDG_DATA_HOME/sddk/projects/*`, y `vault graph` **no crea** el árbol de data:
   crea el de **state**. El `project_id` se deriva de `--root` (aquí
   `p-97b4d42c4a062826`).

Ambos se corrigieron **en el arnés**. El producto no se tocó para hacerlos pasar,
que es la regla: un FAIL de un falsificador se corrige en el guard.

## §5 — Hallazgo colateral: un gate de release estaba rojo en HEAD

`tests/test_vault_adr_mirror_coverage.sh` es gate del pipeline
(`scripts/release.sh:217,234`) y **fallaba antes de este ciclo**, con 3 ADR
`accepted` sin reflejar: **ADR-0151, ADR-0152, ADR-0153**. Se comprobó que el
fallo es **preexistente**, no introducido aquí: `git stash` de los cambios del
ciclo + gate sobre HEAD limpio → **falla igual**.

Al ejecutar el remedio que el propio gate nombra apareció un defecto mayor: el
guion **no podía funcionar en ninguna máquina que no sea esta**. `REPO_ROOT` está
hardcodeado a `/home/rubentxu/Proyectos/agentesIA/sddk-framework`, que aquí solo
funciona porque es un symlink al disco real. Y el fallo **no habría sido visible**:
`Path.glob` sobre un directorio inexistente devuelve un iterador vacío, luego el
guion imprimía `created: 0, skipped: 0` y salía con **0** — «un PASS que no midió
nada», en el guion que se ejecuta **cuando algo ya ha ido mal**.

Medido antes de repararlo: `glob` sobre ruta ausente devuelve `[]`. Reparado en
`93c80e38` como **concernia separada** (mezclarla con `vault-graph` habría roto la
atomicidad), con `REPO_ROOT` derivado de `__file__` y `main` **fallando cerrado**
en los tres estados vacíos. Falsificado con 5 escenarios: **PASS=5 FAIL=0**.

**Consecuencia para 2.5.3:** el gate está verde, pero 2.5.3 **sigue sin poder
publicarse** por el bloqueo de la clave KMS, que no es de este ciclo.

## §6 — Discrepancia del propio SCOPE, no resuelta

El PRE-FLIGHT §«Gates que deben seguir verdes» nombra
`test_docs_script_contamination` y `test_gate_coverage`. **Ninguno de los dos
existe** con ese nombre en `tests/`. Se ejecutaron los gates de lint que sí
existen y cubren esa intención — `test_deny_lint_zero_hits.sh` y
`test_advisory_lint_explanations.sh`, ambos exit 0 — pero **eso es una
interpretación mía, no equivalencia demostrada**. Se registra sin arreglar: el
nombre correcto de cada gate no se ha determinado.

## §7 — Riesgos

1. `cycle_count` es **saturado por diseño**, no exacto. Un grafo con 200 ciclos
   declara `null` + `multiple_cycles: true`, que es menos información de la que
   un entero daría — a cambio de no poder mentir. Riesgo aceptado y escrito en el
   propio doc del campo.
2. `remove_cycle_edges` reconstruye aristas por id, no por índice, porque los dos
   grafos son objetos separados. Si `sample_cycle` devolviera ids repetidos, el
   `zip` cortaría y quedarían aristas sin quitar; F4 (dos ciclos disjuntos) cubre
   el caso real, **no** un ciclo con nodos repetidos, que `dfs_cycle` no produce.
3. La réplica HTML de `export.rs:24-25,41-42` **no** muestra los campos nuevos. STOP
   4 exige que no se contradigan, y no se contradicen —pero HTML y texto JSON
   dicen cosas distintas. **No verificado** si es intencional.
4. `scripts/mirror_adrs_to_vault.py` deriva ahora `REPO_ROOT` de `__file__`; si
   alguien lo invoca por ruta con symlink intermedio, resuelve distinto. No medido.

## §8 — UAT

No hay ejecución UAT en este ciclo: es un cambio de **forma de salida** sobre un
comando sin superficie de usuario final, y la evidencia son los seis escenarios
del falsificador más los cuatro tests del repo. **No se declara PASS de UAT.**
Los IDs de trazabilidad aplicables son los de `docs/roadmap/UAT-MATRIX.md` para
superficies de CLI; no se ha tocado ninguna fila.
