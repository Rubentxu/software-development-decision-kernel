# REVISIÓN DE VIGENCIA R2 — las cuatro deudas que faltaban, medidas hoy

**Fecha:** 2026-10-03 · **Alcance:** `INC-DEBT-050`, `060`, `061`, `063`
(`049` la revisó el commit `a2b0bd13`, que registró su defecto vivo).
**Ledger de este repo:** `~/.local/state/sddk/projects/p-63676b11dc0ef88f/ledger.sqlite`
**Binarios usados, y son dos distintos:** `/home/rubentxu/.local/bin/sddk` (instalado,
`2.5.3`, mtime **2026-10-01 21:17:29 CEST**) y un build de HEAD de esta sesión
(`/var/home/rubentxu/cargo-targets/debug/sddk`, `2.5.3`, `commit e2754504`, `source: git`).

> **Qué es este documento y qué no.** `REVALIDACION-R2.md` comprobó si las cinco
> deudas seguían diciendo verdad. Este hace la otra mitad de lo que el diario
> nombró como paso preciso: **la revisión de vigencia, que NO es un cierre.**
> Ninguna deuda se cierra aquí. Lo que se hace es sustituir, donde el número
> medido hoy no coincide con el publicado, el número por el medido — y dejar
> escrito cuándo se midió, porque este clúster **cambia mientras se mide**.

---

## 0. Dos correcciones a la revalidación, y una es la que casi cerró una deuda

### 0.1 `INC-DEBT-063` **no era `NOT_VERIFIABLE`. Era verificable y sigue vigente**

La revalidación buscó los tres recibos en `tests/cycle-artifacts/`, no los
encontró entre los 48 `RECEIPT.md` y declaró la deuda no verificable. **Los tres
existen**, y en exactamente las rutas que la **propia deuda** cita en su
frontmatter `references:`:

```
docs/roadmap/receipts/cl-vault-graph/RECEIPT.md          10014 bytes
docs/roadmap/receipts/cl-vault-html-replica/RECEIPT.md    5538 bytes
docs/roadmap/receipts/cl-vault-node-projection/RECEIPT.md 11161 bytes
```

Los tres declaran su `**Cycle:**`, y **ninguno de los tres identificadores existe
en la autoridad**:

| Recibo | `cycle_id` declarado | ¿existe en el ledger? |
|---|---|---|
| `cl-vault-graph` | `p-63676b11dc0ef88f/vault-graph` | **0 filas** |
| `cl-vault-html-replica` | `p-63676b11dc0ef88f/vault-html-replica` | **0 filas** |
| `cl-vault-node-projection` | `p-63676b11dc0ef88f/vault-node-projection` | **0 filas** |

**El `NOT_VERIFIABLE` era un error de directorio, no una propiedad del mundo.** Es
la **séptima vez** en esta serie que se mide la cosa equivocada, y la segunda
consecuencia grave: la primera dio un `0` tranquilizador sobre `INC-DEBT-061`, y
esta iba a dejar una `medium/P2` sin verificar por no haber mirado donde la deuda
ella misma dice que está. La deuda **no se cierra: se confirma**, con la evidencia
que le faltaba.

### 0.2 Los recuentos «187 ciclos / 107 `OPEN`» **mezclan dos poblaciones**

La revalidación (§3) mide la tabla `cycles` entera. **Esa tabla contiene dos
poblaciones distintas que comparten fichero**, y sumarlas produce un número que no
describe ninguna:

| Población | `project_id` / `workspace_id` | filas | `OPEN` | manifiesto |
|---|---|---|---|---|
| **Ciclos reales** | `p-63676b11dc0ef88f` / `w-2e7853aadc28217a6649e309` | **108** | **28** | 106 legibles, 2 ilegibles |
| **Importación spine** | `__spine_import__` / `__spine_import_ws__` | **79** | **79** | las 79 con `{}` |
| Suma de las dos | — | 187 | 107 | — |

**Las 79 filas de `__spine_import__` son las 79 con manifiesto `{}`, las 79
`OPEN`, y las 79 que `INC-DEBT-060` ya identificaba como población aparte.** Sus
identificadores no son ciclos de este proyecto sino entradas importadas
(`GOV-ROADMAP-001`, `DW-IR-001`, `TEST-MODEL-001`…). Por eso el `107` de la
revalidación no es «los `OPEN` de este proyecto»: es 28 reales más 79 importados.

