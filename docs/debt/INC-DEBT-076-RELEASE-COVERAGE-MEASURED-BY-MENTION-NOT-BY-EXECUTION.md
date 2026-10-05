---
id: INC-DEBT-076
title: "cinco guards publicados en el changelog como gates de la release no los ejecutaba NADIE, y otros tres solo los ejecutaba un CI que no bloquea: la cobertura se mediia por MENCION, y hay sitios donde mencionar no es ejecutar"
status: resolved
severity: high
priority: P1
fingerprint: "release_coverage_measured_by_mention_not_by_execution"
fingerprint_aliases: []
cluster_id: CL-AUTHORITY-SPLIT
created: 2026-10-05
created_by: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
owner: miniMax Code (mvs_fac515d56e784fc081d64fefb38323aa)
detected_at: 2026-10-05
detected_in_session: session-84
resolved_at: 2026-10-05
component: release pipeline / cobertura de gates
surface: scripts/release.sh, tests/test_gate_coverage.py, .github/workflows/ci.yml, CHANGELOG.md
related: [INC-DEBT-064, INC-DEBT-055, INC-DEBT-047]
references:
  - scripts/release.sh
  - tests/test_gate_coverage.py
  - tests/test_gate_coverage_ci_mutation.py
  - .github/workflows/ci.yml
  - CHANGELOG.md
  - AGENTS.md
---

# INC-DEBT-076 — RESUELTO — la cobertura del 1b se mide por sintaxis que ejecuta, no por el nombre del test

## Que se midio

Ocho de los 82 tests de `tests/test_*` **no se ejecutaban nunca antes de publicar**.
No por la misma razon, y por eso el caso tiene dos mitades.

### Mitad A — tres con un runner que no bloquea

`test_gate_coverage.py` incloia `.github/workflows/*.yml` en sus `RUNNERS`, con
un motivo escrito y correcto para su contrato. Lo que ese contrato no cubria es
una norma del propio repo, **AGENTS.md seccion 2.5**:

> GitHub Actions cloud **NO bloquea**: sin required status checks, runs =
> evidencia asincrona.
> **Prohibido** esperar runs de la nube.

Un runner de CI, aqui, no bloquea nada en el camino de publicacion. Tres tests
tenian su unico runner ahi, los tres hermeticos por medicion (0 red, 0
contenedores, 0 artefactos externos):

| test | runtime | resultado | runner real |
|---|---|---|---|
| `test_workflow_contract.py` | 0,12 s | 508 PASS | solo `ci.yml` |
| `test_golden_dataset_contract.py` | 1,56 s | 17 OK | solo `ci.yml` |
| `test_release_state_pointer_mutation.sh` | 4,80 s | PASS=6 | solo `ci.yml` |

6,5 s en total. **Y los tres PASABAN**: el defecto no era de correccion, era de
cobertura.

No es cosmetico. El tercero es el **autofalsador** de
`test_release_state_pointer.sh`, el guard que `af98f9af` cita como causa de 41
commits de deriva en `STATE.yaml`. La release verificaba el puntero y **no
verificaba que su verificacion tuviera dientes**.

### Mitad B — cinco que no los ejecutaba NADIE

Estos son peores, y no aparecieron hasta la segunda medicion.

`release.sh` mantiene, en el paso 1b, un **scope de shellcheck**: una lista de
ficheros que shellcheck revisa. Cinco guards estaban en esa lista y en ningun
otro sitio:

| test | release.sh | ci.yml | otro script | log del 1b |
|---|---|---|---|---|
| `test_release_final_state_figures.sh` | 2 (shellcheck + comentario) | 0 | 0 | **0** |
| `test_release_final_state_figures_mutation.sh` | 1 (shellcheck) | 0 | 0 | **0** |
| `test_lint_gate_scope_severity_mutation.sh` | 1 (shellcheck) | 0 | 0 | **0** |
| `test_release_bump_pointer_sync.sh` | 1 (shellcheck) | 0 | 0 | **0** |
| `test_reconcile_pointer_yaml_safety.sh` | 1 (shellcheck) | 0 | 0 | **0** |

La columna de la derecha es contra **el log de un release que paso el 1b
entero**: cero apariciones de los cinco. Y los cinco **estan publicados en
`CHANGELOG.md` como gates de la release, con su PASS**:

