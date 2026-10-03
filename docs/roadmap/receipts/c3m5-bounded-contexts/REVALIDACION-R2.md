# REVALIDACIÓN R2 — las cinco deudas de identidad, medidas hoy

> ## ⚠️ LÉASE ESTO ANTES QUE §3 Y §5 (corrección de 2026-10-03, sesión 69s)
>
> **Este documento no se reescribe**: se conserva entero como evidencia de lo
> que se midió, y por separado se declara lo que dio mal. Tres de sus resultados
> están mal, y los tres son del mismo tipo — **medir una tabla sin separar las
> poblaciones que contiene**, o buscar un artefacto donde la propia deuda dice que
> no está:
>
> | Sección | Afirma | Corrección medida |
> |---|---|---|
> | §3 | «187 ciclos», «107 `OPEN`» | **187 = 108 reales + 79 `__spine_import__`**; **107 = 28 + 79**. Ninguno de los dos números describe este proyecto |
> | §5 | `INC-DEBT-063` **NO VERIFICABLE** | Los tres recibos **existen**, en las rutas que la `references:` de la deuda cita. La deuda queda **confirmada** |
> | cabecera | «Binario usado: `sddk 2.5.3` (build del 2026-10-03 06:57)» | El fichero es de **2026-10-01 21:17:29 CEST**. **La revalidación se hizo con el binario viejo**, y por eso su §3 no vio que `sddk cycle list` ya existía |
>
> Lo que **sí** se sostiene de este documento, y por eso no se toca: `061`
> VIGENTE con 6 de 15 alias, `050` VIGENTE, `049` con premisa caducada y
> defecto nuevo vivo, y la observación de que **la población cambia mientras se
> mide** — que la revisión de vigencia de esta sesión confirma con
> `51 → 52 → 53` en un día.
>
> Corrección completa y su evidencia:
> [`REVISION-VIGENCIA-R2.md`](REVISION-VIGENCIA-R2.md).

---

**Fecha:** 2026-10-03 · **Alcance:** `INC-DEBT-049`, `050`, `060`, `061`, `063`
**Binario usado:** `sddk 2.5.3` (build del 2026-10-03 06:57)
**Ledger de este repo:** `~/.local/state/sddk/projects/p-63676b11dc0ef88f/ledger.sqlite`

> **Por qué existe este documento:** la instrucción del operador era *«deuda
> técnica severa reciente **verificando que sus criterios sigan vigentes**»*.
> Eso no es «resolver las cinco», es **comprobar si lo que dicen sigue siendo
> cierto**. Una deuda cuya premisa se movió no se resuelve borrándola ni se
> ignora: se revalida, y el resultado puede ser que el defecto sigue, que la
> cifra cambió, o que la premisa ya no se sostiene.

---

## 0. El error que casi me hace publicar un «resuelto» falso

**El ledger no está donde lo busqué primero.** Miré
`~/.local/share/sddk/projects/` y encontré nueve ficheros de **0 bytes**, y estuve
a punto de escribir *«no hay historia en esta máquina»*. **Era falso**, y lo
demontró el propio producto: `adopt status` imprime la ruta real del ledger, y
está en `~/.local/state/sddk/`. 333 proyectos, **37 con historia**.

**Y la segunda medición equivocada fue más cara.** Al revalidar INC-DEBT-061
conté la tabla `events` y obtuve **0 de 15** alias con historia partida, contra
los **6 de 15** que publica la deuda. Casi cerré una deuda `high/P1` con un
`0` que era de la tabla equivocada: los `from_id` tienen **0 eventos** pero sí
**ciclos**. Es la **sexta vez en esta sesión que se mide la cosa equivocada** y
produce un número que parece tranquilizador. Repetido sobre `cycles`: **6 de 15,
converge.**

**Las dos correcciones van escritas porque el patrón es el que se repite, no el
error concreto.**

---

## 1. INC-DEBT-061 — **VIGENTE**, con deriva de +1

`project-aliases.json` tiene 15 entradas (`schema_version: 1`). Alias cuyo
`from_id` **conserva ciclos**, es decir historia partida que la redirección
esconde:

