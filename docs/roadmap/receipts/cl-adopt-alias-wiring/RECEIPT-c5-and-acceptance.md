# RECEIPT — ADR-0152 criterio 5 cerrado y ADR promovido a `accepted`

- **Ciclo:** `cl-adopt-alias-wiring`
- **Fecha UTC:** 2026-10-02
- **WorkItem:** cierre del criterio 5 de
  [`ADR-0152`](../../../architecture/adrs/ADR-0152-STORAGE-LEVEL-PROJECT-IDENTITY-ALIAS.md)
  y promoción del ADR
- **Veredicto:** criterio 5 **VERDE** (audit: 0 huérfanos), **PASS=20 FAIL=0**;
  ADR `proposed` → **`accepted`**
- **HEAD al empezar:** `d88c92df`
- **Decisión del operador:** obtenida por cuestionario en esta sesión

---

## 1. Por qué estaba ROJO, y por qué la causa hubo que medirla

El audit daba **1 id divergente** sobre 161 receipts:
`p-74299cf88f51dab9 -> p-b7740b96d79ec013`, de `skillgraph`.

Había dos reparaciones posibles y son opuestas:

| Reparación | Qué afirma | Coste |
|---|---|---|
| **Declarar el alias** | Ese checkout es ese proyecto, siempre lo fue | Ninguno: un append |
| **Retirar el recibo** | Ese id fue una derivación errónea | Destruye el registro de una adopción real |

Elegir mal es irreversible en una dirección y en la otra no, así que la causa se
midió antes de decidir. Los dos recibos:

| | `p-74299cf88f51dab9` | `p-b7740b96d79ec013` |
|---|---|---|
| `remote_url` | `https://github.com/Rubentxu/skillgraph` | el mismo |
| `canonical_workspace_path` | `…/Proyectos/python/skillgraph` | el mismo |
| `identity_source` | `remote` | `remote` |
| `timestamp` | 2026-09-26T11:47:36Z | 2026-10-01T19:40:15Z |
| `runtime_version` | 1.171.2 | 2.5.3 |

Mismo remoto, misma ruta, cinco días y dos runtimes de distancia. **El mismo
proyecto adoptado dos veces porque el normalizador de remote cambió** —
INC-DEBT-050, que es literalmente esto. El recibo no era espurio: registrar un
evento que ocurrió no es un residuo.

## 2. Por qué estaba bloqueado, y por qué el bloqueo caducó

El alias se había retirado porque había una sesión SDDK concurrente sobre
`wi-72-p3-expansion-apply`. Esa sesión se verificó antes de asumir nada:

- el ciclo `p-b7740b96d79ec013/wi-72-p3-expansion-apply` está **`CLOSED`**
  (`state: release`);
- `cycle_leases`: **0 filas**, ningún lease vivo;
- último evento del ledger: `2026-10-02T14:14:24Z`, más de 2 h 45 min antes de la
  comprobación.

**La razón del bloqueo había caducado.** Ese es el dato que convirtió una
decisión cerrada en una decisión abierta, y por eso se comprobó en vez de
recordarlo.

## 3. La declaración

Superficie canónica: `sddk project alias --from … --to … --reason …`. Exige
`--reason`, rechaza un `--to` sin ledger («un alias debe apuntar a un proyecto que
ya existe, o redirige este proyecto a un estado vacío») y tiene `--dry-run`.

```
$ sddk project alias --from … --to … --reason … --dry-run
would declare: p-74299cf88f51dab9 -> p-b7740b96d79ec013
resolves to: p-b7740b96d79ec013
# sha256 del store antes y después del dry-run: 3052169c… (sin cambios)

$ sddk project alias --from … --to … --reason …
declared: p-74299cf88f51dab9 -> p-b7740b96d79ec013
```

Store: **14 → 15 entradas**, sha256 `3052169c…` → `f5d3e88d…`. Backup previo en
`/var/home/rubentxu/inc059-backup/alias-c5/project-aliases.json.bak`, con el
mismo sha256 que el original.

## 4. Resultado

```
$ python3 scripts/migrate_project_identity.py audit
selfcheck: OK (21 normalizaciones + 8 rechazos + 2 ids heredados, contra el Rust real)
receipts revisados: 161
ids que NO coinciden con la derivacion actual: 0
```

Segunda cláusula del criterio —conservación de filas— también en verde, frente
al baseline de las 09:49:

```
4.359 filas -> 4.714 ahora; 4 tablas crecen, 0 decrecen
```

## 5. Falsificador

`/var/home/rubentxu/c5/01-falsify.py` — **PASS=20 FAIL=0**.