> Guard `test_release_final_state_figures.sh` `PASS=12 FAIL=0` ·
> `test_lint_gate_scope_severity_mutation.sh` `PASS=7 FAIL=0` ·
> `test_reconcile_pointer_yaml_safety.sh` `PASS=7 FAIL=0` ·
> `test_release_bump_pointer_sync.sh` `PASS=7 FAIL=0`

Es decir: **el changelog de un artefacto publicado declara como evidencia de
gates cinco pruebas que la release no ejecuta**. No es que faltara cobertura;
es que la declaracion de cobertura era falsa. Medido: los cinco pasan, 17 s en
total (0,74 + 8,09 + 0,96 + 3,14 + 4,37).

## La causa raiz, que es una sola: MENCIONAR no es EJECUTAR

`test_gate_coverage.py` respondia «¿este test tiene runner?» buscando su
**nombre** en el texto de `release.sh`. Y hay al menos tres sitios donde el
nombre aparece sin que ocurra nada:

1. el **scope de shellcheck** — una lista de lint, no de ejecucion;
2. un **comentario** (el caso de `test_release_final_state_figures.sh:2258`,
   que lo nombra al explicar que llama a una libreria);
3. un **`ok`** o un `echo` decorativo.

`code_only` ya descartaba comentarios, luego el defecto vivo era el scope de
shellcheck. Y ese es **el mismo punto ciego que la Regla 3 ya tuvo en este
mismo fichero**, con el remedy escrito en su propio comentario desde
`af98f9af`:

> El alcance es el bucle gateado, NO todo el fichero [...] Un guard que acusa
> de rojo a algo que corre bien entrena a ignorar sus rojos.

La leccion se aplico a la Regla 3 y **no** a la regla siguiente, en el mismo
fichero. Eso es lo que hace que una leccion escrita no se aprenda.

## Es la CUARTA vez que aparece la misma clase

1. `session-65j` — 13 de 36 tests sin runner. Se creo este gate. El comentario
   que quedo en `release.sh:547` dice *«`ci.yml:58,64` enumera A MANO dos de
   ellos»*, que son exactamente dos de los tres de la Mitad A. Quedaron
   contados como cubiertos.
2. `session-75` / `af98f9af` — cinco tests enumerados sin bit de ejecucion, que
   el paso se saltaba en silencio. Nacio la Regla 3 y la frase «la cobertura
   que cuenta el nombre no es la cobertura que ejecuta el bit».
3. `session-83` / `9abec31b` — un test sin runner mato la publicacion de 2.11.1
   en el 1b (quinto intento). Se cableo el que faltaba.
4. Esta — ocho mas, ninguno de los cuales habria hecho fallar nada, porque el
   gate que los vigilaba los contaba como cubiertos.

## Que se cambio

- **`tests/test_gate_coverage.py`**: nace `tests_ejecutados()`, que separa
  EJECUTAR de NOMBRAR por **sintaxis** — un test cuenta si esta en un bucle
  `for t in` / `for p in` o en una invocacion (`test_gate "X"`,
  `bash tests/X`, `python3 tests/X`). Nace la **Regla 4**: un test cuyo unico
  runner es `.github/workflows/` es un FAIL, y el mensaje cita AGENTS.md 2.5.
  Los globs (`tests/test_*.sh` en el scope de shellcheck de `ci.yml`) **no**
  inflan el conjunto, porque un glob no es un nombre. `code_only` desaparece:
  su filtrado esta dentro del extractor.
- **`scripts/release.sh`**: los ocho entran en el 1b — los tres `.py` en el
  bucle de python y los cinco shell en el bucle gateado, todos con bit de
  ejecucion (la Regla 3 lo exige y lo comprueba). `test_release_state_pointer_mutation.sh`
  entra ademas en el scope de shellcheck, junto a sus hermanos.

## El segundo defecto: lo destapo el falsador, no la inspeccion

La **Regla 2** (una excepcion para un test que ya tiene runner es un FAIL)
miraba el conjunto que incluía CI. Con AGENTS.md 2.5, declarar el motivo de un
test que CI cubre es **lo correcto** — se excepciona del 1b porque necesita
red, y que CI lo corra es lo deseado, no la senal de que la razon caducó.