| `from_id` | ciclos | OPEN | → `to_id` (ciclos) |
|---|---|---|---|
| `p-74299cf88f51dab9` | 12 | 6 | `p-b7740b96d79ec013` (56) |
| `p-1f3622e11c093341` | 14 | 6 | `p-733fb505b5a6bd2d` (125) |
| `p-2c63a808fcee924a` | 10 | 7 | `p-c1fac1fea05615c6` (72) |
| `p-0121424743c59ce2` | 10 | 3 | `p-f4d8f28cd78d443e` (39) |
| `p-de82af3e9774d9a0` | 5 | 1 | `p-7c4aff45199a2069` (30) |
| `p-4713ecf51e2080a6` | 1 | 1 | `p-033dccc0fef1a91f` (1) |
| **total** | **52** | **24** | |

**6 de 15 — coincide con lo publicado. Los ciclos: 52, donde la deuda dice 51.**
La diferencia es un ciclo nuevo creado desde que se midió, no un cambio de
criterio. **La propiedad sigue intacta y el único trabajo aquí es recontar.**

**Y el caso de este repo es el primero de la tabla:** `p-74299cf88f51dab9` con
**12 ciclos, 6 de ellos `OPEN`**, redirigidos a `p-b7740b96d79ec013` (56
ciclos). El propio alias declara en su `reason` que es el caso de
INC-DEBT-050. **Un ciclo `OPEN` que la autoridad no puede presentar** es
exactamente la forma que INC-DEBT-060 describe, y aquí está en el `from_id` de
un alias, no en la identidad que el CLI resuelve.

## 2. INC-DEBT-049 — la premisa central **ya no se sostiene**

La deuda afirma, en su título, que `adopt status` *«sigue reportando
`complete` sobre un storage vacío»*. **Las dos mitas son falsas hoy:**

| Lo que afirma | Medido |
|---|---|
| `adopt status` → `complete` | **`status: conflict`** — `detail: receipt identity differs from plan; refresh only accepts runtime metadata drift` |
| storage vacío | **187 ciclos**, 3,91 MB, 107 `OPEN` |
| el histórico (65 ciclos) vive bajo la identidad anterior | `p-995939af668a53d8` tiene **0 ciclos, 0,38 MB**; el histórico está en `p-63676b11dc0ef88f` |

**Causa medida:** existe el alias `p-995939af668a53d8 → p-63676b11dc0ef88f`,
creado el **2026-10-02T10:24:11Z** con razón *«re-adopcion derivo […] el receipt
y el historico estan en p-63676b11dc0ef88f (590 eventos frente a 0 en
p-995939af668a53d8)»*. La identidad que el CLI resuelve **es ahora la que tiene
la historia**, que es lo contrario de lo que la deuda denuncia.

**Lo que NO se cierra, y es importante:** el `status: conflict` es un defecto
**vivo y distinto**. Ya no es «PASS falso sobre storage vacío»; es *«el receipt
de adopción y el plan de identidad no coinciden, y `refresh` solo acepta deriva
de metadatos de runtime»*. **La deuda no se resuelve: su premisa se replaces,
y el defecto que la acompaña sigue ahí con otra forma.** Por eso su
`status:` sigue `open` y su hallazgo sigue sin registrar en ningún sitio.

## 3. INC-DEBT-060 — cifras caducas, **propiedad sin re-medir**

| | Publica | Medido |
|---|---|---|
| ciclos totales | 179 | **187** |
| `OPEN` | 91 (de 97 sin nombrar) | **107** en total |

Reparto actual: `OPEN` 107 · `CLOSED` 72 · `RELEASE_PENDING` 6 · `PAUSED` 1 ·
`RELEASED` 1.

**Lo que la deuda propone —que ninguna superficie enumera los ciclos— no se ha
re-medido aquí y no se da por bueno.** Lo que sí está medido es que **la
población cambió**: 8 ciclos nuevos y 16 `OPEN` nuevos desde la medición. Un
recuento de «97 de 179» aplicado a 187 filas es un número sobre una población
que ya no es la que se midió. **La propiedad puede seguir siendo cierta y el
número publicado es, con toda seguridad, falso hoy.**

