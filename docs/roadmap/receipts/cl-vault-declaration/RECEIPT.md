# RECEIPT — cl-vault-declaration

**Bloque:** `cl-vault-declaration` — bloque de roadmap, **no** es un ciclo del ledger
**Ciclo SDDK:** ninguno. El ledger de `p-63676b11dc0ef88f` da **0 filas** para
este nombre (medido 2026-10-06). El encabezado declaraba antes un `cycle_id`
completo que la autoridad nunca emitió; corregido por INC-DEBT-063.
**SCOPE:** [SCOPE-CONTRACT.md](./SCOPE-CONTRACT.md) · **PRE-FLIGHT:** [PRE-FLIGHT.md](./PRE-FLIGHT.md) (`Readiness: READY`)
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)
**Fecha:** 2026-10-02

---

## 1. Qué entrega

`sddk vault search` declara cuánto dejó fuera, y `--limit 0` pasa a significar
«todos». Misma clase de defecto que F63, otra superficie.

| Fichero | Cambio |
|---|---|
| `crates/sddk-vault/src/search.rs` | `count_matches` **nuevo**; `search_index` **sin tocar** |
| `crates/sddk-vault/src/lib.rs` | re-export de `count_matches` |
| `crates/sddk-cli/src/vault_cmd.rs` | `SearchOutput` · `--limit 0` = todos · `search_text` declara |
| `crates/sddk-cli/tests/vault_search_declaration.rs` | **nuevo**, 6 tests (5 RED + 1 guard) |
| `crates/sddk-cli/tests/cli.rs:8536` | único consumidor del JSON, actualizado declarando el cambio |

## 2. El coste, medido antes de decidir

En `ledger events` el total venía gratis. Aquí **no**: `search_index` corta en SQL
(`LIMIT ?2`), luego declararlo exige una segunda consulta. La alternativa de
pedir `LIMIT n+1` y deducir «al menos uno más» se descartó midiendo:

```
índice real (75 docs)     búsqueda 0,376 ms   COUNT 0,119 ms   +31,7 %
índice sintético (750)    búsqueda 0,814 ms   COUNT 0,064 ms    +7,8 %
índice sintético (7500)   búsqueda 8,129 ms   COUNT 0,424 ms    +5,2 %
```

**El COUNT es más barato que la propia búsqueda y el sobrecoste baja con la
escala.** `ORDER BY rank LIMIT 20` ordena todos los matchs; `COUNT … WHERE MATCH`
solo los recorre. Por eso se paga el total **exacto**: el dato completo sale más
barato que el parcial.

## 3. El arreglo

- **Texto**: `hits: {shown} of {total} (truncated|complete)`, **siempre**.
- **JSON**: de array a `{hits, total_hits, shown, truncated}`.
- **`--limit 0`**: ahora «todos», como `ledger export --limit 0`,
  `ledger events --limit 0` y `ledger watch --max-events 0`. Antes `LIMIT 0`
  llegaba a SQL sin tratar y devolvía cero filas — **cuarta contradicción de la
  convención del cero en este mismo binario**.
- **Sin coincidencias**: declara `hits: 0 of 0 (complete)` en vez del viejo
  `no hits`.

## 4. Verificación

```
cargo test --workspace --no-fail-fast   EXIT=0   5384 passed / 0 failed / 24 ignored (278 binarios)
cargo clippy --workspace --all-targets -- -D warnings   EXIT=0
cargo fmt --check                        limpio
vault_search_declaration                 6 passed
falsificador O1–O4 contra el índice real   PASS=9 FAIL=0
```

Baseline 5378 → **+6**, exactamente los tests nuevos.

```
F0-copy-is-byte-identical     copy sha256 == real
F1-text-declares-when-complete        declared=61 expected=61
F2-text-declares-when-truncated       shown=20 declared=61, dice truncated
F3-json-carries-and-agrees            shown=20 total=61 truncated=True, coherente
F4-limit-zero-means-all               shown=61   (antes: 0)
F5-bounded-limit-bounds-and-declares  shown=5 total=61 truncated=True
F6-no-matches-declares-zero           'hits: 0 of 0 (complete)', sin 'no hits'
F7-declared-total-matches-index       declared=61 sql=61   <- STOP 4
F8-real-index-untouched               sha256 idéntico
```

## 5. Dos fallos que fueron míos, ninguno del producto

**El fixture de R1 no cassaba, y dejó R6 vacío.** El documento de prueba decía
`cryptography` y la consulta `crypto`: FTS5 hace coincidencia de **token exacto**,
sin stemming, luego no hay nada que encontrar. R1 falló contra un producto que
declaraba `hits: 0 of 0 (complete)` — correctamente, porque no había coincidencia.
Corregido con un token que es la misma forma exacta en el documento y en la
consulta. **Lo importante no es el fallo, sino lo que el fallo tapaba:** R6
tenía una rama «con coincidencias» que en realidad estaba ejercitando la rama
«sin coincidencias», y pasaba igual. Un test que pasa porque está probando otra
cosa es un test que no mide, y no se vio hasta que R1 lo dejó al descubierto.

**El falsificador se cerró la conexión antes de usarla**, y luego leía `parts[1]`
de una línea `hits: 20 of 61 (…)` donde el índice 1 es la palabra `of`. Los dos
fallos del arnés, ninguno del producto.

## 6. STOP conditions del SCOPE

| STOP | ¿Disparado? |
|---|---|
| 1 — cambiar la firma de `search_index` | **No.** Sus **8 tests unitarios** pasan sin reescribir y la API pública queda igual. El total se obtiene con una función nueva. |
| 2 — reescribir un test verde | **Sí, y declarado.** Solo `cli.rs:8541`, cuyo `as_array()` cambia. Su aserción de fondo —«el único hit es TERM-Auth»— no se mueve. |
| 3 — sobrecoste material del COUNT | **No.** 5,2 % a escala realista, medido antes de decidir. |
| 4 — el total no cuadra | **No.** `declared=61` contra `sql=61`, F7. |
| 5 — trunca y no lo dice | **No.** F2 lo mide sobre el índice real. |

## 7. Almacenamiento

**No se escribió en el índice real.** Verificado por hash:
`78e46f4c…54d3af` idéntico antes y después, sobre una copia byte a byte.

## 8. Lo que este lote NO cierra

- **`vault graph` y `vault show` NO se han medido.** También proyectan datos y
  podrían tener la misma clase. §2.3 los excluye, y la exclusión dice
  explícitamente que es «no medido», **no** «correcto»: esa distinción es
  precisamente la que faltó con `ledger watch`, y repetirla sería repetir el
  error.
- **El índice FTS no se toca**: ni esquema, ni `rebuild_search_index`, ni la
  sanitización de la consulta.
- **INC-DEBT-060 sigue `open`**, sin relación con este lote.

## 9. Ficheros de falsificación

```
/var/home/rubentxu/vault/01-coste.py     coste del COUNT a 75 / 750 / 7500 documentos
/var/home/rubentxu/vault/02-falsify.py   O1–O4 contra el índice real, copia byte a byte
```