Con la regla como estaba, la Regla 4 **no tenia salida legitima**: solo se
podía cumplir cableando en el 1b, y para un test hermetico que no se quisiera
ahi por coste no habia donde escribir el motivo. Una regla sin salida obliga a
la unica accion que no siempre es correcta. Lo vio el caso M4 del falsador, no
la lectura del codigo. La Regla 2 pasa a mirar lo que el release **ejecuta**.

## Falsacion

`tests/test_gate_coverage_ci_mutation.py`, `PASS=23 FAIL=0 SKIP=0`. Siete
mutaciones, una por propiedad:

- **M1/M2** — quitar `test_workflow_contract.py` / `test_golden_dataset_contract.py`
  del bucle de python: el gate CAE y nombra el runner de CI.
- **M3** — quitar el autofalsador del bucle gateado: el gate CAE.
- **M4** — quitar del 1b **y** escribir el motivo: el gate PASA. La salida
  legitima son las dos cosas, y por eso M6 es su otra cara.
- **M5** — **el extractor**: `if esta_en_bucle:` -> `if False and esta_en_bucle:`.
  El gate CAE y reporta SIN runner donde antes habia cero. Mutar el extractor y
  no la regla que lo consume es lo que corresponde: si contara de mas, la
  Regla 4 daria verde con los cinco huerfanos en su sitio.
- **M6** — escribir el motivo **sin** quitar el cableado: el gate CAE por la
  Regla 2, nombrando el cajon de sastre.
- **M7** — quitar los cinco guards de la Mitad B: el gate CAE y nombra a
  **cada uno** como «NADIE lo ejecuta». Es la falsacion directa del hallazgo.
- **Cierre** — sha256 de `release.sh`, del gate y del propio falsador tomados
  antes de empezar y comparados al final. Todo ocurre en un **mini-repo**
  construido con `copy2`, nunca sobre el arbol real.

## Tres instrumentos que se encontraron fallando a si mismos

1. **La v1 del falsador mutaba `release.sh` y el gate EN SITIO.** Al anadir el
   fichero a `tests/`, el gate lo ve como un test nuevo sin runner y el control
   falla: el falsador se delata como cobertura ausente. Y mutar `release.sh`
   desde un falsador que corre DENTRO del 1b es la bomba de reloj que
   `5edcef00` acaba de arreglar en el otro falsador. Rehecho con mini-repo.
2. **Su cierre comparaba `... == rel_bak or True`.** Un `or True` pasa aunque
   el falsador hubiera dejado el arbol mutado, que es justo lo que el fichero
   existe para probar que no hace. Ahora son sha256 de los tres sujetos.
3. **El extractor de cobertura tuvo el mismo bug TRES veces**, y las tres las
   encontro el dato y no la lectura: (1) `continue` en la linea que abre el
   bucle perdia el PRIMER elemento; (2) cerrar el bucle antes de recolectar
   perdia el ULTIMO, que es el unico sin continuacion; (3) olvidar poner
   `en_bucle = True` dejaba los bucles casi vacios. Se valida **contra el log
   de un release real**: el extractor da 56 tests ejecutados y el 1b de
   `/tmp/rel-2.11.1d.log` ejecuto 56, con 0 falsos negativos. Un instrumento
   que se equivoca en el caso mas obvio no puede usarse para acusar a nadie.

## Lo que NO se arreglo aqui

Que la superficie de gates del 1b siga siendo una **lista escrita a mano** es
lo que hace que cada olvido cueste una release: `9abec31b` a las 11:40 de
2026-10-05, quinto intento de 2.11.1, por un solo test. La Regla 4 y el
extractor convierten un olvido en un **rojo de 23 s** en vez de un incidente,
pero no lo eliminan. Enumerar por convencion sigue siendo la decision que
falta, y es decision de quien tiene el criterio, no de este ciclo.

Tampoco se corrige la linea de `CHANGELOG.md` que declara los cinco gates:
**esta publicada** y pertenece a un tag. Se corrige el mecanismo que la
produce, y el hecho queda escrito aqui para que no se lea como una
certificacion vigente.
