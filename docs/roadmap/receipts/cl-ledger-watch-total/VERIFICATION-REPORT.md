# VERIFICATION-REPORT — cl-ledger-watch-total

**Cycle:** `p-63676b11dc0ef88f/ledger-watch-total` (`OPEN/verify`)
**Subject SHA:** `4f67bfa2` (`== origin/main` en el momento de verificar)
**Workspace:** 2.5.3 declarada, no publicada (último tag remoto `v2.5.2`)
**Date:** 2026-10-02

---

## §1 — Qué se afirma, y qué no

**Se afirma:** que `ledger watch` declara cuántos eventos existían para su
consulta, y que esa declaración es la de **esa** consulta —su cursor inicial y
sus filtros— y no un número más barato de otro sitio.

**No se afirma:** que `ledger watch` deba emitir el ledger entero. `--max-events`
y `--from-tail` son peticiones legítimas. Lo que se verifica es que el comando
**no presenta una ventana como si fuera el total**.

## §2 — Verificación de que el defecto estaba y de que ya no

El mismo instrumento, antes y después, sobre la misma clase de fixture:

| | antes (`4f67bfa2^`) | después (`4f67bfa2`) |
|---|---|---|
| texto | `[watch] emitted 5 events, exiting` | `[watch] emitted 5 of 598 (593 not emitted), exiting` |
| JSON | `{"__watch_complete":true,"emitted":5}` | `{"__watch_complete":true,"emitted":5,"total_events":598,"pending":593}` |
|GAP en `ledger watch` | **3 de 3** | **0 de 3** |

El total (598) se lee de `ledger events`, la superficie hermana que ya declara:
**los tests y el instrumento comparan dos comandos entre sí**, no contra un
literal.

## §3 — Falsificación: 5 mutaciones, 5 detectadas

Un test verde solo demuestra que el test y el producto están de acuerdo. Las
mutaciones comprueban que están de acuerdo **por la razón correcta**.

| # | mutación del producto | guard que cae |
|---|---|---|
| M1 | `pending` escrito a mano (constante) | R3 |
| M2 | total contado desde el cursor **final** | R1, R3 (+R2, R4) |
| M3 | total **sin filtro** — el `COUNT(*)` barato | R4 |
| M4 | declarar **solo** cuando queda algo fuera | R1 |
| M5 | el **bucle** sin filtro, el total con filtro | R6 |

**M2–M5 no se pueden satisfacer con el defecto presente**, y M3 en concreto es
la que impide la solución que se descartó: imprimir *cualquier* número pasa R1 y
R2.

## §4 — Perfil completo

| gate | resultado |
|---|---|
| `cargo test --workspace --no-fail-fast` | **5403 passed, 0 failed**, 24 ignored, **282 binarios** (baseline 5397, **+6**) |
| `cargo test -p sddk-cli` | **1449 passed, 0 failed** |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `tests/test_changelog_coverage.sh` | **PASS=64 FAIL=0** |
| `tests/test_debt_index_coherence.sh` | exit 0 |
| `tests/test_release_state_pointer.sh` | exit 0 |
| `tests/test_docs_script_contamination.py` | exit 0 |
| scanner de alfabetos no latinos | **CLEAN** |

**Ningún verde reescrito.** Los cambios son aditivos: el JSON gana campos y
conserva `__watch_complete`; el texto cambia de forma.

## §5 — Deuda: severidad y prioridad

| id | severidad | prioridad | por qué |
|---|---|---|---|
| **INC-DEBT-062** `ledger export` declara solo los que exportó | **high** | **P1** | cuarta superficie de la misma clase; trunca en un **fichero**, luego el artefacto parece completo. Sin pérdida de datos → `high`, no `critical` |
| **INC-DEBT-063** tres recibos declaran un `cycle_id` inexistente | **medium** | **P2** | daño documental; no toca el runtime. **No se corrige** retro-creando ciclos: eso es escribir historia en la autoridad |

Ambas nacieron de los gates `debt-severity-assigned` y
`debt-priority-assigned`, que exigían exactamente esto.

## §6 — Lo que NO se verificó, y se declara

1. **El coste con un ledger grande.** La declaración añade una materialización
   del stream al final de la corrida. Es **una iteración más** de un trabajo que
   el bucle ya hace cada 500 ms (`canonical_events` recorre todos los streams en
   cada poll), pero no se midió con un ledger de 10⁵ o 10⁶ eventos. **No
   medido.**
2. **La declaración en un stream vivo.** El total es una foto del final de la
   corrida; los eventos que llegan durante ella cuentan hacia el total y no se
   emitieron. Es cierto en los dos sentidos, y no se ejercitó con un escritor
   concurrente. **NOT_RUN.**
3. **`--from-tail` con eventos nuevos llegando durante la corrida.** La ruta
   usa el mismo código y el mismo filtro, pero no se midió. **NOT_RUN.**

## §7 — Contexto

Todo el código se hizo en el checkout real. La medición corre contra **copia**
del ledger en árbol temporal con `XDG_*` propio y `SDDK_DATA_DIR` eliminado, y el
mtime del original se comprueba antes y después. Lo único escrito en el ledger
real son los **10 eventos** de este ciclo en la autoridad: 1 `cycle.created`, 5
`cycle.transitioned`, 3 `workflow.*` y 1 recibo de gate.
