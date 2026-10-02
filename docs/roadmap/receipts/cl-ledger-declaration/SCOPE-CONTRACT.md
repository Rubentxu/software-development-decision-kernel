# SCOPE-CONTRACT — cl-ledger-declaration

**Cycle:** `p-63676b11dc0ef88f/ledger-declaration`
**Date:** 2026-10-02T20:40:00Z
**Authority:** [INC-DEBT-060](../../../debt/INC-DEBT-060-NO-SURFACE-ENUMERATES-CYCLES-97-OF-179-ARE-NAMED-BY-NO-COMMAND.md) (`high`/`P1`, `open`), falsificador **F63**
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)

---

## §0 — La medición, hecha antes de decidir nada

Falsificador: `/var/home/rubentxu/f63/01-medir.py`, contra una **copia
byte-idéntica** del ledger real (sha256 `91ea0352…fbf32c` verificado antes y
después: el ledger real no se tocó).

Verdad de fondo, leída de SQLite en `mode=ro`:

```
eventos en events_v1            : 590
stream_id distintos             : 114
```

Comportamiento medido del comando:

| invocación | imprime | declara el total | exit |
|---|---|---|---|
| `ledger events` | **50 de 590** | **NO** | 0 |
| `ledger events --limit 0` | **0** | no aplica | 0 |
| `ledger events --limit 100000` | 590 | no aplica | 0 |
| `ledger events --format json` | 50, array desnudo | **NO** — ningún campo `total`/`count` | 0 |

La ventana por defecto cubre las secuencias **12 a 20**: el comando se queda con
las **50 más recientes** y oculta las 540 anteriores. Nombra **19 de los 114
ciclos**. Quien lo use para «ver el estado del mundo» se lleva una foto del
presente inmediato y cree que es el inventario.

### Tres cosas que la medición añade y el enunciado no decía

1. **`--limit 0` imprime cero eventos.** Para `ledger export`, `0` significa
   «todos» (`ledger.rs:442-445`). Aquí significa «ninguno». Es la convención
   inversa en el mismo binario, y quien la conozca de `export` Teclea `--limit 0`
   para desbloquearse y se lleva una lista vacía con **exit 0**.
2. **La salida JSON no puede declarar nada.** Es un array desnudo, y un array no
   tiene dónde llevar un total. No es que falte el campo: es que la forma lo
   impide.
3. **El total ya está en mano y se tira.** `Storage::list_events`
   (`crates/sddk-storage/src/lib.rs:974`) llama a `canonical_events`, que recorre
   **todos** los streams con `u32::MAX` y devuelve el vector entero. El
   truncamiento ocurre después, en memoria, con `.take(args.limit)`
   (`ledger.rs:410`). Declarar el total **no cuesta una consulta, ni una API de
   storage, ni una migración**: cuesta no descartar el número que ya se tiene.

> La primera versión del falsificador imprimió **0 eventos** contra un comando que
> imprime 50, porque asumió una forma `sequence: N` en vez de la real —una línea
> por evento—. Es el **cuarto** guard mal escrito de esta serie, y se arregla el
> guard. Se deja escrito porque el número de un guard es tan evidencia como el
> del producto.

## §1 — Objetivo, falsable

1. **O1.** La salida de texto **declara** cuántos eventos existen en total y
   cuántos se muestran, siempre — también cuando no trunca, porque una
   declaración que solo aparece al trucar no se puede leer en un log.
2. **O2.** La salida JSON lleva el total, cuántos se muestran y si trunca.
3. **O3.** `--limit 0` significa **todos**, igual que en `ledger export`, y el
   `--help` lo dice.
4. **O4.** El camino vacío —cero eventos en el ledger— declara **cero**, no
   imprime una línea que se puede leer como «no hay nada que ver» cuando en
   realidad no se miró nada.

## §2 — No-objetivos

1. **NO** se cambia `Storage::list_events` ni ninguna función de storage. El
   total ya está disponible; tocar la capa de persistencia para un problema de
   presentación sería ampliar el lote sin motivo.
2. **NO** se cambia el valor por defecto de `--limit` (50). Que 50 sea un número
   razonable para un humano es discutible; que **no se diga** que hay 590 es un
   defecto. El primero es criterio, el segundo es mentira.
