# VERIFICATION-REPORT — cl-doctor-build-identity

Ciclo `p-63676b11dc0ef88f/cl-doctor-build-identity`, fase `verify`, path
`B-direct`. Transición de cierre: `phase.verify.complete.b-direct`.

**Por qué esta verify no es repetir la suite.** La fase anterior ya graduó
`implementation-complete` sobre `cargo test --workspace`. Lo que esta fase tiene
que medir es lo que la suite **no puede**: que los cuatro estados del check se
distinguen con el **binario real** y el **repo real**, y sobre todo que los dos
estados de «no puedo saber» **no dan rojo**. Un rojo falso en `dev doctor`
entrena a ignorar los rojos, que es peor que no tener check.

---

## 1. Los cuatro estados, medidos con dos binarios reales

Dos binarios, **el mismo commit `41fcb916`**, distinguidos solo por la
procedencia — que es exactamente la variable que STOP 6 gobierna:

| binario | `identity.source` |
|---|---|
| `sddk-concluyente` | `env` (construido con `SDDK_GIT_SHA`) |
| `sddk-no-concluyente` | `git` (construido sin ella, por el fallback) |

Los dos declaran el mismo commit, luego **la identidad sola no basta: la
procedencia acompaña siempre al valor**, que es el STOP 5 del SCOPE de
`cl-build-identity` puesto a prueba.

| Objetivo | Escenario | Veredicto | Detalle |
|---|---|---|---|
| **O2** | checkout de sddk, binario al día | `present` | `OK: el checkout esta en el mismo commit que el binario` |
| **O3** | **el mismo binario**, checkout atrasado | **`missing`** | `FALLO: … es ancestro del HEAD del checkout, luego el checkout tiene trabajo que el binario no contiene` |
| **O5** | repo **impostor** | `present` | `N/A: … no es un checkout de sddk-framework` |
| **O4** | directorio que **no** es repo | `present` | `N/A: … no es un checkout de sddk-framework` |
| **O6** | identidad **no concluyente** (`source: git`) | `present` | `N/A: identidad no concluyente (…, source=git); STOP 6 no le permite decidir` |
| **O7** | todos los veredictos | — | ninguno sin motivo |

**Y el contrato:** con el binario al día `dev doctor` sigue saliendo con **0**.
El cambio declarado es que puede salir con **1** cuando el binario no es el de
este checkout; no es que salga con 1 siempre. Un detector que rompe los usos
legítimos no es un detector.

Instrumento: `tests/test_doctor_identity_states.sh`, **PASS=19 FAIL=0**, con
los dos binarios **recibidos como argumentos** y no construidos dentro — para
que la medición no dependa de cuándo se ejecutó, que es el modo de fallo que ya
se pagó dos veces en esta sesión con el binario del repo.

## 2. `tests-pass`

Evidencia del gate: **un** comando con `argv`, `exit_code` y
`output_digest` (REQ-IPV, spec-v2, cycle-44).

- `cargo test --workspace --no-fail-fast` → **exit 0**
  — `sha256:dea6a43469d477a…`, con el recuento medido a mano en §2bis.

Lote acotado, con digest por comando:

| Comando | Exit |
|---|---|
| `cargo test -p sddk-cli --lib dev::build_id` (18 guards) | 0 |
| `cargo fmt --check` | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 |
| `bash tests/test_doctor_identity_states.sh` (2 binarios reales) | 0 — **PASS=19 FAIL=0** |
| `shellcheck tests/test_doctor_identity_states.sh` | 0 |
| `bash tests/test_build_identity_policy.sh` | 0 — PASS=8 FAIL=0 |
| `bash tests/test_release_build_identity.sh` | 0 — PASS=22 FAIL=0 |
| `bash tests/test_changelog_coverage.sh` | 0 — PASS=74 FAIL=0 |
| `bash tests/test_debt_index_coherence.sh` | 0 — PASS=12 FAIL=0 |
| scanner de redacción sobre lo añadido | 0 — CLEAN |

## 2bis. El recuento, medido y no heredado

El `exit 0` de la evidencia **no viene con un recuento**, y el número que una
suite imprime cambia con cada commit. Sumando las líneas `test result:` de la
misma ejecución que produjo la evidencia:

```
passed=5435 failed=0 ignored=24
```

**Lo que no se afirma aquí:** el informe anterior de este mismo fichero decía
«283 binarios». Ese número **no se ha medido en esta sesión** y no aparece en
ninguna de las dos salidas que se generaron, luego se ha retirado del texto en
lugar de dejarlo ahí con la autoridad de un informe. El recuento que sí se
declara es el que se ha derivado de la salida, y el que la evidencia respalda
es el `exit 0` y su `output_digest`.

## 3. Falsificación

`26-falsify-doctor-identity.py`: 7 guards, 7 mutaciones aplicadas al **source
real**, **7 detectadas, 0 sobrevividas, 0 no medibles**.

**R5 sobrevivió a la primera pasada** — sexta vez en la serie que un guard solo
fija el caso donde el defecto no se manifiesta, y este lo escribí yo. Usaba un
repo **sin** `crates/sddk-cli/Cargo.toml`, luego un marcador demasiado permisivo
nunca se distinguía del correcto: sin el fichero, `read_to_string` falla y
`unwrap_or(false)` responde igual en las dos variantes. Le faltaba el
**impostor**, el único caso donde «acepta cualquier manifiesto» y «acepta solo
el nuestro» dan respuestas distintas. Corregido **en el guard**, con el caso
viejo conservado.

