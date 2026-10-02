# RECEIPT — cl-ledger-watch-total

**Cycle:** `p-63676b11dc0ef88f/ledger-watch-total` — **ciclo real en SDDK**
(`cycle.start` → 4 gates evaluados con evidencia → `OPEN/build`)
**Date:** 2026-10-02T21:12:36Z → cierre 2026-10-02
**Workspace:** 2.5.3 (declarada, **no publicada**; último tag remoto `v2.5.2`)
**Baseline:** `b0153dc0` (`chore(roadmap): reconcilia el puntero de estado tras el lote 2`)

---

## §1 — Qué se cambió

| commit | concernia | contenido |
|---|---|---|
| `c9588db8` | `docs(roadmap)` | SCOPE-CONTRACT, PRE-FLIGHT y EXPLORATION-REPORT |
| `f0fbef78` | `docs(roadmap)` | el mapa objetivo → guard, que el instrumento.converter exige |
| `617b3ab1` | `docs(roadmap)` | DISENO |
| `aa655f70` | `docs(roadmap)` | PLAN |
| `883d0396` | `test(cli)` | R1–R5, **árbol rojo a propósito** (0 passed, 5 failed) |
| `c6890ed7` | `test(cli)` | R6, caracterización |
| `cf56a6c6` | `fix(cli)` | `apply_watch_filters` + `LedgerWatchSummary` + cursor inicial |
| `68cabce6` | `test(cli)` | R6 con fixture que discrimina, tras el veto de M5 |
| este ciclo | `fix(cli)` | el doc de `run_ledger_watch` que el arreglo había partido |
| este ciclo | `chore(changelog)` | las cuatro entradas de este ciclo |

**Producción tocada:** `crates/sddk-cli/src/ledger.rs` — **un solo fichero**,
dentro de `run_ledger_watch` y sus dos ayudantes nuevos. **Ningún** cambio en
storage, ni en `ledger events`, ni en `ledger export`.

## §2 — El defecto, medido

Contra una **copia** del ledger real, con el mtime del original comprobado:

| | antes | después |
|---|---|---|
| texto | `[watch] emitted 5 events, exiting` | `[watch] emitted 5 of 598 (593 not emitted), exiting` |
| JSON | `{"__watch_complete":true,"emitted":5}` | `{"__watch_complete":true,"emitted":5,"total_events":598,"pending":593}` |
| total real | — | **598** |

5 + 593 = 598. El único GAP que queda es `ledger export`, **no-objetivo
declarado**.

## §3 — Comandos ejecutados y resultados

| comando | resultado |
|---|---|
| `cargo test --workspace --no-fail-fast` | **5403 passed, 0 failed**, 24 ignored, **282 binarios** (baseline 5397, **+6**) |
| `cargo test -p sddk-cli` | **1449 passed, 0 failed**, sin reescribir un verde |
| `cargo fmt --check` | exit 0 (tras `cargo fmt`) |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `tests/test_changelog_coverage.sh` | **PASS=63 FAIL=0** |
| `tests/test_debt_index_coherence.sh` | exit 0 |
| `tests/test_docs_script_contamination.py` | exit 0 |
| `python3 ce/06-scan.py` | **CLEAN** |
| `03-medir-watch.py` (ledger real) | **0/3 GAP** en `ledger watch`; 1 GAP en `ledger export` |
| `04-req-testable.py` | 4 objetivos, 5 guards, cobertura cerrada |
| `05-diseno.py` | una regla, un filtro, un struct, un derivado |
| `06-plan.py` | 5 ficheros, 3 instrumentos, 1 script |
| `07-falsify-watch.py` | **5/5 mutaciones detectadas** |

## §4 — El ciclo en la autoridad, no en un documento

Es el **primer** ciclo de esta sesión que nace en SDDK. Cuatro gates, cada uno
con su `argv`, `exit_code` y `output_digest` reales:

| gate | receipt | instrumento |
|---|---|---|
| `exploration-sufficient` | `gate-exploration-sufficient-d037420d9ab7c9d9-1` | `03-medir-watch.py` |
| `requirements-testable` | `gate-requirements-testable-3f183a668f87f696-1` | `04-req-testable.py` |
| `architecture-consistent` | `gate-architecture-consistent-27d7d22cb33abb86-1` | `05-diseno.py` |
| `plan-executable` | `gate-plan-executable-4466ec08f85b6084-1` | `06-plan.py` |

**Ninguno se estampó.** El primero corre con `exit_code: 1` y eso se declara en
la propia evidencia, con su significado: 4 de 9 comprobaciones en GAP, 3 de ellas
el defecto que este ciclo cierra y 1 el no-objetivo.

## §5 — Lo que el ciclo encontró que no buscaba

1. **Los ciclos de 69h, 69i y 69k no existen en SDDK.** Sus recibos declaran un
   `cycle_id` que la autoridad nunca emitió. Se declara y **no se corrige**
   retro-creándolos: eso sería escribir historia en la autoridad, que es el
   fallo de `INC-DEBT-061` aplicado a los recibos propios. Queda como decisión
   del operador (§8).
2. **`SDDK_DATA_DIR` no manda sobre el ledger.** Un `cycle list` con esa variable
   puesta leyó el ledger real: el ledger vive bajo `XDG_STATE_HOME`, y
   `SDDK_DATA_DIR` solo rige el control-plane. Un `cycle start` sobre un almacén
   vacío falla por `FOREIGN KEY`, luego el ciclo tiene que nacer en la autoridad
   real.
