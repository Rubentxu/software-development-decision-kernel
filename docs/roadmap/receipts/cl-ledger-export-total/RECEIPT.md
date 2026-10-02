# RECEIPT — cl-ledger-export-total

**Cycle:** `p-63676b11dc0ef88f/ledger-export-total`
**Session:** session-69m
**Date:** 2026-10-03
**Concernia:** la declaración de `sddk ledger export`, y nada más
**Deuda que cierra:** INC-DEBT-062 (`ledger_export_truncation_undeclared`)

---

## Qué se midió antes de tocar nada

Con `/var/home/rubentxu/f63/09-medir-export.py`, sobre una **copia byte-identica**
del ledger real (el `mtime` del original se comprueba antes y después):

```
GAP : `ledger export` declara cuantos eventos existian en total
      -- numeros que declara=[5], total real=600
GAP : `ledger export` tiene una salida que una maquina pueda leer
      -- con --format json -> exit=2: unexpected argument '--format'
```

**2 de 6 comprobaciones en rojo.** El comando salía con **exit 0** escribiendo
5 eventos de 600 a un fichero y respondiendo `exported 5 events to <path>`.

El segundo GAP no apareció buscando truncamientos: apareció leyendo el mismo
fichero. **`ExportOutput` derivaba `Serialize` y nunca se serializaba.** El
resumen era un `format!` escrito a mano dentro del `match` de éxito, y el
comando no tenía `--format` ninguno. La forma declarada no era la que estaba en
vigor, y sin `--format` una máquina no tenía dónde leer la respuesta.

## Qué se cambió

Un solo fichero de producto, `crates/sddk-cli/src/ledger.rs`:

- `LedgerExportArgs` gana `format: OutputFormat` con default `text`.
- `ExportOutput`: `count` → `written`, más `total_events`, más `fn pending()`
  derivado por `saturating_sub`.
- `total_events = all_events.len()` **antes** del `.take(limit)`.
- `fn export_text(&ExportOutput) -> String`, y texto y JSON leen el mismo struct.
- `impl Serialize for ExportOutput` **manual**, no derivado.

Dos decisiones que no son obvias:

**El `Serialize` es manual porque el derivado no puede servir.** `#[derive(Serialize)]`
emite *campos*, y `pending()` es un *método*: el primer intento dejó la forma
derivada fuera del JSON sin que ningún test lo dijera. Añadir `pending` como
campo arregla eso y reintroduce el otro defecto —un tercer número almacenado que
una edición posterior puede poner en contradicción con los otros dos—. El
doc del impl (`ledger.rs:591`) escribe los dos modos de fallo y por qué se
descartaron.

**No se extrae función de filtro, al contrario que en `ledger watch`.** Allí el
`retain` corría dentro del bucle de sondeo sobre una página acotada, y por eso
necesitaba un único sitio compartido entre el bucle y la cuenta. Aquí hay **un**
vector y **un** filtro, ya aplicado por elegir qué listado llamar. Copiar el
remedio anterior habría añadido una segunda regla que no puede divergir porque
no hay nada de qué divergir. Un remedio se porta, no se copia.

## Verificación

| Comprobación | Resultado |
|---|---|
| `cargo test -p sddk-cli --test ledger_export_declaration` | **6/6** |
| `cargo test -p sddk-cli` | **1456 passed / 0 failed** (base 1449) |
| `cargo test --workspace --no-fail-fast` | **5409 passed / 0 failed / 24 ignored / 283 binarios** (base 5403 / 24 / 282) |
| `cargo fmt --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `tests/test_changelog_coverage.sh` | **PASS=66 FAIL=0**, 63 commits feat/fix/test representados |
| `tests/test_debt_index_coherence.sh` | **PASS=12 FAIL=0** |
| `10-falsify-export.py` (5 mutaciones) | **5/5 DETECTADA**, exit 0 |
| `09-medir-export.py` (6 comprobaciones) | **0/6 GAP**, exit 0 |
| `13-falsify-sonda.py` (3 mutaciones) | **3/3**, exit 0 |
| scanner de caracteres no latinos | CLEAN |

La aritmética del workspace **cierra sola**: el único binario nuevo es
`ledger_export_declaration` con sus 6 tests, y 5403 + 6 = 5409, 282 + 1 = 283.
Ningún verde fue reescrito.

### Medición tras el cambio

```
OK : `ledger export --limit 5` escribe cinco y sale con 0
     -- dice='exported 5 of 608 events (603 not written) to /…/export.jsonl'
OK : `ledger export` declara cuantos eventos existian
     -- numeros que declara=[5, 603, 608], total real=608
OK : `ledger export` tiene una salida que una maquina pueda leer
     -- '{"path":"/…/export.jsonl","written":5,"total_events":608,"pending":603}'
