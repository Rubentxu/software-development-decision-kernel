---
id: INC-DEBT-061
title: "Seis de los quince alias de proyecto ocultan 51 ciclos con nombre: el alias redirige, y un reparto no se puede redirigir"
status: open
severity: high
priority: P1
revalidated_at: 2026-10-03
revalidated_in_session: session-69s
fingerprint: "project_alias_redirect_hides_split_history"
fingerprint_aliases: []
cluster_id: CL-IDENTITY
created: 2026-10-02
created_by: miniMax Code (mvs_b98f2520808543c8bfd72b7d38e01c34)
owner: miniMax Code (mvs_b98f2520808543c8bfd72b7d38e01c34)
detected_at: 2026-10-02
detected_in_session: session-69j
component: identity
surface: ~/.local/state/sddk/project-aliases.json (artefacto de runtime, no versionado)
related: [INC-DEBT-050, INC-DEBT-059, INC-DEBT-060]
references:
  - crates/sddk-cli/src/lib.rs
  - crates/sddk-cli/src/project_alias.rs
  - docs/debt/INC-DEBT-050-REMOTE-CASE-NORMALIZATION-REASSIGNS-PROJECT-IDS-WITHOUT-MIGRATION.md
  - docs/architecture/adrs/ADR-0152-STORAGE-LEVEL-PROJECT-IDENTITY-ALIAS.md
  - scripts/audit_project_alias_split.py
---

## Qué es

> ## Revisión de vigencia (2026-10-03, sesión 69s) — VIGENTE, y el número es una fecha
>
> La deuda **sigue diciendo verdad**: **6 de los 15 alias** conservan historia
> partida. Ese 6 es estable desde que se detectó. **Lo que envejece es el
> recuento**, y conviene que la tabla de arriba se lea como la medición del día
> en que se hizo y no como el estado actual:
>
> | | publica (session-69j) | revalidación (69s) | **hoy (69s)** |
> |---|---|---|---|
> | alias con historia partida | 6 de 15 | 6 de 15 | **6 de 15** |
> | **ciclos apartados** | **51** | **52** | **53** |
> | `OPEN` apartados | — | 24 | **25** |
> | `p-74299cf88f51dab9` → `p-b7740b96d79ec013` | 12 / 442 | 12 / 442 | **12 / 443** |
>
> **51 → 52 → 53 en un día, sin que nadie toque el código.** La deriva viene del
> lado al que se redirige (`p-b7740b96d79ec013` pasó de 442 a 443), no de que el
> criterio cambie: los 12 ciclos del `from_id` siguen ahí, con 6 `OPEN`, y la
> redirección los esconde igual. **Un número reescrito a mano en esta deuda es una
> fecha, no un hecho** — y por eso el remedio que esta deuda necesita sigue siendo
> un guard que mida la propiedad cada vez.
>
> **Una medición mía que falló y va escrita porque es la segunda vez:** consulté
> `project_id` dentro del ledger de `p-63676b11dc0ef88f` y obtuve **0 alias con
> historia partida** — un `0` tranquilizador del mismo tipo que el `0 de 15` que
> la revalidación del mismo día ya había registrado. Los ciclos del lado apartado
> **no están en ese fichero**: están en el ledger del proyecto apartado. Medir
> esta deuda exige los **ledgers por proyecto**, no una tabla.
>
> `status: open`, `high`/`P1`, severidad sin cambio.
>
> Evidencia: `docs/roadmap/receipts/c3m5-bounded-contexts/REVISION-VIGENCIA-R2.md` §2.

**El alias de proyecto resuelve un problema que en 6 de 15 casos no era un
problema de identidad, sino de historia partida.** ADR-0152 autorizó una
tabla `from_id -> to_id` que el resolver aplica como **redirección**: cuando la
identidad derivada cae en un `from_id`, se responde con el `to_id`. Eso está
bien cuando la historia está **entera** en el destino, que es lo que el motivo de
cada alias afirma.

Medido sobre el storage real, **6 de los 15 alias tienen historia propia en el
lado `from`** — la historia que la redirección aparta. Cifras de `events_v1` y
de `cycles`, que es lo que el script del final reproduce:

