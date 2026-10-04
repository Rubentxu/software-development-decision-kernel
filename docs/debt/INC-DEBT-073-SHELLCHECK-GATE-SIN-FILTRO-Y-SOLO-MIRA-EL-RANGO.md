---
id: INC-DEBT-073
title: "El gate de shellcheck no filtra por severidad y ademas solo mira los ficheros TOCADOS: un aviso preexistente en una linea que nadie cambio puede parar una release, y un fichero con deuda de lint solo se limpia si alguien lo toca por otra razon"
status: open
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
resolved_at:
component: verification / release pipeline
surface: tests/test_build_identity_policy.sh (el bloque "shellcheck en el shell tocado")
related: [INC-DEBT-041, INC-DEBT-071]
references:
  - tests/test_build_identity_policy.sh
  - scripts/release.sh
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
