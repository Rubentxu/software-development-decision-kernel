---
id: INC-DEBT-073
title: "El gate de shellcheck no filtra por severidad y ademas solo mira los ficheros TOCADOS: un aviso preexistente en una linea que nadie cambio puede parar una release, y un fichero con deuda de lint solo se limpia si alguien lo toca por otra razon"
status: resolved
severity: medium
priority: P2
fingerprint: "shellcheck_gate_unsevered_and_range_scoped"
fingerprint_aliases: []
cluster_id: CL-VERIFY
created: 2026-10-05
created_by: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
owner: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
detected_at: 2026-10-05
detected_in_session: session-81
resolved_at: 2026-10-05
resolved_in_session: session-83
component: verification / release pipeline
surface: tests/test_build_identity_policy.sh (el bloque "shellcheck en el shell tocado")
related: [INC-DEBT-041, INC-DEBT-071]
references:
  - tests/test_build_identity_policy.sh
  - scripts/lib/lint_gate.sh
  - tests/test_lint_gate_scope_severity_mutation.sh
  - scripts/release.sh
  - tests/lib_public_release_gate.sh
---

## Que es

`tests/test_build_identity_policy.sh` construye la lista de ficheros a
lintear asi:

```sh
SH="$(git diff --name-only "$BASE"..HEAD | grep -E '\.sh$' | tr '\n' ' ')"
if shellcheck $SH >/dev/null 2>&1; then
```

Dos propiedades, y las dos estan medidas:

1. **Sin filtro de severidad.** Un `info` cuenta igual que un `error`.
2. **Solo los ficheros del rango.** Un fichero con deuda de lint acumulada
   no se ve hasta que alguien lo toca por un motivo **distinto**.

Juntas producen un efecto que ninguna de las dos describe: **el conjunto de
ficheros que el gate vigila depende de que trabajo haya hecho la gente, no de
que trabajo haya que hacer.**

## MEDIDO

Session-81, publicando 2.9.1. Dos lineas de `tests/clean_machine_uat.sh`
arreglando un `| grep -q` con `pipefail` (INC-DEBT-071) metieron el fichero
bajo el lint y la release murio en el 1b:

```
[FAIL] shellcheck reporta avisos:
In tests/clean_machine_uat.sh line 522:  SC2140 (warning)
... 12 avisos en total: SC2140 x8, SC2004 x2, SC2034 x2
```

**Ninguno era mio.** Los doce estaban en lineas que el cambio no tocaba, y
el fichero **nunca habia estado en el rango de ningun cambio**, luego el
gate no lo habia mirado jamas. Y lo que habia debajo era real: `$*` sin
comillas partiendo argumentos en el shell del host, y la funcion
`sddk_with_path` definida **cuatro veces**. Todo eso llevaba ahi desde antes
y era invisible por construccion, no por un descuido — por el alcance del gate.

Arreglado en `cd1dc41a` para ese fichero, lo que **no** arregla el defecto:
los proximos ficheros que entren en un rango por otra razon traeran su
propia deuda.

## Por que P2 y no P1

No hay perdida de datos ni corrupcion: el gate es **estricto de mas**, luego
lo que produce es un bloqueo espurio, no un falso verde. Un falso verde es
peor que un falso rojo, asi que la severidad no es la maxima.

Pero el falso rojo **cuesta una release entera**, y el modo de fallo es el
mas caro de este repo: no dice "el codigo esta mal", dice "el codigo esta mal
en una linea que no has escrito", y quien lo recibe no tiene forma de
saber si el problema es suyo o heredado. MEDIDO: el release 2.9.1 gasto el
1b entero en un fichero que no era el objeto del cambio.

## Por que es la TERCERA vez

Session-80 ya lo encontro y lo registro: "Gate latente de shellcheck
introducido por este bloque. `test_build_identity_policy.sh` corre shellcheck
SIN filtro de severidad, luego un `info` es igual que un `error`". Lo que
hizo fue **arreglar los ficheros de aquel bloque**. El diseno del gate no se
toco, y por eso la misma clase vuelve ahora con la misma forma.

La leccion es la de siempre y tiene nombre propio en este repo: **arreglar
el que el gate te enseno no es arreglar el gate.**

## Que habria que hacer

1. **Decidir la severidad minima que el gate cobra**, y escribirla. La
   opcion que este repo ya usa en otros sitios es `-S error` para lo que
   bloquea publicacion y el resto como aviso; hoy el gate cobra TODO y no
   declara que lo haga.
2. **Un censo de la deuda de lint del repo, corrido una vez**, para que
   "cero avisos" sea un objetivo alcanzable y no una sorpresa por fichero.
   Sin ese censo, el gate solo puede dar la noticia cuando el fichero entra
   en un rango, y la noticia llega como un fallo de release.
3. Si se decide que el gate vigile todo el arbol y no solo el rango, que lo
   diga explicitamente y mida el coste, porque ese bucle es O(repo) por
   release y con ~82 ficheros `.sh` hay que saber si aguanta.

## Lo que NO se afirma

- **No es un defecto de `shellcheck`.** Los avisos son correctos; el
  problema es a quien se le cobran y cuando.
- **No se ha cambiado el diseno del gate.** Este documento lo registra; el
  arreglo de `clean_machine_uat.sh` es de ese fichero, no del gate.
- **No se afirma que la severidad de P2 sea la correcta.** Es la que se
  propone segun "no hay perdida de datos y el fallo es un bloqueo
  espurio", que es el mismo razonamiento que bajo INC-DEBT-060 y 062 a
  `high` cuando no habia perdida de datos pero habia contrato roto en
  silencio. Aqui el contrato no se rompe en silencio: se rompe ruidoso, y
  por eso baja.

---

## CIERRE — session-83, 2026-10-05