---

## 1. `INC-DEBT-060` — la propiedad es **falsa hoy** para la población que describía

Esta es la medición que la revalidación declaró explícitamente **no hecha**, y es
la que R2 tenía que hacer.

`sddk cycle list` (introducido en `113f84ba`, 2026-10-02) contra el binario de
HEAD y el ledger real:

```
project: p-63676b11dc0ef88f
cycles: 108
unreadable_manifests: 2
status[CLOSED]: 72   status[OPEN]: 28   status[PAUSED]: 1
status[RELEASED]: 1  status[RELEASE_PENDING]: 6
```

Contraste fila a fila entre las 108 filas reales de la tabla y las 108 entradas
que el comando nombra:

| | |
|---|---|
| filas reales del proyecto | **108** |
| nombradas por `cycle list` | **108** |
| **reales NO nombradas** | **0** |
| nombradas que no son filas reales | **0** |

**El criterio de la deuda —«97 de los 179 ciclos no los nombra ningún
comando»— es falso hoy para la población real: 0 de 108.** Y el `2` que declara
`unreadable_manifests` **coincide exactamente** con las 2 filas reales cuyo
manifiesto no deserializa, o sea que marca lo que no puede leer en vez de
omitirlo en silencio. Las 79 filas de `__spine_import__` quedan fuera **por
alcance de `project_id`**, que es lo correcto: no son ciclos de este proyecto.

**Lo que queda abierto, y no es lo que el título dice:** no es que la autoridad no
pueda enumerarse, es que hay una **segunda población** — 79 filas importadas,
todas `OPEN`, todas con manifiesto `{}`, en el mismo fichero — que ninguna
superficie del producto nombra ni declara. Eso es lo que `CURRENT.md` ya
conseguía decir con «solo las 79 filas `__spine_import__`», y ahora está medido
en vez de coursework. **`status:` sigue `open`; lo que caduca son las cifras.**

---

## 2. `INC-DEBT-061` — VIGENTE, y el número sigue moviéndose

Medido sobre los **ledgers por proyecto** (que es donde están los ciclos del lado
apartado), no sobre la tabla `cycles` de un solo fichero:

| `from_id` | ciclos | `OPEN` | → `to_id` (ciclos) |
|---|---|---|---|
| `p-74299cf88f51dab9` | 12 | 6 | `p-b7740b96d79ec013` (**57**) |
| `p-1f3622e11c093341` | 15 | 7 | `p-733fb505b5a6bd2d` (125) |
| `p-2c63a808fcee924a` | 10 | 7 | `p-c1fac1fea05615c6` (72) |
| `p-0121424743c59ce2` | 10 | 3 | `p-f4d8f28cd78d443e` (39) |
| `p-de82af3e9774d9a0` | 5 | 1 | `p-7c4aff45199a2069` (30) |
| `p-4713ecf51e2080a6` | 1 | 1 | `p-033dccc0fef1a91f` (1) |
| **6 de 15 alias** | **53** | **25** | |

**6 de 15 es estable** desde la creación de la deuda. Los **ciclos apartados no
lo son**: la deuda publica **51**, la revalidación del mismo día mide **52**, y
esta mide **53**. Lo que crece es el lado al que se redirige (`56 → 57`), o sea
que la deriva viene de que el proyecto vive, no de que el criterio cambie.

**Mi propia medición falló aquí una vez y por el motivo exacto que la
revalidación ya había documentado.** Consulté `project_id` dentro del ledger de
`p-63676b11dc0ef88f` y obtuve **0 alias con historia partida** — un `0` igual de
tranquilizador que el `0 de 15` de aquella. Los ciclos del lado apartado **no
están en ese fichero**: están en el ledger del proyecto apartado. Un `0`
obtenido así no es un dato débil, es un dato sobre la tabla equivocada.

## 3. `INC-DEBT-050` — VIGENTE, y la historia partida es la de siempre

El alias que origina la deuda existe y su `reason` la cita textualmente:

```
p-74299cf88f51dab9 -> p-b7740b96d79ec013
created_at: 2026-10-02T17:03:33Z
reason: "Normalizacion de remote cambiada entre runtime 1.171.2 y 2.5.3
         (INC-DEBT-050): mismo remoto y misma ruta canonica,
         dos project_id distintos para el mismo proyecto"
```

