# RECEIPT — C3m-identity-alias, lote 4 (los 15 aliases sobre el storage real)

**Fecha:** 2026-10-02T09:55:00Z
**Commits:** `2d7fac4c` (lote 2), `81027c94` (lote 3), `20ddd25c` (audit alias-aware)
**Binario:** `/var/home/rubentxu/cargo-targets/debug/sddk` 2.5.3, compilado de este
checkout. **No** se usó `~/.local/bin/sddk`, que sigue stale (ver B2).
**Storage real:** `~/.local/state/sddk/` — la tabla se escribió en
`project-aliases.json`, sha256 `3052169cc5d0bd1c`, **14 entradas**,
`schema_version: 1`. (Fueron 15, sha `bc981da9…`, hasta la retirada del par
de skillgraph; ver "Decisión del operador" abajo.)
**Rollback:** borrar `~/.local/state/sddk/project-aliases.json`. No hay más que
deshacer: el mecanismo no toca ningún ledger.

## Qué se declaró

15 aliases, cada uno con `--dry-run` previo. La dirección es la **inversa** de la
flecha que imprime `audit`, y a propósito: `audit` dice «el receipt nombra X, hoy
se deriva Y»; el alias lleva Y a X.

## Los seis criterios del SCOPE

| # | criterio | resultado | evidencia |
|---|---|---|---|
| 1 | Sin alias, nada cambia | OK | checkout sin pin y tabla vacía: 25 huérfanos, igual que antes de declarar |
| 2 | Ciclo ⇒ error duro | OK | 5/5 falsificaciones; ciclo sale con exit 4 nombrando la cadena |
| 3 | La salida declara el salto | OK | `identity_alias: p-995939af668a53d8 -> p-63676b11dc0ef88f` |
| 4 | Borrar un alias falla | OK | `AliasTable` no tiene `remove`; el store solo añade |
| 5 | `audit` en 0 y recuento idéntico | **parcial, ver abajo** | audit 0; 23 de 24 proyectos byte-idénticos |
| 6 | `verify_stream_chain` OK | OK | `ledger verify-chain` vía alias: 114 streams, 590 eventos, `status: PASS` |

## Prueba de extremo a extremo, con el binario real

```text
# checkout temporal SIN pin, remote de este repo -> deriva Y
$ sddk project resolve --root $T --scope . --remote https://github.com/Rubentxu/software-development-decision-kernel
project_id: p-63676b11dc0ef88f
identity_source: remote
identity_alias: p-995939af668a53d8 -> p-63676b11dc0ef88f

# el CWD real, pineado a X -> intacto
$ sddk project resolve --root <este repo> --scope .
project_id: p-63676b11dc0ef88f
identity_source: pinned
identity_alias: none

# el verificador de cadena REAL, alcanzado a través del alias
$ sddk ledger verify-chain --root $T --scope . --remote <remote>
stream: all streams of p-63676b11dc0ef88f
streams: 114
event_count: 590
status: PASS
```

Ese último es el criterio que ningún otro cubre: no es un hash de hashes mío,
es `verify_chain_integrity` de Rust recalculando sobre el ledger histórico, y
llegando a él por el alias y no por el pin.

## Criterio 5: por qué no es un verde limpio

La comprobación de filas dio **PASS** inmediatamente después de declarar los 15
(`events_v1: 3937`, 24 cadenas de eventos idénticas). Doce minutos más tarde, la
misma comprobación dio **FAIL**: `p-b7740b96d79ec013` pasó de 105 a 107 eventos.

**No lo causó el alias.** La atribución está hecha, no supuesta:

- Los 2 eventos son `cycle.transitioned` con payload
  `{"transition_id":"phase.build.complete","outcome":"succeeded"}` y
  `lease.released`, sobre el work item `wi-72-p3-expansion-apply`. Ninguna ruta
  del alias puede emitir una transición de fase de ciclo.
- Actor `{"kind":"system","id":"rubentxu"}`, el mismo que escribió el evento
  anterior de la misma serie a las 09:44:07Z.
- En la misma ventana se modificaron **7** ledgers, tres de ellos
  (`p-e132b9bf59646d4b`, `p-d55b20670622127c`, `p-24541d4f59ead4f3`) **fuera**
  del conjunto de 15: nada de este trabajo puede tocarlos.
- Una muestra de 75 s con el agente sin hacer nada dio 0 cambios: el actor
  escribe en ráfagas, no de forma continua.

Estado final: **23 de 24 proyectos byte-idénticos**; el único con delta es el que
un tercero está usando ahora mismo.