Las tres salidas que la deuda fijaba, en el orden en que las fijaba.

### Salida 1 — la severidad minima que el gate cobra, ESCRITA

`scripts/lib/lint_gate.sh` declara `LINT_GATE_SEVERITY="warning"`.

**MEDIDO el desacuerdo que la obligaba:** los dos gates del mismo pipeline
no cobran lo mismo.

| gate | severidad que cobra |
|---|---|
| `scripts/release.sh` 1b | `--severity=warning` |
| `test_build_identity_policy.sh` | sin filtro: cobra tambien `info` y `style` |

Dos gates del mismo repositorio que responden distinto a la misma pregunta
sobre los mismos ficheros. Se elige `warning` y **se escribe**, no se deriva:
es la que ya cobraba el 1b, y bajar la severidad general del repositorio es
una decision que este arreglo no toma.

### Salida 2 — el censo de deuda de lint, corrido una vez

Exit 2 de la deuda: que "cero avisos" sea un objetivo alcanzable y no una
sorpresa por fichero. MEDIDO sobre los **100 `.sh` versionados** de este
repo:

| severidad | avisos |
|---|---|
| `error` | **0** |
| `warning` | **1** |
| `info` (warning + info) | 13 |
| `style` (todo) | 13 |

Repartido, que es el dato que la deuda no tenia: **0 errores, 1 warning, 12
info, 0 style-only.** El unico warning era SC2034 en
`tests/lib_public_release_gate.sh:84` — un `for i` cuyo contador no se leia.

Los 12 de info **NO se corrigen aqui**, y se dice por que: bajarlos de
severidad es una DECISION que se escribe, no un efecto colateral de este
cambio. Quedan como censo medido.

### Salida 3 — el alcance, y su coste medido

Exit 3: si el gate vigila todo el arbol, que lo diga y mida el coste.

**MEDIDO con el `BASE` que el gate usaba (`dc69e6f2`):**

```
.sh vigilados por el gate :  51 de 100
.sh NO vigilados NUNCA     :  49
warning dentro de los 49  :   1   <- el unico del repo
warning dentro de los 51  :   0
```

**El gate estaba VERDE con deuda real dentro**, y no por un descuido: por su
alcance. El unico aviso de warning del repositorio vivia en uno de los 49
ficheros que la logica del rango no puede ver. Eso era la segunda mitad del
defecto, MEDIDA y no temida: *el conjunto de ficheros que el gate vigila
depende de que trabajo haya hecho la gente y de que commit historico se
eligiera como base, no de que trabajo haya que hacer.*

Correccion de la causa de fondo: **`BASE` no era "el cambio"**, era un commit
historico fijado a mano. El alcance pasa a ser `git ls-files '*.sh'`.

**Coste medido del bucle O(repo):**

| alcance | tiempo |
|---|---|
| 1 fichero | 0,08 s |
| arbol entero, 100 ficheros, `--severity=warning` | 16–22 s |

Se acepta a proposito: el 1b ya corre ~100 tests de shell, luego 20 s no es
el cuello de botella, y a cambio el conjunto vigilado deja de depender de
quien haya tocado que.

### El unico warning, corregido

`tests/lib_public_release_gate.sh:84`, `for i` -> `for _`. El contador no se
leia en el cuerpo; solo se contaba el numero de intentos. Con eso, **el arbol
entero da 0 avisos a `warning`**, que es la precondicion que hace posible el
gate ancho: sin ella, ensanchar el alcance habria fallenado de inmediato por
deuda preexistente, que es exactamente la sorpresa que la deuda denuncia.

### Guard y autofalsacion

`tests/test_lint_gate_scope_severity_mutation.sh` — `PASS=7 FAIL=0`. El
autofalsador es lo que hace el cierre creible, y tiene tres controles que no
son decorativos:

- **M3 es el control anti-falso-verde.** M2 demuestra que un `info` NO se
  cobra; con eso solo, un "0 findings" salida de un linter que no se ejecuta
  seria indistinguible de un arbol limpio. M3 exige que el mismo finding, a
  severidad `info`, **si** aparezca. Sin M3, M2 pasa con el gate inerte.
- **M4 es el control del cuelgue.** Sin ficheros, `shellcheck` sin argumentos
  lee stdin y se queda colgado. Un colgado se parece a un gate lento. El gate
  no lo invoca cuando la lista esta vacia, y M4 lo verifica con `timeout`.
- **M5 es el que falsifica la DEUDA, no el guard.** Monta un arbol donde el
  aviso vive en un fichero que un gate por rango no puede ver, y exige que el
  gate lo vea igual. Sin M5, el fichero probaria que el codigo corre, no que
  arregla lo que la deuda describe.

El harness tambien distingue "no muto" de "pasó", que es la cuarta vez que
esta repo hace esa distincion.

### Un defecto del AUTOFALSADOR, encontrado por el

La primera version de M2 falló, y no era del gate: el fixture traia
`name=world` sin usar, que genera SC2034 — severidad **warning** — y no el
SC2016 de severidad `info` que el caso queria aislar. El falsador rechazo una
afirmacion mia porque el caso **no aislaba la variable que decia aislar**,
que es justo la forma de la que un falsador existe. Corregido y medido: el
fixture correcto da 0 avisos a `warning` y 1 a `info`.

### Lo que este cierre NO afirma

No se baja la severidad de nada que no se haya escrito. No se corrigen los 12
avisos de `info`: son censo, no deuda oculta. No se toca
`tests/test_vault_coherence_alignment.sh`, que el 1b excluye por sus avisos
preexistentes — queda fuera y sigue fuera. No se afirma que el repo entero
esté limpio: esta limpio **a la severidad que este gate cobra y declara**,
que es una afirmacion mas pequeña y por eso es cierta.