| `from_id` (apartado) | `to_id` (destino) | eventos `from` | ciclos `from` | eventos `to` |
|---|---|---:|---:|---:|
| `p-4713ecf51e2080a6` | `p-033dccc0fef1a91f` | 6 | 1 | 16 |
| `p-1f3622e11c093341` | `p-733fb505b5a6bd2d` | **129** | **14** | 582 |
| `p-de82af3e9774d9a0` | `p-7c4aff45199a2069` | 68 | 4 | 199 |
| `p-2c63a808fcee924a` | `p-c1fac1fea05615c6` | 34 | 10 | 698 |
| `p-0121424743c59ce2` | `p-f4d8f28cd78d443e` | 124 | 10 | 348 |
| `p-74299cf88f51dab9` | `p-b7740b96d79ec013` | 84 | 12 | 442 |
| **total** | | **445** | **51** | **2 285** |

Contando **las seis** tablas append-only y no solo `events_v1`, el lado apartado
suman **556** filas —el reparto no es de un solo tipo de hecho—, y el lado
destino **4 221**.

Los 51 ciclos tienen nombre y son trabajo real: `h2-body-execution-engine`,
`coordinator-collapse`, `backlog-contract-hygiene`, `q05-release-action-pins`,
`ci-repair-release-closure`, `persistence-boundary-closure`… **Ninguno lo nombra
ningún comando**, porque ningún comando deriva un id que caiga en el lado
apartado.

## Por qué son inalcanzables, y por qué un pin no los salva

No es una consecuencia de la implementación: es **la intención declarada**.
`resolve_identity_honoring_pin_with` (`crates/sddk-cli/src/lib.rs:1590`) aplica
el pin **y después** el alias, y su propio comentario dice por qué en ese orden:

> a pin naming a retired id is exactly the input the alias exists to correct

Es decir: **un pin que nombre el id apartado es precisamente lo que el alias
existe para corregir.** No hay ruta canónica que llegue al lado `from`, y no es
un olvido — es la propiedad que ADR-0152 compró.

## El `reason` de 5 de esos 6 es falso

El campo `reason` de la tabla es un **registro durable de una decisión**, y dice
literalmente, para `p-1f3622e11c093341`:

> re-adopcion derivo p-1f3622e11c093341 desde
> `https://github.com/Rubentxu/pipeline-kotlin`; el receipt y el historico estan
> en p-733fb505b5a6bd2d

Hay 129 eventos y 14 ciclos con nombre **en el `from`**. El texto afirma algo que
el storage contradice, y quien lea la tabla para entender por qué existe ese alias
se lleva una conclusión falsa. El sexto (`p-74299cf88f51dab9`) tiene otro motivo
—normalización entre runtimes— y no afirma lo mismo, así que **5 de 6**, no 6.

## Lo que NO es este defecto

Tres hipótesis se mids y se **descartaron**, y escribirlas es la mitad del valor
del documento:

1. **«El arreglo de ADR-0152 está roto y escribe en el id retirado.»** Los
   ciclos de `p-1f3622e11c093341` tienen eventos hasta `2026-10-02T17:44:21Z`, y
   el commit que cerró el trabajo de alias es `bdb1da3c` (19:14 local = 17:14
   UTC), **30 minutos antes**. Leído así, el arreglo falló.
   **Medido y falso:** el binario instalado es **`sddk 2.2.27`, del 2026-09-29
   20:25** — anterior a la tabla de alias, que no puede leer porque no existe en
   su código. Los tres ciclos (`rp6a-lock`, `rp6b-input`, `rp6c-http-request`)
   los escribió un binario que **no podía** consultar el alias. **El arreglo no
   está roto: no está desplegado.** La medición que lo separa de la hipótesis
   equivocada fue una sola: **qué binario escribió**, no cuándo.
2. **«Los receipts contradicen la derivación.»** `scripts/migrate_project_identity.py audit`
   da **0 ids divergentes sobre 177 receipts**, con `selfcheck: OK (21
   normalizaciones + 8 rechazos + 2 ids heredados, contra el Rust real)`. Y no
   hay ningún `pinned_project_id` en el storage que nombre un `from_id`. El
   mecanismo funciona en lo que dice funcionar.
