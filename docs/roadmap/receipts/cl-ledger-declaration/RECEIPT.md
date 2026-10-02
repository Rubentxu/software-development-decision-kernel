# RECEIPT — cl-ledger-declaration

**Cycle:** `p-63676b11dc0ef88f/ledger-declaration`
**Authority:** [INC-DEBT-060](../../../debt/INC-DEBT-060-NO-SURFACE-ENUMERATES-CYCLES-97-OF-179-ARE-NAMED-BY-NO-COMMAND.md) (`high`/`P1`, `open`), falsificador **F63**
**SCOPE:** [SCOPE-CONTRACT.md](./SCOPE-CONTRACT.md) · **PRE-FLIGHT:** [PRE-FLIGHT.md](./PRE-FLIGHT.md) (`Readiness: READY`)
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)
**Fecha:** 2026-10-02

---

## 1. Qué entrega

`sddk ledger events` deja de truncar en silencio y `--limit 0` deja de significar
«cero» para significar «todos». Cierra F63 y con él los cuatro falsificadores de
INC-DEBT-060 (F60–F62 los entregó el lote 2 de `cl-cycle-enumeration`).

| Fichero | Cambio |
|---|---|
| `crates/sddk-cli/src/ledger.rs` | `LedgerEventsOutput` (envoltura) · `--limit 0` = todos · `ledger_events_text` declara |
| `crates/sddk-cli/tests/ledger_events_declaration.rs` | **nuevo**, 4 tests (lote 1, RED) |
| `crates/sddk-cli/tests/cli.rs:1436` | consumidor del JSON, actualizado declarando el cambio |
| `crates/sddk-cli/tests/aiw_s8_x07_real_binary_boundary.rs:292` | **segundo** consumidor, encontrado tarde |

## 2. Lo que se midió antes de decidir

Contra una **copia byte-idéntica** del ledger real (`/var/home/rubentxu/f63/01-medir.py`,
sha256 verificado antes y después):

```
eventos en events_v1 : 590        stream_id distintos : 114

sddk ledger events                  imprime   declara el total   exit
  (sin flags)                        50 de 590         NO            0
  --limit 0                             0             n/a            0
  --limit 100000                      590             n/a            0
  --format json                  50, array desnudo     NO            0
```

La ventana por defecto cubre las secuencias **12 a 20**: son las 50 más recientes,
y las 540 anteriores quedan invisibles sin que nada en pantalla lo diga. Nombra
**19 de los 114 ciclos**.

## 3. El arreglo

**El total ya estaba en mano y se tiraba.** `Storage::list_events`
(`lib.rs:974`) llama a `canonical_events`, que recorre **todos** los streams con
`u32::MAX` y devuelve el vector entero; el truncamiento ocurre después, en
memoria, con `.take(args.limit)`. `total_events` es `all.len()` **antes** del
`take`. Por eso el lote no toca storage —y por eso STOP 1 no saltó— y no cuesta
ninguna consulta nueva.

- **Texto**: `events: {shown} of {total} (truncated|complete)`, siempre y **en
  ambos casos**. Una declaración que solo aparece al trucar no se puede leer en
  un log donde no truncó, y «no truncó» es justo lo que hay que poder comprobar.
- **JSON**: de array a `{ events, total_events, shown, truncated }`.
- **`--limit 0`**: ahora «todos», igual que `ledger export --limit 0`
  (`ledger.rs:442-445`) y que `ledger watch --max-events 0`, en el mismo binario.
- **Ledger vacío**: declara `events: 0 of 0 (complete)` en vez del viejo
  `no events`, que se leía como «no había nada que mirar» cuando lo que quería
  decir es «no se dejó nada fuera».

## 4. La medición de impacto fue FALSA, y eso también va escrito

El §3 del SCOPE afirmaba, antes de decidir, **1** consumidor en `crates/` y **0**
fuera. Era falso por partida doble:

1. **Dentro de `crates/` hay dos.** Se buscó con un `grep` sobre una **lista de
   ficheros elegida a mano** —los que ya sabíamos que lo tocaban— en vez de sobre
   el árbol. El segundo, `aiw_s8_x07_real_binary_boundary.rs:292`, salió **por el
   perfil completo del workspace**, es decir, después de romper el build.
   Búsqueda exhaustiva correcta: 2 ficheros.
2. **`skills/` ni se miró**, y es superficie del bundle que se distribuye.
   `skills/sddk-cycle-resume/SKILL.md:62` ejecuta
   `sddk ledger events … --limit 10 --format json`. **Examinado y no es rotura**:
   la skill no parsea el array, pide al agente que reconstruya la cadena causal
   leyéndola, y una envoltura que dice «10 de 590» es más informativa para ese
   agente. Pero pudo no serlo, y no se comprobó a tiempo.

Es la **quinta** vez en este ciclo que medir con el instrumento equivocado —el
sitio en vez del árbol— produce un número falso, y la quinta vez el número iba
destinado a un documento. La corrección está escrita en §3 del SCOPE, con el
número falso delante.

## 5. Verificación realmente ejecutada

