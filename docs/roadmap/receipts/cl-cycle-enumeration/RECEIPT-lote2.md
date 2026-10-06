# RECEIPT — cl-cycle-enumeration, lote 2

**Bloque:** `cl-cycle-enumeration` — bloque de roadmap, **no** es un ciclo del ledger
**Ciclo SDDK:** ninguno. El ledger de `p-63676b11dc0ef88f` da **0 filas** para
`cycle-enumeration` (medido 2026-10-06). El encabezado declaraba antes un
`cycle_id` completo que la autoridad nunca emitió; corregido por INC-DEBT-063.
**Authority:** [INC-DEBT-060](../../../debt/INC-DEBT-060-NO-SURFACE-ENUMERATES-CYCLES-97-OF-179-ARE-NAMED-BY-NO-COMMAND.md) (`high`/`P1`, `open`)
**SCOPE:** [SCOPE-CONTRACT.md](./SCOPE-CONTRACT.md) · **PRE-FLIGHT:** [PRE-FLIGHT.md](./PRE-FLIGHT.md) (`Readiness: READY`)
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)
**Fecha:** 2026-10-02

---

## 1. Qué entrega

`Storage::list_cycles` / `Storage::list_cycles_by_status` y `sddk cycle list`, que
cerran O1–O4 del SCOPE. El árbol pasó de **ROJO a propósito** (3 tests de CLI
RED por subcomando inexistente, commit `7ec7100e`) a **verde**.

| Fichero | Cambio |
|---|---|
| `crates/sddk-storage/src/lib.rs` | `CycleSummary` + `list_cycles` + `list_cycles_by_status` (+109) |
| `crates/sddk-storage/tests/cycle_enumeration.rs` | **nuevo**, 6 tests de storage |
| `crates/sddk-cli/src/cycle.rs` | `CycleCommand::List`, `run_cycle_list`, `CycleListOutput` (+128) |
| `crates/sddk-cli/tests/cycle_list_e2e.rs` | 4 tests e2e (3 del lote 1, +R5) |
| `docs/architecture/tests/fixtures/cli_golden/1.168.8/sddk-cycle-help.txt` | +1 línea: el subcomando nuevo |

## 2. Verificación realmente ejecutada

Todo local, nada cloud. No se ejecutó nada que no se liste.

```
cargo fmt --check                                          limpio (solo 4 ficheros del lote)
cargo clippy --workspace --all-targets -- -D warnings      EXIT=0
cargo test --workspace --no-fail-fast                      EXIT=0
                                                            5374 passed / 0 failed / 24 ignored
                                                            276 binarios de test
  - tests/cycle_enumeration.rs        6 passed
  - tests/cycle_list_e2e.rs           4 passed
  - tests/cycle_manifest_readability.rs 3 passed
  - tests/cli_golden.rs               1 passed  (tras regenerar el fixture)
```

El perfil completo lo exige §6 del SCOPE y es lo que se corrió; no se sustituyó
por el lote dirigido.

## 3. `cli_golden` cayó, y por qué eso es buena señal

La primera pasada de `cargo test --workspace` **falló** en
`cli_golden_surface_matches_blessed_snapshot`. El delta era de una línea: el
subcomando `list` aparecía en `sddk cycle --help`.

Dos cosas que se hacen aquí y no en cualquier sitio:

1. **La deriva se regeneró, no se ajustó el test.** El propio mensaje del test da
   el comando, y el diff se revisó línea a línea antes de aplicarlo: **exactamente
   una línea añadida**, ninguna otra movida. Un snapshot que se regenera sin mirar
   el diff es un snapshot que deja de medir.
2. **El primer comando de la sesión lo enmascaró.** Se ejecutó
   `cargo test --workspace | tail -60`, y el código de salida que devolvió el
   pipeline es el de `tail`, no el de `cargo`: **parecía un `EXIT=0` con la suite
   roja debajo.** Se repitió con `> log 2>&1; echo EXIT=$?`, que es la forma que
   no puede mentir. Queda escrito porque un gate que se lee por su exit code y
   está envuelto en un pipe no es un gate.

## 4. El falsificador de R6 falló, y el fallo era del falsificador