El falsificador falló dos veces por su cuenta: una mutación no aterrizó porque
`cargo fmt` había cambiado la indentación del `match`, y otra no compilaba.
**NO MEDIBLE no es DETECTADA**, y reportarlas como tales habría inventado una
capacidad que el guard no tiene.

## 4. El instrumento de esta verify falló dos veces, y ninguna era del producto

**La primera: PASS=10 FAIL=5 con el producto completamente en verde.** Los
cinco eran del **aserto**: comparaba contra `"PRESENT|"` cuando `${V%%|*}` ya
quita la barra. Ninguna medición estaba mal; las cinco estaban bien y el test
las contaba como fallos.

**Un instrumento que se queja del instrumento en lugar de fijar el defecto es un
instrumento que hay que arreglar antes de creerse nada.** Y este es el espejo
del guard R5, que era ciego: aquel **ocultaba** el defecto, este lo
**exagera**. Los dos extremos fallan igual y los dos se corrigen mirando, no
leyendo el número.

**La segunda la encontró el falsificador, y era peor: el instrumento no medía
lo que declaraba medir.** O7 afirmaba «ningún veredicto sin motivo» y **medía 2
veredictos de 5**. Los directorios temporales de los escenarios impostor y
sin-checkout se enviaban a la basura en las líneas 91 y 99, y el
`[ -d "$dir" ] || continue` del propio bucle de O7 se los saltaba en silencio.
Los demás asertos de O7 seguían contando lo suyo, luego el resultado global
seguía siendo `FAIL=0` y el hueco era invisible desde el número.

Los veredictos se recogen ahora **al medirse**, y el **recuento de muestras es
un aserto por derecho propio**: `se midieron los cinco veredictos`. Sin él, un
escenario futuro que dejara de registrarse volvería a pasar mudo — que es
justo lo que se acaba de corregir.

**Y el aserto nuevo tampoco bastaba, y esto lo dijo la falsificación.** Anular
la aritmética que contaba los motivos vacíos **no lo detectaba nadie**: el
único guard que dependía de ella era el propio aserto anulado, luego anularlo
era indistinguible de no tener el guard. El recuento vive ahora en una función
`contar_vacios` que el test **se autocomprueba** contra una lista conocida
(`uno dos '' tres cuatro` → `1`) *antes* de fiarse de ella.

**Una copia del código no vigila el código**, y por eso el meta-aserto llama a
la **misma** función que el aserto real en vez de reimplementar el conteo. Es la
séptima vez en la serie que esto se paga, y las seis anteriores eran guards que
solo fijaban el caso donde el defecto no se manifiesta.

## 4bis. La falsificación de este instrumento, con lo que NO detectó

7 mutaciones al source real, **6 detectadas, 1 sobrevive y se declara**:

| # | Mutación | Resultado |
|---|---|---|
| M1 | anular la función `contar_vacios` | **detectada** (exit 1) — el meta-aserto la delata |
| M2 | anular el recuento por `grep` | **sobrevive**, y se explica abajo |
| M3 | borrar un `registrar` | **detectada** — la muestra queda incompleta |
| M4 | motivo vacío + anular la función | **detectada** — la que escapaba en la 1ª forma |
| M5 | la función cuenta de más | **detectada** — declara vacío lo que no lo está |
| M6 | un veredicto sin motivo, con la función íntegra | **detectada** |
| M7 | anular `grep` **con** un motivo vacío presente | **detectada** (FAIL=2) |

**M2 sobrevive por una razón que conviene decir con sus palabras: anular el
recuento por `grep` cuando no hay ningún motivo vacío es indistinguible de la
constante `0`.** No es un defecto del guard; es una mutación **equivalente a la
base**, y reportarla como sobrevida sería inventar una capacidad que el guard no
tiene. Lo que demuestra que los dos caminos se necesitan el uno al otro es
**M7**: la misma anulación *compuesta con* un motivo vacío sí cae.

La asimetría con M1 es la lección: M1 cae sin necesitar motivo vacío porque la
función se autocomprueba; M2 no, porque nadie autocomprueba el `grep`. Decirlo
es más útil que forzar un `7/7` que no se ha medido.

## 5. Lo que esta verify NO declara

- **INC-DEBT-064 sigue `open`.** Los binarios **ya instalados** declaran
  `source: git`, luego el check es N/A para ellos y **no tiene dientes hasta la
  próxima release** — bloqueada por la clave KMS. El mecanismo existe; todavía
  no vigila nada en la máquina que lo ejecutaría.
- **El cambio de contrato es real y declarado**: `dev doctor` puede salir con 1
  donde antes salía con 0. Está en el changelog, no en una nota al pie.
- **La ruta forge contra un GitHub real**: `NOT_RUN`.
- **No se ejecutó ninguna matriz UAT nueva.** Esto son gates y mediciones de
  comportamiento; la UAT de este ciclo sigue sin hacerse, y decirlo aquí evita
  que este informe se lea como si la cubriera.
