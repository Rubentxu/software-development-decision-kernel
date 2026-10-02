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

> ### ⚠️ CORRECCIÓN session-69d — esa medición estaba mal, por partida doble
>
> La tabla de arriba es la que se escribió **antes** de decidir, y **está mal**.
> Se conserva porque el razonamiento que la produjo es el que hay que corregir,
> y porque un §3 con el número correcto y sin el error no explica por qué se
> Audaron dos consumidores.
>
> **1. Dentro de `crates/` hay un consumidor más.** Se buscó con un `grep` de
> patrón `"ledger", "events"` sobre una **lista de ficheros elegida a mano** —los
> que ya sabíamos que lo tocaban— en vez de sobre el árbol. El segundo es
> `crates/sddk-cli/tests/aiw_s8_x07_real_binary_boundary.rs:292`
> (`real_binary_reads_cycle_and_events`, `events.as_array()`), y salió **por el
> perfil completo del workspace**, no por la medición previa. Dos consumidores,
> no uno. Búsqueda exhaustiva correcta: `grep -rn --include=*.rs -E '"ledger"[[:space:]]*,[[:space:]]*"events"'`
> sobre el repo → 2 ficheros, uno de ellos los tests nuevos de este ciclo.
>
> **2. `skills/` ni siquiera se miró**, y es superficie del bundle que se
> distribuye a los usuarios. `skills/sddk-cycle-resume/SKILL.md:62` ejecuta
> `sddk ledger events … --limit 10 --format json`. **Examinado y NO es una
> rotura**: la skill no parsea el array, pide al agente que «reconstruya la cadena
> causal reciente» leyéndola, y una envoltura que dice «10 de 590» es estrictamente
> más informativa para ese agente que un array que no dice nada. Pero pudo haberlo
> sido, y no se comprobó hasta después de romper el build.
>
> **Lo que esto cuesta y por qué se escribe:** el cambio de forma está en la
> categoría que §4.2 declaró asumida, así que el procedimiento no cambia. Lo que
> estaba mal era el **recuento**, y un recuento que sostiene una decisión de
> compatibilidad tiene que poder rehacerse. Es la quinta vez en este ciclo que
> medir con el instrumento equivocado —el sitio en vez del árbol— produce un
> número falso, y la quinta vez el número iba a un documento publicado.

`scripts/`, `.github/` y `tests/` no lo parsean. Los únicos consumidores son los
dos tests del repo, y los dos se actualizan **declarando** el cambio, no se
ajustan en silencio.

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
   para y se escribe por qué. La **única** excepción prevista es la que el cambio
   de forma rompe, y hay que **contarla a mano antes de cambiar nada**:
   `cli.rs:1443` y `aiw_s8_x07_real_binary_boundary.rs:292`, ambos por
   `as_array()`. **La cuenta inicial fue 1 y era falsa** (ver la corrección de
   §3): son 2, y una tercera superficie —`skills/`— no se miró hasta después de
   romper el build. Se cuentan como cambio de contrato asumido, no como tests
   acomodados; en ambos, la aserción que significa algo —cuántos eventos y
   cuáles— sobrevive intacta y solo cambia el camino para llegar a ella.
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
