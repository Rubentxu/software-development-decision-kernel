# CLOSURE — VA15 `cl-canario-de-aislamiento-entre-proyectos`

## Veredicto

**El cross-project leak NO se reproduce.** Base: 0 violaciones sobre 4 pares de
fixtures. Y las cuatro mutaciones que podían matar el canario lo mataron.

```
test_cross_project_isolation_canary.sh    PASS=4 FAIL=0 SKIP=1
base: NO se reproduce el leak (0 violaciones)
M1 project_id sin remote   -> caido (V1 + V2)
M2 bindings sin project_id -> caido (V2)
M3 fallback seed sin ruta  -> caido (V1)
M4 resolutor sin remote    -> caido (V2) — yo habia pre-declarado que no caeria
```

El `SKIP=1` es M4, con el motivo escrito en su propia linea del script. Los cuatro
`PASS` son la base mas M1, M2 y M3.

## Reconciliacion con el PRE-FLIGHT

El PRE-FLIGHT declara **cinco** mutaciones y el canario ejecuta **cuatro**. La
diferencia esta medida, y una de las dos es un error mio de diseno:

- **M5 no se implemento, y no por falta de tiempo.** El brief pide mutar «la
  comparacion `target.project_id == project_id observado`». **Esa comparacion no
  existe en produccion**: es el hallazgo del propio canario, en la seccion de
  abajo. No se puede mutar lo que no esta escrito. Declararla ROJO fue un error mio —
  disene el falsificador contra el codigo que el brief asumia, no contra el codigo
  que hay.
- **M3 cambio de `SKIP esperado` a `ROJO`, y cayo.** El analisis previo decia que
  el fallback no interviene porque todas las parejas tienen remote. Es cierto para
  las dos primeras y falso para `sin-remote-uno/proyecto` y
  `sin-remote-dos/proyecto`, la pareja sin remote que el canario anadio precisamente
  para poder ejercer ese camino. Sin esa pareja M3 no habria podido dar rojo, y
  habria sido otro test declarado rojo que no puede estarlo.

## Lo que el canario mira, y por que puede fallar

Cuatro fixtures. Las dos primeras llevan los nombres parecidos del incidente; las
dos segundas tienen **basename identico** (`proyecto`) en directorios distintos,
que es lo que separa «identidad por remote» de «identidad por nombre»; y una
cuarta pareja **sin remote**, para poder ejercer el camino de fallback.

Tres violaciones observadas: colisión de `project_id` (V1), un binding que
declara el `project_id` del otro proyecto o que no aparece en su directorio (V2),
y un directorio de bindings que no depende del `project_id` (V3).

El detalle que hace el canario útil: **imprime donde caen los bindings de
verdad**, no donde uno supone. Ese `find` es lo que delató a M2.

## M2 es la que importa: el namespacing ES la defensa

M2 quita el `project_id` del directorio de bindings y el canario cae. Es decir:
**la razon por la que un binding de Fabric no es alcanzable desde PipelineK no
es que nadie lo compruebe, es que vive en otro directorio.** La comprobacion
`binding.project_id == project_id observado` que pide el brief **no existe en
ningun sitio**, y no hace falta: el sistema de ficheros ya la impone.

Eso es una defensa mas fuerte que un `if`, porque no se puede saltarse sin
cambiar el codigo. Y es tambien mas fragil en un sentido: si el directorio deja
de depender del `project_id`, no queda nada.

## Mi prediccion sobre M4 era incorrecta, y el motivo importa

Pre-declaree que M4 «no puede dar rojo»: sin remote cae al seed de **ruta**, y
dos rutas distintas dan seeds distintos. **La primera mitad es correcta** —V1, la
colision, no salio— **y la segunda es falsa.**

M4 si caido, por V2. El motivo es que **mi mutacion no es neutra**: deja de leer
el remote solo en `resolve_via_canonical` (`context_cmd.rs:1012`), mientras que
`sddk project resolve` lo lee por su cuenta (`lib.rs:1990`). Los dos llamadores
comparten `resolve_identity_honoring_pin`, luego comparten la funcion, pero
M4 les pasa **entradas distintas**: uno con remote y otro sin el. El canario
detecto que el bootstrap escribia sus bindings bajo un project_id que
`project resolve` no reconoce para el mismo checkout.

Eso **no** es un defecto del producto: es exactamente la desincronizacion entre
resolutores que corrigieron INC-DEBT-049 e INC-DEBT-059, y la mutacion la
provoca a proposito. Pero la distincion importa y no la voy a maquillar: yo
declare que ese test no caeria, y cayo. **Un test pre-declarado que no puede
estar rojo no prueba nada, y uno que creia que no caeria y cae tampoco: los dos
son el mismo error, escrito en distinto sitio.**

La leccion sobre M4 es que **mutar un solo llamador de un resolutor compartido
no prueba la hipotesis que uno cree que prueba**. Para esa hipotesis haria falta
mutar los dos lados a la vez.

## El canario tenia un falso negativo, y era del mismo tipo que investiga

La primera corrida dio `PASS=3 FAIL=1`, con M2 en verde **puesto el defecto**.
Causa: el canario reutilizaba el mismo `SDDK_DATA_DIR` en todas las corridas, de
modo que los bindings que escribio la corrida BASE seguian ahi. Con M2 —que
justamente hace que todos los proyectos compartan directorio— la comprobacion
«el binding de A existe» segia viendo el fichero de la corrida base, con el
project_id correcto.

El mutado funcionaba. **El instrumento no lo veia.**

Cada corrida usa ahora su propio directorio. Es la misma clase que el `id` del
`PRE-FLIGHT` de VA14, con el signo cambiado: ahi no mire donde decia, aqui no
miraba lo que decia. **Un control que pasa porque ve un fichero que dejo la
corrida anterior no esta mirando lo que dice mirar.**

## Que queda abierto, y NO se ha tocado

- **`AgenticBinding::reattach` sigue siendo `persisted.clone()`** y el error
  cerrado sigue sin tener una variante de mismatch. No es alcanzable hoy porque
  el namespacing lo hace moot, pero es una fragilidad latente: si mañana alguien
  lo llama con un binding de otro proyecto, no hay red.
- **El binding no lleva `workspace_id`**, luego mismo proyecto con worktree
  distinto reusa el binding en silencio. Es X6 y no se ha medido: el canario de
  este bloque es cross-project, no same-project-different-worktree.
- **No hay canario para el alias** (X7) ni para la higiene ciclo/roadmap (X11).

## Por que no hay causa raiz declarada

Porque no hay incidente que explicar: el canario no lo reproduce y los artefactos
del incidente real no estan en esta maquina. Declarar una causa aqui seria
exactamente el error que este bloque existe para evitar. Lo que hay es un
veredicto con falsificadores, que es lo que se puede afirmar.

**H1, H2, H3 y H5 quedan sin ruta de produccion medida.** H1 tiene falsificador
directo y funciona (M1). H2 y H3 dependen del namespacing y su falsificador es
M2. H5 no encontro ruta: `resolve_cycle_context` delega la identidad en
`RuntimeContext::open`, luego la inferencia de ciclo tambien es project-scoped.

## Verificacion

El canario construye `sddk-cli` desde cero en un target aislado y deja la fuente
byte-identica tras cada mutacion (sha verificado, `exit 1` si no). Datos
aislados con `SDDK_DATA_DIR`; esta maquina tiene 265 proyectos reales y el canario
no toca ninguno. `bash -n` y `shellcheck` limpios. Scanners de contaminacion
limpios.