`/var/home/rubentxu/ce/01-falsify.py`, contra una **copia byte-idéntica** del
ledger real (sha256 `91ea0352…fbf32c`, verificado antes y después).

```
python3 /var/home/rubentxu/ce/01-falsify.py
  FAIL: F1-declared-count-matches-table   -- declared=100 table=179
  FAIL: F2-eventless-cycles-now-named     -- 23/102 eventless cycles named
  PASS=6 FAIL=2
```

La lista devolvió 100 y el baseline pedía 179. **El producto era correcto**: el
baseline contaba `SELECT COUNT(*) FROM cycles` sin filtro de proyecto, y esa tabla
contiene dos poblaciones (§5). Era la tercera vez en este ciclo que un FAIL
resulta ser un guard mal escrito; las dos anteriores también se corrigió el
guard, nunca el producto.

La reparación correcta era la que menosTAIN mejor no se tomaba: el camino fácil
era cambiar `list_cycles` para que enumerase las 179 filas, y habría hecho pasar
el falsificador, y habría sido **un defecto** —filtrar por proyecto es lo que la
función debe hacer.

Guard con el baseline corregido, más uno nuevo que fija la distinción para que no
vuelva a derivar sola:

```
PASS=9 FAIL=0
  F0-copy-is-byte-identical              copy sha256 == real sha256
  F1-declared-count-matches-table        declared=100 table=100
  F2-eventless-cycles-now-named          23/23 named, missing=[]
  F3a-unreadable-declared                declared=2, rows_marked_false=2
  F3b-readable-rows-marked               readable=98 unreadable=2 overlap=[]
  F3c-count-reconciles                   100 == 98 + 2
  F4-real-ledger-untouched               sha256 idéntico antes y después
  F5-listing-excludes-the-other-project  79 filas de spine, ninguna en la lista
```

**F4 es la prueba de solo lectura**, no su declaración: el sha256 del ledger real
es el mismo antes y después. La intención de solo lectura se+violó
estructuralmente —la CLI solo ve una copia— y luego se verificó.

## 5. La corrección de magnitud

```
ALL rows in cycles                          : 179
  project_id = p-63676b11dc0ef88f           : 100
  project_id = '__spine_import__'           :  79
forma de cycle_id
  <project_id>/<slug>                       : 100
  slug desnudo                              :  79
proyectos en ESTE ledger
  ('p-63676b11dc0ef88f', 'sddk-framework')
  ('__spine_import__', 'Spine Import Project')
de los 100 de este proyecto
  nombrados por un evento                   :  77
  NO nombrados por ninguna superficie       :  23  (17 OPEN)
  manifiesto legible / ilegible             :  98 / 2
de los 79 de __spine_import__
  con algun evento                          :   0
```

`__spine_import__` **no es un marcador**: es una fila real de `projects`, con su
propio `display_name` y su propio workspace (`'spine-import'`, que no es una
ruta). Las 79 filas con `manifest_json = {}` que el SCOPE atribuía a «este
proyecto» son exactamente esa población.

**Consecuencia publicada:** el titular de INC-DEBT-060 y la tabla §0 del SCOPE
usaban 179 / 97 / 91 / 81. Los valores reales para este proyecto son **100 / 23 /
17 / 2**. La magnitud era casi **4× mayor** de lo declarado. Corregido en los tres
sitios —documento de deuda (bloque de corrección delante + addendum), índice
`docs/debt/README.md`, y enmienda de §0 del SCOPE— **sin borrar la evidencia
original**, que se conserva tachada y marcada.

**La clase de defecto no cambia**, y por eso la deuda sigue `open` y `high`: sigue
sin existir superficie que declare qué son las 79 filas importadas, y 17 ciclos
`OPEN` no estaban nombrados por nada. Con la magnitud corregida, **bajar a
`medium` es decisión del operador** y no se toma aquí.

## 6. STOP conditions del SCOPE

