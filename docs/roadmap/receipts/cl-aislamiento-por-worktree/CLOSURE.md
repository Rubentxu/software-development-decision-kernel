# CLOSURE — VA16 `cl-aislamiento-por-worktree`

## Veredicto

**El aislamiento por worktree NO existe, y es alcanzable.** Base: 4 violaciones.

```
test_worktree_isolation_canary.sh   PASS=4 FAIL=0 SKIP=0

W0  precondicion: mismo project_id, distinto workspace_id   ok
W1  el binding es inalcanzable desde el otro worktree        VIOLACION
W2  el binding declara de que worktree vino                  VIOLACION
W3  B no absorbe en silencio la sesion de A                  VIOLACION
W3b B no hereda el trabajo acumulado por A                    VIOLACION
W4  control positivo: adoption SI separado por workspace     ok

F1  project_data cuelga del workspace_id     4 -> 2
F2  bootstrap deja de cargar el previo       4 -> 2
F3  comentario inocuo (control negativo)     4 -> 4
```

## Por que este caso es peor que el que el brief prioriza

VA15 concluyo que el cross-project leak **no es alcanzable**: el namespacing en
disco por `project_id` lo hace moot. Aqui el `project_id` **coincide
legitimamente**, porque es el mismo proyecto.

**Ninguna ley por `project_id` puede detectar este leak. Las cinco del brief
(C1-C5) comparan todas `project_id`, y aqui el `project_id` es el
correcto.** Un `ExecutionContextAdmission` construido como el brief lo describe
pasaria en verde mientras sirve contexto de otro worktree.

Lo unico que distingue los dos worktrees es `workspace_id`, y `workspace_id` no
entra en ningun almacen de contexto. Se calcula (`context_cmd.rs:208`) y se
expone en el resultado (`:370`), y no discrimina nada: de los cinco almacenes
que cuelgan de `project_data`, cuatro son de sesion y **ninguno** es por
workspace. El unico que si lo es, `workspaces/<w-id>/adoption.json`, es el que
nadie mira.

## W3b es la que duele, y por eso existe

W1, W2 y W3 son sobre **metadatos**: un fichero compartido, un campo ausente,
un `written: false`. Importan, pero podrian argumentarse como una decision de diseño.

W3b no admite esa lectura. Se marca el binding de A como si hubiera acumulado
refs, se deja que B pase por el binding, y **el marcador de A sigue ahi**:

```
W3b VIOLACION: tras pasar B, el binding sigue llevando el trabajo acumulado por A
```

No es que B este mal informado. Es que **B eredita el trabajo de A y lo sigue
presentando como suyo**, sin marca, sin aviso y sin entrada en su
`context_basis`. Eso es la clase exacta del incidente reportado, en la dimension
que el brief no habia mirado.

## W4 es el control que mas vale, y va antes que el arreglo

`adoption` **si** esta separado por workspace: dos ficheros, `w-5af3.../` y
`w-7797.../`. El mecanismo por workspace existe, funciona y se usa. Lo que falta
es aplicarlo a los cuatro almacenes de sesion.

Sin W4, el veredicto seria "el sistema no puede aislar por worktree", que es una
excusa. Con W4, el veredicto es **"nadie lo pidio"**, que es una tarea. Un
control positivo vale mas que una violacion cuando lo que se decide es si algo
es un limite o una omision.

## F1 y F2 son las dos mitades, y ninguna basta

Cada una quita dos de las cuatro violaciones. **Ninguna quita W2**, porque W2 no
es un comportamiento: es un campo que no existe. Ponerlo exige cambiar el modelo
del binding, no una linea.

Y F1, que yo llame «el fix real en una linea», **tiene un efecto colateral que
no es neutro**:

```
p-f1bf.../w-5af3.../knowledge-profile.json
p-f1bf.../w-7797.../knowledge-profile.json
```

Colgar `project_data` del `workspace_id` parte tambien el knowledge profile,
`artifacts`, `cycle_artifacts` y `generated`, que **si** son cosas del proyecto.
`project_data` mezcla hoy dos clases que no tienen por que compartir directorio,
y colgarlo entero confunde las dos.