| # | Aserción |
|---|---------|
| M1 | el audit dice 0 ids divergentes y sale con código 0 |
| M2 | el camino canónico resuelve al id canónico y **no** declara saltos |
| M3 | resolviendo el id **retirado** (vía pin, en un checkout desechable) resuelve al canónico, declara el salto y el texto imprime `identity_alias: from -> to` |
| M4 | ningún proyecto con filas en el baseline ha perdido filas; el total no ha bajado; los 6 que ya venían sin ledger siguen sin él |
| M5 | `events_v1` sigue rechazando `UPDATE` y `DELETE` |
| M6 | ningún alias apunta a un id sin ledger; la tabla tiene 15 entradas |
| M7 | el store cambió en **exactamente** una entrada, la declarada; ninguna retirada; `schema_version` intacto |

M2 merece nombres: `alias_hops` **vacío** en el camino canónico no es un fallo de
la tabla, es que hoy la derivación cae directa en el id canónico y no hay alias
que consultar. El salto se ve al resolver el id retirado (M3).

M3 se midió en un checkout desechable con el pin en el id retirado **a propósito**:
escribir ese pin en el repo real de `skillgraph` sería intrusión (AGENTS.md §1).

## 6. Dos FAIL que eran míos

Ninguno era un defecto del producto; los dos están aquí porque el camino que los
produce es información.

1. **`alias_hops` no son objetos.** La primera aserción pedía
   `[(from, to) por salto]` y los saltos son cadenas: `alias_hops` es la lista de
   ids **retirados** atravesados y el superviviente va en `project_id`. El `from`
   y el `to` que el criterio 3 exige salen en el formato de texto.
2. **Seis proyectos «perdidos» que no se perdieron.** El primer recuento marcó
   como pérdida a seis proyectos que el propio baseline registra como
   `{"missing": true}`: ya no tenían ledger cuando se tomó a las 09:49. Contar un
   artefacto del baseline como regresión habría producido un FAIL falso, que es
   peor que ningún FAIL, porque fabrica una alarma sobre datos que están bien.

## 7. Los otros tres criterios, medidos para poder promover

`/var/home/rubentxu/c1c2/01-falsify.py` — **PASS=14 FAIL=0 SKIP=1**.

- **C1 verde y falsificado.** Sin alias el id es el que dice el pin; declarado el
  alias, el mismo checkout resuelve al `to_id` y declara el salto. El
  falsificador `A -> A` falla con código no cero, **no se escribe**, dice
  `self-alias`, y resolver ni se cuelga ni cambia de id.
- **C2 verde y falsificado.** `B -> A` sobre una tabla donde `A -> B` ya existe
  falla con código no cero, nombra el ciclo, **no se escribe**, y la resolución
  sigue en `B`.
- **C4 no tiene falsificador ejecutable.** No existe superficie de borrado:
  `remove_alias`, `unalias` y `delete_alias` dan **cero** coincidencias, y
  `project unalias`, `project alias --delete` y `project alias --remove` no
  existen como comando. El criterio decía «intentar borrar uno falla», que suena
  a un rechazo comprobable y **no lo es**: no hay nada que intentar. Declararlo
  PASS habría sido la forma más barata de mentir del documento, así que **el
  criterio 4 se reescribió** a lo que sí es cierto y sí es falsificable —la
  superficie de la CLI es append-only, y un auto-alias se rechaza— dejando escrito
  el límite que la redacción anterior escondía: **el invariante es de la
  herramienta, no del almacenamiento**. `project-aliases.json` es JSON plano, y
  quien lo edite a mano cambia la tabla sin que nada lo note. Ya ocurrió una vez
  en esta máquina: el alias de `skillgraph` se retiró por esa vía, y el
  resultado fue que el criterio 5 dio ROJO.

## 8. Promoción

`status: proposed` → **`accepted`**, con `accepted_at: 2026-10-02` y
`accepted_by_cycle: p-63676b11dc0ef88f/identity-alias`, que es lo que exige
ADR-0001 §3.4 (`tests/test_adr_promotion_format.sh`).

La razón de promover no es que seis criterios estén en verde —uno se reescribió
para serlo—, sino que **la decisión ya está en vigor en la máquina**: 15 aliases
declarados, el storage resolviéndose por ellos, y el operador eligiendo
explícitamente este mecanismo sobre el alternativo.

**Lo que la aceptación NO cierra:** `closes:` pasa a `[]` y las dos pasan a
`addresses:`. INC-DEBT-050 (`critical`) e INC-DEBT-049 (`high`) **siguen
`open`**. Sus criterios de cierre están escritos en esos documentos, no en este
ADR, y cerrarlos sería un paso de gobernanza que nadie pidió. Lo que este trabajo
les aporta queda escrito en sus propios documentos: el alias logra el efecto
observable que la migración destructiva buscaba, sin destruir receipts.

## 9. Reproducir

```bash
python3 /var/home/rubentxu/c5/01-falsify.py    # PASS=20 FAIL=0
python3 /var/home/rubentxu/c1c2/01-falsify.py   # PASS=14 FAIL=0 SKIP=1
python3 /var/home/rubentxu/c6/02-falsify.py    # PASS=12 FAIL=0  (criterio 6)
python3 scripts/migrate_project_identity.py audit
```
