# RECEIPT — `cl-release-step1-memory-guard`

**WorkItem:** backlog `bl-bl-01M42JGYG4000388551BF9NZ40` — P1, Triaged
**Bloque:** que el release **diga por qué se para**, también cuando lo para la máquina
**Cerrado:** 2026-10-06
**Workspace:** 2.12.1 (declarada, no publicada; último tag remoto `v2.12.0`)
**PRE-FLIGHT:** [`PRE-FLIGHT.md`](./PRE-FLIGHT.md) (`Readiness: READY`)

---

## §1 — El ítem no decía lo que pasaba

El backlog dice: *«el paso 1 del release no tiene cota de memoria»*. Leyendo el
código sale otra cosa, y es lo que cambió el arreglo:

```
scripts/release.sh:312   release_check_resources "$RELEASE_SCRATCH"   ← UNA vez, en el paso 0
scripts/release.sh:389   step "1/15 — cargo fmt + clippy + test (workspace)"
```

**La comprobación de memoria ya existía y ya era fail-closed.** Lo que no existía
era medirla **donde se consume**: el paso 0 mide la máquina *antes* y el paso 1
es el pico de consumo de la operación entera. Entre uno y otro la carga la
máquina lo que sea, y MEDIDO fue justo entonces: aquel día había ~313 clones de
otro proyecto sobre un tmpfs de 48 G y el CI de otro más corriendo, con 2.4 GiB
libres y el swap al 100 %, y el log se cortó exactamente en 122880 bytes sin
línea `EXIT=`.

**Y hay un segundo dato que desarma la solución obvia:** el umbral declarado es
`SDDK_RELEASE_MIN_AVAIL_MB=2048` MB, y el fallo ocurrió con **2.4 GiB ≈ 2457 MB**
libres. Subir el umbral parecería el arreglo y sería **adivinar el pico de
`cargo test --workspace`, que nadie ha medido**. Por eso este bloque **no toca
el umbral**: mueve la medición, que es lo único que está medido que falla.

## §2 — Qué se cambió

| fichero | qué |
|---|---|
| `scripts/lib/release_diagnostics.sh` | **Nuevo** `release_resources_now`: el *hecho* de una línea —memoria disponible, su mínimo, swap usado/total, disco libre del scratch y su mínimo—. No decide nada y no falla: existe para incrustarse en un `die`. |
| `scripts/release.sh` (paso 1) | Se vuelve a llamar `release_check_resources` **justo antes** del primer `cargo`, y los **cuatro** `die` del bloque (fmt, clippy, test y el propio pre-check) imprimen el estado de recursos **de ese instante**. |
| `tests/test_release_diagnostics_wiring.sh` | **Caso E9** (8 aserciones), en el guard de cableado y no en otro fichero: uno que vigila la librería y otro que vigila su cableado miden cosas distintas, y una puede estar impecable mientras la otra no la invoca. |
| `tests/test_release_diagnostics_mutation.sh` | **M25..M28**, una por diente de E9. |

**El umbral no se toca, y por qué está escrito en el código, no solo en el recibo.**

## §3 — Gates

```
tests/test_release_diagnostics.sh           PASS=43 FAIL=0     (sin regresión en la lib)
tests/test_release_diagnostics_wiring.sh    PASS=66 FAIL=0     (E9: 8 aserciones nuevas)
tests/test_release_diagnostics_mutation.sh  PASS=24 FAIL=0 SKIP=0
shellcheck --severity=warning (los 4)       limpio
```

Las cuatro mutaciones caen **E9**, cada una por su propia causa:

| | quita | cae porque |
|---|---|---|
| **M25** | la medición del paso 1 | queda solo la del paso 0, que es el defecto del P1 |
| **M26** | el hecho en **uno** de los cuatro `die` | la aserción mide **cobertura**, no redacción |
| **M27** | el retorno de `release_resources_now` | una función que existe y no imprime deja el mensaje tan mudo como antes |
| **M28** | mueve la medición **detrás** del primer `cargo` | medir tarde no impide nada: es decoracion |

## §4 — Tres cosas que este bloque se encontró a sí mismo

**M24 — mi primera parcheó el fichero dejándolo sin sintaxis, y el falsador lo contó como `SKIP`, no como `PASS`.** Insertaba la comprobación mutada entre la línea `cargo fmt --all -- --check \` y su continuación `|| die …`, partiendo la barra de continuación. El helper tiene exactamente esa regla (`bash -n` antes de juzgar, y parche degenerado = `SKIP` con su motivo), y por eso el resultado fue `SKIP` y no una detección falsa. Rehecha con el ancla de la sentencia completa: cae E9.

**Mi primera aserción de E9 cayó por buscar la palabra donde se nombra.** `grep -n 'cargo '` daba la línea 1 del bloque, porque el *título* del paso es `1/15 — cargo fmt + clippy + test (workspace)`. Comparaba la posición de la comprobación contra el título y moría, con un fallo que parecía del código y era del instrumento. El ancla pasó a ser `^[[:space:]]*cargo `.

**Numeré mis mutaciones M21..M24 y M21 ya existía.** El falsador no se quejó —no comprueba unicidad de etiquetas— y por eso conviene decirlo: renumeradas a M25..M28. Una etiqueta repetida hace que un humano lea dos mediciones como si fueran la misma.

## §5 — Conocimiento negativo: lo que este bloque NO hace

1. **No pone una cota dura al `cargo`.** `ulimit -v` está descartado (Rust reserva
   espacio virtual muy por encima de su RSS: moriría por un motivo inventado) y
   `systemd-run --scope -p MemoryMax=` necesita un bus de sesión que no existe en
   todos los entornos donde corre este release. Añadirlo al camino de entrega sin
   haberlo medido aquí sería cambiar el pipeline a ciegas.
2. **No arregla el caso en que el OOM killer se lleva el `release.sh` entero.**
   Si muere el proceso del release, el `trap` no corre y no hay `EXIT=` que
   imprimir. Eso es detección post-mortem sobre un log anterior, y es otro bloque.
3. **No mide el pico de `cargo test --workspace`.** Por eso el umbral se queda
   como está: subirlo sin medir la magnitud sería fabricar una cifra.
4. **No toca `c3n`** (en `approval-waiting` por `surface.cycle_state#cycle_supersede`)
   ni `INC-DEBT-050`/`061`.

## §6 — Commit atómico bajo un escritor concurrente

`scripts/release.sh` contenía, al commitear, **este cambio y el de otra sesión**
(pasos 3o/3p, +100 líneas, que cablea el canario de worktree al pipeline). No se
puede commitear el fichero entero sin atribuirse trabajo ajeno, así que el commit
se hizo con `git apply --cached` sobre un parche de **solo los tres hunks de este
bloque** (líneas 390–394), y el trabajo del otro proceso quedó **sin stagear** en
el árbol, intacto.

Es el mismo motivo por el que este bloque se mide sobre `git diff`, no sobre lo
que el fichero "debería" decir: con dos escritores en un árbol, la afirmación de
que un cambio está es la que hay que verificar, no la que hay que suponer.