```
cargo test --workspace --no-fail-fast   EXIT=0   5378 passed / 0 failed / 24 ignored (277 binarios)
cargo clippy --workspace --all-targets -- -D warnings   EXIT=0
cargo fmt --check                        limpio
cargo test -p sddk-cli --test ledger_events_declaration            4 passed
cargo test -p sddk-cli --test aiw_s8_x07_real_binary_boundary       6 passed
falsificador O1–O4 contra el almacenamiento real               PASS=7 FAIL=0
```

Baseline antes del lote: 5374 passed / 0 failed. **+4**, exactamente los tests
nuevos: ningún test verde preexisting se perdió.

### Falsificador, contra la realidad

```
F0-copy-is-byte-identical            copy sha256 == real sha256
F1-text-declares-when-complete       590 líneas, declara total, exit 0
F2-text-declares-when-truncated      shown=50, dice "truncated", dice "of 590"
F3-json-carries-and-agrees           keys=[events, shown, total_events, truncated], coherente
F4-limit-zero-means-all              shown=590 total=590 truncated=false   (antes: shown=0)
F5-declared-total-matches-reality    declared=590 sql=590
F6-real-ledger-untouched             sha256 idéntico antes y después
```

**F5 es STOP 4**: el total declarado se contrasta contra
`SELECT COUNT(*) FROM events_v1` y no se ajusta el producto para que cuadre.
**F6 es la prueba de solo lectura**, no su declaración.

## 6. STOP conditions del SCOPE

| STOP | ¿Disparado? |
|---|---|
| 1 — tocar storage | **No.** Cero cambios en `crates/sddk-storage`. El total venía de memoria, que era la hipótesis de §0.3 del SCOPE y queda confirmada. |
| 2 — reescribir un test verde | **Sí, y declarado.** Los dos `as_array()` cambian por el cambio de forma. En ambos, **la aserción que significa algo sobrevive intacta** —cuántos eventos y cuáles— y solo cambia el camino para llegar a ella. El recuento de consumidores era 1 y era 2: corrección escrita en §3 y §4.2 del SCOPE. |
| 3 — `--limit 0` usado en el repo | **No.** Ningún test ni script del repo invoca `ledger events --limit 0`. El único uso es el nuevo R4. |
| 4 — el total no cuadra | **No.** `declared=590` contra `sql=590`. Se verificó contra la realidad y no se ajustó nada para hacerlo cuadrar. |
| 5 — trunca y no lo dice | **No.** F2 lo mide sobre los datos reales. |

## 7. Almacenamiento real

**No se escribió ni una fila**, verificado por hash: `91ea0352…fbf32c` idéntico
antes y después. La copia se comprueba byte a byte antes de usarla porque
`RuntimeContext::open` no abre en solo lectura —su tercer parámetro es
`generate_seed` y dentro hace `Storage::open`, que abre en escritura—.

## 8. Contaminación

CJK/cirílico/árabe/tailandés/coreano sobre los cinco ficheros del lote:
**CLEAN**. Segunda clase: el escaneo automático cubre token glueado dentro de
palabra y palabra extranjera en prosa española; la redacción de este lote se
contaminó **dos veces** (`si_resulta`, `，`+CJK en el SCOPE y el PRE-FLIGHT) y
ambas se corrigieron antes de commitear. Quinta y sexta ocurrencia de la sesión.

## 9. Lo que este lote NO cierra

> **CORRECCIÓN session-69f: el primer punto de esta lista era falso.** Se decía
> que `ledger watch --max-events` «sigue sin declarar su truncamiento». **Medido:
> sí declara, en los dos formatos** —`[watch] emitted 5 events, exiting` en texto
> y `{"__watch_complete":true,"emitted":5}` en JSON (`ledger.rs:785-788`)— y su
> `--max-events` está documentado como `0 = unlimited`. Es **el modelo del
> comportamiento correcto**, no un defecto. La afirmación salió de analogía de
> nombre: tres comandos tienen bandera de tope, luego se les trató el mismo
> defecto sin ejecutar ninguno. Corrección completa en el SCOPE §2.3-bis.
> Lo que sí es la misma clase, medido, es **`vault search`** (§2.3-ter).

- **`sddk vault search` repite las tres cosas.** Medido: **20 de 75 documentos**
  sin declarar nada, **`--limit 0` devuelve `no hits`** en vez de todos, y el JSON
  es un **array desnudo**. Misma clase que `ledger events`, otra superficie,
  **sin tocar**. Slice propio con SCOPE propio.
- **INC-DEBT-060 sigue `open`.** F63 es el último de sus cuatro falsificadores,
  pero cerrarlo no es lo mismo que cerrar la incidencia: siguen abiertas la
  decisión del operador sobre las 79 filas de `__spine_import__` y sobre los 23
  ciclos sin hecho (17 `OPEN`).
- **No se toca `get_cycle`** ni ninguna fila de `cycles`.

## 10. Ficheros de falsificación (fuera del repo)

```
/var/home/rubentxu/f63/01-medir.py    medición de F63 antes de decidir
/var/home/rubentxu/f63/02-falsify.py  O1–O4 contra el almacenamiento real
```