| STOP | ¿Disparado? |
|---|---|
| 1 — tocar `get_cycle` | **No.** Sin cambios en `get_cycle`/`cycle_from_row`. R4 sigue verde y sigue characterizing D2: `get_cycle` sigue dando error ante `{}`. |
| 2 — reescribir un test verde | **No.** Los 6 de caracterización de `cycle_manifest_readability` pasan sin tocar. Ningún test verde se reescribió; `cli_golden` se **regeneró su fixture** por una adición de superficie declarada, no para acomodar un fallo. |
| 3 — migración de esquema | **No.** Cero migraciones; `SELECT` y nada más. |
| 4 — R4 no cae | **No.** R4 cae: `get_cycle` sigue dando error para `{}`. La premisa de D2 se sostiene, con 2 filas en vez de 81. |
| 5 — R6 no cuadra | **No.** Cuadra: `cycles: 100` = 98 legibles + 2 ilegibles. Se investigó la causa del descuadre inicial en vez de ajustar el test. |

## 7. Almacenamiento real

**No se escribió ni una fila.** Verificado por hash, no por intención:

```
antes : 91ea03529587553ace193bd05eee9aceace84a0b2809f764bdd1f72c7efbf32c
después: 91ea03529587553ace193bd05eee9aceace84a0b2809f764bdd1f72c7efbf32c
```

El razonamiento de por qué hizo falta copia: `RuntimeContext::open`
(`cycle.rs:437`) no abre en solo lectura — su tercer parámetro es
`generate_seed`, y dentro llama a `Storage::open`, que abre la conexión en
escritura. Apuntar la CLI al ledger real lo habría mutado. La copia se verifica
byte a byte antes de usarla, para que el ejercicio siga siendo sobre los datos
reales.

## 8. Contaminación

`[\u0400-\u04FF\u0600-\u06FF\u0900-\u097F\u0E00-\u0EFF\u3040-\u30FF\u3400-\u4DBF\u4E00-\u9FFF\uAC00-\uD7AF\ufffd]`
sobre los 5 ficheros del lote: **CLEAN**.

Segunda clase (token de otro idioma dentro de una palabra, o prosa española con
palabra inglesa suelta), `02-mixed.py`: **NO MIXED LANGUAGE**. Los comentarios del
código van en inglés, que es la convención de los crates vecinos.

**La redacción de este mismo lote se contaminó nueve veces**, y una de ellas fue
este párrafo: al describir los caracteres CJK que había que corregir, los escribí
literalmente dentro de la frase que los prohibía. Se corrige aquí nombrándolos por
bloque de Unicode (`CJK Unified Ideographs`) y no por su forma.

Se declara porque el control de contaminación es **retroactivo sobre lo ya
escrito**, no sobre la intención al escribir: cada borrador necesita su propio
escaneo justo después de redactarlo, y por eso este párrafo existe — para que el
próximo lote no lo tome por opcional. Todas las ocurrencias detectadas se
corrigieron antes de commitear; ninguna llegó al árbol.

## 9. Lo que este lote NO cierra

- **INC-DEBT-060 sigue `open`.** Se cierra por decisión del operador cuando se
  resuelvan las 79 filas de `__spine_import__` y los 23 ciclos `OPEN` sin hecho.
- **F63 no se toca.** `sddk ledger events` sigue truncando en 50 de 590 sin
  declararlo. Es otra superficie y otro defecto (§2.3), y queda como slice propio.
- **`__spine_import__` no se midió.** Si esas 79 filas son alcanzables desde algún
  checkout es una pregunta **sin medir**, y se declara sin medir. Este lote solo
  estableció que no son ciclos de este proyecto.
- **No se toca `get_cycle`.** Las 2 filas ilegibles de este proyecto siguen sin
  poder leerse por `get_cycle`, solo listarse marcadas. Es STOP 1.

## 10. Ficheros de falsificación (fuera del repo)

```
/var/home/rubentxu/ce/01-falsify.py        R6 + solo lectura (PASS=9 FAIL=0)
/var/home/rubentxu/ce/02-mixed.py         contaminación de 2ª clase
/var/home/rubentxu/ce/03-project-scope.py por qué 179 vs 100
/var/home/rubentxu/ce/04-spine-import.py  caracterización de __spine_import__
/var/home/rubentxu/ce/05-readme-entry.py   reescritura de la entrada del índice
```