3. **NO** se toca `ledger export`, `ledger replay` ni `ledger watch`, aunque
   `watch` tenga su propio `--max-events` con la misma familia de truncamiento.
   Cada uno con su SCOPE, por el motivo de §2.3 del SCOPE de `cl-cycle-enumeration`.
4. **NO** se cambia el **orden** de los eventos. Se devuelven los 50 últimos en
   orden ascendente, que es lo que hace hoy.
5. **NO** se escribe en el almacenamiento real de esta máquina.

## §3 — La decisión que hay que tomar antes de escribir código

**El JSON de `ledger events` pasa de array a objeto.** Antes:

```json
[ { "sequence": 20, ... }, ... ]
```

Después:

```json
{ "events": [ ... ], "total_events": 590, "shown": 50, "truncated": true }
```

Es un **cambio de forma de una salida de CLI**, y hay que declararlo como tal
porque rompe a quien haga `jq '.[0]'`. Se ha medido el radio de impacto **antes**
de decidir:

```
consumidores de `ledger events` fuera de crates/ : 0
consumidores dentro de crates/                  : 1
  crates/sddk-cli/tests/cli.rs:1443
    assert_eq!(events_json.as_array().unwrap().len(), 6);
```

`scripts/`, `.github/` y `tests/` no lo parsean. El único consumidor es un test
del propio repo, que se actualiza **declarando** el cambio, no se ajusta en
silencio.

La alternativa era **declarar solo en texto** y dejar el JSON como array. Se
descarta, y se escribe por qué: el modo de fallo que F63 describe es
exactamente el que golpea a la automatización, porque un humano que lee 50
líneas sin declaración tiene una sospecha y un script no tiene ninguna. Dejar
el array significa que **la superficie que más consume la salida es la que
sigue mintiendo**, y F63 quedaría verde en el papel y falso en el uso.

## §4 — STOP conditions

1. **Si el arreglo exige tocar storage** (una consulta de recuento, un índice,
   una migración), se para. El total ya está en memoria; si resulta que no, el
   razonamiento de §0.3 es falso y hay que volver a medir.
2. **Si algún test que ya estaba verde hay que reescribirlo para que pase**, se
   para y se escribe por qué. La **única** excepción prevista y declarada es
   `cli.rs:1443`, cuyo `as_array()` **debe** cambiar porque la forma cambia, y
   que se cuenta como cambio de contrato asumido, no como test acomodado.
3. **Si `--limit 0` resulta ser usado por alguien en este repo**, se para: el
   comportamiento observable cambiaría de cero a todos y eso hay que saberlo
   antes, no después.
4. **Si el total declarado no cuadra con `SELECT COUNT(*)` sobre el ledger
   real**, se para y se investiga antes de ajustar nada.
5. **Si trunca y la salida no lo dice**, se para, aunque el resto sea correcto.

## §5 — Riesgo

1. **Compatibilidad de la salida JSON.** Es el único riesgo real y está medido en
   §3. Se mitiga declarando el cambio en el changelog y actualizando el único
   consumidor.
2. **Coste.** Ninguno: no hay consulta nueva. `list_events` ya carga el vector
   entero; el número se toma de `events.len()` **antes** del `.take`.
3. **El riesgo de este lote es de alcance**, no de datos: la tentación es arreglar
   de paso el `--max-events` de `ledger watch`, que es el mismo defecto en la
   superficie vecina y en el mismo fichero. §2.3 lo prohíbe y por eso es STOP de
   alcance, no de comportamiento.
4. **Lectura, no escritura.** Los tests usan sandboxes con
   `env_remove("SDDK_DATA_DIR")`, porque un `CliSandbox` que hereda el entorno
   escribiría en el storage de verdad.

## §6 — Gates que deben seguir verdes al cerrar

`cargo test --workspace` **sin reescribir ningún test verde salvo el declarado en
§4.2** · `cargo fmt --check` · `cargo clippy --workspace --all-targets -- -D
warnings` · `check_debt_index_coherence` · `test_changelog_coverage` ·
`test_adr_promotion_format` · `test_docs_script_contamination` ·
`test_gate_coverage` · `test_release_state_pointer` · falsificador propio que
compruebe O1–O4 contra el almacenamiento real, en **solo lectura**, declarando su
recuento.