OK : el resumen JSON no se produce serializando `ExportOutput`
OK : la copia del ledger no se ha modificado -- mtime antes=… despues=…
```

## Lo que la falsificación encontró, y dónde estaba el defecto

### Un guard que no podía fallar

El falsificador cerró con `MUTACIONES NO DETECTADAS: 1` para M1
(`total_events = 1usize`), porque R2 no cayó — cuando **R1 y R3 la detectaron**.

Medido antes de tocar el guard, con `11-medir-fixture-r2.py` sobre el binario
real y los mismos fixtures que el test: **1 ciclo deja 1 evento, 2 dejan 2, 3
dejan 3**. El fixture de R2 era de un solo ciclo, luego su total real era
exactamente 1 y la constante `1usize` **coincidía con la verdad**. La aserción
`total_events == total` no podía caer nunca ahí. No era un defecto del producto:
era un fixture que no discrimina, y una aserción que no puede fallar por la razón
que nombra es decoración.

Corregido **en el guard**: el fixture abre dos ciclos y afirma `total > 1`, de
modo que la vacuidad no se pueda reintroducir en silencio.

### El instrumento tenía su propio defecto

`10-falsify-export.py` confundía *"este guard no disparó"* con *"esta mutación
sobrevivió"*. Son cosas con consecuencias opuestas —la primera es defecto del
producto, la segunda es la matriz del falsificador mal escrita— y las reportaba
bajo el mismo encabezado y con el mismo código de salida. Un instrumento con una
lista mal escrita no puede parecer que ha encontrado un fallo de producto. Ahora
son tres desenlaces: `SOBREVIVIDA`, `DETECTADA` y `DERIVA`.

### La sonda de medición mentía tres veces

`09-medir-export.py` reportaba `GAP` en una línea que decía
`deriva Serialize=True, el RESUMEN se serializa=True`. Medido con
`12-medir-sonda.py`, eran tres defectos distintos:

1. **Polaridad invertida**: pedía `declared and not summary_serialized`, o sea
   que reportaba GAP exactamente cuando el resumen **sí** se serializaba. No
   podía decir OK nunca.
2. **Leía prosa, no código**: buscaba el literal `#[derive(Serialize)]` y lo
   encontraba en la línea 594, que está **dentro del doc comment** de la
   implementación manual —el doc cita el derive para explicar por qué no se
   usa—. Un detector que busca una palabra encuentra la palabra, y un doc que
   explica el defecto es, para un detector textual, el defecto.
3. **Anclaba un detalle de implementación**: exigía que `ExportOutput` *derivara*
   `Serialize`, que es exactamente lo que el arreglo abandonó a propósito. Una
   comprobación que falla cuando la implementación mejora está midiendo el
   mecanismo, no la propiedad.

Reescrita para preguntar la propiedad —`ExportOutput` es serializable, y el
resumen sale de serializarlo— y para leer el fuente **con los comentarios de
línea eliminados antes de buscar**.

### Y la sonda reparada se falsificó antes de creérsela

Corregir un detector y verlo decir «OK» no prueba nada: un detector que dice OK
siempre también pasa. `13-falsify-sonda.py` mete el defecto de vuelta en el
fuente y exige que la sonda vuelva a reportarlo:

```
OK : M1 el resumen pasa a un `json!` paralelo -- la sonda vuelve a reportarlo
OK : M2 ExportOutput deja de ser serializable -- la sonda vuelve a reportarlo
OK : M3 un doc que cita el derive no cuenta como capacidad -- la sonda lee codigo
```

M3 es la novena vez que un detector lee prosa, y la prueba de que el filtro de
comentarios funciona.

## Documentos y deuda

- `EXPLORATION-REPORT.md`, `SCOPE-CONTRACT.md` (O1–O5), `PRE-FLIGHT.md`
  (R1–R5, `Readiness: READY`), `DESIGN.md`, `PLAN.md`, este `RECEIPT.md` y
  `VERIFICATION-REPORT.md`.
- **INC-DEBT-062** pasa a `status: resolved`. Su sección *Medición* citaba
  `03-medir-watch.py` y 598 eventos; lo correcto es `09-medir-export.py` y 600
  al abrir el ciclo. Corregido al cerrar, no antes: una cita de medición
  equivocada en un documento de deuda es un documento que afirma algo falso
  sobre cómo se encontró el defecto.
- `docs/debt/README.md` actualizado; `test_debt_index_coherence` PASS=12.

## Fuera de alcance, declarado

- No se arregla `ledger watch`: cerrado y verificado en el ciclo anterior.
- No cambia el payload del fichero exportado, ni `--limit 0`, ni los filtros.
- No se repite la auditoría de superficies que truncan: medida y agotada.
- No se bumpea la versión: workspace **2.5.3** sobre tag publicado `v2.5.2` →
  la siguiente release **es 2.5.3**.
- La release sigue bloqueada por la **clave KMS**, que es una decisión del
  operador. Este ciclo no la necesita.

## Commits

```
f47db330  docs(roadmap): session-69m, cl-ledger-export-total SCOPE/PRE-FLIGHT/DISENO/PLAN
6ca0bdca  test(cli): R1-R5, ledger export debe declarar cuantos eventos existian
2226bb9f  fix(cli): ledger export declara cuantos eventos existian, y tiene forma legible
04e129f7  test(cli): R2 con fixture que puede discriminar, tras una deriva que el
          falsificador reporto como mutacion sobreviviente
```

Contexto **real** en todas las verificaciones: el ledger que se mide es una copia
byte-identica del de `p-63676b11dc0ef88f`, y los fixtures de los tests corren
contra el binario real en un directorio temporal con `SDDK_DATA_DIR` eliminado.
Ningún resultado es simulado.