**12 ciclos, 6 de ellos `OPEN`, siguen en el `from_id` y la redirección los
esconde igual.** Registrar el caso no lo resolvió entonces y no lo resuelve
ahora: es `INC-DEBT-061` aplicada a la deuda que la originó, y por eso las dos
son **un solo hecho medido desde dos ángulos**. Su `open_part` (las dos salidas
son del operador, y la segunda —«una segunda autoridad de lectura»— es
correctamente unreachable por migración) **no ha cambiado**. `status:` sigue
`open`, `critical`/`P1`, y sin cambio de severidad: no hay ninguna vía por la que
esta dejar de ser `critical` mientras haya 6 `OPEN` que la autoridad no presenta.

---

## 4. El hallazgo que no estaba en ninguna deuda: el binario instalado no es el código

Esto no es R2 de identidad de proyecto, pero salió de medir R2 con el binario
real y es la medición más dura de esta sesión, así que va aquí y no en un
paréntesis.

El binario del `PATH` **reporta `2.5.3`** y **está 202 commits por detrás de
`HEAD`**:

| | instalado | HEAD |
|---|---|---|
| versión que declara | **`2.5.3`** | `2.5.3` (workspace) |
| origen | `90f16ad2` (2026-10-01 21:10:43) | `a2b0bd13` |
| `sddk cycle list` | **no existe** | existe (`113f84ba`) |
| `sddk dev build-id` | **no existe** | existe (`032e9553`) |
| tag publicado de referencia | — | `v2.5.2` en `818d4ff9` |

**La versión que declara es exactamente la del workspace todavía sin
publicar.** `v2.5.2` se publicó a las 18:22:15Z y el binario se construyó a las
21:17:29 CEST, tres horas después, sobre un árbol que ya decía `2.5.3`. O sea
que **comparar números de versión no puede detectar este binario**: coincide
con el código de hoy, y es otro código. Y el mecanismo que sí lo detectaría
—`dev build-id --check`— **no está en el binario**, luego **el artefacto que
tiene el problema no puede ejecutar el check que lo encuentra**. Eso no estaba
escrito: `INC-DEBT-064` dice que el check «no tiene dientes hasta la próxima
release», y aquí está el caso exacto, con la forma más difícil — la
autoinspección ausente.

**Y una corrección de la revalidación, que fechó mal el binario que usó.** Su
cabecera dice *«Binario usado: `sddk 2.5.3` (build del 2026-10-03 06:57)»*. El
fichero en disco es de **2026-10-01 21:17:29 CEST**. Es decir: **la medición de
`REVALIDACION-R2.md` se hizo con el binario viejo**, y eso explica por qué no vio
que `cycle list` ya existía — un binario de antes de `113f84ba` no lo tiene, y
por eso la propiedad parecía no medir.

**Lo que esto hace con `INC-DEBT-064`, y lo que no hace.** La condición de escalada
—«si `dev doctor` declara coherencia donde no la hay»— **sigue sin cumplirse**:
`dev doctor` en el binario instalado no tiene el check que la evaluaría. La deuda
**sigue `open`, `high`/`P1`, sin cambio de severidad**, y este hallazgo **no la
cierra**: la confirma con el artefacto en la mano. Lo que sí establece es que
**R1 no puede cerrarse sobre un artefacto de este repo instalado a mano**: el
gate que el objetivo define (`instalar artefacto publicado` → `build-id --check`
→ `Matches`) tiene que ejecutarse contra lo que publica `scripts/release.sh`, y
`release.sh` es lo que exporta `SDDK_GIT_SHA`. Un binario construido a mano no
puede producir `source: env` por construcción, luego **el gate de R1 es
precisamente el que detecta esta condición**, y por eso la ventana de
«declarado pero no publicado» es exactamente donde este defecto es observable.

**Nota aparte, medida y no causada por lo anterior:** el build de HEAD de esta
sesión declara `commit: e2754504` siendo `HEAD` `a2b0bd13`, y `e2754504` es un
**ancestro real** de las 08:10 de hoy. `source: git`, luego por STOP 6 el
`--check` contesta `relation: Unknown` + exit 1 en vez de `diverged`, que es el
comportamiento declarado. Un build de debug sin `SDDK_GIT_SHA` embebido puede
quedarse con la identidad de una ejecución anterior; los builds de release la
toman del entorno y no tienen esa vía. **No es una regresión**, pero es la razón
por la que este documento **no** usa el binario de debug para afirmar nada sobre
build identity, solo para medir enumeración.