**El fix que el brief da por hecho —«una precondicion fail-closed antes de
entregar contexto»— no cabe en una linea, porque la pregunta real no es «¿que
directorio uso?» sino «¿que almacenes son de proyecto y cuales son de
worktree?».** Y eso no lo responde un canario: es una decision de diseno, con la
decada de 265 proyectos reales encima.

No la tomo aqui. Este bloque mide.

## Los dos defectos del canario, y el segundo casi se cuela

**El primero se corrigio antes de ejecutar.** La primera version construia la
ruta del binding a mano
(`$DATA/sddk/projects/<pid>/context/bindings/S-wt.json`) en vez de buscarla. Con
F1 puesto el binding se mueve de directorio —que es justo lo que F1 hace— y el
canario devolvia «no se encontro binding» en vez de medir. Es la misma clase que
M2 delato en VA15: **buscar donde uno supone en vez de donde cae**, y un
instrumento que se apaga justo cuando la mutacion funciona no mide la propiedad,
mide su propia suposicion.

**El segundo se corrigio despues de la primera corrida, y la primera corrida no
lo delato — lo enmascaro.** Devolvio `PASS=2 FAIL=0 SKIP=2` con F1 y F2 marcados
`SKIP`, cuando **los dos habian funcionado**:

```
[SKIP] F1 project_data cuelga del workspace_id — con la mutacion el fixture ya no mide el caso
[SKIP] F2 bootstrap deja de cargar el binding previo — con la mutacion el fixture ya no mide el caso
```

La causa era mia y era de las que el informe no enseña: `measure` devolvia el
numero de violaciones por codigo de salida, y **use `return 2` como centinela de
«el fixture no mide el caso»**. Pero `2` es tambien un veredicto valido: F1 y F2
dejan exactamente 2 violaciones, y el canario leia su propio exito como «no
midio».

**Dos estados distintos por el mismo canal, y el segundo gana.** Ahora van por
`MEDIBLE` aparte y `return 0`.

Que saliera verde no lo delato nadie: `FAIL=0` es indistinguible de «todo bien»
cuando lo que se pregunta es si las comprobaciones vigilan algo. Lo que lo
delato fue mirar el detalle de las dos SKIP y ver que las cuatro violaciones de
la base eran plausibles con la mutacion puesta.

## Que queda abierto, y NO se ha tocado

- **El binding no lleva `workspace_id`.** Es W2, y es la parte que exige cambio
  de modelo: X3.
- **`AgenticBinding::reattach` sigue siendo `persisted.clone()`** y
  `AgenticBindingError` sigue sin variante de mismatch. Aqui **si** es alcanzable
  — a diferencia de VA15 — porque el binding del otro worktree esta en el mismo
  directorio y se lee sin filtro.
- **La politica no se decide.** Si un reattach cross-worktree se permite, exige
  rebind o exige confirmacion: es la pregunta que X6 hace y que este bloque
  deja abierta porque contestarla es diseno, no medicion.
- **No se ha medido la capsule.** Con ciclo real, `capsule_root` sigue siendo
  por proyecto, luego un `context: recovered` podria servir una capsule
  compilada en otro worktree. Es el mismo mecanismo por otra via y **no lo he
  medido**: no tengo el ciclo montado y no lo voy a afirmar.
- **`ledger.sqlite` es por proyecto** (`paths.rs:161`). Que dos worktrees
  compartan ledger es discutible y no es de este bloque.

## Verificacion

El target de cargo vive FUERA del temporal, a proposito: con el dentro, cada
mutacion recompilaba el workspace entero desde cero (~9 min) y este bloque
seria inviable. Con el fuera, las recompilaciones son incrementales. Cada
mutacion se restaura byte-identica por sha y el script hace `exit 1` si no; el
arbol quedo limpio con `HEAD` sin mover. Datos aislados con `SDDK_DATA_DIR`;
esta maquina tiene 265 proyectos reales y el canario no toca ninguno.
`bash -n` y `shellcheck` limpios. Resultado reproducible: `PASS=4 FAIL=0 SKIP=0`
en tres ejecuciones seguidas.