3. **8 eventos nuevos en el ledger**, todos de este ciclo: 1 `cycle.created`,
   4 `cycle.transitioned`, 3 `workflow.*`. De 590 a 598. Ninguna escritura
   ajena a este ciclo.

## §6 — Instrumentos que hubo que corregir, no productos

Cinco veces el FAIL era del guard. Vale la pena enumerarlas porque son la
mitad del valor de este ciclo:

1. El detector de `watch` buscaba `of`/`total`/`pending` en **toda** la salida, y
   el payload de los eventos contiene esas palabras: podía pasar con el defecto
   presente.
2. `ledger export` se invocó con `--format`, que **no existe**: 2 GAP que eran
   un error mío de invocación.
3. El detector de `export` contaba los dígitos del **tmpdir** (`[0, 5, 9]`).
4. `04-req-testable.py` falló con 6 problemas porque **el mapa objetivo → guard no
   existía** en el PRE-FLIGHT: los guards estaban en prosa y nada decía cuál
   cubría cuál.
5. `06-plan.py` trataba «lo que este plan va a crear» como «lo que no existe»,
   lo que hacía el gate **insatisfacible**: un gate que no puede pasar es un
   adorno.
6. **El falsificador restauraba con `git checkout`, y eso le borró trabajo ajeno.**
   `git checkout -- <fichero>` repone el **último commit**, luego revirtió el
   arreglo del doc de `run_ledger_watch` que estaba sin commitear. La
   comprobación de sha lo delató —«la restauración NO devolvió el fichero»— y
   por eso el aviso existe, pero **un instrumento que puede perder trabajo no es
   un instrumento aceptable**. Ahora guarda y repone **bytes**, y verificó que
   no vuelve a usar git para eso.
7. **Las anclas de M3 y M5 dejaron de existir** cuando `cargo fmt` reindentó las
   llamadas a `apply_watch_filters`. El falsificador **se negó a declarar
   detección**: dijo «el texto a mutar NO existe — el guard no mide lo que dice
   medir» y salió en 1. Es el comportamiento correcto, y es la razón por la que
   un guard que no puede correr se delata en vez de dar un PASS vacío. Las anclas
   se actualizaron a la forma que produce `cargo fmt`, y la corrida dio **5/5**.

**Sexta vez en esta sesión que un guard se delata a sí mismo antes que dar un
PASS falso**, y las tres últimas por la misma causa: el instrumento no medía lo
que declaraba.

## §7 — Dos defectos en cosas que yo acababa de escribir

1. **El arreglo partía el doc de `run_ledger_watch`.** Al insertar
   `apply_watch_filters` entre su comentario y la función, `run_ledger_watch` se
   quedó **sin documentación** y la nueva heredó la lista «Exit conditions».
   Lo detectó `clippy` con `doc_lazy_continuation`, que yo leí como un problema
   de formato y era un problema de acoplamiento. Se movió el bloque **antes** del
   doc original.
2. **M5 no lo detectaba ningún test, y el guard era el culpable.** La primera R6
   elegía el ciclo con **menos** eventos; en un fixture de ciclos de un solo
   evento eso es el **primero** del ledger, luego una corrida sin filtrar emitía
   justo ese y parecía correcta. Con la corrección —apuntar al **último** ciclo
   escrito y subir el tope por encima de su propia cuenta— M5 cae.

**Sexta vez en esta sesión que una herramienta encuentra un defecto en algo que
el propio agente acaba de escribir.** Y la segunda de esta sesión que el
instrumento tenía la culpa y no el producto.

## §8 — Lo que queda abierto, escalado al operador

1. **Clave KMS** — único bloqueo de 2.5.3.
2. **INC-DEBT-050** — las dos salidas.
3. **INC-DEBT-061** — los 51 ciclos.
4. **Los tres recibos con `cycle_id` inexistente** (§5.1). Dos salidas: enmendar
   los recibos para que declaren lo cierto, o registrar la divergencia como
   deuda. Se recomienda la primera. **No se ejecuta aquí**: son documentos de
   ciclos cerrados y reescribirlos es otra concernia.
5. **79 filas `__spine_import__` y 23 ciclos sin hecho** (17 `OPEN`).
6. **INC-DEBT-049**.
7. **`ledger export`** — misma clase, medido, ciclo siguiente.

## §9 — Una nota de higiene que no se arregla

El commit `ee7aa82a` (publicado) tiene el **subject partido en dos líneas**: el
`%s` de git concatena las líneas no vacías, así que su resumen se lee entero en
una línea. Es cosmético y el commit es válido, pero **reescribir historia
publicada no procede** en un proyecto lineal con tags. Se deja anotado para que
no se confunda con un defecto del mensaje actual, que sí lleva la línea en blanco
correcta.

## §10 — Contexto real vs. sintético

- Todo el trabajo de código se hizo en el **checkout real**.
- La medición del defecto y la de la verificación corren contra una **copia** del
  ledger en un árbol temporal con `XDG_*` propio y `SDDK_DATA_DIR` eliminado, y
  el mtime del original se comprueba antes y después.
- Lo **único** que se escribió en el ledger real son los **8 eventos** de este
  ciclo (§5.3), y son los que la autoridad exige para registrar un ciclo.
- El ledger real **no se puede** meter en el arnés de los tests: la suite escribe
  en él, luego los fixtures son sandboxes con `adopt apply` + `cycle start`.