---

## 5. Resumen de la revisión de vigencia

| Deuda | Severidad | Vigencia medida | Qué cambia en el fichero |
|---|---|---|---|
| `INC-DEBT-060` | high/P1 | **PROPIEDAD CADUCADA**: 0 de 108 sin nombrar. Queda la 2.ª población (79 `__spine_import__`) | cifras 179→108 y 91→28; `status` sigue `open` |
| `INC-DEBT-061` | high/P1 | **VIGENTE**: 6 de 15 alias, **53** ciclos (publica 51) | 51→53 |
| `INC-DEBT-063` | medium/P2 | **CONFIRMADA**: los 3 recibos existen y sus 3 `cycle_id` no | se retira el `NOT_VERIFIABLE` |
| `INC-DEBT-050` | critical/P1 | **VIGENTE**: 12 ciclos, 6 `OPEN` en el `from_id` | sin cambio de severidad |
| `INC-DEBT-064` | high/P1 | **VIGENTE y confirmada con el artefacto**: binario instalado 202 commits atrás declarando la versión no publicada | sin cambio de severidad |

**Ninguna deuda se cierra. Ninguna severidad baja.** Lo que se corrige son
cifras y una conclusión equivocada, y la corrección va **delante** del texto
original en cada fichero, sin reescribir lo que se creía al detectar.

**Y la lección de esta revisión, que es la segunda vez que la misma serie la
paga: un recuento de este clúster envejece mientras se lee.** En un día y sin
que nadie toque el código: 51 → 52 → 53 ciclos apartados, 56 → 57 en el lado
redirigido, 179 → 187 filas, 81 → 79 ilegibles. **Cualquier número reescrito a
mano en estas deudas es una fecha, no un hecho.** El remedio sigue siendo un
guard que mida la propiedad cada vez, y el guard que falta sigue siendo el
mismo que la revalidación señaló: por eso R2 **no puede cerrarse** con esta
revisión, solo puede quedar **revalidada**.

---

## 6. El guard que R2 necesita: **uno sí, otro no**, y el que no se escribe

Las secciones anteriores concluyen que *«cualquier recuento de este clúster
envejece más rápido de lo que se corrige, y el remedio tiene que ser un guard
que mida la propiedad cada vez»*. Eso admite **dos** guards distintos, y solo uno
se puede escribir hoy. Decirlo es parte del trabajo, porque un guard que mide una
cosa y se llama como si midiera la otra es un guard que miente por el nombre.

### 6.1 `INC-DEBT-060` — guard entregado

`tests/test_cycle_list_total_reconciliation.sh`, con su autofalsación en
`tests/test_cycle_list_total_reconciliation_mutation.sh`.

**La propiedad que mide, y por qué es la única que no es vacía.** `cycle list`
imprime su total como `output.cycles.len()` (`crates/sddk-cli/src/cycle.rs:2115`),
o sea **después** del filtro y sobre el **mismo** vector que emite. Por eso
*total declarado == filas emitidas* es insatisfacible: no puede fallar sin que el
comando no corra, y un guard que solo mide eso da verde siempre. **Se intentó
primero y se descartó por esa razón**, y el error estaba en el diseño del
guard, no en el producto, que es por lo que va escrito aquí.

La comparación que sí tiene dientes es **de fuente cruzada**: lo que declara el
producto contra las filas que la autoridad tiene para el proyecto que el propio
producto declara. Vienen de sitios distintos, luego pueden discrepar, y
discrepan en cuanto `list_cycles` gana un filtro, un `LIMIT`, un alcance por
`workspace_id`, o empieza a descartar en vez de contar los manifiestos ilegibles.
**Ese es exactamente el modo de fallo que produjo las cifras de «187 ciclos» y
«107 `OPEN`».**

**Hermético y con el producto real, no un mock.** El state home es temporal por
caso (`SDDK_STATE_HOME`) y el ledger lo crea **el propio producto**, con lo que el
esquema es el suyo y este guard no lleva una segunda copia de las migraciones. El
`project_id` se descubre del stdout del producto, no se supone. **Seis** casos:
una población; **dos poblaciones** (el que discrimina: debe declarar 5, no 9);
ilegibles contados y marcados; las dos poblaciones con `{}` a la vez; proyecto
vacío; y el camino del filtro por estado.