## Un riesgo REAL que esto abre, y que no se resuelve solo

`p-b7740b96d79ec013` —el lado derivado de skillgraph— tiene una **sesión SDDK
con viva** trabajando `wi-72-p3-expansion-apply`, con la fase build recién
completada. El alias `p-b7740b96d79ec013 -> p-74299cf88f51dab9` redirige la
identidad de ese proyecto.

Cada invocación de `sddk` vuelve a resolver identidad. Esa sesión, en su
siguiente comando, resolvería X. Sus ciclos nuevos caerían en X mientras los
anteriores quedan en Y: exactamente la separación que el alias existe para
evitar, pero en el sentido contrario.

Este caso es además el único de los 15 donde el lado derivado tiene **más**
eventos que el del receipt (105 frente a 84). Se anotó explícitamente en su
`reason` en vez de esconderlo, pero la dirección elegida no cambia porque el
heurístico de «quién tiene más» no es el criterio: el criterio es que el receipt
sea alcanzable, y porque un ledger sin receipt se deja de resolver mientras que
un receipt inalcanzable se pierde en silencio.

**Pendiente de decisión del operador:** deixar el alias de skillgraph, o retirarlo
hasta que esa sesión termine su work item. Retirarlo sube el audit a ≥1; dejarlo
es el estado actual.

## Decisión del operador: retirado

Elegido **retirarlo hasta que la sesión termine su work item**. Se reconstruyó la
tabla con los **14** aliases restantes; la anterior de 15 se conservó en
`/var/home/rubentxu/aliases-15-superseded.json` por si hay que volver.

**Cómo se retiró, y por qué así.** El store es append-only por construcción —
`AliasTable` no tiene `remove`—, así que retirar una entrada **no** es borrar una
línea del JSON a mano: sería editar por debajo del store, saltándose la única
garantía que el store existe para dar. Se reconstruyó la tabla desde cero
pasando por el store, de modo que cada una de las 14 vuelve a pasar validación
de formato, de resolución y de ciclo. Una retirada hecha por la vía sancionada
cuesta un segundo; una hecha a mano cuesta un alias que el store nunca vio.

### Verificación tras la retirada

```text
$ sddk project resolve --root $T --scope . --remote https://github.com/Rubentxu/CogniCode
project_id: p-c1fac1fea05615c6
identity_alias: p-2c63a808fcee924a -> p-c1fac1fea05615c6

$ sddk project resolve --root $T --scope . --remote https://github.com/Rubentxu/chronos
project_id: p-3416cfb8288f8964
identity_alias: p-55f14aab9263c12f -> p-3416cfb8288f8964

$ sddk project resolve --root $T --scope . --remote https://github.com/Rubentxu/skillgraph
project_id: p-b7740b96d79ec013
identity_alias: none

$ python3 scripts/migrate_project_identity.py audit
ids que NO coinciden con la derivacion actual: 1
  p-74299cf88f51dab9  ->  p-b7740b96d79ec013   (1 receipt)
      remote: https://github.com/Rubentxu/skillgraph
```

El audit baja a **1**, y esa única entrada es exactamente el par retirado: los
otros 14 siguen efectivos. El pin de este checkout no se toca
(`identity_source: pinned`, `identity_alias: none`).

**Reanudar cuando la sesión cierre su work item:** declarar
`p-b7740b96d79ec013 -> p-74299cf88f51dab9` con su `reason` (que incluye la
excepción de que el lado derivado tiene más eventos), y el audit vuelve a 0.


## Dirección de los 15, y por qué no es «el lado con más eventos»

| | receipts (X) | derivados (Y) |
|---|---|---|
| total de eventos | **3.504** | 433 |
| proyectos donde gana | 14 de 15 | 1 de 15 (skillgraph) |

El `reason` del pin de este repo lo dice sin ambigüedad: `p-63676b11dc0ef88f` es
«la identidad historica» y `p-995939af668a53d8` la creó la re-adopción del
2026-09-30. Ninguna dirección destruye datos — el alias redirige la resolución,
no mueve ficheros, y ambos ledgers siguen en disco. La pregunta es solo a qué
lado resuelven los checkouts: al del receipt.

## Lo que este lote NO hizo

- No tocó un solo ledger. Verificado por recuento de filas y por hash de cadena.
- No borró `.sddk/project-pin.json` de este checkout, como exigía el SCOPE.
- No promotionó ADR-0152 de `proposed` a `accepted`: sigue con un tercio de sus
  criterios verificados contra storage real, y el sexto depende de una decisión
  del operador.
- No publicó nada.