3. **«La tabla no existe / el audit está vacuo.»** La tabla **no** es una tabla
   SQLite: es `~/.local/state/sddk/project-aliases.json`, y `load_aliases` falla
   cerrado si no la puede leer. Se buscó primero en SQLite y se obtuvo «no
   existe», que era la cuarta vez en esta sesión que una conclusión salía de
   medir la superficie equivocada.

## Por qué migrar no lo arregla

**La migración está medida como imposible, y esta incidencia lo remide.** El
`project_id` entra en el `content_hash` de `events_v1`:

```
ledger p-63676b11dc0ef88f, primer evento, sequence=1
  content_hash original                sha256:ce02abb239b67d540…
  trigger BEFORE UPDATE                -> RECHAZA: "events_v1 are append-only"
  trigger retirado, UPDATE ejecutado   -> ACEPTADO
  content_hash después                 sha256:ce02abb239b67d540…   (NO cambia)
```

Con el trigger puesto, la escritura no pasa. **Sin** el trigger, pasa — y el
hash no se recalcula, luego `verify_stream_chain` falla con `hash_drift` para
siempre. Rehacer los 445 eventos con el hash recalculado es **reescribir un fact
log encadenado**, que es exactamente lo que el fact log es.

## Gravedad: `high` y no `critical`, y por qué

**No hay pérdida de datos.** Las 445 filas y los 51 ciclos están íntegros,
consistentes y legibles — por SQLite, que es rodear el producto entero. Es el
mismo razonamiento por el que INC-DEBT-060 bajó de `critical` a `high`: el daño
es de **visibilidad por el producto**, y el único rodeo es leer la base a mano.

Sube a `critical` si se cumple alguna de estas dos, y ninguna se puede afirmar
hoy:

- una ruta de escritura que **vuelva a crear** historia en un lado apartado
  (imposible mientras 2.5.3 no esté instalado, porque ningún binario desplegado
  conoce la tabla);
- o que el operador decida que esos 51 ciclos deben ser **nombrables por
  comandos**, en cuyo caso la respuesta no es una migración sino una segunda
  autoridad de lectura.

## Remedios, con su coste medido

**1. Corregir el `reason` de los 5 alias que afirma algo falso.** Aditivo,
reversible, sin tocar el storage. Es lo único que es claramente correcto y
**queda escrito aquí como recomendación, no ejecutado**: el fichero está fuera
del repo y `store_alias_table` es append-only por diseño, luego reescribir un
`reason` es una decisión sobre la inmutabilidad de la tabla, no un ajuste de
texto.

**2. Declarar el reparto en vez de ocultarlo.** Un alias que además de `to_id`
declare `also_reads: [<from_id>]` y un resolver que **una** en la lectura —no en
la escritura— dejaría los 51 ciclos nombrables sin mover una fila. Coste: es
**diseño, no parche**, y roza ADR-0152, que hoy dice que el alias es una
redirección. Requiere SCOPE + ADR propio.

**3. Aceptar la partición y escribirla.** Los 51 ciclos quedan bajo el id que
los-originó, y el puntero del repo lo dice. Coste: cero código; pérdida: no hay
`cycle list` que los muestre, y esa es la misma queja que INC-DEBT-060.

**Lo que NO es una opción:** migrar. Medido imposible, y por el motivo del
apartado anterior.

## Límites declarados

- La medición es **de esta máquina**. El mecanismo es independiente del entorno,
  pero los 6 ids affected son de aquí.
- **No se ha modificado** `project-aliases.json` ni ningún ledger. Todo lo de
  arriba es lectura.
- El recuento de 51 ciclos usa `cycles.project_id`; **no** se ha medido si
  alguno de esos 51 aparece ya en el lado `to` por otro camino, luego 51 es un
  **techo**, no un número de ciclos distintos.
- `scripts/audit_project_alias_split.py` es nuevo y **no** está cableado a
  `release.sh`: informa, no bloquea. Escribir en la tabla de alias desde un
  gate de release sería el defecto que esta misma incidencia describe.

## Reproducir

```bash
python3 scripts/audit_project_alias_split.py
```

Imprime, por alias, las filas append-only de cada lado y **declara** los que
apartan historia. Sale distinto de cero cuando hay alguno, y es un
**informe**, no un gate.