**Falsado con nueve modos de mentira, todos detectados, y un control que exige que
el caso bueno se ACEPTE.** El control es lo que separa un guard de un guard
vacío: uno que rechazase todo pasaría la falsación sin vigilar nada. **Y cada
mutación corrompe UNA sola declaración**, porque una mutación compuesta no puede
decir cuál de las comprobaciones la.CASCADE — que fue el error del primer
falsificador, que llegó a «descubrir» que cinco comprobaciones eran la misma
repetida. Y una comprobación sin mutación propia es una comprobación cuya
necesidad queda sin demostrar: por eso hay nueve y no seis.

**Y la autofalsación de la autofalsación:** el segundo fichero exige que **cada
comprobación sea load-bearing por separado** (G0..G8), borrándola en una copia de
sandbox y requiriendo que se pierda exactamente la mutación que dice. Sin eso, un
guard puede rechazar las nueve mutaciones **por un efecto colateral** y seguir
pareciendo que vigila la propiedad. `PASS=10 FAIL=0`, con las ocho comprobaciones
siendo necesarias una por una.

### 6.2 `INC-DEBT-061` — **no hay guard, y está decidido que no lo haya**

`061` **no tiene guard y no se le escribe uno en este ciclo.** No es una
prioridad pospuesta: es que **no hay superficie de producto que pueda
reconciliarse contra la autoridad**. La propiedad de `061` —*un `from_id` de
alias que conserva ciclos y la redirección los esconde*— es cierta **por
construcción del almacenamiento**, no por un defecto de una declaración. Hoy se
midió contra la tabla, que es el mismo camino que la revalidación del mismo día ya
usó, y este guard no puede cerrarlo porque no hay un comando cuya salida contrastar.

Escribir un script que mida esa propiedad y no tenga consumidor sería **reproducir
exactamente el hueco de `INC-DEBT-065`**: una estructura a medio hacer que nadie
consulta, con la forma de un gate y el consumo de una nota. **Un guard que mide
algo que ningún gate consulta no vigila: informa.** Y lo que `061` necesita no es
un guard, es la **decisión del operador** sobre sus dos salidas, escritas en su
propio `open_part` y en §3 de este documento.

**La consecuencia práctica, y por qué esto no retrasa nada:** cuando el operador
decida, el remedio de `061` será una superficie —un comando que nombre la historia
apartada, o una segunda autoridad de lectura— y **entonces** habrá algo que
reconciliar y el guard tendrá sentido. **El guard de `060` no espera a esa decisión
y no la suplanta.**

### 6.3 Lo que este guard **no** demuestra, escrito para que no se lea al revés

- **No demuestra que el ledger real de este repo esté reconciliado.** Mide
  fixtures cuya verdad el propio guard conoce. El contraste contra el ledger real
  se hizo **a mano** en §1 (108 de 108) y es evidencia de esta sesión, no del
  guard. Los fixtures existen para que el guard sea determinista y hermético, no
  para fingir que vigilan el ledger del operador.
- **No cubre la población `__spine_import__`.** Esas 79 filas quedan fuera del
  alcance de `project_id` del proyecto, luego el guard **no puede** detectarlas y
  su ausencia es correcta. Vigilarla requiere poder *pedir* esa población, y hoy
  no hay comando que lo haga (§1 lo deja escrito).
- **No comprueba `sddk ledger events` ni ninguna otra superficie.** Solo la de
  `cycle list`. La clase de defecto —*declarar una población que la autoridad no
  tiene*— es la misma que se corrigió en `F63`, pero el guard no la generaliza a
  otros comandos: hacerlo sin medirlos sería afirmar sin comprobar.
- **Necesita un binario que tenga `cycle list`, y hoy el del `PATH` no lo tiene.**
  El guard **falla cerrado** si se le pasa uno sin el subcomando, con el motivo
  escrito. Es la condición de `INC-DEBT-064` en su forma más literal, y por eso
  este guard **no puede correr en la ventana declarada-pero-no-publicada sin que
  se le pase `SDDK_GUARD_BIN` o se construya el binario**: el guard que mide la
  reconciliación no se puede instalar con el artefacto viejo. Verificado, no
  supuesto — con `SDDK_GUARD_BIN` apuntando al binario del `PATH` el guard sale
  con **1** y nombra el motivo.
