# EXPLORATION-REPORT — cl-ledger-watch-total

**Cycle:** `p-63676b11dc0ef88f/ledger-watch-total` (ciclo **real** en SDDK:
`OPEN/explore`, `cycle.start` emitido, `evt-51530d33-…`)
**Date:** 2026-10-02T21:12:36Z
**Baseline:** `b0153dc0` (`chore(roadmap): reconcilia el puntero de estado tras el lote 2`), `HEAD == origin/main`
**Workspace:** 2.5.3 (declarada, no publicada; último tag remoto `v2.5.2`)

---

## §0 — La medición

`/var/home/rubentxu/f63/03-medir-watch.py`, contra una **copia** del ledger real
(el original se comprueba por mtime al final: no se toca).

```
OK  : el total real se puede leer de `ledger events` -- exit=0 total_events=591
OK  : `ledger watch --max-events 5` emite cinco eventos y sale -- exit=0 emitidos=5
      control=['[watch] emitted 5 events, exiting']
GAP : `ledger watch` en TEXTO declara cuantos eventos existen en total
      -- numeros en las lineas de control=[5], total real=591
OK  : `ledger watch` en JSON cierra con su marcador de fin
      -- el ultimo objeto es {'__watch_complete': True, 'emitted': 5}
GAP : `ledger watch` en JSON declara cuantos eventos existen en total -- declarado=None real=591
GAP : el total declarado es consistente con lo emitido -- emitido=5 declarado=None real=591
OK  : `ledger export --limit 5` escribe cinco eventos y sale
      -- exit=0 escritos=5 dice=['exported 5 events to /tmp/.../export.jsonl']
GAP : `ledger export` declara cuantos eventos existian en total -- numeros que declara=[5], total real=591
OK  : la copia del ledger no se ha modificado
```

591 = 590 del corpus + **el evento `cycle.start` de este ciclo**, que es la
primera escritura al ledger real de esta sesión y la esperada.

## §1 — Qué es, y qué NO es

`ledger watch` **sí declara**: escribe `[watch] emitted 5 events, exiting` y
`{"__watch_complete":true,"emitted":5}`. La sesión-69f lo nombró «el modelo
del comportamiento correcto» y la afirmación **era cierta** — y por eso
llevó a la conclusión equivocada. La distinción que faltó:

> **Declarar que ha emitido N no es declarar que había M.**

Con `--max-events 5` sobre 591, el comando dice «5» y no dice nada de las 586.
Un operador que ve `[watch] emitted 5 events, exiting` no puede distinguir
«se acabó el ledger» de «paré yo a los 5». Es **el defecto de F63** —una
ventana que se presenta como el total sin declarar que es una ventana— en una
tercera superficie, y el caso de F63 por construcción: en `ledger events` el
vector completo ya estaba en memoria y se descartaba el largo
(`ledger.rs:437-440`).

**Lo que NO se afirma.** Que `watch` deba emitir todo el ledger. `--max-events`
es una petición legítima, y con `--from-tail` el operador pide explícitamente
no ver el histórico. El defecto no es el tope: es **no declarar que el tope
dejó algo fuera**.

## §2 — Auditoría por criterio, y su resultado

La pregunta: *«¿qué más lee el ledger y declara cuántos hay, y cada uno lo
declara igual?»*. Resultado **medido y leído**, no supuesto:

| superficie | veredicto |
|---|---|
| `ledger events` | **declara** — `total_events`/`shown`/`truncated` (`ledger.rs:248-256`), texto `events: N of M (…)` (`:651-657`) |
| `ledger watch` | **DEFECTO — este ciclo** (`ledger.rs:785-788`) |
| `ledger export` | **DEFECTO, misma clase, otra superficie** (`ledger.rs:512-525`): escribe `limit` eventos a un fichero y dice `exported 5 events to …`. Medido arriba. **No entra en este ciclo**: un ciclo, una concernia (precedente 69h → 69i) |
| `cycle list` | **ya declara** (`cycle.rs:2064-2069`): lista las 81 filas de 179 con `manifest_json` ilegible y las reporta aparte, con el motivo escrito |
| `telemetry status` | **declara** (`telemetry.rs:500`, `:554`): `total_cycles` |

**`cycle` no es un no-objetivo pendiente**: ya estaba cerrado. Eso responde al
elemento «extender la auditoría a cycle» de la lista de trabajo sin abrir un
ciclo nuevo.

## §3 — El coste del remedio, medido antes de escribirlo

Dos vías opuestas, medidas sobre la copia:

```
D  COUNT(*)                      :    0.077 ms
A  materializar las 591 filas    :    4.015 ms   -> 51.8x D
```