## 4. INC-DEBT-050 (`critical/P1`) — el alias existe, **la historia partida también**

El último alias de la tabla es, literalmente, el caso de esta deuda:

```
p-74299cf88f51dab9 -> p-b7740b96d79ec013
reason: "Normalizacion de remote cambiada entre runtime 1.171.2 y 2.5.3
         (INC-DEBT-050): mismo remoto y misma ruta canonica,
         dos project_id distintos para el mismo proyecto"
created_at: 2026-10-02T17:03:33Z
```

**Existe el registro del caso**, pero registrarlo no lo resuelve: el `from_id`
conserva **12 ciclos con 6 `OPEN`** y la redirección los esconde igual. Es la
forma exacta de INC-DEBT-061 aplicada a la deuda que la originó. **Las dos
deudas son el mismo hecho medido desde dos ángulos**, y por eso resolver 061 sin
050 (o al revés) deja el hecho entero en pie.

## 5. INC-DEBT-063 — **no verificable tal como está escrita**

La deuda afirma que `cl-vault-graph`, `cl-vault-html-replica` y
`cl-vault-node-projection` se cerraron declarando un `cycle_id` inexistente.
**Los tres recibos no se encuentran** con esos nombres en
`tests/cycle-artifacts/`: hay **48 `RECEIPT.md`**, ninguno bajo esos tres
directorios, y `grep` de `p-63676b11dc0ef88f/<slug>` sobre los que existen no
devuelve nada para esos ciclos.

**No se cierra ni se confirma: se declara no verificable contra el estado
actual.** Las tres causas posibles —que los recibos se renombraran, que se
borraran, o que la cita de la deuda apunte a una ruta que ya no existe— **no se
distinguen midiendo el resultado**, y elegir una sin evidencia sería escribir la
misma clase de afirmación que esta sesión viene corrigiendo. Queda registrado
como `NOT_VERIFIABLE`, no como `resolved`.

---

## Resumen de lo que R2 puede y no puede hacer

| Deuda | Severidad | Estado revalidado |
|---|---|---|
| `INC-DEBT-061` | high/P1 | **VIGENTE.** 6 de 15 alias con historia partida; 52 ciclos (publica 51). Solo recontar. |
| `INC-DEBT-050` | critical/P1 | **VIGENTE.** El alias existe, la historia partida también (12 ciclos, 6 `OPEN`). |
| `INC-DEBT-049` | high/P1 | **PREMISA CADUCA**, defecto nuevo vivo: `status: conflict` con identidad divergente. |
| `INC-DEBT-060` | high/P1 | **CIFRAS CADUCAS** (179→187, 91→107). Propiedad sin re-medir. |
| `INC-DEBT-063` | medium/P2 | **NOT_VERIFIABLE**: los tres recibos citados no existen con esos nombres. |

**Lo que R2 es, con esto medido:** no es «resolver cinco deudas de identidad»,
es **«una sola deuda de identidad medida desde cinco ángulos»** — la
normalización del remote bifurcó el `project_id` (050), el alias lo redirige y
esconde la historia partida (061), la identidad que el CLI resuelve tiene ya la
historia pero su receipt no casa con el plan (049), la población de ciclos cambió
(060), y un recibo de esa historia no se puede localizar (063).

**Y hay un hecho que R2 no puede arreglar y que conviene escribir:** la
población de este clúster **cambia mientras se mide**. Entre la medición de
INC-DEBT-061 (51 ciclos) y esta (52) hay un ciclo nuevo; entre la de INC-DEBT-060
(179 filas) y esta (187), ocho. **Cualquier recuento de este clúster envejece
más rápido de lo que se corrige**, y por eso el remedio tiene que ser un
**guard que mida la propiedad cada vez**, no un número que se reescriba a mano.
Es el mismo motivo por el que `INC-DEBT-058` lleva una revisión de vigencia.