- **D** — `SELECT COUNT(*) FROM events_v1`, que ya existe como
  `SqliteEventStore::count()` (`event_store.rs:402-408`), con test en
  `event_store.rs:171`.
- **A** — `list_events_after(cursor, i64::MAX)` y contar, que es literalmente
  lo que hace el bucle en cada poll (`ledger.rs:719-722`).

D gana por 51,8×, y la razón **no** es solo el precio:

> `canonical_events()` (`lib.rs:982-990`) recorre **todos** los streams con
> `u32::MAX` en **cada llamada**, y `list_events_after` (`:1022-1027`) tira el
> largo después. O sea que el total **ya está en memoria y se descarta en cada
> poll**. F63 con otro comando.

D es tentador, y se descartó por una razón concreta: para que D sea cierto
habría que **probar** que `COUNT(*)` coincide con el filtro en Rust de `watch`
(`ev.cycle_id.as_deref() == Some(..)`, `ev.frame_id == *frame`) en las **tres**
formas —sin filtro, con `--cycle`, con `--frame`— y esa prueba no está hecha.
Un `COUNT` con predicado SQL que no coincide con el predicado en Rust no es una
declaración más barata: es **una declaración falsa con mejor rendimiento**.

La objeción simétrica —«A es caro»— **tampoco se sostiene**: una materialización
extra al final de una corrida que ya materializa el ledger entero cada 500 ms
es **una iteración más de un trabajo que ya se está pagando**. Se elige **A**, y
por una razón que no es de rendimiento:

> A **no puede discrepar** del emisor por construcción, porque es el mismo
> código y el mismo filtro. D puede. Un segundo camino de cómputo es
> exactamente el defecto que este ciclo viene a cerrar (lote 2 de
> `cl-vault-node-projection`: la lista a mano que nadie derivaba).

## §4 — La medición evitó un defecto, no solo un trabajo

El primer detector buscaba `of`/`total`/`pending` en **toda** la salida. Con el
defecto presente, el payload de los 5 eventos emitted contiene prosa con esas
palabras: el detector podía pasar sin que el defecto existiera. Es la misma
falla que en 69k (detector acoplado a la redacción) y que en el guard de
`export` (los dígitos del `tmpdir` contaban como «números declarados»:
`[0, 5, 9]`).

El detector final **mide sustancia**: se queda con las líneas de control —las
que el comando escribe para declararse— y pregunta si el número real del total
está ahí. Si el total se declara, aparece; si no, no. Ninguna redacción lo hace
pasar y ningún payload lo hace fallar. Todos los FAIL de esta medición fueron
**del detector**, y se corrigieron en el detector.

## §5 — Lo que la autoridad y los documentos no coinciden

Al crear el ciclo real (el primero de esta sesión) se midió algo que no se
buscaba: **el ledger real no contiene ningún ciclo de las sesiones 69h, 69i ni
69k**.

```
$ python3 -c "... SELECT cycle_id FROM cycles WHERE cycle_id LIKE '%vault%' ..."
p-63676b11dc0ef88f/vault-mirror-accepted-adrs
p-63676b11dc0ef88f/vault-mirror-auto
(total: 179 ciclos)
```

`cl-vault-graph`, `cl-vault-html-replica` y `cl-vault-node-projection` existen
como documentos en `docs/roadmap/receipts/`, y sus recibos declaran
`**Cycle:** p-63676b11dc0ef88f/vault-node-projection` — un identificador que la
autoridad nunca emitió.

**Lo que NO se hace, y por qué:** no se crean esos ciclos a posteriori. Eso sería
**escribir historia en la autoridad**, que es exactamente el fallo de
`INC-DEBT-061` (un identificador que no se puede corregir redirigiendo) aplicado
a los recibos propios. Queda como decisión del operador (§6), no como trabajo
silencioso.

**Lo que sí cambia:** este ciclo nace en la autoridad real, y sus documentos se
refieren a un ciclo que existe. La divergencia anterior queda **medida y
declarada**, no heredada.

## §6 — Decisión que se escala al operador

Los tres recibos anteriores declaran un `cycle_id` que SDDK no conoce. Dos
salidas, y ninguna es «crear los ciclos»:

1. **Amendar los recibos** para que declaren lo que es cierto (trabajo
   documental, sin tocar la autoridad).
2. **Registrar la divergencia** como deuda, con la autoridad como fuente de
   verdad y los recibos como recio histórico que mentía del ciclo.

Se recomienda la **1** para los tres, y no se ejecuta en este ciclo: son
documentos de ciclos cerrados, y reescribirlos es otra concernia.
