# CURRENT — puntero de reanudación de SDDK
**Estado (session-69s bis 6, 2026-10-03): C3n.2 MEDIDO — tabla de exit gate `AIW-S0..S8` con veredicto y evidence SHA, y una afirmación mía del commit anterior queda desmentida por `grep`.** `HEAD` = `43c7aac2` + este commit. Workspace **2.5.3 declarada, no publicada** (último tag `v2.5.2`).

**Providers medidos antes de nada:** `cognicode-mcp` **PRESENTE**, `chronos-mcp` **AUSENTE**. Evidence SHA de toda la tabla: `43c7aac2`.

| Item | Medido | Veredicto |
|---|---|---|
| `AIW-S1` | `2 passed; 3 ignored` por defecto · con `COGNICODE_MCP_BIN` + `--ignored`: **`3 passed`, 141,60 s** | **VERIFIED, pero fuera del gate** |
| `AIW-S4` | `7 passed` + `12 passed` | **IMPLEMENTED** — residual intacto, no se promueve |
| `AIW-S5` | `2 passed; 2 ignored` · con `--ignored`: `BlockedExternalDependency` | **BLOCKED**, nunca PASS |
| `AIW-S7a` | `5 passed` | **IMPLEMENTED** (ver hallazgo) |
| `AIW-S8` X04 / X07 | `9 passed` / `6 passed` | **VERIFIED** |

**`AIW-S1` lleva dos filas porque las dos son ciertas, y esa es la parte que importa.** El producto **funciona** contra el provider real. Pero **por defecto el gate no lo ve**: sin `--ignored` la suite da `2 passed; 3 ignored`, y **uno de los dos que corren es `a03_spawn_failure_is_unavailable`** — un test que afirma que el provider **no** está disponible. El `VERIFIED` de la matriz **solo es reproducible si alguien exporta una variable y pasa un flag**; `cargo test --workspace` no lo cruza nunca. El precio está medido (**141,60 s** por publicación) para que la decisión sea revisable. **Es cobertura de sesión, no de pipeline.**

**`AIW-S4` NO se promueve, y es la tentación que este repo rechaza:** sus 19 tests pasan, pero el residual declarado —`executed_node_ids` es contabilidad de nodos, no evaluación de operadores— **no ha cambiado**, y **pasar tests no cierra un residual que nadie tocó**.

**⚠️ Corrección a una afirmación publicada en `43c7aac2` (un commit antes).** Escribí que `PROCESS/SQLITE_DURABLE` «no aparece en ningún otro documento y lo usan exactamente esas 4 filas». **Es falso:** aparece en **cinco sitios más**, uno en **`crates/sddk-storage/tests/x04_multi_process_concurrency.rs:17`**, más la columna `Frontier real` de **AIW-S8** y los receipts de X04 y X07. **Lo que cambia es el diagnóstico, no la frase:** no es un *typo* huérfano, es una clasificación **viva** que la propia regla 3 de la matriz **excluye** (ésta declara `SQLITE_MULTI_PROCESS`, no `SQLITE_DURABLE`). La reconciliación de las 4 filas **no cambia**; cambia su fuerza: ya no se sostiene en «no existe en ningún otro sitio» sino en lo medible — **X04 es concurrencia de ≥2 PIDs reales**, luego `SQLITE_MULTI_PROCESS` describe lo que el test hace.

**Hallazgo de fondo, mayor que la frase equivocada: este repo tiene DOS vocabularios cerrados de frontera que no coinciden**, en dos documentos que ambos se declaran autoridad. C3n.1 declara 8 niveles; `ACCEPTANCE-TRUTHFULNESS-MATRIX.md` regla 3 declara 9. Se cruzan en 6. Y la matriz **además viola su propia regla 3** (usa `IN_PROCESS/SQLITE` y `PROCESS / SQLITE_DURABLE`, que no declara). Reconciliarlos es trabajo de C3n.3 y **no se finge cerrar aquí**.

**⚠️ Y la regla 4 dice que «la aplicación mecánica de esta política es C3n.1», y las dos mitades de esa frase son falsas.** El guard de C3n.1 lee las 26 filas `AT-UAT` y **no lee esa matriz ni ningún nombre de test**. Y en `AIW-S7a`, **tres de cinco** tests se llaman `*_e2e` y **los tres, leídos, construyen su `ProducerEvent` a mano y lo despachan in-process** — no hay CogniCode, no hay Chronos, no hay proceso, y la fila declara `IN_PROCESS`. **`AIW-S7a` no se degrada:** sus 5 tests pasan y el falsificador de C3l.2 sigue valiendo. Lo que no vale es el **nombre**.

**Siguiente paso preciso:** el **guard de política de nombres** que la regla 4 dice existir y no existe — un `e2e`/`real`/`external`/`two-cli`/`second-binary` cuya fila declare un nivel no exigente es un nombre que no describe lo que el test cruza. Con el rename de los tres tests de S7a. **R1** sigue esperando la clave de firma.

---
**Estado (session-69s bis 5, 2026-10-03): C3n.1 CERRADO — el vocabulario de frontera es una sola autoridad y hay un gate que lo vigila. Ese gate se falsificó a sí mismo dos veces antes de valer.** `HEAD` = `dfd5c306` + este commit. Workspace **2.5.3 declarada, no publicada** (último tag `v2.5.2`).

**Lo que se entrega:** `tests/test_uat_boundary_receipt.sh` (`PASS=14 FAIL=0`), cableado en `release.sh` como paso **`3e/14`**, más la reescritura de `ROADMAP-ACCEPTANCE-TRUTHFULNESS.md` §C3n.1 con el vocabulario canónico y la reconciliación de las 4 filas discordantes de `UAT-MATRIX.md`.

**El problema que resolvió, medido sobre las 26 filas `AT-UAT`:** el conjunto de niveles «que exigen receipt de frontera» estaba escrito **en dos sitios que no coincidían**, y **el nombre del nivel decidía si una fila exigía o no**. 13 filas de las 26 quedaban exentas sin que nadie lo hubiera decidido. Ahora el conjunto está en **un solo sitio** y el guard **lo lee de ahí** — un guard con el conjunto copiado dentro habría sido *la tercera* declaración, y la tercera es la que diverge.

**Conjunto, por decisión explícita del operador y lectura literal de §C3n.1:** exigen `PROCESS` (4 filas), `MCP_EXTERNAL` (2) y `RELEASE_ARTIFACT` (1) = **7**. Quedan **13 declaradas exentas** — y el guard comprueba que ese número **no crece en silencio**. No es un cierre: es una cifra escrita, con un control que la vigila.

**El guard valida tipo, no solo presencia.** `process_count: por medir` —que es el texto literal de un receipt que no midió nada— **no pasa**: `process_count` tiene que ser entero ≥ 1, y `binary_sha256` 64 hexadecimales. La presencia la pone cualquiera; el tipo no.

**Lo que la falsificación encontró, y era el guard, no el producto — dos veces:**

1. **Un literal escrito a mano que no casaba y no fallaba.** El título de la sección estaba escrito **tres veces** en el guard, y una de las tres tenía **las tres últimas letras transpuestas** (`a-r-i-e` donde el documento dice `a-r-i-o`). `find` devolvió **-1 sin error**, y en Python `-1` como segundo argumento de `find` significa «buscar desde el final»: la mutación **no mutó**, escribió un fichero de **42 KB** —el documento entero *más* el bloque nuevo— donde debía escribir el documento con el bloque cambiado. **Un literal escrito a mano que no casa no degrada a un fallo: degrada a silencioso.** Es la misma clase que el guard existe para cazar, escrita en el guard. El título se declara **una vez** (`SEC_HEADING`), los tres lectores lo reciben y salen con código 2, y la mutación **se relee tras escribir**: una mutación que no se puede observar no es una mutación, es un cambio de fichero.
2. **La comprobación (e) era vacía, y lo dijo el mismo examen.** Afirmaba que el gate no tiene una tercera declaración del conjunto, pero **releía el spec con un extractor propio** — luego con el conjunto copiado dentro del gate habría dado `PASS` igual. Ahora prueba la **decisión**: muta la autoridad a `MIXED` (un nivel que hoy **no** exige), reextrae por la **misma función** que usa (a) y pasa un receipt de ese nivel sin campos de frontera por el **validador real**. Con el conjunto real se acepta (hueco declarado); con el mutado **pasa a RECHAZAR**. Ese cambio de veredicto es la prueba.

**Falsado en las dos direcciones:** inyectando el conjunto dentro de `leer_conjunto` cae (`PASS=13 FAIL=1`, y el único fallo es (e)); y con una mutación imposible declara `(e) no se ha ejecutado` en vez de emitir un veredicto falso. Un guard que sólo sabe decir `PASS`/`FAIL` no distingue «no vigilar» de «mirar y no ver nada».

**Siguiente paso: `C3n.2` (recertificación AIW)**, que es lo que no depende de ninguna decisión del operador. `R1` sigue esperando la clave de firma.

**Decisiones del operador que siguen abiertas:** (a) dónde vive la clave de firma de la release (KMS frente a fichero local) — bloquea `R1`; (b) qué superficie de producto produce `HostEvent` — bloquea `C3m.1`; (c) las dos salidas de `INC-DEBT-050` y `INC-DEBT-061`, que son un solo hecho; (d) bajar `INC-DEBT-060` a `medium`, reservado al operador desde `69m`; (e) archivar `a4-1-generic-verify` (publicado como `v1.169.46`, sin transicionar).

---
**Estado (session-69s, 2026-10-03): R2 REVALIDADA, no cerrada. La propiedad de `INC-DEBT-060` es falsa hoy, y el binario del `PATH` es 202 commits más viejo que el código declarando la misma versión.** `HEAD` = `537e73e3` + `0b5a9351` + `a2b0bd13` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag `v2.5.2`).

Se ha hecho la **otra mitad** de lo que la revalidación nombró: no comprobar si las deudas siguen diciendo verdad —eso ya está— sino la **revisión de vigencia**, que **no es un cierre**. Ninguna deuda se cierra aquí y **ninguna severidad baja**. Evidencia: [`receipts/c3m5-bounded-contexts/REVISION-VIGENCIA-R2.md`](receipts/c3m5-bounded-contexts/REVISION-VIGENCIA-R2.md).

**Lo que se corrige, y una de las correcciones es de una conclusión ya publicada:**

| Deuda | Publica | **Medido hoy** |
|---|---|---|
| `INC-DEBT-060` | 179 filas, 97 sin nombrar, 91 `OPEN` | **108 ciclos reales, 0 sin nombrar.** 187 filas = 108 reales + **79 `__spine_import__`** |
| `INC-DEBT-061` | 51 ciclos apartados | **53** (6 de 15 alias, estable) |
| `INC-DEBT-063` | tres recibos con `cycle_id` inexistente | **CONFIRMADA** — los tres existen, y sus tres ids siguen sin existir |
| `INC-DEBT-050` | alias existe, historia partida | **VIGENTE** — 12 ciclos, 6 `OPEN`, en el `from_id` |

**`INC-DEBT-060`: el titular es falso y por fin está medido fila a fila.** `sddk cycle list` (build de HEAD, ledger real, sin copia) nombra **108 de 108** ciclos reales, **0 sin nombrar**, y declara `unreadable_manifests: 2` — que **coincide exactamente** con las 2 filas reales cuyo manifiesto no deserializa, o sea que marca lo que no puede leer en vez de omitirlo. Es la tercera medición de la propiedad (`69c`, `69m`, esta) y las tres convergen. **Lo que queda abierto es la segunda población:** las **79** filas `__spine_import__`, que llevan `79, 79, 79` desde `69d`.

**`REVALIDACION-R2.md`-publicó dos números que son la suma de dos poblaciones, y esa corrección va en su propia deuda:** §3 da «187 ciclos» y «107 `OPEN`» contando la tabla entera. Los 187 son 108 + 79 y los 107 son 28 + 79. **Ninguno de los dos describe este proyecto.** El documento de revalidación **no se reescribe** —la descomposición va en `INC-DEBT-060`— pero su §3 y su §5 se marcan como leídos con esta corrección delante.

**`INC-DEBT-063` era `NOT_VERIFIABLE` y era un error de directorio.** Los tres recibos **existen**, en las rutas que la `references:` de la propia deuda cita. La revalidación los buscó en `tests/cycle-artifacts/`. **Séptima vez en esta serie que se mide la cosa equivocada**, y la segunda con consecuencia grave: iba a dejar sin verificar una `medium/P2` **que es cierta**.

**El hallazgo que no estaba en ninguna deuda: el binario instalado no es el código, y no puede decírselo.** `~/.local/bin/sddk` declara **`2.5.3`** —la versión del workspace **sin publicar**— y es de `90f16ad2`, **202 commits por detrás de `HEAD`**. No tiene `sddk cycle list` ni `sddk dev build-id`: **el artefacto que tiene el problema no puede ejecutar el check que lo encuentra.** Es la forma más difícil de la condición de `INC-DEBT-064`, y confirma su `open`/`high` sin cambiarlo.

**Y una medición publicada que fechó mal el binario que usó:** `REVALIDACION-R2.md` dice *«build del 2026-10-03 06:57»*; el fichero es de **2026-10-01 21:17:29 CEST**. **La revalidación de R2 se hizo con el binario viejo**, y por eso no vio que `cycle list` ya existía. Medir la vigencia de una deuda con el artefacto que la deuda describe es medir con el instrumento que la deuda dice que no sirve.

**⚠️ Y el veto de `test_gate_coverage.py` destapó que `R1` tenía un segundo bloqueo, y este no era de seguridad: `bash scripts/release.sh` MORÍA antes de compilar.** Ese gate corre en el paso 1b y su fallo hace `die`, y estaba en rojo con cuatro `tests/test_*.sh` sin runner. Medidos uno a uno antes de tocar nada: tres son herméticos y pasan (**PASS=8**, **PASS=6**, **PASS=22**) → cableados; el cuarto exige dos binarios con procedencia distinta como argv → a `EXCEPTIONS` con el motivo **y con la consecuencia declarada** (los cuatro estados de la identidad **no se verifican en el camino de release** hasta que exista el arnés). Gate ahora `RESULT: PASS`, `SIN runner y SIN motivo: 0` sobre 47 tests.

**Los guards de reconciliación van en pasos nuevos `3b/14` y `3c/14`, después del build y no en el 1b** — porque el 1b corre antes de compilar y el guard **fallaría cerrado por un binario que todavía no existe**. Así reconcilian **el artefacto que se va a publicar**: 3,6 s y 36,5 s, medidos. La autofalsación entró **en contra de mi primera decisión** (argumenté que 36,5 s no compensan) y el gate la vetó por la vía correcta: los motivos que `EXCEPTIONS` acepta son semánticos, no «es lento».

**Y los cuatro estados de `binary.build_identity` de `dev doctor` también volvieron al camino de release (paso `3d/14`), en contra de lo que se había escrito.** El guard estaba excepcionado porque **exige dos binarios como argv** y el release solo tiene el concluyente. **Medido**: pasando el concluyente en los dos huecos, **O2–O5 y O7 pasan y solo O6 falla** — O6 es el único que depende de la procedencia **no** concluyente. Y el precio de medirlo entero está medido: una compilación en frío de debug son **118,77 s**, o sea ~2 min por publicación para medir un estado en el que el check **por diseño no decide**. El guard ganó un **modo de un solo binario** que declara O6 `NOT_RUN` con su motivo y baja la cuenta de 5 a 4 veredictos, y el umbral **se falsificó**: quitar un `registrar` lo detectan dos mecanismos independientes en ambos modos. **Residuo honesto:** el estado no concluyente sigue siendo cobertura de sesión, no de pipeline.

**Ese guard ya existe para `INC-DEBT-060`, y para `INC-DEBT-061` está decidido que no exista.** `tests/test_cycle_list_total_reconciliation.sh` reconcilia lo que `cycle list` declara contra las filas que la autoridad tiene para el proyecto que el propio producto declara. **La comparación trivial no sirve y se descartó:** `cycle list` imprime su total como `output.cycles.len()` (`cycle.rs:2115`), o sea del mismo vector que emite, así que *declarado == emitido* es insatisfacible. La que tiene dientes es **de fuente cruzada**, y es justo la que habría detectado el «187 = 108 + 79». **6 casos con el binario real sobre fixtures herméticos, 9 modos de mentira falsados y un control que exige aceptar el caso bueno** (`PASS=16 FAIL=0`). Y `…_mutation.sh` exige que **cada comprobación sea load-bearing por separado** (`PASS=10 FAIL=0`): sin eso un guard puede rechazar las nueve mutaciones por efecto colateral y parecer que vigila.

**`061` no lleva guard, y no es una prioridad pospuesta:** su propiedad es cierta por construcción del almacenamiento y **no hay superficie de producto con la que reconciliarla**. Un script que la mida sin consumidor sería el hueco de `INC-DEBT-065` con forma de gate. Lo que `061` necesita es la decisión del operador.

**Siguiente paso: `R2` sigue abierta, y lo que le falta ya no es un guard de `060`.** El guard mide la propiedad; lo que queda es la **segunda población** —las 79 filas `__spine_import__`, que ningún comando puede ni pedir— y las **dos decisiones del operador** sobre `050`/`061`, que son un solo hecho. Ninguna de las dos se resuelve midiendo: las dos son de disposición de datos.

**Decisiones del operador que siguen abiertas:** (a) dónde vive la clave de firma de la release (KMS frente a fichero local) — bloquea `R1`; (b) qué superficie de producto produce `HostEvent` — bloquea `C3m.1`; (c) las dos salidas de `INC-DEBT-050` y `INC-DEBT-061`, que son un solo hecho; (d) bajar `INC-DEBT-060` a `medium`, reservado al operador desde `69m`; (e) archivar `a4-1-generic-verify` (publicado como `v1.169.46`, sin transicionar).

---
**Estado (session-69r, 2026-10-03): C3m.5 MEDIDO. La decisión de bounded contexts NO se toma como taxonomía de módulos, y el motivo está medido.** `HEAD` = `08081930` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag `v2.5.2`).

C3m.5 es literalmente una línea en el roadmap (*«R0 bounded-context decision»*) y **no existe taxonomía declarada en ninguna parte del repo**, así que lo medido no es qué contexto es cada módulo sino **cuánto del motor deja fuera cualquier taxonomía que se escriba**: con una convención de nombres propuesta, **68 de 133 raíces y 36.007 líneas quedan fuera de toda caja**, y ese cajón sin nombre es el mayor de todos.

**Tres hechos invalidan un mapa de contextos dibujado sobre módulos:**

1. **El módulo no es una unidad** — hay tres layouts, y `authority_engine` es raíz-partida: su `.rs` (1.235 líneas) declara `pub mod bridge/runner` cuyas fuentes viven en el directorio homónimo. **1.804 líneas en un módulo con dos estilos de fichero**, y ningún guard lo vigila.
2. **La superficie pública tiene dos formas** — son **133** raíces, no 131: `adoption` y `paths` son `mod X;` privados con `pub use X::*;`. **1.643 líneas llegan al usuario sin que exista `sddk_engine::<módulo>`.**
3. **El consumo no va por rutas, va por una fachada** — `pub struct Engine` lo extienden 5 ficheros con 40 `pub fn` y el CLI llama 16 métodos. `cycle.rs:1967` llama `context.engine.cycle_pause(...)` y el método está en `cycle_pause.rs:68`.

**Decisión que la medición sí sostiene:** *la frontera de contexto de `sddk-engine` es el tipo `Engine`, no el directorio de módulos.* El R0 se aplica al conjunto de métodos de la fachada. **C3m.5 queda en `explore` con `Readiness: NOT_READY`**: qué se hace con los **38 módulos, 14.852 líneas**, anunciados en la superficie pública y que ningún comando mueve, es trabajo de `R2`, no de este ciclo.

**`INC-DEBT-065` corregida 24 → 30** en su propio fichero, sin reescribir la medición original. Cuatro diferencias corrigen a la entrada (un enlace en un doc-comment, una subcadena `is_up_to_date` dentro de otro método, dos `pub use X::*` que no prueban consumo) y dos no están en su universo.

**La lección que más cuesta, y va escrita porque es la que se repite:** el instrumento se reconstruyó desde cero, dio **37** contra los 24 publicados, y **el contraste con el instrumento de C3m.1 —escrito por separado, mismo repo, misma pregunta— fue lo que destapó el defecto**. No era criterio: era que un grafo de rutas no ve la vía del método. Al corregir las cuatro vías apareció **un bug propio** (para un módulo fichero, `relative_to(SRC).parts[0]` devuelve el nombre con extensión y la autoexclusión nunca casa). *Dos mediciones que comparten un supuesto no son un contraste.* Y la mutación M2 de la primera falsación **no probaba lo que decía**: se corrigió en el guard.

Verificación: autoprueba **16/16**, **3** controles contra el grafo real, convergencia declarada módulo a módulo (**30 vs 24**, superconjunto estricto, 0 diferencias sin explicar), **falsación 5/5 detectadas y 0 sobrevividas** con el source restaurado y verificado. `shellcheck` limpio, `tests/test_changelog_coverage.sh` **PASS=79 FAIL=0**, escáner `CLEAN`. `cargo test` **NOT_RUN**: no hay cambio de producto.

**Siguiente paso: `R2`** — `INC-DEBT-050` (critical/P1), `061`, `060`, `063`, `049`, y archivar `a4-1-generic-verify`. Es lo único de la cola que no depende de una decisión de seguridad del operador.

**Decisiones del operador que siguen abiertas:** (a) dónde vive la clave de firma de la release (KMS frente a fichero local) — bloquea `R1`; (b) qué superficie de producto produce `HostEvent` — bloquea `C3m.1`.

**Estado (session-69n bis 8, 2026-10-03): fase `verify` de `cl-doctor-build-identity` CERRADA. Los cuatro estados del check se han medido con dos binarios reales, y el instrumento que los midió tenía dos defectos que se han corregido en el propio instrumento.** `HEAD` = `261a578c` + este commit documental. Workspace **2.5.3 declarada, no publicada**.

**Lo que midió la verify, y no es repetir la suite:** que los cuatro estados se distinguen con el **binario real** y el **repo real**, y sobre todo que los dos estados de «no puedo saber» **no dan rojo**. Un rojo falso en `dev doctor` entrena a ignorar los rojos.

| Objetivo | Escenario | Veredicto | `doctor` |
|---|---|---|---|
| O2 | checkout de sddk, binario al día | `present` — `OK: el checkout esta en el mismo commit que el binario` | exit **0** |
| O3 | **el mismo binario**, checkout atrasado | **`missing`** — nombra la relación | exit **1** |
| O5 | repo **impostor** | `present` — `N/A: no es un checkout de sddk-framework` | — |
| O4 | directorio que **no** es repo | `present` — `N/A: no es un checkout de sddk-framework` | — |
| O6 | identidad **no concluyente** (`source: git`) | `present` — `N/A: …; STOP 6 no le permite decidir` | — |

Los dos binarios son del **mismo commit** y se diferencian solo en la procedencia (`env` frente a `git`), lo que confirma por medición lo que el SCOPE afirmaba: **la identidad sola no basta, la procedencia acompaña siempre al valor**. Se construyen **fuera** del test y se reciben como argumentos: medir con el binario equivocado ocurrió **dos veces** en esta sesión, y la razón está en el SCOPE de `cl-build-identity`.

**El instrumento falló dos veces, y ninguna era del producto.**

La primera: `PASS=10 FAIL=5` con el producto en verde. Los cinco fallos eran del **aserto**, que comparaba contra `"PRESENT|"` cuando `${V%%|*}` ya había quitado la barra. Ninguna medición estaba mal.

La segunda la encontró el falsificador y era peor: **O7 afirmaba «ningún veredicto sin motivo» y medía 2 veredictos de 5.** Los temporales del impostor y del «sin checkout» se enviaban a la basura antes de que O7 los recorriera, y su `[ -d ] || continue` se los saltaba en silencio, mientras los demás asertos de O7 seguían contando lo suyo — de modo que el resultado global seguía en `FAIL=0` y **el hueco era invisible desde el número**. Los veredictos se recogen ahora al medirse, y el recuento de muestras es un aserto por derecho propio.

**Y el aserto nuevo tampoco bastaba, y eso lo dijo la falsificación:** anular la aritmética que contaba los motivos vacíos **no lo detectaba nadie**, porque el único guard que dependía de ella era el propio aserto anulado. El recuento vive ahora en una función que el test **se autocomprueba** contra una lista conocida antes de fiarse de ella — **una copia del código no vigila el código**, séptima vez que esta serie lo paga.

**Falsificación del instrumento: 7 mutaciones, 6 detectadas, 1 declarada.** La que sobrevive se explica en vez de maquillarse: anular el recuento por `grep` **cuando no hay ningún motivo vacío** es indistinguible de la constante `0`, luego es una mutación **equivalente a la base**, no un defecto del guard. Compuesta **con** un motivo vacío sí cae (FAIL=2), que es lo que demuestra que los dos caminos se necesitan el uno al otro. **NO MEDIBLE no es DETECTADA**, y un `7/7` que no se ha medido no se escribe.

**Gates, uno a uno, con evidencia ejecutada y aplanada al formato que el validador exige** (`gate_receipt.rs:51-59` exige `argv`/`exit_code`/`output_digest` **en el nivel superior**; un lote anidado bajo `commands` se rechaza):

| Gate | Comando | Exit | Digest |
|---|---|---|---|
| `tests-pass` | `cargo test --workspace --no-fail-fast` | 0 | `sha256:dea6a43469d477a…` |
| `policy-compliant` | `bash tests/test_doctor_identity_states.sh <2 binarios>` | 0 — **PASS=19 FAIL=0** | `sha256:d884e1b358e38e7…` |

**Recuento de la suite, medido y no heredado:** `passed=5435 failed=0 ignored=24`. El «283 binarios» que afirmaba el borrador del informe **no se había medido en esta sesión** y se ha retirado del texto en vez de dejarlo ahí con la autoridad de un informe.

**Estado de la autoridad:**

| Ciclo | Estado |
|---|---|
| **`cl-doctor-build-identity`** | **`RELEASE_PENDING`, sequence 4, 2 artefactos, 2 gates `passed`** (cerrado) |
| `cl-build-identity` | `RELEASE_PENDING`, sequence 4, 2 artefactos, 3 gates |
| `cl-release-forge-testability` | `RELEASE_PENDING`, 9 gates, sequence 7 |
| `cl-ledger-export-total` | `RELEASE_PENDING`, 9 gates |
| `cl-ledger-watch-total` | `RELEASE_PENDING` |
| `a4-1-generic-verify` | `RELEASE_PENDING`, 5 artefactos, path `A-lite` — **publicado hace semanas y sin archivar**, ver abajo |
| `INC-DEBT-064` | **`open`, high/P1 — el mecanismo existe, la condición sigue viva** |

**Y una hipótesis mía que la medición desmintió, antes de que se volviera un
«defecto» inventado.** Buscando regresiones por código duplicado (§4 del
objetivo) encontré que el predicado de SHA está escrito en **tres** sitios con
tres expresiones distintas: `is_hex_sha` en `build.rs:83` usa
`is_ascii_hexdigit()` —que acepta mayúsculas—, y las copias de `release.sh:561`
y `reconcile_state_pointer.sh:200` usan `[0-9a-f]`, que no. Medido:
`'ABC1234'` → `Rust=True | bash=False | python=False`.

Concluí que **nadie vigilaba la relación entre las copias** y estuve a punto de
declararlo defecto. **Es falso, y la medición lo dice: 3 de 3 mutaciones que
introducen esa divergencia caen.** Las tres la detectan
`tests/test_build_identity_policy.sh` y `tests/test_doctor_identity_states.sh`.
**Una hipótesis de hueco que no se falsifica se convierte en deuda inventada**,
y esta sesión ya lleva siete casos de un guard que resultaba ser otro problema.
Lo que sí es cierto, y es más pequeño: las tres copias **difieren** en el rango
de caracteres, y eso **no es alcanzable hoy** — medido, `git rev-parse HEAD`
devuelve 40 hex en minúsculas — luego no es un fallo de comportamiento, es una
divergencia latente que nadie sincroniza cuando una de las tres cambie.

**El sexto ciclo no es uno más, y por eso no estaba en la tabla (R0).**
`a4-1-generic-verify` figura en el ledger como `RELEASE_PENDING` desde el
**2026-09-16**, y su release **sí salió**: el tag `v1.169.46` existe, y el
handoff de la sesión lo registra como publicado. Los otros cinco se movieron
esta semana. Es decir, **no espera la clave KMS ni nada**: está publicado y sin
transicionar a `archive`, y por eso inflaba el recuento de «ciclos bloqueados»
a seis cuando los bloqueados son cinco.

**Medido, no supuesto:** `cycle list` sobre los 105 ciclos del proyecto da
exactamente seis en `RELEASE_PENDING`, y `git tag --list 'v1.169.46'` devuelve
la etiqueta. Archivar ese ciclo es una escritura sobre el ledger y **queda
como decisión del operador**; lo que se corrige aquí es el recuento que lo
presentaba como pendiente.

**Por qué INC-DEBT-064 sigue `open`:** los binarios **ya instalados** declaran `source: git` porque se construyeron antes del cambio de `release.sh`, luego el check es N/A para ellos y **no tiene dientes hasta la próxima release**, bloqueada por la clave KMS. No es un fallo del check: es que el check es honesto con lo que sabe. Severidad sin cambio, `high` y no `critical`: la condición de escalada —*«si `dev doctor` declara coherencia donde no la hay»*— **sigue sin cumplirse**; este trabajo le quita una vía para que se cumpla, no la cumple.

**Lo que sigue abierto, sin adornos:**

1. **Clave KMS** — bloqueo de la release 2.5.3 y de los **cinco** ciclos de esta semana, del operador. El sexto de la tabla, `a4-1-generic-verify`, **no espera la KMS**: su release salió como `v1.169.46` y lo que le falta es la transición a `archive`.
2. **INC-DEBT-050**: las dos salidas. La migración está **medida como inalcanzable**.
3. **INC-DEBT-061**: los 51 ciclos de la mitad apartada.
4. **INC-DEBT-060**: solo las 79 filas `__spine_import__`.
5. **INC-DEBT-063**: los tres recibos con `cycle_id` inexistente.
6. **INC-DEBT-049**: el operador reescribe F49 o cierra.
7. **La ruta forge contra un GitHub real**: `NOT_RUN`.
8. **Aviso lateral sin investigar**: `dev doctor` informa que `impeccable-primary.md` excede el presupuesto de 300 líneas.

---
**Estado (session-69n bis 7, 2026-10-03): el detector de identidad ya tiene consumidor — `dev doctor` consulta si el binario es el de este checkout. Un detector que funciona y al que nadie pregunta no vigila nada.** `HEAD` = `a5c18b97` + este commit documental. Workspace **2.5.3 declarada, no publicada**.

**Lo que faltaba, medido:** `grep -n 'build_id|BuildIdentity|SDDK_BUILD_SHA|SDDK_GIT_SHA|identity' crates/sddk-cli/src/dev/doctor.rs` devuelve **cero coincidencias**, y `binary.bundle_coherence` valida el recibo, el directorio versionado y el manifiesto — **ningún commit**. Eso cumple la condición de escalada que el propio INC-DEBT-064 declara: *«si `dev doctor` declara coherencia donde no la hay»*.

**Medido end-to-end, con el binario real y el repo real:**

| Escenario | `binary.build_identity` | `all_present` | Exit |
|---|---|---|---|
| repo real, binario en HEAD | `present` | `true` | 0 |
| el mismo repo, binario en HEAD~1 | **`missing` — FALLO**, y nombra la relación | `false` | **1** |
| repo **impostor** (`crates/sddk-cli/Cargo.toml` de otro paquete) | `present` | — | — |

La tercera fila es la que demuestra que la segunda es una **detección** y no una alarma.

**Son cuatro estados y solo uno es fallo:** al día → verde; **atrás o divergido → ROJO**; **sin checkout de sddk-framework → verde con N/A**; **repo de git ajeno → verde con N/A**. Y la identidad **no concluyente** (el fallback `.git`) también va a verde con su motivo, por STOP 6: decidir con ella reproduciría, en un segundo sitio, el defecto que el módulo existe para quitar.

**Los dos N/A gobiernan el diseño y no estaban en el encargo.** `resolve_root` sube desde el cwd buscando un marcador de proyecto, luego sin esa guarda compararía el commit de sddk contra la historia de **otro** repo y declararía `diverged` con toda la apariencia de un hallazgo. **Un check rojo que se equivoca entrena a ignorar los rojos**, que es peor que no tener check; el precedente está en el propio doctor, en la rama `flat_install`.

**⚠️ Cambia el contrato: `dev doctor` puede salir con 1 donde antes salía con 0.** No es efecto colateral, es el objetivo, y está declarado en el changelog en vez de descubrirse en producción.

**Falsificación: 7 guards, 7/7, y uno sobrevivió a la primera pasada** — sexta vez en la serie que un guard solo fija el caso donde el defecto no se manifiesta, y este lo escribí yo hacía media hora. R5 usaba un repo **sin** `crates/sddk-cli/Cargo.toml`, luego un marcador demasiado permisivo nunca se distinguía del correcto. Le faltaba el **impostor**. Corregido en el guard, conservando el caso viejo.

**Estado de la autoridad:**

| Ciclo | Estado |
|---|---|
| **`cl-doctor-build-identity`** | **`OPEN/verify`, sequence 2, 1 artefacto, 1 gate `passed`** (nuevo) |
| `cl-build-identity` | `RELEASE_PENDING`, sequence 4, 2 artefactos, 3 gates |
| `cl-release-forge-testability` | `RELEASE_PENDING`, 9 gates, sequence 7 |
| `cl-ledger-export-total` | `RELEASE_PENDING`, 9 gates |
| `cl-ledger-watch-total` | `RELEASE_PENDING` |
| `INC-DEBT-064` | **`open`, high/P1 — el mecanismo existe, la condición sigue viva** |

**Por qué INC-DEBT-064 sigue `open`:** los binarios **ya instalados** declaran `source: git` porque se construyeron antes del cambio de `release.sh`, luego el check es N/A para ellos y **no tiene dientes hasta la próxima release**, bloqueada por la clave KMS. No es un fallo del check: es que el check es honesto con lo que sabe. Y el binario del PATH sigue obsoleto. Severidad sin cambio, `high` y no `critical`: la condición de escalada **sigue sin cumplirse** — este commit le quita una vía para que se cumpla, no la cumple.

**Lo que sigue abierto, sin adornos:**

1. **Fase verify de `cl-doctor-build-identity`** — y lo que tiene que medir no es repetir la suite, sino que los cuatro estados se distinguen con el binario real, sobre todo que impostor y «sin checkout» **no** dan rojo.
2. **Clave KMS** — bloqueo de la release 2.5.3 y de los cinco ciclos, del operador.
3. **INC-DEBT-050**: las dos salidas. La migración está **medida como inalcanzable**.
4. **INC-DEBT-061**: los 51 ciclos de la mitad apartada.
5. **INC-DEBT-060**: solo las 79 filas `__spine_import__`.
6. **INC-DEBT-063**: los tres recibos con `cycle_id` inexistente.
7. **INC-DEBT-049**: el operador reescribe F49 o cierra.
8. **La ruta forge contra un GitHub real**: `NOT_RUN`.

---
**Estado (session-69n bis 6, 2026-10-03): fase `verify` de `cl-build-identity` CERRADA con los dos gates graduados sobre evidencia ejecutada. El ciclo pasa a `RELEASE_PENDING` y se une a los otros tres, todos bloqueados por la clave KMS.** `HEAD` = `68874b35` + este commit documental. Workspace **2.5.3 declarada, no publicada**.

**La medición que sostiene la verify, y que es la que faltaba:** `--check` se ejercita **como lo usaría un gate real**, con el **mismo binario** contra dos checkouts distintos. Contra `HEAD`: `relation: Matches`, **exit 0**. Contra un checkout un commit atrasado: **exit 1**. Un detector que no distingue entre «el binario es el del checkout» y «el checkout tiene trabajo que el binario no tiene» no sirve como gate — y esa es exactamente la propiedad que faltaba antes del arreglo de `11c8e1d9`, donde el segundo caso salía con **exit 0** mientras su propia línea de `reason` decía lo contrario.

**Los dos gates, con evidencia ejecutada y no redactada.** REQ-IPV exige **un** comando con `argv`, `exit_code` y `output_digest` en el nivel superior; un lote se rechaza con `ENGINE_INVALID_PASS_EVIDENCE`, comprobado.

| Gate | Comando | Exit | Digest |
|---|---|---|---|
| `tests-pass` | `cargo test --workspace --no-fail-fast` | 0 | `sha256:70120e1c1944cd…` |
| `policy-compliant` | `bash tests/test_build_identity_policy.sh` | 0 | `sha256:eb688ff0c1fb1ef…` |

**El instrumento de políticas falló tres veces antes de decir la verdad, las tres por su cuenta y no por la del producto.** (1) `cargo test | grep -q` con `pipefail` devuelve error porque `grep -q` sale al primer match, `cargo test` recibe SIGPIPE y `pipefail` lo propaga: el guard se ponía verde por el motivo equivocado en cuanto la salida crecía. (2) El escáner pasaba los **ficheros enteros**, y `SESSION-JOURNAL.md` está en el rango porque este cambio le añade una entrada — luego reportaba los 27 caracteres no latinos **históricos** que el repo declara y no reescribe, y fallaba por deuda ajena. **Medir el cambio y medir la historia no es lo mismo**, y es el mismo error que el `case` gloton de la biseca 5. (3) Y una nueva: el helper pasaba `"$filter"` entrecomillado a cargo, con lo que `--lib dev::build_id` llegaba como un argumento único, cargo no lo entendía, y el guard contestaba `no-ok` — **un guard que miente porque el comando nunca llegó a correr**. Se comprobó contra el mismo patrón a mano, que sí casaba: la diferencia era la comilla.

**Corrección de dato propio:** el `fencing_token` de este ciclo es **1**, no 2. La biseca 5 afirmaba que readquirirlo lo incrementaba «ya va por 2». El incremento ocurre al **reemplazar** un lease caducado; la transición de fase lo **liberó** — borró la fila — y un lease ausente arranca en 1. La afirmación general era cierta, el número concreto era falso, y se ha medido antes de propagarlo.

**Estado de la autoridad:**

| Ciclo | Estado |
|---|---|
| **`cl-build-identity`** | **`RELEASE_PENDING`, sequence 4, 2 artefactos, 3 gates `passed`** |
| `cl-release-forge-testability` | `RELEASE_PENDING`, 9 gates, sequence 7 |
| `cl-ledger-export-total` | `RELEASE_PENDING`, 9 gates |
| `cl-ledger-watch-total` | `RELEASE_PENDING` |
| `INC-DEBT-064` | **`open`, high/P1 — con detector entregado y la condición viva** |

**Lo que esta verify NO declara, escrito para que no se lea al contrario:** INC-DEBT-064 sigue `open` — el changelog dice «avanza», no «cierra». **`dev doctor` no invoca `build-id --check`**, luego el detector existe y funciona pero la detección depende de que alguien la pida. La ruta forge contra un GitHub real sigue `NOT_RUN`. Y no se ejecutó ninguna matriz UAT nueva: esto son gates y mediciones de comportamiento.

**Lo que sigue abierto, sin adornos:**

1. **Cablear `build-id --check` al `dev doctor`** — único paso propio, y el que convierte el detector en mecanismo en vez de en dato. Antes de escribir, medir: leer el bloque `binary.bundle_coherence` (`doctor.rs:425-468`) y comprobar que hoy no invoca la comparación, no suponerlo.
2. **Clave KMS** — único bloqueo de la release 2.5.3 y de los cuatro ciclos en `RELEASE_PENDING`, del operador.
3. **INC-DEBT-050**: las dos salidas. La migración está **medida como inalcanzable**.
4. **INC-DEBT-061**: los 51 ciclos de la mitad apartada.
5. **INC-DEBT-060**: solo las 79 filas `__spine_import__`.
6. **INC-DEBT-063**: los tres recibos con `cycle_id` inexistente.
7. **INC-DEBT-049**: el operador reescribe F49 o cierra.
8. **La ruta forge contra un GitHub real**: `NOT_RUN`.

---
**Estado (session-69n bis 5, 2026-10-03): el punto que faltaba para que el remedio de INC-DEBT-064 pudiese funcionar está hecho — y la deuda sigue `open`, que no es lo mismo.** `HEAD` = `c30dcf89` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**Lo que faltaba:** `release.sh` no exportaba `SDDK_GIT_SHA`, así que `build.rs` caía al fallback `.git`, el binario publicado declaraba `source: git`, y `dev build-id --check` **no salía nunca de `unknown`**. El detector no tenía dónde fallar justo en el caso que motiva INC-DEBT-064: un binario **publicado** y obsoleto. Correcto, pero inútil.

**El SHA se mide, y el predicado lo encontró el falsificador de su propio commit.** La primera versión anclaba 7 dígitos con un glob `[0-9a-f]{7}*` y **aceptaba `abc1234 (HEAD detached)`** — el `*` se come lo que venga detrás. Ahora es el mismo predicado que `is_hex_sha` en `build.rs`: 7 a 40 caracteres, todos hexadecimales.

**El umbral distingue lo que rompe la identidad de lo que no.** Cambios rastreados bloquean siempre; entre los sin seguimiento solo bloquean los que entran en el binario (`Cargo.toml`, `Cargo.lock`, `crates/**`, `build.rs`), porque cargo los compila y ningún commit los identifica. Un fichero suelto en `docs/` no cambia el binario, y bloquear la release por eso sería **endurecer el gate del operador sin que nadie lo pidiera** — el preflight usa `git diff --quiet`, que no ve sin seguimiento. Medido en repos git de verdad.

**Y se corrigió una sobreafirmación en el mismo commit.** El changelog decía «**cierra** INC-DEBT-064» mientras el documento seguía `open`. Es la misma clase de mentira que el documento denuncia: afirmar por el número en lugar de por el estado. Ahora dice «**avanza**», y los dos son verdaderos el mismo día.

**Por qué la deuda no baja a `resolved`, en tres puntos concretos:** (a) el binario del PATH **sigue obsoleto** y lo seguirá hasta que se publique una release, bloqueada por la clave KMS; (b) la fase `verify` de `cl-build-identity` está abierta; (c) **`dev doctor` todavía no invoca `build-id --check`**, así que la detección depende de que alguien la pida. Conectar la detección al doctor es trabajo por hacer, y mientras tanto la deuda sigue siendo deuda: hay que acordarse de mirar.

**El guard de esta vez falló tres formas seguidas, las tres del mismo tipo: no miraba el producto.** Llevaba una **copia** del predicado, con lo que la primera mutación no tenía nada que detectar. Al cambiarlo a extracción del source, el regex gloton se llevó desde el `if` del paso 2b hasta el `fi` mío — media lista de `release.sh` dentro de la función, y el guard rechazaba SHA válidos sin que nadie supiera por qué. Corregida a extracción por líneas, el filtro de la tercera mutación se extrajo con una búsqueda sobre el fichero entero, y `release.sh` tiene **cinco** `grep -E`: se llevó el de la línea 418, no el suyo. **Un guard que ejercita una copia no vigila el código, y uno que se ancla a la primera coincidencia mide algo distinto de lo que cree.**

**Estado de la autoridad:**

| Ciclo | Estado |
|---|---|
| `cl-release-forge-testability` | `RELEASE_PENDING`, 9 gates, sequence 7 |
| `cl-ledger-export-total` | `RELEASE_PENDING`, 9 gates |
| `cl-ledger-watch-total` | `RELEASE_PENDING` |
| **`cl-build-identity`** | **`OPEN/verify`, `B-direct`, sequence 2, 1 artefacto, 1 gate `passed`** |
| `INC-DEBT-064` | **`open`, high/P1 — con detector entregado y la condición viva** |

**Lo que sigue abierto, sin adornos:**

1. **Clave KMS** — único bloqueo de 2.5.3 y de los tres ciclos en `RELEASE_PENDING`, del operador.
2. **Fase verify de `cl-build-identity`**: readquirir el lease (el `fencing_token` se incrementa) y ejercitar `--check` como lo usaría un gate real.
3. **Cablear `build-id --check` al doctor** — es lo que convierte el detector en mecanismo en vez de en dato, y es la lección de INC-DEBT-061 aplicada donde toca: *«el arreglo no está roto, no está desplegado»* como **mecanismo**, no como dato de un caso.
4. **INC-DEBT-050**: las dos salidas. La migración está **medida como inalcanzable**.
5. **INC-DEBT-061**: los 51 ciclos de la mitad apartada.
6. **INC-DEBT-060**: solo las 79 filas `__spine_import__`.
7. **INC-DEBT-063**: los tres recibos con `cycle_id` inexistente.
8. **INC-DEBT-049**: el operador reescribe F49 o cierra.
9. **La ruta forge contra un GitHub real**: `NOT_RUN`.

---

**Estado (session-69n bis 4, 2026-10-03): `cl-build-identity` implementado y la fase de build CERRADA — pero el detector que lo entregaba pasaba sobre un binario obsoleto, y lo encontró el producto en uso, no un test.** `HEAD` = `11c8e1d9` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**Lo entregado** (`032e9553`): `crates/sddk-cli/build.rs` que embebe commit, procedencia y suciedad; `sddk dev build-id` en texto y JSON; y `dev build-id --check`, que compara contra el checkout y nombra la relación. Verificado con **dos binarios reales**: sin `SDDK_GIT_SHA` declara `source: git` y `--check` sale `Unknown`; con ella declara `source: env` y sale `Matches`.

**El defecto, y por qué importa más que el arreglo.** `is_answer()` metía `Behind` en el conjunto que pasa, con la lectura de que «`Behind` es una respuesta, luego pasa». Y lo es — pero respecto a una pregunta **distinta**: `is_answer` contestaba *¿hubo relación?*, y el código de salida tiene que contestar *¿este binario es el del checkout?*. Medido, reconstruyendo un binario con `SDDK_GIT_SHA` clavado a un commit que no es el HEAD — el escenario de INC-DEBT-064:

```
relation: Behind
reason: …el checkout tiene trabajo que el binario no contiene
EXIT=0
```

**El texto y el código de salida se decían lo contrario en la misma pantalla.** El `reason` afirma que le falta trabajo; el `EXIT=0` afirma que no le falta nada. Para quien use `--check` como gate, un binario obsoleto se presenta como conforme.

STOP 3 —«una relación que no se pudo establecer no sale con 0»— se cumplía: `Behind` sí establece la relación, la dirección va probada con `--is-ancestor`. Lo que STOP 3 no dice, y este módulo sí tenía que decir, es que **una relación establecida y suficiente para responder no tiene por qué ser suficiente para pasar**. Es el mismo principio un nivel más arriba, y leer STOP 3 como techo en vez de como suelo lo dejó pasar. Arreglo: `is_answer` desaparece, le sigue `is_current`, que es `Matches` y nada más. Con el mismo binario y el mismo checkout, `Behind` pasa de **0** a **1**.

**Por qué ningún test lo vio, que es la parte que pesa más que el arreglo:** el guard unitario fijaba el **predicado** y el guard e2e fijaba el **formato**. El código de salida no lo fijaba nadie. El guard e2e nuevo resultó además **ciego por defecto** — cuarta vez en esta serie que un guard solo fija el caso donde el defecto no se manifiesta: sin `SDDK_GIT_SHA` el binario de test cae por el fallback `.git`, que STOP 6 declara no concluyente, luego la relación es siempre `unknown` y la rama `behind` no se alcanza. **Comprobado que pasaba con el defecto puesto.** Falsificado de verdad con `SDDK_GIT_SHA=$(git rev-parse HEAD~1)`, donde sí cae. La condición de ceguera queda escrita en el propio guard, porque un guard que parece cubrir un caso y no lo cubre es peor que uno ausente.

**Que no lo encontrara ninguna mutación es el dato.** Las seis de `23-falsify-build-identity.py` pasaban en verde sobre un `--check` que era peor que no tenerlo. **Un falsificador que solo muta la implementación no ve los defectos de la interfaz observable.**

**La forma de la evidencia de gate no es la que parecía** (REQ-IPV, spec-v2 cycle-44): es **un** comando con `argv`, `exit_code` y `output_digest` en el nivel superior, no un lote. El lote de ocho que estaba preparado falló con `ENGINE_INVALID_PASS_EVIDENCE: passed outcome is missing argv`; el validador está en `crates/sddk-domain/src/models/gate_receipt.rs:47`.

**Estado de la autoridad:**

| Ciclo | Estado |
|---|---|
| `cl-release-forge-testability` | `RELEASE_PENDING`, 9 gates, sequence 7 |
| `cl-ledger-export-total` | `RELEASE_PENDING`, 9 gates |
| `cl-ledger-watch-total` | `RELEASE_PENDING` |
| **`cl-build-identity`** | **`OPEN/verify`, `B-direct`, sequence 2, 1 artefacto, 1 gate `passed`** |

`phase.build.complete.b-direct` aplicada con `implementation-complete` graduado sobre `cargo test --workspace --no-fail-fast` (`exit=0`, digest `sha256:6f6258c662…`) y `--artifact implementation-receipt=docs/roadmap/receipts/cl-build-identity/RECEIPT.md`. La transición **liberó el lease** al cambiar de fase; el `fencing_token` pasó de 1 a 2 al readquirirlo, porque estaba caducado.

**Lo que sigue abierto, sin adornos:**

1. **Clave KMS** — único bloqueo de 2.5.3 y de los tres ciclos en `RELEASE_PENDING`, del operador.
2. **Fase verify de `cl-build-identity`**: `phase.verify.complete.b-direct`, que exige `tests-pass` y los demás gates de laverify.
3. **Concernia propia declarada, no hecha**: `release.sh` no exporta `SDDK_GIT_SHA`, así que una release publicada seguirá declarando `source: git` — correcto pero poco útil, y es lo que hace que el design diga «env es la fuente de verdad».
4. **INC-DEBT-050**: las dos salidas. La migración está **medida como inalcanzable**.
5. **INC-DEBT-061**: los 51 ciclos de la mitad apartada.
6. **INC-DEBT-060**: solo las 79 filas `__spine_import__`.
7. **INC-DEBT-063**: los tres recibos con `cycle_id` inexistente.
8. **INC-DEBT-049**: el operador reescribe F49 o cierra.
9. **La ruta forge contra un GitHub real**: `NOT_RUN`.

---

**Estado (session-69n bis 3, 2026-10-03): abierto `cl-build-identity`, el remedio de INC-DEBT-064, con el SCOPE ya reescrito porque la medición desmontó su propia propuesta.** `HEAD` = `86dea47d` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**La propuesta original del SCOPE era «un `build.rs` embebe el SHA del checkout». Se midió en un crate mínimo con el `build.rs` real, y los cuatro escenarios son fallos:**

| Escenario | Resultado medido |
|---|---|
| sin `.git` | exit 0, `sha=unknown source=absent` — degrada bien |
| con `.git`, sin `rerun-if-changed` | **congelada** en el primer build |
| con `rerun-if-changed` solo sobre `.git/HEAD` | **congelada**: `.git/HEAD` no cambia de contenido al commitear |
| con el ref resuelto también, y `packed-refs` | **congelada e incorrecta**: declaraba `9b3f0076` con el HEAD en `247e808d` |

**El cuarto decide el diseño:** un detector que emite un SHA obsoleto sin señal es peor que no tener detector, porque su salida es indistinguible de la correcta. **La identidad la fija quien lanza el build** —`SDDK_GIT_SHA`—, y el `build.rs` queda como respaldo degradado que declara `unknown` con procedencia `absent`. **STOP 6** prohíbe usar la variante `git` para decidir nada: existe como dato de diagnóstico.

**`--version` no se toca, y el motivo es medido:** `install.sh:416` hace `awk '{print $NF}'` sobre su salida, y añadir el SHA al final haría que el último campo fuera `)` — el instalador guardaría un paréntesis como versión.

**El ciclo se abrió con `--path b-direct` y el trabajo de diseño no estaba hecho. Está escrito.** Su única transición de avance es `phase.build.complete.b-direct`, sin fases de diseño. Se intentó superseder y **falló cerrado** con `ADMISSION: approval required before mutating 'cycle_state'`; **no se forzó**, porque forzar una aprobación que el operador no ha dado es el atajo que esta serie critica. Lo que se hizo fue **denegar la aprobación que yo mismo había solicitado**, y `runtime_state: approval-waiting` desapareció.

**Y el instrumento de requisitos llevaba un PASS falso.** `04-req-testable.py` daba `guards=0` porque **mi PRE-FLIGHT** usaba otra forma de declararlos; leído el caso bueno, corregido el documento (4 objetivos, 6 guards, exit 0). Pero **al verificar ese OK**, con la fila de R5 puesta a `| R5 |  |  |` el instrumento seguía reportando `O4 cubierto por ['R4','R5']` y **exit 0**: buscaba el **nombre** del guard y no miraba el contenido. Reparado y falsificado en los dos sentidos; el ciclo forge sigue en verde, luego no es regresión.

**Estado de la autoridad:**

| Ciclo | Estado |
|---|---|
| `cl-release-forge-testability` | `RELEASE_PENDING`, 9 gates, sequence 7 |
| `cl-ledger-export-total` | `RELEASE_PENDING`, 9 gates |
| `cl-ledger-watch-total` | `RELEASE_PENDING` |
| **`cl-build-identity`** | **`OPEN/build`, `B-direct`, 0 gates, groundwork commiteado** |

`cl-build-identity` tiene SCOPE-CONTRACT y PRE-FLIGHT, **sin implementación y por eso sin gate evaluado**: no se gradúa lo que no está hecho.

**Lo que sigue abierto, sin adornos:**

1. **Clave KMS** — único bloqueo de 2.5.3 y de los tres ciclos en `RELEASE_PENDING`, del operador.
2. **Implementar `cl-build-identity`**: el `build.rs` con degradación, la superficie propia, el estado sucio, y la comparación que dice `retrasado` con dos binarios reales.
3. **INC-DEBT-050**: las dos salidas. La migración está **medida como inalcanzable**.
4. **INC-DEBT-061**: los 51 ciclos de la mitad apartada.
5. **INC-DEBT-060**: solo las 79 filas `__spine_import__`, y si su severidad baja a `medium`.
6. **INC-DEBT-063**: los tres recibos con `cycle_id` inexistente.
7. **INC-DEBT-049**: el operador reescribe F49 o cierra.
8. **La ruta forge contra un GitHub real**: `NOT_RUN`.

---

**Estado (session-69n bis 2, 2026-10-03): el binario del PATH va 1,24 días por detrás del código y declara la misma versión. La comprobación que todo el mundo hace sale verde y no dice nada.** `HEAD` = `289bdb98` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**La regla que sale de aquí, y que se aplica desde ya:** mientras haya ventana declarada-pero-no-publicada, **las mediciones de comportamiento se hacen con el binario construido del repo** (`CARGO_TARGET_DIR=/var/home/rubentxu/cargo-targets cargo build --release --bin sddk`), **nunca con el del PATH**. No es una preferencia: el número de versión no dice qué contiene.

Medido con `22-medir-binario-al-dia.py`:

```
VERSION_DECLARADA_POR_EL_BINARIO=sddk 2.5.3
VERSION_DEL_WORKSPACE=2.5.3
VERSIONES_COINCIDEN=True          <-- la comprobación habitual dice que sí
ESTRUCTURAL_tiene_cycle_list=False
DIAS_DE_RETRASO=1.24
BINARIO_MIDE_EL_CODIGO_ACTUAL=False
```

**La prueba de que no es cosmético es la misma superficie medida con los dos binarios**, sobre un fixture propio de 25 documentos que no toca ningún vault real:

| Comprobación | binario del PATH | binario del código |
|---|---|---|
| `vault search --limit 5` declara el total | **no** | **sí** |
| `vault search --limit 0` devuelve todos | **no** (`no hits`) | **sí** |
| JSON con dónde llevar el total | **array desnudo** | **objeto** |

`vault search` **no tiene ningún defecto en el código**: `run_vault_search` (`vault_cmd.rs:464-488`) llama `count_matches`, traduce `limit == 0` a `usize::MAX` y construye `SearchOutput { truncated, shown, total_hits, hits }`. El arreglo es `37870817`, del 2-oct 21:09, en `origin/main`. **Lo roto es el artefacto que se ejecuta.**

**La afirmación que este mismo puntero hizo hace una hora era falsa, y se conserva al lado en vez de borrarse:** decía que el candidato natural ya medido era `vault search` con sus tres defectos. Era cierto **del binario**, no del código.

**El antecedente ya se había pagado una vez.** La nota (a) de INC-DEBT-061 resolvió que «el arreglo no está roto, no está desplegado» con `sddk 2.2.27`. **Esa lección se aplicó como dato de un caso y no como mecanismo**, y este es el documento que la convierte en mecanismo: **INC-DEBT-064**, high/P1, con el guard propuesto (`dev doctor` comparando la fecha del binario con la del repo) y la regla escrita.

**El instrumento que produce la evidencia falló tres veces antes de decir la verdad**, y las tres son modos ya registrados en esta serie: leía stdout cuando `--version` escribe en stderr; buscó la palabra `--cycle` en la prosa del `--help`, donde `--no-infer` la explica; y usó como criterio `ledger events --cycle`, **que es inventado** —`LedgerEventsArgs` (`ledger.rs:71-87`) solo tiene `--frame`, `--limit` y `--format`—. El tercero es el peor: **un criterio que nunca puede dar `True` sale `False` en ambos lados y parece corroborar lo que dice el otro.**

**No invalida la fase `verify`.** Los cuatro gates de `phase.verify.complete` y el requisito `verification-report` que aplicó el binario viejo son **los mismos** que declara el código actual (`cli.rs:5626-5660`). Verificado leyendo el código, no supuesto.

**Lo que sigue abierto, sin adornos:**

1. **Clave KMS** — único bloqueo de 2.5.3 y de los tres ciclos en `RELEASE_PENDING`, del operador.
2. **INC-DEBT-064**: el binario obsoleto. La salida (1), un check en `dev doctor`, y la (3), que es la regla de arriba.
3. **INC-DEBT-050**: las dos salidas. La migración está **medida como inalcanzable**.
4. **INC-DEBT-061**: los 51 ciclos de la mitad apartada.
5. **INC-DEBT-060**: solo las 79 filas `__spine_import__`, y si su severidad baja a `medium`.
6. **INC-DEBT-063**: los tres recibos con `cycle_id` inexistente. Se recomienda enmendar; **no se ejecuta aquí**.
7. **INC-DEBT-049**: el operador reescribe F49 o cierra.
8. **La ruta forge contra un GitHub real**: `NOT_RUN`. Tres escrituras privilegiadas sobre un repositorio ajeno (AGENTS.md §1).

---

**Estado (session-69n verify, 2026-10-03): el ciclo de la ruta forge está en `RELEASE_PENDING` con nueve gates, y el medidor de deuda resultó ser ciego antes de que su cero sirviera de evidencia.** `HEAD` = `8a2ebfd3` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**La fase `verify` se cerró con evidencia re-ejecutada aquí, no heredada del lote de implementación.** Se comprobó primero que el commit sin publicar no movía código bajo prueba —`git diff --name-only a6dfb5f2..HEAD -- crates/` sale vacío— y luego se corrió todo de nuevo: workspace **5414 passed / 0 failed / 24 ignored / 283 binarios**, `sddk-cli` **1461 passed / 0 failed**, `release_cmd` **15 passed / 0 failed**, `fmt` y `clippy -D warnings` exit 0, changelog **PASS=69**, índice de deuda **PASS=12**, falsificador **5/5**, scanner **CLEAN**.

**Un error aritmético propio, corregido antes de dejarlo en un documento.** El `RECEIPT.md` decía «los 11 tests previos del módulo». Medido por commit, la base es **10**: `034d098a~1` tiene 10 `#[test]`, el lote 1 sube a **14** —añade R1, R3, **R4** y R5— y el lote 2 a **15**, con R2 y el renombre de R3. Los diez nombres originales siguen intactos en `HEAD`, luego la afirmación de fondo —ningún verde reescrito— sí era cierta; la cifra no lo era.

**Y el instrumento que iba a demostrar «cero deuda introducida» era tan inútil como el falsificador que ya se había condemned en este ciclo.** `19-medir-deuda-forge.py` respondía `DEUDA_INTRODUCIDA=0` en las cinco clases de deuda a la vez: comparaba `git diff base..HEAD`, o sea *commits*, y una mutación aterriza en el árbol de trabajo, que ese diff no ve. Falsificado con `20-falsify-medidor-deuda.py` dio **0/5**. Reparado para leer el árbol: **5/5**. Al repararlo apareció un segundo defecto, también del falsificador: D1 escribe en `Cargo.toml` y su `finally` solo restauraba `release_cmd.rs`, así que **dejó el repo sucio** sin que su propio chequeo de sha lo notara, porque vigilaba el otro fichero. Es la segunda vez en este ciclo que un instrumento resulta ciego por mirar donde no debía, y la razón por la que se comprueba: **un instrumento que siempre contesta «0» es indistinguible de uno que no mide.**

**El resultado de fondo no cambia:** el ciclo introduce cero deuda, con 369 líneas añadidas, 87 de producción y 282 de tests. Lo que cambia es que la cifra tiene ahora un medidor al que se le ha visto fallar y detectar.

**Autoridad:** `p-63676b11dc0ef88f/cl-release-forge-testability` en **`RELEASE_PENDING`**, fase `release`, `sequence: 7`, **9 gates** todos `passed`. Ojo con el identificador: este ciclo es el único de los tres que lleva el prefijo `cl-`, y el slug desnudo da `STORAGE_NOT_FOUND` sin que sea un fallo del ciclo.

**La fase `release` exige `no-pending-effects`, `release-uat-approved` y los requisitos `merge-receipt` y `release-receipt`.** Está bloqueada por la misma clave KMS que los otros dos ciclos. Los tres están ahora en el mismo punto: `ledger-watch-total`, `ledger-export-total` y `cl-release-forge-testability`, todos `RELEASE_PENDING`, ninguno publicable sin la clave.

**Lo que sigue abierto, sin adornos:**

1. **Clave KMS** — único bloqueo de 2.5.3 y de los tres ciclos, del operador.
2. **INC-DEBT-050**: las dos salidas. La migración está **medida como inalcanzable**.
3. **INC-DEBT-061**: los 51 ciclos de la mitad apartada.
4. **INC-DEBT-060**: solo las 79 filas `__spine_import__`, y si su severidad baja a `medium` al caer el rodeo.
5. **INC-DEBT-063**: los tres recibos con `cycle_id` inexistente. Se recomienda enmendar; **no se ejecuta aquí**.
6. **INC-DEBT-049**: el operador reescribe F49 o cierra.
7. **La ruta forge contra un GitHub real**: `NOT_RUN`. Tres escrituras privilegiadas sobre un repositorio ajeno (AGENTS.md §1), y es decisión del operador.
8. **La auditoría de superficies que truncan está agotada** y no se repite.

---

**Estado (session-69n, 2026-10-03): la ruta de publicación que nunca se ejecutó bajo prueba — y no es por la red.** `HEAD` = `bbeb6301` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**La afirmación que había que verificar son dos comentarios del propio código** (`release_cmd.rs:2064` y `:2110`): que `release apply --route forge` «no tiene test» y «no es alcanzable sin red». La primera es **cierta**; la segunda **no**.

Medido: `GitHubForge` guarda `runner: Box<Runner>` y tiene `pub fn with_runner` (`forge.rs:133`), que **dos tests del gateway ya usan**; `plan_release` toma `&dyn Forge` y `apply_release` toma `&mut dyn Forge` (`release.rs:193`, `:422`); `MockForge` es `pub` y está re-exportado (`sddk-gateway/src/lib.rs:40-41`). El único test de la CLI que nombra forge comprueba que `--repo` **sin** `--route forge` **falla** — ninguno alcanza la rama.

**Todo el mecanismo de inyección existe y funciona. Lo único que falta es el seam en el call site**, que construye `GitHubForge::new(repo)` con el runner real y no deja sustituirlo. Es la diferencia entre *«esto no se puede probar»* y *«esto no se ha conectado para poder probarse»*, y la segunda es un defecto de cableado.

**El remedio es una extracción a una función que recibe `&mut dyn Forge`.** La CLI la llama con `GitHubForge::new(repo)` y el test con `MockForge`: dos llamadores, una sola fuente. **Sin** ensanchar `pub`. La alternativa que menos funciona y más cuesta es el override global `#[cfg(test)]` del runner: no movería una línea y añadiría un test, a cambio de estado mutable global, que es cambiar un defecto por otro.

**No se ejecuta contra un GitHub real:** `pr.create`, `pr.merge` y `create.release` son tres escrituras privilegiadas sobre un repositorio ajeno (AGENTS.md §1). No hay evidencia de que la ruta falle; hay evidencia de que **nunca se ha ejecutado bajo prueba**. Este ciclo entrega que deje de ser imposible comprobarlo, que es condición **necesaria y no suficiente**.

**Cuatro gates, uno a uno, y ninguno estampado: dos de ellos se ejercitaron contra el documento antes de dejarlo pasar.** `04-req-testable.py` dio objetivos=[] y guards=[] — **el defecto eran mis documentos**, escritos con `**O1** —` y `T1..T5` cuando el contrato es `1. **O1.**` y una tabla `| R1 | … |`; un instrumento que dice «no encuentro nada» puede estar roto o ser un documento que no habla su idioma, y la diferencia se establece leyendo el caso bueno antes de tocar ninguna de las dos cosas. `05-diseno.py` aplicó el perfil `watch` y dio 5 GAP **insatisfacibles** —tres de sus checks están codificados a la familia del truncamiento y no existen aquí—, así que el perfil `forge` declara esos tres como `SKIP`. `06-plan.py` dio dos GAP de dos clases: **uno del instrumento** (buscaba literalmente `no bumpea` y el plan escribe `**No** bumpea`: la afirmación está y fallaba la decoración) y **uno del documento** (el plan citaba el instrumento sin marcador de compromiso, que existe justo para distinguir una promesa de una descripción).

**Ciclo `p-63676b11dc0ef88f/cl-release-forge-testability` en `OPEN/build`, `sequence: 5`, 4 artefactos, 4 gates.** El árbol Rust está **intacto**: este ciclo aún no toca código.

**Lo que sigue abierto, sin adornos:**

1. **Clave KMS** — único bloqueo de 2.5.3, del operador.
2. **INC-DEBT-060**: solo las 79 filas `__spine_import__`, y si su severidad baja a `medium` al caer el rodeo.
3. **INC-DEBT-050**: las dos salidas. La migración está **medida como inalcanzable**.
4. **INC-DEBT-061**: los 51 ciclos de la mitad apartada.
5. **INC-DEBT-063**: los tres recibos con `cycle_id` inexistente. Se recomienda enmendar; **no se ejecuta aquí**.
6. **INC-DEBT-049**: el operador reescribe F49 o cierra.
7. **La auditoría de superficies que truncan está agotada** y no se repite.
8. **Lote 1 del PLAN de forge**: los tests RED, declarando el árbol rojo. **STOP 1 manda**: si hacer la rama alcanzable exige debilitar una comprobación de capacidad, reordenar los pasos o mover el `AdmissionTicket`, el arreglo se descarta **aunque los tests passen**.

---

**Estado (session-69m, 2026-10-03): la cuarta superficie de F63, y la peor de las cuatro porque trunca dentro de un fichero.** `HEAD` = `94cf6832` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**El defecto, medido sobre una copia byte-identica del ledger real:** `ledger export --limit 5` escribía 5 eventos de **600** y respondía `exported 5 events to …` con **exit 0**. Los 595 que dejó fuera no se mencionaban. Las otras dos superficies truncan **en pantalla**, donde el lector ve que hay un límite; esta trunca **en un fichero**, y el resultado es un artefacto que parece completo y que otro proceso consume sin ninguna señal de que le falta el 99 %.

**El segundo GAP no salió de buscar truncamientos: salió de leer el mismo fichero. `ExportOutput` derivaba `Serialize` y nunca se serializaba.** El resumen era un `format!` escrito a mano dentro del `match` de éxito, y el comando **no tenía `--format` ninguno** —`--format json` daba exit 2, unexpected argument—, luego la forma declarada no era la que estaba en vigor y una máquina no tenía dónde leer la respuesta.

**El arreglo, y una decisión que NO es la del ciclo anterior.** `ExportOutput` lleva `written`, `total_events` y `pending()` derivado por `saturating_sub`; el total se toma de `all_events.len()` **antes** del `.take(limit)`. **Aquí no se extrae función de filtro**, al contrario que en `ledger watch`: allí el `retain` corría dentro del bucle de sondeo sobre una página acotada y necesitaba un único sitio compartido entre el bucle y la cuenta; aquí hay **un** vector y **un** filtro, ya aplicados por elegir qué listado llamar, y copiar el remedio anterior habría añadido una segunda regla que no puede divergir porque no hay nada de qué divergir. **Un remedio se porta, no se copia.** Y el `Serialize` es **manual**: el derivado emite *campos* y `pending()` es un *método*, luego el primer intento dejó la forma derivada fuera del JSON sin que ningún test lo dijera.

**Verificación:** `cargo test --workspace --no-fail-fast` **5409 passed / 0 failed** en **283** binarios (baseline 5403 / 282, y el único binario nuevo es `ledger_export_declaration` con sus 6 tests: 5403 + 6, 282 + 1) · `clippy --workspace --all-targets -D warnings` exit 0 · `fmt --check` limpio · changelog **PASS=66 FAIL=0** · índice de deuda **PASS=12** · falsificador **5/5** · `09-medir-export.py` de **2/6 GAP** a **0/6** · scanner **CLEAN**. Ningún verde reescrito.

**Cinco herramientas fallaron en este ciclo, y ninguna tenía razón para pasar como verde. La lección se repite exacta: un instrumento que mide lo que tiene al lado tiene la misma forma que uno que mide bien.**

1. **Un guard que no podía fallar.** El falsificador cerró con `MUTACIONES NO DETECTADAS: 1` para M1 (`total_events = 1usize`) porque R2 no cayó —cuando **R1 y R3 la detectaron**. **Medido antes de tocar el guard:** un ciclo deja **1** evento, dos dejan 2, tres dejan 3. El fixture de R2 era de un solo ciclo, luego su total real era exactamente 1 y la constante coincidía **con la verdad**. Una aserción que no puede fallar por la razón que nombra es decoración. Corregido **en el guard**: dos ciclos y `assert total > 1`.
2. **El falsificador confundía dos cosas opuestas.** Llamaba «mutación no detectada» a «este guard no disparó donde el propio instrumento esperaba». Consecuencias opuestas —la primera es defecto del producto, la segunda es la matriz mal escrita— y salían bajo el mismo encabezado y con el mismo código de salida. Un instrumento con una lista mal escrita no puede parecer que ha encontrado un fallo de producto. Ahora hay tres desenlaces: `SOBREVIVIDA`, `DETECTADA`, `DERIVA`.
3. **La sonda de medición mintió tres veces**, y se midió por qué antes de tocarla: **polaridad invertida** (reportaba GAP justo cuando el resumen **sí** se serializaba — no podía decir OK nunca), **leía prosa** (encontró el literal `#[derive(Serialize)]` en la línea 594, **dentro del doc comment** que explica por qué no se usa) y **anclaba un detalle de implementación** que el arreglo abandonó a propósito. Un detector que busca una palabra encuentra la palabra, y un doc que explica el defecto es, para un detector textual, el defecto.
4. **Y la sonda reparada se falsificó antes de creérsela** (`13-falsify-sonda.py`, **3/3**). Corregir un detector y verlo decir «OK» no prueba nada: un detector que dice OK siempre también pasa.
5. **El doc de R2 decía «sobre la FORMA, no el numero» mientras afirmaba el numero**, y cuatro tests seguían diciendo «RED today» con el árbol ya verde. Es la misma clase que este ciclo cierra: una declaración que no describe lo que el código hace.

**El ciclo existe en la autoridad con nueve gates y su evidencia real:** `p-63676b11dc0ef88f/ledger-export-total`, `exploration-sufficient` · `requirements-testable` · `architecture-consistent` · `plan-executable` · `implementation-complete` · `tests-pass` · `policy-compliant` · `debt-severity-assigned` · `debt-priority-assigned`, cada uno con `argv`, `exit_code` y `output_digest` de una corrida. **Ninguno se estampó**: `05-diseno.py` marcó en rojo un párrafo del DISEÑO que **rechaza** `COUNT(*)` porque su check era de línea y no de documento, y `06-plan.py` tenía una lista fija `CREATED_BY_THIS_PLAN` escrita a mano —el defecto bajo prueba con otro sombrero—, ahora **deducida** del plan con ventana de dos líneas.

**El ciclo está en `RELEASE_PENDING`, no cerrado:** la release sigue bloqueada por la clave KMS, que es del operador. Siete eventos y seis artefactos en la autoridad.

**Lo que sigue abierto, sin adornos:**

1. **Clave KMS** — único bloqueo de 2.5.3, del operador.
2. **INC-DEBT-050**: las dos salidas. La migración está **medida como inalcanzable**.
3. **INC-DEBT-061**: los 51 ciclos de la mitad apartada.
4. **INC-DEBT-063**: los tres recibos con `cycle_id` inexistente. Se recomienda enmendar; **no se ejecuta aquí** (son documentos de ciclos cerrados).
5. **INC-DEBT-060**: la pieza 1 del remedio quedó **aplicada** en session-69c y su titular —«ninguna superficie del producto enumera los ciclos»— es **falso desde entonces**: `sddk cycle list` enumera los **102** ciclos del proyecto, incluidos los 18 que no tienen hecho, verificado en vivo. Queda abierta **solo** la decisión del operador sobre las **79** filas `__spine_import__`, que no son alcanzables por ninguna superficie. Reconciliado con la medición de hoy: **97 = 79 + 18** y **91 `OPEN` = 79 + 12** — el 97 original nunca fue una población, era la suma de las dos, que session-69d separó sin volver a sumar.
6. **INC-DEBT-049**: el operador reescribe F49 o cierra.
7. **La auditoría de superficies que truncan está agotada** y no se repite: medida, y de 5 candidatas quedaron 2 defectos reales.

---

**Estado (session-69l, 2026-10-02): la tercera superficie de F63, y el primer ciclo de la sesión que nace en la autoridad real de SDDK en vez de en un documento.** `HEAD` = `0aa12fbe` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**El defecto, medido sobre el ledger real:** `ledger watch --max-events 5` sobre **598** eventos escribía `[watch] emitted 5 events, exiting` y `{"__watch_complete":true,"emitted":5}`. Ahora escribe `[watch] emitted 5 of 598 (593 not emitted), exiting` y `{"__watch_complete":true,"emitted":5,"total_events":598,"pending":593}`. La frase que lo resume es de session-69f y sigue siendo la buena: **declarar que ha emitido N no es declarar que había M.** Aquella sesión llamó a este comando «el modelo del comportamiento correcto» y **la afirmación era cierta** —escribe lo que emite— y por eso llevó a la conclusión equivocada. Es F63 por construcción: `Storage::list_events_after` (`lib.rs:1022`) recorre todos los streams y luego `.take(limit)`, **tirando el largo en cada poll**, desde un `canonical_events()` que ya había cargado el ledger entero.

**`COUNT(*)` era 51,8× más barato y se descartó.** Medido: 0,077 ms frente a 4,015 ms sobre 591 filas. La objeción no es el precio, es la **verdad**: con `--cycle` y `--frame` habría que probar que el predicado SQL equivale al `retain` en Rust, y esa prueba no está hecha. Barato y posiblemente falso no es una mejora. La vía elegida es una materialización que el bucle **ya paga cada 500 ms**, con **el mismo predicado y el mismo cursor** que el emisor: no puede divergir por construcción. Y el filtro sale del bucle a una función libre con **dos** llamadas, porque copiar los dos `retain` sería una segunda regla.

**La auditoría por criterioredujo 5 candidatas a 2 defectos reales.** `ledger events` ya declara (F63). `cycle list` **ya declara** (`cycle.rs:2064-2069`) —responde así que **no hay ciclo pendiente para `cycle`**, y eso cierra un elemento de la lista de trabajo sin abrir nada. `telemetry status` declara `total_cycles`. **`cockpit diff-watch` quedó descartado por lectura**: trae el mismo `{"__watch_complete":true,"emitted":N}` y por eso lo trajo la forma, pero emite **filas de deriva** que aparecen por comparación de digests, y «cuántas existen» no es una pregunta bien formada. **Quinta vez que el criterio trae por forma algo que no es el mismo hecho.**

**Y queda un defecto medido de la misma clase, que no entra en este ciclo:** `ledger export --limit 5` escribe 5 eventos a un fichero y dice `exported 5 events to …`, sin mencionar los 593 que dejó fuera. Un ciclo, una concernia.

**Dos instrumentos mentían y lo admitieron — seis veces en esta sesión que una herramienta encuentra un defecto en algo que el propio agente acaba de escribir.** El falsificador **vetó** su propio guard: M5 quita el filtro del bucle mientras el recuento lo mantiene, emite los eventos de todos los ciclos declarando el total de uno, y los seis tests seguían verdes. La causa era el **fixture**: elegía el ciclo con menos eventos, y en un fixture de ciclos de un solo evento eso es el **primero** del ledger, luego una corrida sin filtrar emitía justo ese. **Guard, no producto** — segunda vez en este ciclo. Y el propio instrumental se rompió de dos maneras: restauraba con `git checkout`, que repone el **último commit** y por tanto **borró cambios sin commitear** ajenos (el arreglo del doc que estaba en curso), y sus anclas de M3 y M5 dejaron de existir cuando `cargo fmt` reindentó las llamadas — y en vez de declarar detección **se negó**, que es lo correcto.

**Verificación:** `cargo test --workspace --no-fail-fast` **5403 passed / 0 failed** en **282** binarios (baseline 5397, **+6**) · `clippy -D warnings` exit 0 · `fmt --check` limpio · scanner **CLEAN** · `03-medir-watch.py` de **3/3 GAP** a **0/3** en `ledger watch` · changelog **PASS=63 FAIL=0** · falsificador **5/5** mutaciones detectadas.

**El ciclo está en `RELEASE_PENDING`, no cerrado:** la release sigue bloqueada por la clave KMS, que es del operador. Siete eventos y seis artefactos en la autoridad.

**El ciclo existe en la autoridad, con sus ocho gates y su evidencia real:** `p-63676b11dc0ef88f/ledger-watch-total`, `exploration-sufficient` · `requirements-testable` · `architecture-consistent` · `plan-executable` · `implementation-complete` · `tests-pass` · `policy-compliant` · `debt-severity-assigned` · `debt-priority-assigned`, cada uno con `argv`, `exit_code` y `output_digest` de una corrida. El primero **corre con `exit_code: 1`** y eso está declarado en su propia evidencia, con su significado. **Ningún gate se estampó**, y dos de ellos_FOUND_ lo que pedían porque el documento estaba mal: `04-req-testable.py` falló con 6 problemas porque el **mapa objetivo → guard no existía** en el PRE-FLIGHT, y `06-plan.py` trataba «lo que este plan va a crear» como «lo que no existe», lo que hacía el gate **insatisfacible**.

**El hallazgo que no se buscaba: los ciclos de 69h, 69i y 69k no existen en SDDK.** Sus recibos declaran un `cycle_id` que la autoridad nunca emitió. Se declara y **no se corrige** retro-creándolos — eso sería escribir historia en la autoridad, que es el fallo de `INC-DEBT-061` aplicado a los recibos propios—; queda como decisión del operador. De paso se midió que **`SDDK_DATA_DIR` no manda sobre el ledger**: con esa variable puesta, `cycle list` leyó el ledger real, porque el ledger vive bajo `XDG_STATE_HOME` y `SDDK_DATA_DIR` solo rige el control-plane. Un `cycle start` sobre un almacén vacío falla por `FOREIGN KEY`.

**Lo que sigue abierto, sin adornos:**

1. **Clave KMS** — único bloqueo de 2.5.3, del operador.
2. **INC-DEBT-050**: las dos salidas. La migración está **medida como inalcanzable**.
3. **INC-DEBT-061**: los 51 ciclos de la mitad apartada.
4. **Los tres recibos con `cycle_id` inexistente**: enmendar para que declaren lo cierto, o registrar la divergencia como deuda. Se recomienda enmendar; **no se ejecuta aquí** (son documentos de ciclos cerrados).
5. **INC-DEBT-060**: 79 filas `__spine_import__` y 23 ciclos sin hecho (17 `OPEN`).
6. **INC-DEBT-049**: el operador reescribe F49 o cierra.
7. **`ledger export`**: misma clase, **medido**, ciclo siguiente.

---


**Estado (session-69k, 2026-10-03): la cuarta superficie de la misma clase, y —lo que más vale— el falsificador encontró un defecto en el remedio. Una afirmación mía llevaba dos commits viva y era falsa.** `HEAD` = `1f6072e7` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**La pregunta que los tres ciclos anteriores no se hacían:** *¿qué más declara el mismo hecho, y cada uno lo declara igual?* De **5 superficies candidatas: 1 defecto, 2 ya cerradas, 2 descartadas.** Los dos descartes salen por **homonimia** — `sddk-domain` tiene **otro** `GraphView`, y el nombre compartido es justo lo que hace que una búsqueda por nombre lo traiga. **Sexta vez en esta sesión que el número de candidatos se reduce al leerlos.**

**`VaultNode` deriva `Serialize` y tiene 8 campos; `export_node` era un `json!` a mano con 6**, que descartaba `tags` y `body` **sin declarar nada**. La asimetría es lo que lo hace defecto y no decisión: `status` viaja y tiene columna propia; `tags` no viaja y no tiene columna — mismo tipo de metadato, dos reglas, ninguna derivable. **Lo que no se afirma** es que `body` falte: eso es diseño, y casi con seguridad está bien. Lo que se afirma es más estrecho — **el artefacto no dice qué omite**, luego no se puede responder «¿esto es todo?» sin leer el código.

**El falsificador encontró un defecto en el remedio, y eso es lo que más valor tiene.** O2 afirmaba que `From<&VaultNode>` hacía que añadir un campo a `VaultNode` fuera **error de compilación**. **Es falso, y medido:** se añadió `mutant_field`, se actualizó el parser, y `cargo build` **pasó**. Un `From` entre dos tipos **distintos** no es exhaustivo por ningún lado. **El mismo doc de `GraphExport::from(&GraphView)` afirmaba lo mismo desde el ciclo anterior**: la afirmación falsa llevaba **dos commits** viva y nadie la había falsificado. Corregidos los dos, con la medición al lado.

**Y el guard que sí existe tampoco era el que se creía.** R2 tuvo dos versiones previas que no medían lo que declaraban: la primera repetía la lista de campos como **literal** — el defecto bajo prueba con otro sombrero —; la segunda construía un `VaultNode` **literal**, con lo que al mutar el error salía en el **fichero de test** y **ninguna aserción llegaba a ejecutarse**: la mutación quedaba «detectada» por el motivo equivocado, y el mensaje que habría servido no se imprimía nunca. La sonda es ahora un nodo **parseado de un fixture real**; con la mutación activa R2 nombra `mutant_field` y R1 reporta «7 de 9». **El script de medición también**: buscaba la palabra `omit`, lo que acopla el guard a una redacción. **El guard, no el producto — tres veces en este ciclo.**

**Verificación:** `cargo test --workspace --no-fail-fast` **5396 passed / 0 failed** en 281 binarios (baseline 5391, **+5**) · `clippy -D warnings` exit 0 · `fmt --check` limpio · scanner **CLEAN** · `08-medir-nodes.py` de **«DEFECTO — omite sin declarar»** a **«correcto: omite y lo declara»** · changelog **PASS=57 FAIL=0** · las **7** aserciones del test existente de `export.rs` siguen verdaderas, sin reescribir.

**Lo que sigue abierto, sin adornos:**

1. **Clave KMS** — único bloqueo de 2.5.3, del operador.
2. **INC-DEBT-050**: las dos salidas — cerrar, o una segunda autoridad de lectura. La migración está **medida como inalcanzable**.
3. **INC-DEBT-061**: los 51 ciclos de la mitad apartada.
4. **INC-DEBT-060**: 79 filas `__spine_import__` y 23 ciclos sin hecho (17 `OPEN`).
5. **INC-DEBT-049**: el operador reescribe F49 o cierra.
6. `OMITTED_NODE_FIELDS` es una lista escrita a mano que R2 contrasta con el tipo pero **no** contra sí misma: si alguien añade ahí un campo que sí viaja, R2 no lo nota. **No medido.**
7. La auditoría por criterio **no se ha aplicado a `ledger` ni a `cycle`**, que son las otras dos superficies que truncan.

---


**Estado (session-69j, 2026-10-02): se aplicó la regla del operador al único `critical` que quedaba abierto — «alerta de deuda sin verificar si sus criterios siguen vigentes no es deuda real» — y al verificlo apareció una incidencia nueva: el alias de proyecto renombra, pero en 6 de 15 casos lo que había era un reparto.** `HEAD` = `19ff78c0` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**INC-DEBT-050 remedida: el efecto observable ya no existe.** `migrate_project_identity.py audit` da **0 ids divergentes sobre 177 receipts**, con `selfcheck: OK`. El síntoma del titular —«un cuarto de las adopciones quedaron con su ledger fuera del alcance del CLI»— está resuelto. Su `open_part` de frontmatter describía el alias como «trabajo de diseño con SCOPE + ADR»: **está hecho**, ADR-0152 `accepted`, 15 aliases declarados. Corregido, y corregido lo que la medición sí establece.

**La mitad que queda del criterio de cierre está medida como inalcanzable por migración.** Que los receipts digan el id que hoy se deriva solo lo logra una migración, y la migración **no existe**: el `project_id` entra en el `content_hash` de `events_v1`; el trigger `BEFORE UPDATE` rechaza con *«events_v1 are append-only»**, y **sin** trigger la escritura pasa pero el hash **no se recalcula**, luego `verify_stream_chain` falla con `hash_drift` para siempre. Quedan dos salidas y **ambas son del operador**: dar la partida por cerrada dejando los receipts como historia verdadera de cuando se escribieron, o declarar que esos ids deben ser nombrables — en cuyo caso la respuesta es una **segunda autoridad de lectura**, no una migración.

**INC-DEBT-061, nueva: 6 de los 15 alias ocultan 51 ciclos.** La tabla `from_id -> to_id` se aplica como **redirección**, y una redirección renombra pero **no une**. En 6 casos hay **445 eventos y 51 ciclos con nombre** en el lado apartado (`h2-body-execution-engine`, `coordinator-collapse`, `q05-release-action-pins`, `ci-repair-release-closure`…). **Ningún comando los nombra**, y **un pin no los salva**: el alias se aplica **después** del pin, y el propio código dice que un pin que nombra el id retirado es «exactamente el input que el alias existe para corregir». En **5 de los 6**, el campo `reason` afirma que el histórico está en el `to_id` y el storage lo contradice: la tabla es un registro durable de decisión que induce a una conclusión falsa.

**La hipótesis que casi se convirtió en informe, y por qué no lo fue.** Los ciclos llegan a `2026-10-02T17:44:21Z`, **30 minutos después** del commit que cerró el trabajo del alias: leído así, el arreglo estaba roto y seguía escribiendo en el id retirado. No lo está. El binario instalado es **`sddk 2.2.27` del 2026-09-29**, anterior a la tabla, y no podía consultarla porque no existe en su código. **El arreglo no está roto: no está desplegado.** La medición que separa el error del acierto fue una sola: **qué binario escribió**, no cuándo. Las otras dos hipótesis —«los receipts contradicen la derivación» y «la tabla no existe»— cayeron igual: el audit da 0, y la tabla **no** es una tabla SQLite sino `~/.local/state/sddk/project-aliases.json`.

**Es la quinta vez en esta sesión que una conclusión sale de medir la superficie equivocada.** Las otras cuatro: `ledger watch` (69f), `env` fuera de ámbito (falsificador de `vault graph`), los dos gates que son `.py` y no `.sh`, y aquí una tabla de alias buscada en SQLite. **Y una sexta, de un instrumento roto:** un normalizador de ids mío colapsaba `INC-DEBT-060` a `INC` y declaró **3 entradas de índice colgantes que no existían** — con el regex correcto son **0**. La forma común es siempre la misma: **aceptar el resultado del instrumento antes de comprobar que el instrumento mide lo que uno cree**.

**Gravedad `high` y no `critical` porque no hay pérdida de datos:** las 445 filas y los 51 ciclos están íntegros, consistentes y legibles — por SQLite, que es rodear el producto entero. Es el mismo razonamiento por el que INC-DEBT-060 bajó de `critical`.

**Verificación:** `test_debt_index_coherence` **PASS=12 FAIL=0** · `test_docs_script_contamination` **PASS** · `test_gate_coverage` **PASS** (48 entradas de índice, **0** colgantes) · `test_changelog_coverage` **PASS=56 FAIL=0** · scanner **CLEAN**. **No se cambió una línea de Rust**, luego no se ejecutó la suite completa: el alcance es `docs/` y un script independiente, y `test_gate_coverage` confirma que el script nuevo no necesita runner. **Storage real intacto**: nada escribe en `project-aliases.json` ni en ningún ledger.

**Lo que sigue abierto, sin adornos:**

1. **Clave KMS** — único bloqueo de 2.5.3, del operador.
2. **INC-DEBT-050**: la decisión de las dos salidas de arriba.
3. **INC-DEBT-061**: los 51 ciclos de la mitad apartado. Remedio 1 (corregir el `reason` falso) es **aditivo y reversible, y queda escrito como recomendación sin ejecutar**: el fichero está fuera del repo y `store_alias_table` es append-only por diseño, luego reescribir un `reason` es una decisión sobre la inmutabilidad de la tabla, no un ajuste de texto. Remedio 2 es **diseño** y roza ADR-0152.
4. **INC-DEBT-060 sigue `open`**: 79 filas `__spine_import__` y 23 ciclos sin hecho (17 `OPEN`).
5. **INC-DEBT-049**: el operador reescribe F49 o cierra.
6. Auditar qué más serializa un `GraphView` o parte (`export_node`, `window.__vault_nodes__`), que siguen **sin medir**.

---

**Estado (session-69i, 2026-10-02): la réplica HTML de `vault export` era una tercera superficie del mismo defecto, y un guard mío era falso — las dos cosas las encontró medir en vez de suponer.** `HEAD` = `3f7efb99` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**El riesgo que session-69h dejó escrito como «no verificado si es intencional»: medido, y no era intencional.** `GraphExport` (`export.rs:90-95`) **no es una vista parcial de `GraphView`**: es una **estructura distinta con tres campos escrita a mano**, luego las dos listas de campos podían separarse sin que nada lo notara. Sobre el mismo vault de dos ciclos disjuntos, el JSON incrustado en la página declaraba `cyclic` y nada más: sin `cycle_count`, sin `multiple_cycles`, y con `topological_order` ausente **sin decir por qué**. El defecto de `cl-vault-graph` **intacto**, en una superficie que nadie miraba.

**Y el STOP 4 del ciclo anterior se cumplió literalmente con el defecto entero presente.** Decía «si `vault export` deja de cuadrar con `vault graph`, se para», y el HTML no afirma nada *falso* sobre el grafo: sencillamente no declara lo mismo. **Una condición que se puede cumplir con el defecto ahí no es un guard**, y queda anotado como débil en el SCOPE. Con esto el defecto estaba cerrado en **dos de tres** declaraciones, no en dos de dos — que es el número que se habría escrito si se hubiera contado por comandos en vez de por declaraciones.

**El arreglo invierte el control:** el mapeo pasa de un literal en el sitio de llamada a un `impl From<&GraphView>`, de modo que **añadir un campo a `GraphView` sin decidir qué dice la página es un error de compilación** y no una divergencia silenciosa. Un sitio de llamada no es donde mira un compilador.

**El primer R1 estaba mal, y no el producto.** Afirmaba que la clave `cycle_count` tiene que estar **presente**, y falló contra una implementación correcta: la forma saturada la **omite** cuando la respuesta es «2 o más», porque `None` *es* la codificación de «2 o más» y `multiple_cycles: true` es lo que la hace distinguible de «ausente». Exigir la clave habría reintroducido la ambigüedad que el ciclo quita. **El script de medición arrastraba la misma aserción equivocada** — primero falló el test, después salió un `GAP` residual en el script—, así que la propiedad comprobada es ahora la **forma**, no la presencia, en los dos sitios.

**Verificación:** `cargo test --workspace --no-fail-fast` **5391 passed / 0 failed** en 280 binarios (baseline 5388, **+3**) · `clippy -D warnings` exit 0 · `fmt --check` limpio · scanner **CLEAN** · `07-medir-html.py` de **3/4 en GAP a 0** · falsificador del ciclo anterior **PASS=6 FAIL=0**, sin regresión · `test_docs_script_contamination` **PASS** · `test_gate_coverage` **PASS**.

**RECONCILIATION: el §6 del recibo de `cl-vault-graph` era falso.** Afirmaba que `test_docs_script_contamination` y `test_gate_coverage` no existían. **Sí existen, y son `.py`, no `.sh`**, cableados en `release.sh:272,274`; mi comprobador usó `[ -x tests/$t.sh ]` y luego busqué un `.sh` donde hay un `.py`. Ejecutados: **ambos PASS** (41 tests, 36 con runner, 5 excepcionados, **0 huérfanos**). §6 se conserva sin tocar y la corrección va al lado, en el §9 de ese mismo recibo. **Es la tercera vez en esta sesión que una conclusión sale de medir la superficie equivocada** — la primera fue el `ledger watch` de session-69f, la segunda la variable `env` fuera de ámbito en el falsificador de `vault graph`. Las tres comparten forma: **aceptar el resultado del instrumento antes de comprobar que el instrumento es el que uno cree que es.**

**Lo que sigue abierto, sin adornos:**

1. **Clave KMS** — único bloqueo de 2.5.3, del operador.
2. **INC-DEBT-060 sigue `open`**: las 79 filas de `__spine_import__` y los 23 ciclos sin hecho (17 `OPEN`).
3. **INC-DEBT-049**: el operador reescribe F49 sobre historia ausente, o cierra.
4. **La familia no está auditada por criterio.** Este ciclo llegó a otra superficie desde un defecto concreto, no desde una pregunta tipo «¿qué más declara el mismo hecho?». Puede haber una cuarta declaración del mismo grafo, y `window.__vault_nodes__` / `export_node` **no se han auditado** contra la clase. **No medido.**
5. La página HTML **no tiene consumidor en el repo** más allá del test: «el consumidor recupera los campos» está probado **contra el artefacto**, no contra una página en funcionamiento. **No verificado en navegador.**

---

**Estado (session-69h, 2026-10-02): los dos comandos que dos sesiones dejaron como «NO MEDIDOS» están medidos. Uno es defecto real y está cerrado; el otro semidió y se descartó. Y un gate de release llevaba rojo en HEAD por un motivo que se encontró al ejecutar su propio remedio.** `HEAD` = `7f2cb04e` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**`vault graph` era un defecto real, y solo aparecía en el caso que la función no promete.** Con un vault **acíclico** de 30 nodos parece correcto: `node_count` 30 cuadra y el orden topológico sale completo, luego nada falla. Con **dos ciclos disjuntos**, `find_sample_cycle` (`graph.rs:88-94`) devuelve el **primero** y para, `GraphView` no tenía ningún campo de recuento, y el resultado daba `sample_cycle` con uno **callando sobre cuántos hay**; además el `topological_order` **desaparecía** sin decir por qué — una línea que no está se lee como «no se computó», que es otra afirmación distinta de «no existe porque el grafo es cíclico».

**La solución obvia se refutó midiendo, y esa es la parte que manda.** Contar ciclos de forma ingenua no es lento: es **incorrecto antes que lento**. Una cadena de 1000 nodos tiene **un** ciclo y el recuento ingenuo devuelve **1000** rotaciones de él (**558 ms**, en el grafo más trivial que existe), y un bouquet de 500 cuenta 1000 porque cada ciclo se recorre en las dos direcciones. Un campo que parece una verdad y no lo es es peor que ningún campo, así que la forma elegida **satura**: `cycle_count` es 0 o 1 exactos y `None` cuando hay más, con `multiple_cycles` al lado para que `None` sea distinguible de «ausente», y `topological_order_absent_because: "cyclic"`. **Todos son campos añadidos, STOP 3 vacío**: el consumidor existente (`cli.rs:8579`) no se reescribe y sus cuatro aserciones siguen verdad sin tocarlo. Es la diferencia con `ledger events` y `vault search`, donde sí hubo que reescribir por cambio de forma.

**`vault show` se midió y NO es de esta clase:** su `backlinks` no tiene cota, es el array completo. **Medir también descarta**, y por eso los dos comandos que session-69f y session-69g dejaron escritos como «no medido» están ahora **medidos** — uno es defecto y el otro no, que era exactamente la distinción que faltó con `ledger watch`.

**Dos bugs del falsificador, ninguno del producto.** F6 fallaba por el arnés dos veces, y las dos por «la razón equivocada»: `vault export` tiene un guard fail-closed (ADR-0082, `writer.rs:76`) que **canonicaliza** el directorio de datos del proyecto y **rechaza si no existe**, y el guion escribía en la raíz temporal; y el segundo leía el `project_id` con un glob sobre el árbol de **data**, que `vault graph` **no crea** — crea el de **state** —, luego la búsqueda no encontraba nada. Se corrigió el arnés. El producto no se tocó para hacerlos pasar.

**Verificación:** `cargo test --workspace --no-fail-fast` **5388 passed / 0 failed** en 279 binarios (baseline 5384, **+4** exactamente los nuevos) · `clippy -D warnings` exit 0 · `fmt --check` limpio · scanner **CLEAN** · falsificador de graph **PASS=6 FAIL=0** · `test_changelog_coverage` **PASS=54 FAIL=0**.

**Hallazgo colateral, y era un bloqueo de publicación.** `tests/test_vault_adr_mirror_coverage.sh` es gate del pipeline (`release.sh:217,234`) y **fallaba en HEAD limpio** —se comprobó con `git stash`, no se supuesto—: ADR-0151, ADR-0152 y ADR-0153, promovidos a `accepted` en estas últimas sesiones, sin reflejar. Al ejecutar el remedio que el propio gate nombra apareció que **no podía funcionar en ninguna máquina que no sea esta**: `REPO_ROOT` hardcodeado a una ruta que aquí solo funciona por ser symlink, y `Path.glob` sobre directorio inexistente devuelve iterador vacío → el guion imprimía `created: 0, skipped: 0` y salía con **0**. «Un PASS que no midió nada», en el guion que corre **cuando algo ya ha ido mal**. Reparado en `93c80e38` como concernia separada: `REPO_ROOT` derivado de `__file__`, `main` **fallando cerrado** en los tres estados vacíos, y recuento de `accepted` declarado. Falsificado: **PASS=5 FAIL=0**. El gate vuelve a PASS con 57 ADRs reflejados.

**Lo que sigue abierto, sin adornos:**

1. **Clave KMS** — único bloqueo de 2.5.3, del operador. El gate de espejo ya no se interpone, pero el gate de release sigue sin poder pasar sin ella.
2. **INC-DEBT-060 sigue `open`**: las 79 filas de `__spine_import__` y los 23 ciclos sin hecho (17 `OPEN`) son decisión del operador, y `get_cycle` sigue dando error en las 2 filas ilegibles (STOP 1).
3. **INC-DEBT-049**: el operador reescribe F49 sobre historia ausente, o cierra.
4. **La réplica HTML de `vault export`** (`export.rs:24-25,41-42`) no muestra los campos nuevos. STOP 4 pide que no se contradigan y no se contradicen, pero HTML y texto JSON dicen cosas distintas. **No verificado** si es intencional: es el siguiente candidato.
5. **Dos gates que el SCOPE de este ciclo nombra no existen** con ese nombre: `test_docs_script_contamination` y `test_gate_coverage`. Se ejecutaron `test_deny_lint_zero_hits.sh` y `test_advisory_lint_explanations.sh` por cubrir esa intención, y ambos pasan, pero **eso es una interpretación, no equivalencia demostrada**, y queda escrito así en el recibo.

---

**Estado (session-69g, 2026-10-02): `vault search` cerrado — la misma clase de defecto que F63, en la segunda superficie que quedaba.** `HEAD` = este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**El defecto, medido antes de decidir nada:** sobre el índice real de esta máquina, `sddk vault search` imprimía **20 de 75 documentos** sin declarar ningún total, con **exit 0**; `--limit 0` devolvía `no hits`; y el JSON era un array desnudo.

**Aquí el total NO viene gratis, y por eso el diseño es distinto al de F63.** En `ledger events` venía en memoria porque el corte es posterior al `take`; aquí `search_index` corta en SQL (`LIMIT ?2`), luego declararlo cuesta una segunda consulta. Se midió antes de decidir, y la alternativa barata se descartó: el `COUNT` es **más barato que la propia búsqueda** (0,119 ms contra 0,376 ms con 75 docs) y el sobrecoste **baja con la escala**, de +31,7 % a **+5,2 %** con 7500 documentos, porque `ORDER BY rank LIMIT 20` ordena todos los matchs mientras que `COUNT … WHERE MATCH` solo los recorre. Por eso el total **exacto**: el dato completo sale más barato que el parcial.

**`search_index` no cambia de firma.** Es API pública de `sddk-vault` con **8 tests unitarios** que la usan; el camino corto los rompía a todos. El total se obtiene con `count_matches`, función nueva.

**`--limit 0` era la cuarta contradicción de la convención del cero en este binario.** `ledger export --limit 0`, `ledger events --limit 0` y `ledger watch --max-events 0` significan todos; aquí `LIMIT 0` en SQL no es caso especial y devuelve cero filas.

**Verificación:** `cargo test --workspace --no-fail-fast` **5384 passed / 0 failed** en 278 binarios (baseline 5378, **+6** exactamente los nuevos) · `clippy -D warnings` exit 0 · `fmt --check` limpio · falsificador O1–O4 **PASS=9 FAIL=0** con `declared=61` contrastado contra `sql=61` y el sha256 del índice real idéntico antes y después · `test_changelog_coverage` **PASS=51 FAIL=0**.

**Un fallo del arnés que valió más que el arreglo:** el fixture de R1 no casaba —FTS5 hace coincidencia de token exacto sin stemming, así que `crypto` no encuentra `cryptography`— y eso dejaba **R6 vacío**: su rama «con coincidencias» estaba ejercitando en realidad la de «sin coincidencias» y pasaba igual. Un test que pasa porque prueba otra cosa es un test que no mide, y no se vio hasta que R1 lo destapó.

**Lo que sigue abierto, sin adornos:**

1. **Clave KMS** — único bloqueo de 2.5.3, del operador.
2. **`vault graph` y `vault show`: NO MEDIDOS.** También proyectan datos y podrían tener la misma clase. El SCOPE lo dice como «no medido», **no** como «correcto» — esa distinción es la que faltó con `ledger watch` y repetirla sería repetir el error.
3. **INC-DEBT-060 sigue `open`**: las 79 filas de `__spine_import__` y los 23 ciclos sin hecho (17 `OPEN`) son decisión del operador, y `get_cycle` sigue dando error en las 2 filas ilegibles (STOP 1).
4. **INC-DEBT-049**: el operador reescribe F49 sobre historia ausente, o cierra.

---

**Estado (session-69f, 2026-10-02): auditoría de familia del defecto de F63 — once candidatos, uno solo. Y una afirmación mía que era falsa, corregida en los cuatro sitios donde estaba.** `HEAD` = `a61948a2` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**La afirmación que era falsa:** se venía diciendo que `ledger watch --max-events` «trunca sin declarar lo mismo». **No es así.** Medido: `ledger watch` **declara** en los dos formatos —`[watch] emitted 5 events, exiting` en texto y `{"__watch_complete":true,"emitted":5}` en JSON (`ledger.rs:785-788`)—, y su `--max-events` está documentado como `0 = unlimited`. **Es el modelo del comportamiento correcto**, y el propio arreglo de F63 lo copia. Salió de **analogía de nombre**: tres comandos con bandera de tope, un mismo tratamiento y **ninguno ejecutado**. Es el camino que produce los demás números falsos de esta sesión —tratar la forma como si fuera el comportamiento—.

**De los 11 candidatos de la auditoría, 10 se eliminaron por lectura:** defaults de escritura, cota de profundidad con `0 = unbounded` ya documentado, tope de bytes sobre un subproceso, `.take()` interno sin superficie de usuario, y nueve ficheros que ya declaran. El único superviviente fue `vault search`, cerrado en session-69g. La reducción va escrita con su tabla porque «11 defectos» es el número que viaja a un documento y se convierte en trabajo que nadie necesitaba.

**El límite del método, declarado:** la auditoría es estática; diez candidatos se eliminaron por lectura y no por ejecución, y un criterio de búsqueda puede tener falsos negativos.

---

**Estado (session-69e, 2026-10-02): F63 cerrado. `sddk ledger events` ya no trunca en silencio y `--limit 0` ya no significa cero.** Con esto, **los cuatro falsificadores de INC-DEBT-060 (F60–F63) están entregados**; la deuda **sigue `open`** y la razón está escrita. `HEAD` = `2fd82cf7` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**El defecto medido otra vez, no heredado:** sobre el ledger real de `p-63676b11dc0ef88f` en copia byte-idéntica —590 eventos, 114 streams—, `sddk ledger events` imprimía **50**, nombraba **19 de los 114 ciclos**, no declaraba ningún total y salía con **exit 0**. La ventana por defecto eran las secuencias **12 a 20**: las 50 más recientes, con las 540 anteriores invisibles y nada en pantalla que lo dijera.

**El total ya estaba en mano y se tiraba, y por eso el lote no toca storage.** `Storage::list_events` (`lib.rs:974`) llama a `canonical_events`, que recorre **todos** los streams con `u32::MAX` y devuelve el vector entero; el truncamiento pasaba después, en memoria, con `.take(args.limit)`. `total_events` es `all.len()` **antes** del `take`: ninguna consulta nueva, ninguna API de storage, ninguna migración. STOP 1 no saltó porque no hizo falta.

**Y `--limit 0` significaba CERO**, con lista vacía y exit 0, mientras `ledger export --limit 0` significa todos (`ledger.rs:442-445`) y `ledger watch --max-events 0` también. La convención invertida a un comando de distancia, en el mismo binario. No estaba en el enunciado de F63.

**Cambio de forma del JSON, declarado.** El payload pasa de array a `{events, total_events, shown, truncated}`: no es un campo que faltara añadir, es que **un array no tiene dónde llevar un total**. Y **la medición de impacto previa fue falsa por partida doble**: son **dos** consumidores en el repo y no uno, porque se buscó con un grep sobre una lista de ficheros elegida a mano en vez de sobre el árbol, y el segundo salió **por el perfil completo del workspace**, es decir después de romper el build; y `skills/` ni se miró, siendo superficie del bundle distribuido. Examinado y **no es rotura** —`sddk-cycle-resume/SKILL.md:62` no parsea el array, pide al agente que reconstruya la cadena leyéndola, y una envoltura que dice «10 de 590» es más informativa—, pero pudo no serlo. Quinta vez en este ciclo que medir con el instrumento equivocado produce un número falso, y la quinta vez iba a un documento.

**Verificación:** `cargo test --workspace --no-fail-fast` **5378 passed / 0 failed** en 277 binarios (baseline 5374, **+4** exactamente los nuevos, ninguno verde preexistente perdido) · `cargo clippy --workspace --all-targets -- -D warnings` exit 0 · `cargo fmt --check` limpio · falsificador O1–O4 **PASS=7 FAIL=0** con `declared=590` contrastado contra `sql=590` y el sha256 del ledger real idéntico antes y después · `test_changelog_coverage` **PASS=49 FAIL=0** · `git diff --check` limpio.

**Lo que sigue abierto, sin adornos:**

1. **Clave KMS** — único bloqueo de 2.5.3, del operador.
2. **INC-DEBT-060 sigue `open`** aunque sus cuatro falsificadores estén verdes: las 79 filas de `__spine_import__` y los 23 ciclos sin hecho (17 `OPEN`) son decisión suya, y `get_cycle` sigue dando error en las 2 filas ilegibles (STOP 1).
3. **`sddk vault search` repite la misma clase de defecto, medido:** **20 de 75 documentos** sin declarar nada, **`--limit 0` devuelve `no hits`** en vez de todos, y el JSON es un array desnudo. Slice propio con SCOPE propio. **Ojo:** se señaló antes `ledger watch --max-events` como el candidato y **era falso** —medido, `watch` sí declara en texto y JSON y es el modelo del comportamiento correcto—. La afirmación salió de analogía de nombre, sin ejecutar el comando.
4. **INC-DEBT-049**: el operador reescribe F49 sobre historia ausente, o cierra.

---

> ## ✅ EL ÁRBOL ESTÁ VERDE
>
> El aviso anterior de este fichero —«el árbol está ROJO a propósito, tres tests
> RED por `cycle list` inexistente»— **queda derogado**. El lote 2 aterrizó y el
> perfil completo pasa: **5374 tests, 0 fallos**, clippy `-D warnings` exit 0,
> `cargo fmt --check` limpio. **2.5.3 vuelve a ser publicable por el lado del
> árbol**; su único bloqueo sigue siendo la clave KMS. Se conserva el aviso viejo
> más abajo, en su sitio histórico, sin editarlo.

**Estado (session-69d, 2026-10-02): entregado el lote 2 de `cl-cycle-enumeration` — `Storage::list_cycles` y `sddk cycle list` — y CORREGIDA la magnitud de INC-DEBT-060, que era casi 4× mayor de lo declarado.** `HEAD` = `190bbd52` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**Las cifras publicadas estaban mal: 179 filas en `cycles` no son 179 ciclos de este proyecto.** La tabla contiene **dos poblaciones**: **100** con `project_id = p-63676b11dc0ef88f` (ids `<project_id>/<slug>`) y **79** con `project_id = '__spine_import__'` (slugs desnudos, `manifest_json = {}` las 79, **cero eventos** cada una). `__spine_import__` **no es un marcador**: es una fila real de la tabla `projects` del mismo ledger (`display_name: "Spine Import Project"`, workspace `spine-import`). Cifras correctas: de los 100, **23 no los nombraba ninguna superficie** y **17 de esos son `OPEN`**; y solo **2** tienen manifiesto ilegible, no 81. La **magnitud** era casi 4× mayor de la declarada; la **clase** de defecto no cambia, y por eso INC-DEBT-060 sigue `open` y `high`.

**Lo encontró el falsificador del propio remedio, y falló por su cuenta, no por la del producto.** R6 devolvió `declared=100 table=179` contra un enumerador **correcto**: su baseline contaba `SELECT COUNT(*) FROM cycles` **sin filtro de proyecto**. Tercera vez en este ciclo que un FAIL es un guard mal escrito. El camino fácil habría sido cambiar `list_cycles` para enumerar las 179 —habría hecho pasar el falsificador y habría sido un defecto—, así que **la causa se midió antes de reparar** porque las dos reparaciones son opuestas. Falsificador corregido, más `F5` para que el guard no vuelva a derivar solo: **PASS=9 FAIL=0**.

**El remedio lee `cycles`, no `events_v1`, y esa elección *es* el remedio:** una enumeración sobre el log de hechos reproduciría el defecto que pretende arreglar, porque los 23 no tienen hechos **por eso** son invisibles. La fila ilegible se lista **marcada** (`manifest_readable: false`) y no se tira: tirarla cambiaría «invisible» por «omitido en silencio» y el recuento dejaría de cuadrar sin explicación. **STOP 1 respetado**: `get_cycle` no se toca y sus 3 tests de caracterización siguen verdes sin reescribir.

**Dos cosas que pasaron y conviene no volver a hacer.** `cli_golden` **cayó** con la suite completa —el subcomando nuevo cambiaba el help— y su fixture se regeneró con el delta **revisado línea a línea: una sola línea añadida**. Y el primer comando de la sesión fue `cargo test --workspace | tail -60`, cuyo exit code es el de `tail` y **no el de cargo**: parecía verde con la suite roja debajo. Se repitió con `> log 2>&1; echo EXIT=$?`, que no puede mentir.

**Gates:** `cargo test --workspace --no-fail-fast` **5374 passed / 0 failed** en 276 binarios · `cargo clippy --workspace --all-targets -- -D warnings` exit 0 · `cargo fmt --check` limpio · falsificador R6 **PASS=9 FAIL=0** con sha256 del ledger real idéntico antes y después · `test_changelog_coverage` **PASS=47 FAIL=0** · `check_debt_index_coherence`, `test_docs_script_contamination`, `test_gate_coverage`, `test_adr_promotion_format` PASS · `test_release_state_pointer` reconciliado en este commit · `git diff --check` limpio · push sin `--no-verify`.

**Lo que sigue abierto, sin adornos:**

1. **Clave KMS**: único bloqueo de v2.5.3, del operador. Ya no está acompañado del árbol rojo.
2. **Las 79 filas de `__spine_import__`**: qué son, y si son alcanzables desde algún checkout, es una pregunta **sin medir**. Se declara sin medir. El lote solo estableció que **no son ciclos de este proyecto**.
3. **Los 23 ciclos sin hecho** (17 `OPEN`): qué hacer con ellos sigue siendo decisión del operador. §2.2 del SCOPE prohíbe limpiarlos o migrarlos.
4. **F63**: `sddk ledger events` trunca en 50 de 590 sin declararlo. Slice propio, excluido del lote a propósito.
5. **INC-DEBT-049**: el operador reescribe F49 sobre historia ausente, o cierra.

---

> ## ⚠ HISTÓRICO — EL ÁRBOL ESTABA ROJO A PROPÓSITO (session-69c 3ª parte)
>
> **Este aviso estaba vigente hasta el lote 2 y ya no lo está.** Se conserva sin
> editar como evidencia de lo que era cierto cuando se escribió.
>
> `HEAD` tenía **tres tests RED** en `crates/sddk-cli/tests/cycle_list_e2e.rs`
> porque `sddk cycle list` todavía no existía. Era el **lote 1** del ciclo
> `p-63676b11dc0ef88f/cycle-enumeration`, escrito para caer *antes* de tocar
> producción. **No era una regresión.** **2.5.3 no era publicable hasta el lote 2**,
> porque el gate de release es `cargo test --workspace` con cero fallos.

**Estado (session-69c 3ª parte, 2026-10-02): abierto el ciclo del remedio de INC-DEBT-060 y entregado el lote 1. Al mapear la superficie apareció un segundo defecto — D2 — más grave que el que abría la incidencia.** `HEAD` = `9bac0845` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**D2: 81 de los 179 ciclos tienen un `manifest_json` que no deserializa, así que `get_cycle` les devuelve error y no registro.** No es que no se puedan nombrar: es que **no se pueden leer**. `CycleManifest` exige once campos sin `#[serde(default)]`, luego un `{}` falla con *missing field `schema_version`* y `json_from_sql_error` lo propaga como `FromSqlConversionFailure`. 79 de los 81 tienen `{}`; los otros 2 son notas de cierre `{reason, notes}` de ciclos docs-only ya `CLOSED`. **[Magnitud corregida en session-69d: solo 2 de los 100 ciclos de este proyecto son ilegibles; las 79 con `{}` pertenecen a `__spine_import__`, que es otro proyecto.]**

**El lote 1 fija ese comportamiento por escrito antes de que la enumeración lo esquive.** Tres tests de caracterización que **pasan** (3/3), con una precondición que evita el paso en vacío: un manifiesto completo, construido con `CycleManifest::new` en vez de JSON a mano, **se lee bien** por la misma llamada. Sin ella, un FAIL probaría que `get_cycle` está roto para *todos* los ciclos, que es otro defecto. Fijarlo antes es lo que convierte un cambio posterior en decisión y no en deriva — es STOP 1 del SCOPE.

**Los tres RED son de CLI a propósito.** Un test que llama a `list_cycles` no compila, y un RED comprado rompiendo el build tumba el crate entero; por eso R3 queda para el lote 2. Caen con `unrecognized subcommand 'list'` y el andamiaje de `adopt apply` + `cycle start` **funcionando**: caer por el fixture habría sido un FAIL que no media nada. Uno de ellos exige que dos ciclos coexistentes sean **nombrados los dos** sin convertirse en el error de ambigüedad de la resolución por lease — que es exactamente por lo que 91 ciclos `OPEN` son invisibles hoy—, y otro exige que el recuento se **declare** y cuadre, porque el modo de fallo que este ciclo existe para evitar es el de `ledger events` devolviendo 50 de 590 sin decirlo.

**Corregí un error de mi propio contrato antes de escribir un solo test:** el SCOPE etiquetaba R4 como RED, y un test que afirma el comportamiento actual **pasa hoy**. Es de caracterización, y llamarlo RED era falso. La enmienda quedó escrita en el SCOPE, no corregida de memoria.

**Gates:** `cargo fmt --check` limpio · `clippy -p sddk-storage --tests -D warnings` exit 0 · `test_changelog_coverage` **PASS=46 FAIL=0** tras declarar el lote · `git diff --check` limpio · push sin `--no-verify`.

**Lo que sigue abierto, sin adornos:**

1. **Lote 2** del ciclo: `list_cycles` + `sddk cycle list`, y R3. Es lo que devuelve el árbol a verde.
2. **81 filas ilegibles**: qué son es **decisión del operador**. No se limpian ni se migran (§2.2 del SCOPE).
3. **F63**: `sddk ledger events` trunca en 50 de 590 sin declararlo. Slice propio, fuera de este lote a propósito.
4. **INC-DEBT-049**: el operador reescribe F49 sobre historia ausente, o cierra.
5. **Clave KMS**: único bloqueo de v2.5.3, y ahora además el árbol rojo.

---

**Estado (session-69c 2ª parte, 2026-10-02): dos hallazgos nuevos medidos y falsificados. INC-DEBT-060 abierta — la autoridad no puede enumerar sus propios ciclos. Y la premisa de la parte abierta de INC-DEBT-049 resulta FALSA, medida.** `HEAD` = `0f9613cd` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**INC-DEBT-060 — 97 de 179 ciclos no los nombra ningún comando.** No salió de leer el roadmap: salió de medir la premisa de INC-DEBT-049 y chocar con que `cycle status` decía `no active cycle found` mientras la tabla tenía 107 ciclos no terminales. La primera hipótesis —«es un falso negativo»— era **falsa**: el código infiere el ciclo actual por **lease vivo** y los 30 leases estaban caducados, así que su respuesta era correcta para la pregunta que hace. La segunda, ya leída la implementación, sí: el hueco es de **enumeración**. No hay `list_cycles` en el storage ni `cycle list` en la CLI, solo `get_cycle(id)` — hay que **saber** el id y nada lo da. Y `sddk ledger events` trunca en **50 de 590** eventos sin decirlo, así que sin `--limit` solo ve 19 ciclos de los 82 alcanzables.

No es legado anterior al fact log: `events_v1` arranca el 2026-08-31 y estos ciclos son del 2026-09-07 en adelante. La tabla muestra **dos vías de escritura**: 100 filas RFC3339 —82 con evento y **18 sin él**— y 79 con timestamp de espacio y `manifest_json` vacío, **ninguna** con evento. Por AGENTS.md §2.7 son **Object sin Fact**: una proyección que no es reconstruible desde los hechos.

**La premisa abierta de INC-DEBT-049 era falsa, y eso cambia qué hay que implementarse.** El id hermano `p-995939af668a53d8` tiene **0 eventos y 0 ciclos**: no hay historia fuera de vista. Medido en cuatro sandboxes con el store real copiado (**PASS=12 FAIL=0**): el alias se aplica a **las dos** ramas —la derivada del remoto y la fijada por el pin—, pin y alias son **salvaguardas redundantes** y cualquiera basta, y perder **las dos** degrada en **silencio** porque `load_alias_table` trata un fichero ausente como tabla vacía y no como error. En este repo el síntoma sería un `complete` sobre un storage vacío: la enfermedad que ADR-0152 vino a cerrar, reintroducida por perder el fichero en vez de por cambiar el normalizador.

Por eso **no** se implementa la advertencia que pide F49: hoy dispararía sobre una carcasa sin contenido, que es justo el ruido que su guard hermano F52 existe para impedir. **F49 y F52 se contradicen bajo el estado actual**, y eso es defecto de los falsificadores, no del producto. Lo propuesto es reescribir F49 para triangular sobre *historia ausente* y no sobre *id hermano presente*. INC-DEBT-049 sigue `open`: su cierre formal es del operador.

**Cinco FAIL fueron míos en esta sesión y ninguno era del producto**, y todos salen del mismo sitio: medir con el site incorrecto y creer el número. Dos por el límite por defecto de `ledger events`, uno por exigir un reparto de estados copiado de un cálculo anterior, uno por poner el pin en el id retirado donde el repo real lo tiene en el canónico, y uno por copiar el store a `$XDG_STATE_HOME` sin el segmento `sddk/`, que lo volvía tabla vacía y hacía parecer que el alias no se aplicaba. Todos quedan escritos.

**Gates:** falsificadores **PASS=7 FAIL=0 SKIP=1** (INC-DEBT-060) y **PASS=12 FAIL=0** (INC-DEBT-049) · `check_debt_index_coherence` PASS con 45 entradas · `test_docs_script_contamination`, `test_gate_coverage`, `test_adr_promotion_format` (`accepted ADRs: 57`) PASS · `git diff --check` limpio · push sin `--no-verify`.

**Lo que sigue abierto, sin adornos:**

1. **INC-DEBT-060** necesita `list_cycles` + `sddk cycle list`, y **F63** sobre el truncamiento silencioso. Y una decisión del operador sobre qué son las 97 filas sin hecho: si son Objects legítimos necesitan fuente de verdad declarada; si son residuo de una importación, limpiarlas es destructivo.
2. **INC-DEBT-049** sigue `open` con la parte abierta vaciada de objeto, a la espera de que el operador reescriba F49 o cierre.
3. **La ruta forge de `release apply` contra un GitHub real** sigue sin medir.
4. **La clave KMS** sigue sin aprovisionar, único bloqueo de v2.5.3.

---

**Estado (session-69c, 2026-10-02): ADR-0152 PROMOVIDO A `accepted`. Los seis criterios están medidos; cinco en verde y falsificados, y el cuarto reescrito porque su redacción anterior no tenía falsificador ejecutable. El huérfano de `skillgraph` está cerrado y el storage real da 0 ids divergentes sobre 161 receipts.** `HEAD` = `7702b3bf` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**El criterio 5 estaba ROJO y se cerró declarando el último alias, no retirando el recibo.** La diferencia no es de estilo: declarar el alias afirma que ese checkout es ese proyecto siempre; retirar el recibo **destruye el registro de una adopción que ocurrió de verdad**. Así que la causa se midió antes de reparar, y resultó que los dos recibos de `skillgraph` declaran **el mismo remoto y la misma ruta canónica** y difieren solo en `project_id`: mismo proyecto adoptado dos veces, con cinco días y dos runtimes de distancia (1.171.2 el 26-sep, 2.5.3 el 1-oct) porque el normalizador de remote cambió. Eso es INC-DEBT-050 literalmente. El recibo no era espurio.

**El bloqueo que impedía tocarlo había caducado, y hubo que comprobarlo en vez de recordarlo.** El alias se retiró porque había una sesión concurrente sobre `wi-72-p3-expansion-apply`. Al mirarla: ciclo `CLOSED`, **cero** leases, último evento de más de 2 h 45 min. Ese es el dato que convirtió una decisión cerrada en abierta, y por eso se midió. El operador autorizó declarar el alias; el store pasó de 14 a 15 entradas y el audit de **0** huérfanos sobre 161 receipts. La segunda cláusula del criterio —conservación de filas— también: **4.359** filas en el baseline de las 09:49 frente a **4.714** ahora, cuatro tablas creciendo y ninguna decreciendo.

**El hallazgo del lote es que el criterio 4 no tenía falsificador.** Decía «intentar borrar uno falla», que suena a un rechazo comprobable y no lo es: `remove_alias`, `unalias` y `delete_alias` dan **cero** coincidencias — no hay nada que intentar. Declararlo PASS habría sido la forma más barata de mentir del documento, así que **se reescribió** a lo que sí es cierto y falsificable (la superficie de la CLI es append-only, y un auto-alias se rechaza) y se dejó escrito el límite que la redacción anterior escondía: **el invariante es de la herramienta, no del almacenamiento**. `project-aliases.json` es JSON plano; quien lo edite a mano cambia la tabla sin que nada lo note. Ya ocurrió una vez en esta máquina, y fue exactamente lo que dejó el criterio 5 en rojo.

**El criterio 6 se midió sobre una copia, no sobre el storage real, y eso no es un detalle de forma.** `RuntimeContext::open` **no** abre en solo lectura: su tercer parámetro es `generate_seed`, y dentro hace `Storage::open`, que abre en escritura. Una medición que promete no tocar el storage real no puede correr contra él. Resultado sobre la copia byte-idéntica: 590 eventos, 114 streams, salida 0, vía `Storage::verify_ledger`, que corre `verify_stream_chain` **y** `verify_chain_integrity` sobre todos los streams canónicos. Tres cosas que el enunciado no dice y la medición sí: `events_v1` lleva triggers que abortan `UPDATE` y `DELETE`, luego ese criterio **solo puede caer por corrupción, nunca por una escritura legítima**; las 590 filas tienen `chain_hash` no vacío, luego `verify_chain_integrity` no se salta ninguna fila heredada y la verificación es **total**; y los dos verificadores distinguen la **misma** corrupción por diagnóstico, así que la medición sí distingue el que el criterio nombra de su vecino.

**Gates de session-69c:** falsificador de C5 **PASS=20 FAIL=0** · de C1/C2/C4 **PASS=14 FAIL=0 SKIP=1** · de C6 **PASS=12 FAIL=0** · `test_adr_promotion_format` PASS con `accepted ADRs: 56 → 57` · `check_debt_index_coherence`, `test_docs_script_contamination`, `test_gate_coverage`, `test_release_state_pointer` PASS · `git diff --check` limpio · tres push sin `--no-verify`.

**Dos FAIL fueron míos y ninguno era del producto.** En C5 contaba como pérdida de filas a seis proyectos que el propio baseline marca `{"missing": true}` — ya no tenían ledger cuando se tomó a las 09:49—, y un FAIL falso sobre datos que están bien es peor que ningún FAIL. En C6 esperaba que `ledger verify-chain` no viera el rehash de `content_hash`, y sí lo ve, porque la cadena se compromete con el hash de contenido. Los dos quedan escritos en sus recibos: el camino que lleva a ellos es información.

**Lo que este trabajo NO cierra, escrito para que no se lea como verde:**

1. **INC-DEBT-050 y INC-DEBT-049 siguen `open`.** El frontmatter del ADR pasó de `closes: [INC-DEBT-050, INC-DEBT-049]` a `closes: []` con las dos en `addresses:`. Sus criterios de cierre están escritos en esos documentos, no en el ADR, y cerrarlos es gobernanza que nadie pidió.
2. **El alias protege la lectura por la CLI, no el fichero.** Un artefacto que no pase por el resolver canónico —un script suelto, una consulta directa al recibo— seguirá viendo el id viejo. El golden pin de `stable_project_id` es el que evita que el problema reaparezca; el alias solo lo racciona.
3. **El contrato de read-option de INC-DEBT-049 sigue pendiente**, y el alias no lo toca: que `adopt status` y `cycle status` declaren la existencia de historial bajo otra identidad con el mismo `vault_path` es un problema de **declarar**, no de resolver.
4. **La ruta forge de `release apply` contra un GitHub real** sigue sin medir. Pendiente propio, declarado.
5. **La clave KMS sigue sin aprovisionar**, que es el único bloqueo de v2.5.3.

---

# CURRENT — puntero de reanudación de SDDK

**Estado (session-69b, 2026-10-02): INC-DEBT-059 RESUELTA en dos lotes. La identidad se resuelve una vez y entra ya resuelta en el engine. El criterio 3 de ADR-0152 pasa de ROJO a medido — y el ADR sigue sin promoverse, por el 5 y el 6.** `HEAD` = `ff0bacda` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**El arreglo, y por que es eliminacion y no un campo opcional:**

`AdoptionPlanInput` deja de llevar `remote_url`, `pinned_project_id`, `scope` y `fallback_seed`, y lleva `identity: ResolvedProjectIdentity`. `plan_adoption` deja de llamar a `resolve_project_identity` **por completo**. Con la identidad ya resuelta, derivar por dentro es **imposible** porque el input ya no tiene de que derivar: la afirmacion de «un solo punto de decision» queda cierta **por construccion**. Con un campo opcional, alguien readane la llamada y ningun test de comportamiento lo nota.

La forma corta —pasar el id resuelto por `pinned_project_id`— no exige tocar nada y **no funciona**: el engine lo trataria como pin, `identity_source` no viajaria y `alias_origin` se perderia un nivel mas adentro, con una forma que *parece* correcta. Y `context_cmd.rs` ya hacia exactamente eso, condicionado a `identity_source == Pinned`, que es por lo que un checkout con alias y **sin** pin era el caso que se rompia. Esa condicion se **elimina**, no se propaga.

**Dos lotes, y el primero no era opcional.** El lote 1 son tests que caen **antes** de tocar produccion (`07c3fd5c`), que es lo que permite que el lote 2 sea «hacerlos verdes» y no «comprobar a posteriori si algo se movio». Los cuatro caian, pero **tres por el motivo equivocado** —faltaba `--scope`, y luego el helper exigia exito y esas dos superficies salen con codigo no cero—, asi que median el andamiaje en vez de la propiedad. Segunda vez en la sesion que un FAIL propio tapa el defecto.

**La cuarta superficie la encontro el falsificador, no la lectura.** `adopt`, `context bootstrap` (que tenia **dos** sitios de resolucion) y el engine eran las tres conocidas. `sddk generate docs` escribia la documentacion generada bajo el data dir del id **retirado** mientras `project resolve` nominaba el superviviente. No salio leyendo el SCOPE ni midiendo el arranque: salio **contando puntos de llamada**, con las otras tres ya arregladas. Es la cuarta afirmacion de convergencia que este trabajo producia y era falsa — y por eso el guard nuevo cuenta llamadas con parentesis y exige **exactamente una**: prohibir el nombre dejaria fuera la llamada legitima del resolver canonico, y contarlo sin parentesis contaba tambien la linea `use` (que es como el guard dio 2 en vez de 1 en su primera version).

**Un arreglo demasiado amplio, cazado por un test que ya existia.** El defecto de paso —`find_persisted_fallback_seed` no veia recibos **pinneados**, porque el pin sobrescribe `identity_source`— se corrigio primero derivando la semilla de la ruta canonica, copiando lo que hace `resolve_project_ids`. Eso convierte cualquier directorio en un proyecto y deja muerto el fallback in-repo; lo cazo `real_cli_exit_status_tracks_lint_errors_and_stale_checks` con `SDDK009`. El arreglo correcto era **una clausula en el predicado**: el pin sobrescribe el `project_id`, no la semilla.

**Gates de session-69b:** `cargo test --workspace` **5361 passed, 0 failed**, `cargo exit=0` · costura **6/6** · `cargo fmt --check` limpio · `clippy -D warnings` **exit 0** · falsificador **PASS=4 FAIL=0 SKIP=0**, con las tres mutaciones detectadas: borrar la declaracion (6→5, el que el criterio 3 exige), el engine vuelve a derivar (6→1), la CLI introduce un segundo resolutor (6→4) · `check_debt_index_coherence`, `test_docs_script_contamination`, `test_gate_coverage`, `test_release_state_pointer`, `test_adr_promotion_format` PASS · `test_changelog_coverage` **PASS=45 FAIL=0**.

**Lo que este trabajo NO cierra, escrito para que no se lea como verde:**

1. **El storage real no se limpia.** El arreglo impide crear mas huerfanos; los que ya existen **siguen ahi**, y los bindings que `context bootstrap` dejo atrapados bajo ids retirados **siguen atrapados**.
2. **El criterio 5 de ADR-0152 no se puede cerrar aqui.** Exige 0 huerfanos en el audit del storage real, y depende de (1). Que receipts espurios se retiran es **decision del operador**.
3. **ADR-0152 no se promueve.** Su criterio 3 esta medido; el **5** y el **6** nunca se han medido, y seis criterios no se suman.
4. **La ruta forge de `release apply` contra un GitHub real** sigue sin medir. Pendiente propio, declarado.

**Nota sobre el puntero:** `current_sha` nombra `ff0bacda`, el commit **anterior** a este fichero documental, por la razon que la tercera sesion ya dejo escrita: actualizar el puntero convierte a este commit en HEAD, y un commit documental no es evidencia del SHA que dice contener.

---

**Estado (session-69, 2026-10-02): `adopt` no llega al store de alias, y `adopt apply` recrea el huérfano que ADR-0152 existe para cerrar. Criterio 3 ROJO, el ADR no se promueve, INC-DEBT-059 abierta.** `HEAD` = `1d613bbf` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**Lo que se descubrió, en tres líneas:**

1. **El store de alias se resuelve en un sitio, y `adopt` no es ese sitio.** Hay tres puntos de llamada de `resolve_identity_honoring_pin*` y `prepare_adoption_plan` (`lib.rs:2133`) no está entre ellos: llama a `plan_adoption`, que llama a `resolve_project_identity` **directamente** (`adoption.rs:203`). Es el único camino de identidad del CLI que no consulta la tabla. El pin sí se respeta, pero por un mecanismo **paralelo** dentro del engine — hay **dos** resolutores, que es lo contrario de lo que ADR-0152 autorizó al fijar un punto único de decisión.
2. **Medido, no inferido.** El mismo checkout, sin pin, en el mismo instante: `project resolve` da el `to` y declara `from -> to` (exit 0); `adopt status` da el `from`, `status: absent` y **mira un ledger que no existe** (exit 1). Dos comandos, dos respuestas sobre el mismo estado, sin aviso. Y `adopt apply` sobre ese checkout **escribe un segundo recibo bajo el id retirado**, sin avisar — el comando sale `complete`. Con los 14 aliases del storage real, la condición es alcanzable en 14 proyectos.
3. **El arreglo evidente no cerraba nada, y se descartó sin commitear.** Añadir `alias_origin` a `AdoptionStatus` **compila**, pero `plan.identity.alias_origin()` es `None` **siempre**: el campo serializaría `none` en el 100% de los casos. Un campo que declara un alias que nunca ocurre hace el criterio *parecer* satisfecho a quien lea la estructura.

**Por qué ningún test lo cazaba:** `grep -c alias` da **0** en `adoption_contract.rs` y en `project_pin_e2e.rs`. Los tests viven a ambos lados de la costura y ninguno la cruza — la **misma** forma que la mutación `resolve_bypasses_the_wiring` del lote 3, que escapó por idéntica razón. Y el doc de `ProjectPin` (`lib.rs:1699-1703`) afirma «All resolvers now go through [`resolve_identity_honoring_pin`]. INC-DEBT-049»: frase **falsa**, escrita por el mismo doc que se acusa a sí mismo de haber sido una afirmación falsa, y nombrando `adopt status` como uno de los tres ofensores originales.

**Un defecto propio de session-68, encontrado al correr un guard que nunca se había corrido:** `tests/test_adr_0153_criteria.sh` existía y **ningún runner lo ejecutaba** — `test_gate_coverage.py` llevaba rojo. Y no era ejecutable (`-rw-r--r--`), con lo que añadirlo a la lista sin el `chmod` lo habría convertido en un **skip silencioso**: cableado en apariencia, ejecutado nunca.

**Contaminación propia, registrada porque es el dato que INC-DEBT-058 necesita:** al escribir la INC nueva cometí **doce** contaminaciones en dos ficheros. **Cinco** las cazó el barrido de regexes —un fragmento CJK donde iba una palabra (no se reproduce aquí, porque citarlo contaminaría este fichero y obligaría a meter `CURRENT.md` en la allowlist de INC-DEBT-057, que es un coste que una frase de este tipo no compensa), más `se.crossó`, `seorga`, `estaINC` y `ADR-0152ymmó`— y **siete no las cazó ningún barrido**: `La motivation`, `El mechanism`, `sin warning`, `Passar`, `call sites`, `se Ingramó`, `la mecanismo`. Son palabras inglesas sueltas o un género equivocado, no un token pegado dentro de una palabra — la clase que INC-DEBT-058 declara **no automatizable con una expresión regular**. Solo la lectura completa del fichero las limpió. Es la segunda vez en esta sesión que el guard que debía cazar la contaminación resulta incapaz y el trabajo lo hace la lectura.

**Gates de session-69:** `test_adr_0153_criteria` **PASS=7 FAIL=0**, ahora cableado a `release.sh` · `test_gate_coverage` `con runner: 36 · SIN runner y SIN motivo: 0` · `check_debt_index_coherence` PASS, 44 entradas · `test_docs_script_contamination` PASS · `test_adr_promotion_format` PASS, 56 aceptados, 0 violaciones · `test_uat_authority_citations` PASS, 0 avisos · `shellcheck scripts/release.sh` limpio. No se tocó Rust: `git diff` de la sesión son dos ficheros de deuda, un ADR y el runner.

**Lo que sigue abierto:** **el arreglo de INC-DEBT-059**, que necesita `SCOPE-CONTRACT` + `PRE-FLIGHT` propios y test **RED antes** del arreglo, no después · la **clave del KMS**, único bloqueo que queda para publicar v2.5.3 · el **contrato de read-option** de INC-DEBT-049 · la **publicación del harness** Pipelinek-Test-Hardness (44 commits sin publicar) · el **alias de skillgraph**, retirado hasta que la sesión concurrente cierre `wi-72-p3-expansion-apply` · la clase ASCII de contaminación en `docs/` (INC-DEBT-058: no automatizable).

**Nota sobre el puntero:** `current_sha` nombra `1d613bbf`, el commit **anterior** a este fichero documental, por la razón que la tercera sesión ya dejó escrita: actualizar el puntero convierte a este commit en HEAD, y un commit documental no es evidencia del SHA que dice contener.

---

# CURRENT — puntero de reanudación de SDDK

**Estado (session-68, 2026-10-02): ADR-0153 pasa a `accepted` e INC-DEBT-051 queda resuelta, con sus cuatro falsificadores medidos contra el binario. `release plan` ya no está acotado a proyectos Rust.** `HEAD` = `c35e9a1c` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**Lo que cambió, en tres líneas:**

1. **La aceptación no se declaró por suma.** `bash tests/test_adr_0153_criteria.sh` ejecuta cada criterio **por separado** y exige que **pasen todos** los tests de un criterio que tiene varios, para que un verde agregado no pueda tapar uno rojo. **PASS=7 FAIL=0.**
2. **El criterio 1 estaba redactado de forma que ninguna implementación correcta podía cumplir**: decía «`Cargo.toml` no aparece en `version.rs`», y los tests de paridad de Rust tienen que *construir* un `Cargo.toml`. Medido: trece apariciones, **cero** en producción. Se reescribió a la propiedad con dientes —«el código que resuelve no nombra ningún manifiesto»— y se hizo cumplir con un test **estructural**, además **falsificado**: inyectar un `root.join("Cargo.toml")` lo hace fallar.
3. **F58 estaba a medio camino y no se dio por bueno.** El error listaba los 13 manifiestos buscados pero **no nominaba `.sddk/version-source.json`**, que es la salida. El mensaje la nombra ahora y un test lo mide.

**Falsificadores de INC-DEBT-051, medidos uno a uno:** F56 salida 0 declarando la autoridad · F57 el lockstep de Rust sigue exigiendo y rechaza el tag discrepante · F58 falla sin un `No such file` y nombra dónde buscó **y** la salida · F59 nombra el ecosistema y el manifiesto leído. **Reconciliación de redacción, escrita y no omitida:** F56 y F59 hablan de «adapter» y el contrato elegido no tiene adapters; cambia el sustantivo, no la exigencia.

**Lo que esto NO afirma:** la ruta **forge** contra un GitHub real no se ha ejecutado. F56, F57 y F59 se midieron con la ruta local, que no necesita red.

**Gates de session-68:** criterio de ADR-0153 `PASS=7 FAIL=0` · `engine --lib version` 51/0 · `cli --test cli release` 33/0 · fmt y clippy `-D warnings` limpios · `test_adr_promotion_format` PASS, 56 aceptados, 0 violaciones · `check_debt_index_coherence` PASS · `test_docs_script_contamination` PASS · `test_changelog_coverage` **PASS=42 FAIL=0**.

**Lo que sigue abierto:** **promover ADR-0152 a `accepted`**, con el mismo método criterio a criterio · la **clave del KMS**, único bloqueo que queda para publicar v2.5.3 · el **contrato de read-option** de INC-DEBT-049 · la **publicación del harness** Pipelinek-Test-Hardness (44 commits sin publicar) · el **alias de skillgraph**, retirado hasta que la sesión concurrente cierre `wi-72-p3-expansion-apply` · la clase ASCII de contaminación en `docs/` (INC-DEBT-058: no automatizable).

**Nota sobre el puntero:** `current_sha` nombra `c35e9a1c`, el commit **anterior** a este fichero documental, por la razón que la tercera sesión ya dejó escrita: actualizar el puntero convierte a este commit en HEAD, y un commit documental no es evidencia del SHA que dice contener.

---

# CURRENT — puntero de reanudación de SDDK

**Estado (session-67b, 2026-10-02): D2 cerrado. `release apply` ya no afirma un lockstep que nadie comprobó, y con esto los siete criterios de ADR-0153 son medibles uno a uno.** `HEAD` = `1a8f8ff8` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**Lo que cambió en esta sesión, en dos líneas:**

1. **`ReleaseOutcome.version_lockstep_passed` lo escribía a mano** (`release_cmd.rs:847-848`): `ensure_version_lockstep(...)?` y después `let version_lockstep_passed = true;`. Sobre un proyecto Go o Bazel informaba que el lockstep había pasado sin que hubiera pasado nada. Ahora el resultado lleva `version_authority`, el **tipo del engine**, y la ruta forge lo deriva. **D2 cerrado.**
2. **El otro campo, que se llamaba igual, NO se renombró**, y el motivo está medido: es una **puerta** que `release.rs:205` lee para abortar, y su valor llega al storage como la cadena de `failed_precondition`, que tres tests comparan literalmente. Con el resultado tipado, los dos dejan de llamarse igual y la homonimia desaparece **por construcción**.

**El falsificador encontró tres huecos reales, de la misma clase:** `release apply` no tiene ninguna cobertura, porque la ruta forge necesita red. El grave era la **puerta local** — pasarla a `was_cross_checked()` habría **bloqueado a Go y a Bazel para siempre**, con la suite en verde, porque todos los fixtures de esa ruta son de Rust. Se extrajeron dos funciones con nombre y se cubrieron con fixtures reales.

**Y falló contra sí mismo cuatro veces**, todas por su construcción: anclaje que `cargo fmt` movió, un escape de regex en un reemplazo, un `[^)]*` que se comía el cierre de la función, y un detector apuntado al binario de integración en vez de al de las pruebas unitarias. La regla que queda: **una mutación que no aterriza, no compila, o que se busca donde no vive su test se marca `SKIP`, nunca `FAIL`.**

**Gates de session-67b:** `release_flow` 12/0 · `release_blockers` 3/0 · `engine --lib version` 50/0 · `cli --lib release` 22/0 · `cli --test cli release` 33/0, con los 3 tests de `failed_precondition` verdes **sin reescribirlos** · fmt y clippy `-D warnings` limpios · falsificador **PASS=9 FAIL=0 SKIP=0**, 5 mutaciones, las 5 detectadas · `test_changelog_coverage` **PASS=40 FAIL=0**.

**Lo que sigue abierto:** la **clave del KMS**, único bloqueo que queda para publicar v2.5.3 · **promover ADR-0153 y ADR-0152 a `accepted`**, que con D2 cerrado ya es medible criterio a criterio · el **contrato de read-option** de INC-DEBT-049 · la **publicación del harness** Pipelinek-Test-Hardness (44 commits sin publicar) · el **alias de skillgraph**, retirado por decisión del operador hasta que la sesión concurrente cierre `wi-72-p3-expansion-apply` · la clase ASCII de contaminación en `docs/` (INC-DEBT-058: no automatizable).

**Nota sobre el puntero:** `current_sha` nombra `1a8f8ff8`, el commit **anterior** a este fichero documental, por la razón que la tercera sesión ya dejó escrita: actualizar el puntero convierte a este commit en HEAD, y un commit documental no es evidencia del SHA que dice contener.

---

**Estado (session-67, 2026-10-02): `release plan` ya no disimula cuándo no hubo nada que comparar, y el gate de cobertura del changelog estaba ROJO desde el lote 1 y ya no lo está. v2.5.3 tenía DOS bloqueos, no uno.** `HEAD` = `1f93dc1a` + este commit documental. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**Lo que cambió en esta sesión, en dos líneas:**

1. **D1 cerrado.** `release plan` llevaba `ensure_version_lockstep`, que acaba en `map(|_| ())`: la autoridad de la versión se tiraba. Medido **antes** del arreglo sobre un proyecto Go con `go.mod` y sin `Cargo.toml`: **exit 0** y una salida de cinco campos **indistinguible** de la de un repo Rust que sí se comprueba. Ahora la salida lleva `version_authority` con `kind`, la versión, los manifiestos leídos y los ecosistemas que no la declaran. Cambio **aditivo**, con test que fija los seis campos previos.
2. **El gate 2b del release estaba rojo y no se veía.** `tests/test_changelog_coverage.sh` daba **PASS=29 FAIL=8**; solo uno de los ocho fallos era de este lote. Los otros siete eran trabajo del mismo objetivo, sin declarar en la sección `## [2.5.3]`. Con el gate rojo, **v2.5.3 no se podía publicar ni con la clave del KMS aprovisionada**.

**Lo que NO se cierra, escrito y medido:** **D2**. La ruta forge de `release_cmd.rs:847-848` sigue escribiendo `let version_lockstep_passed = true;` a mano. No se arregla junto a D1 porque los **dos** campos llamados `version_lockstep_passed` tienen que significar cosas distintas: el de `LocalReleasePreconditions` es una puerta que `release.rs:205` lee para abortar, y pasarla a `was_cross_checked()` **dejaría a Go y a Bazel sin poder publicar jamás**. Es contrato de `sddk-gateway` y lleva lote propio.

**Gates de session-67:** `cargo test -p sddk-cli --lib release` 17/0 · `cargo test -p sddk-cli --test cli release` 33/0, el test de mismatch del lockstep Rust pasado **sin reescribirlo** · `cargo fmt --check` y `clippy -D warnings` limpios · falsificador end-to-end Go+Rust **PASS=14 FAIL=0**, 4 mutaciones, las 4 detectadas · `test_changelog_coverage` **PASS=38 FAIL=0** · `test_docs_script_contamination` PASS.

**El falsificador encontró un defecto en sí mismo, y es la lección de la sesión:** su primera pasada dio 12/13. La mutación que sobrevivió no era un hueco del código — quitar un campo no opcional de una construcción de struct **no compila**, el binario viejo sigue en su sitio, el comando sale con `exit 0` y la aserción lee **el artefacto que no se mutó**. Se declaró satisfied midiendo lo contrario de lo que creía. Arreglado en el arnés: toda mutación comprueba que su build terminó antes de preguntarle nada, y una que no compila se marca `SKIP`, nunca `PASS`.

**Lo que sigue abierto:** la **clave del KMS** (bloqueo 1 de 2 de v2.5.3) · **D2**, contrato de los dos `version_lockstep_passed` de `sddk-gateway`, con el doc de `release.rs:395-396` que sigue nombrando `Cargo.toml` · **promover ADR-0153 y ADR-0152 a `accepted`**, que llega cuando todos sus criterios estén verdes uno a uno · el **contrato de read-option** de INC-DEBT-049 · la **publicación del harness** Pipelinek-Test-Hardness (44 commits sin publicar) · el **alias de skillgraph**, retirado por decisión del operador hasta que la sesión concurrente cierre `wi-72-p3-expansion-apply` · la clase ASCII de contaminación en `docs/` (INC-DEBT-058: no automatizable).

**Nota sobre el puntero:** `current_sha` nombra `1f93dc1a`, el commit **anterior** a este fichero documental, por la razón que la tercera sesión ya dejó escrita: actualizar el puntero convierte a este commit en HEAD, y un commit documental no es evidencia del SHA que dice contener.

---

**Estado (session-66, 2026-10-02): la migración de `project_id` NO EXISTE, y eso se discovered ejecutando el apply, no leyendo el plan. INC-DEBT-048 queda CERRADA por decisión del operador.** `HEAD` = `ff3849cf` (dos commits de session-66 publicados sin `--no-verify`, por la variante A-v2 del pre-push: workspace 2.5.3 > tag publicado v2.5.2). Workspace **2.5.3 declarada, no publicada** y **no publicable**: falta la clave del KMS.

**Lo que cambia el diagnóstico, en dos líneas:**

1. **`project_id` está horneado en un fact log encadenado por hash.** `EventEnvelopeV1::compute_content_hash` anula únicamente `content_hash`, `sequence` y `recorded_at`; `project_id`, `stream_id` y `cycle_id` entran en el hash. Además `events_v1` y otras cinco tablas llevan `BEFORE UPDATE` que hace `RAISE(ABORT)`, con un test que lo exige. **La identidad de un proyecto es inmutable desde su primer evento**, luego los 25 receipts huérfanos no se pueden migrar: 3.477 filas append-only, 0 migrables. Se intentó y el storage rechazó la escritura; nada se escribió.
2. **Los 8 proyectos con dos ids no son un efecto secundario del normalizador: son la re-adopción**, la única vía que existe cuando la identidad ya no se puede cambiar. Su coste es partir el historial en dos. Camino de cierre propuesto, no ejecutado: una tabla de alias `from_id -> to_id` que el CLI resuelva al derivar (lo que hace git con un rename); no toca el fact log.

**Hecho en session-66, por encima del detalle de abajo:**

- **Tres defectos del `apply` de INC-DEBT-050**, todos anteriores a esta sesión y todos invisibles para los guardas que ya existían (los tres corren *antes* de escribir, y el defecto estaba *dentro* de la escritura): `UPDATE` **sin `WHERE`** (habría reasignado 79 ciclos centinela `__spine_import__` y 10 ciclos de **otro proyecto**); el **directorio de estado nunca se renombraba**; y `Path.replace` usado como sustitución de cadena, que lanza `TypeError`. El `apply` llevaba dos sesiones «a un paso» y abortaba en el primero de los ocho destinos ya ocupados.
- **INC-DEBT-048 cerrada** por la opción (a): spec `arch-spec-A3-S1` a `accepted` con REQ-A3S1-021 reescrita, **ADR-0126 reconciliado sin enmendar su Decision**, y **AT-UAT-019** reescrito para citar autoridades que existen. El guard de citas pasa de **1 aviso a 0**.
- **Un número del UAT era incorrecto y se corrigió:** F20 daba 4 FAIL según session-62; re-ejecutado hoy da **5**, porque session-65i añadió un test que depende de la derivación. F19 da 2, como se decía.
- **ADR-0151 incumplía ADR-0001 §3.4** — declaraba `status: accepted` sin `accepted_at` ni `accepted_by_cycle`. Encontrado por el guard, corregido. `violations: 0`.
- **Hallazgo abierto y NO arreglado:** 13 ficheros de `docs/` tienen caracteres CJK, cirílicos o de reemplazo sustituyendo palabras españolas. No se corrigen en masa porque la corrupción se detecta con fiabilidad pero **la palabra original no**: sustituirlas sería fabricar.

**Gates de session-66:** `test_migrate_project_identity_write` 24 casos verdes con **11/11 mutaciones detectadas** · los 7 tests de python de `tests/` verdes · `check_debt_index_coherence` PASS · `test_adr_promotion_format` PASS, 0 violaciones · `test_uat_authority_citations` PASS, 0 avisos · `cargo test -p sddk-engine --lib knowledge` 27 passed / 0 failed / 1 ignored.

**Lo que sigue abierto:** la **clave del KMS**, que bloquea v2.5.3 · el **alias de proyecto** (camino de cierre de INC-DEBT-050, SCOPE + ADR) · el **contrato de versión para repos no-Rust** (cierre de INC-DEBT-051, SCOPE + ADR) · el **contrato de read-option** de INC-DEBT-049 · la **publicación del harness** Pipelinek-Test-Hardness (44 commits sin publicar) · la corrupción de caracteres en `docs/`, que necesita que quien escribió cada frase diga qué quiso decir.

**Nota sobre el puntero:** `STATE.yaml` dice `ff3849cf`, que es el commit **anterior** a este fichero documental. Escribir el puntero convierte a este commit en HEAD, y un commit documental no es evidencia del SHA que dice contener. Es la tercera vez que el puntero se autocita; queda dicho para que no se lea como contradicción.

---

## Estado (session-65j, 2026-10-01): un guard llevaba ROJO desde session-65b y nadie lo ejecutaba. La causa de fondo no era ese guard: la superficie de gates es una lista escrita a mano, y 13 de 36 tests no estaban en ella.** `HEAD` = `b15cf610`. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`).

**Hecho en session-65j:**

1. **`scripts/check_debt_index_coherence.sh` salía con exit 1 sobre este repo desde session-65b.** Dos causas encadenadas: INC-DEBT-052 declaraba su estado como `**status:** resolved` en markdown bold — un dialecto que el guard no lee, porque lee frontmatter YAML y la prosa `**Estado:**` — y el guard no estaba referenciado por ningún runner. `ci.yml:46` hace `shellcheck` (lint, no ejecución) y `release.sh` corría el **test de fixtures**, que monta un árbol desechable por caso. Sus 10 casos PASABAN, y ese verde leía como cobertura.
2. **El método casi se diagnostica al revés.** La primera hipótesis fue que el guard era ciego al dialecto, y estuvo a punto de «ablandarse» para aceptar la entrada. Lo refutó la evidencia: lo que se había ejecutado era el TEST, no el GUARD. Un `PASS=10 FAIL=0` leído como «el guard funciona» es exactamente el falso positivo que este repo lleva slices persiguiendo.
3. **Corrección en la dirección correcta:** el DOCUMENTO se ajustó al contrato (frontmatter YAML canónico), no el contrato al documento. `resolved` sigue siendo `resolved`; ahora es legible por máquina. El guard queda **más estricto** después, no antes.
4. **Barrido sistémico: 13 de 36 tests sin runner — no los 7 de la primera medición.** La diferencia es la lección: un test nombrado en un comentario no está gated, y la nota de exclusión de `release.sh` nombra dos tests precisamente porque NO se ejecutan. Contar prosa como cobertura es el mismo error que contar una declaración como obediencia.
5. **Cableados 11 tests herméticos** (medidos: 36–299 ms cada uno), 6 shell y 5 python. Una regresión en el staging del bundle ahora falla el release en vez de publicarse.
6. **Se quedaron fuera 6, con motivo escrito.** El caso importante: `test_h05_isolation.sh` **pasa sin medir** — sin el rlib release imprime `skip:` y aun así reporta `PASS=1 FAIL=0`. Cablearlo habría devuelto un verde vacío, la misma forma que INC-DEBT-054. Un PASS que no midió nada es peor que un gate ausente, porque además tapa el defecto.
7. **`tests/test_gate_coverage.py` ata la propiedad** y se protege con dos reglas que impiden que la lista de excepciones se pudre: una excepción a un fichero inexistente es FAIL, y una excepción para un test que ya tiene runner es FAIL.
8. **Dos falsificadores encontrados, incluidos en el guard mismo.** Tres mutaciones cazadas (test huérfano, excepción obsoleta, excepción cajón de sastre). Y al falsarlo, el guard resulto con su propia contabilidad mal: reportaba «excepcionados: 0» con seis excepciones vivas, porque la derivaba por resta. **Un guard que miente sobre sus propias cifras no puede usarse para justificar por qué el resto pasa.**

**Gates:** `test_debt_index_coherence` 12/12 · `test_gate_coverage` 31 con runner + 6 excepcionados + 0 huérfanos · guard real 41/41 sobre el repo · `shellcheck` limpio · `bash -n release.sh` OK · 14/15 gates de release verdes, el rojo siendo `test_release_state_pointer` detectando este mismo puntero desfasado.

**Lo que este slice NO cierra:** la **decisión normativa (a)/(b) de INC-DEBT-048** es del operador, y ahora con el radio corregido: REQ-A3S1-021 alimenta `IntelligenceLoopReceiptId` vía **ADR-0126, que está `accepted`**. El **gate de citas de `UAT-MATRIX.md`** sigue sin implementar. INC-DEBT-050 (migración de los 25 receipts) sigue en espera. El **contrato de versión para repos no-Rust** (cierre real de INC-DEBT-051) sigue necesitando SCOPE + ADR.


---

**Estado (session-65i, 2026-10-01): `adopt status` reportaba `conflict` sobre el storage ya convergido de este repo. Tres sitios de la cadena de identidad, no uno. Verificado end-to-end contra el binario: release 2.5.3 = `conflict`, binario con el arreglo = `complete`.** `HEAD` = `35b33e8c` (session-65i) sobre `e0628686` ya publicado. Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`). **SIGUIENTE: publicar, o abrir el ciclo de brevedad que devuelve `doctor --strict` a verde.**

**Hecho en session-65i:**

1. **El síntoma era real, no un artefacto de tests.** Con el pin activo (`.sddk/project-pin.json` -> `p-63676b11dc0ef88f`) y los 65 ciclos bajo ese id, el release 2.5.3 respondía `status: conflict` con detalle *"receipt identity differs from plan; refresh only accepts runtime metadata drift"*. Contrato absurdo aplicado a un caso que no es runtime drift: es el plan olvidando quien es.
2. **Tres sitios, y dos de ellos independientes.** `plan_adoption` (rama `Some(pinned)`) construía la identidad con `remote_url: None`, tirando el resto — el pin debe sobreescribir **solo** `project_id`. `same_identity` comparaba `remote_url` **crudo**. `inspect_ledger` comparaba `existing.remote_url` **crudo** contra la fila `projects`. Arreglar el segundo sin el tercero solo traslada el conflicto: de ahí una única función `remote_urls_match` para ambos.
3. **La comparación cruda contradecía una decisión ya tomada.** El dominio normaliza owner/repo a minúsculas **antes** de hashear el `project_id` (test golden `case_change_in_owner_or_repo_resolves_to_same_project_id`). El recibo y la fila `projects` guardan `Rubentxu/...` porque se acuñaron el 2026-09-30, antes del commit `52182522`. Mismo `project_id`, misma identidad por definición — y `conflict`.
4. **El falsador encontró la mitad negativa que faltaba.** Sustituir la comparación por `right == *right` —`true` siempre que ambos lados normalicen— dejó los tests positivos **EN VERDE**: fijaban «el mismo repo con otro case ya no es conflicto», pero no «un repo distinto sigue siendo conflicto». Un guard que declara siempre coincidencia era aceptable. **Cuarta vez que un falsador encuentra en sí mismo lo que la inspección no**; aquí encontró algo que ni había considerado: que un arreglo puede **degradar** la detección de drift en vez de afinarla.
5. **Los negativos usan pin a propósito.** Sin pin, un remoto distinto acuña otro `project_id`, apunta a rutas inexistentes y el veredicto es `Absent`: el test discriminaba, pero por el guard equivocado (las rutas). El pin es la única forma de que dos remotos genuinamente distintos compartan `project_id` y rutas — «esto es el proyecto X» mientras el checkout apunta al repo Y.
6. **Verificación end-to-end, no solo test.** Mismo repo, mismo pin, mismo storage; solo cambia el binario: `/home/rubentxu/.local/bin/sddk` (2.5.3) -> `conflict`; `/var/home/rubentxu/cargo-targets/debug/sddk` -> `complete`.
7. **Corrección de superficie en INC-DEBT-049:** declaraba `crates/sddk-cli/src/adopt.rs`, fichero **inexistente**. Las superficies reales son `crates/sddk-engine/src/adoption.rs` y `crates/sddk-cli/src/{lib,context_cmd}.rs`.

**Gates:** `cargo fmt --check` limpio · `clippy -p sddk-engine --all-targets -D warnings` limpio · **1375/1375 tests del engine**, 0 fallos, 6/6 en `adoption::tests` · 4 mutaciones aplicadas (RED crudo + mutación always-true) y las 4 detectadas · 10/10 índice de deuda · 3/3 integridad de referencias de superficie.

**Lo que este slice NO cierra:** INC-DEBT-049 **no se cierra** — la parte grave ya estaba resuelta en session-63 y la abierta (declarar historial bajo otra identidad) sigue necesitando SCOPE + ADR. INC-DEBT-050 **tampoco se ve afectada**: esto cambia cómo se **compara** la identidad, no reubica los 25 recibos huérfanos. Este repo no necesita migración porque tiene pin; los otros 24 sin pin siguen huérfanos. La comparación normalizada es la contraparte **no destructiva** de la migración, y ambas pueden convivir.


---

**Estado (session-65h, 2026-10-01): el staging del bundle era una QUINTA copia del contrato, y produjo dos defectos medidos. Uno habría abortado el release; el otro publicaba dos ficheros sin digest.** `HEAD` = `bc6e2cfd`, **sin publicar** (op-5). Workspace **2.5.3 declarada, no publicada** (último tag remoto `v2.5.2`). **SIGUIENTE: publicar `bc6e2cfd`; después, derivar también la ruta cloud.**

**Hecho en session-65h:**

1. **Defecto (a): el `tar` nombraba superficies que el staging nunca copiaba.** Al añadir `specs` y `docs/impeccable-reference` a la lista del `tar` se olvidó en la del `cp -r`. Fase 5 aislada, RED medido: `tar: specs: No se puede efectuar stat` → `exit=2`. Con `set -euo pipefail` (`release.sh:55`) eso es un release **abortado**: ruidoso, pero incompleto. Un subdirectorio necesita además su **padre** creado antes del `cp -r`, o aterriza plano como `dst/<hoja>` y el `tar`, que pide el camino con prefijo, no encuentra nada — regla ya documentada para `prompts/sddk`, que `docs/impeccable-reference` sufría por primera vez.
2. **Defecto (b): `cp -r` copiaba ficheros que el manifest no lista.** `cp -r <superficie>` copia lo que hay en disco, **incluido lo que `.gitignore` excluye**. Contando ficheros reales del tar contra entradas del manifest: `agents` daba **73/72** y `assets` **18/17**. Los dos sobrantes son `agents/.atl/.skill-registry.cache.json` (`.gitignore:26`) y `assets/agent-models.yaml.bak` (`.gitignore:17`). Consecuencia: **`manifest_sha256` en `BUNDLE.toml` no describía el propio tarball** — dos ficheros sin digest que la instalación no puede verificar. La ruta cloud **no** sufre (b): empaqueta un checkout limpio.
3. **Resolución (Ruta 1, la autoridad manda): el staging se deriva de `MANIFEST.sha256`**, que step 4 ya verifica fail-closed. Lo que el manifest lista viaja, y nada más puede viajar. Desaparecen las dos clases de defecto a la vez, y con la lista escrita a mano desaparece el coste de editarla cada vez que se añade una superficie — que es exactamente lo que produjo (a). Contrato fail-closed añadido: el conjunto del staging es exactamente el del manifest + `BUNDLE.toml`.
4. **El guard de session-65g no podía ver ninguno de los dos, y no por casualidad.** Comparaba la lista de superficies del `tar` contra `MANIFEST_SURFACES`: (a) pasaba porque metía `specs` en la lista, y el `cp -r` no estaba en ninguna lista que el gate mirara; (b) es **invisible por construcción**, porque son ficheros que correctamente no están en ninguna lista. **Un guard que compara dos listas solo ve divergencia entre declaraciones; no ve que una declaración deje de ser la que se obedece, ni lo que se publica sin declarar.** Las dos cosas aparecieron al **ejecutar** el staging y contar ficheros.
5. **Guard reescrito para atar la propiedad, no la lista: 12 tests.** Staging derivado del manifest (estructural); `tar` empaquetando el árbol entero; manifest casa con `git ls-files` **en ambas direcciones**; y un **canario** untracked bajo una superficie real que **no debe** llegar al staging — el único test que puede ver (b).
6. **Falsificadores: 9 mutaciones, las 9 detectadas**, árbol restaurado en verde. Y la falsificación encontró **dos puntos ciegos en el guard nuevo mismo**: la contención (`assertNotIn` + `len == 1`) la satisfacía un subconjunto, y `test_staged_tree_matches_the_manifest_exactly` comparaba el manifest **consigo mismo** —borrar una entrada a mano la borraba de las dos mitades y el gate seguía verde mientras el fichero dejaba de publicarse en silencio—. **Sexta vez que un falsador encuentra en sí mismo lo que la inspección no.**
7. **Sexta superficie `docs/impeccable-reference`** (2 ficheros, 194 líneas), con su `impeccable_reference_count`, su brazo de conteo y los dos `tar` actualizados. Existe porque `agents/impeccable-primary.md` **sí viaja** en el bundle y citaba dos ficheros que el bundle no llevaba: la misma clase que cerró `specs`. Se eligió el subdirectorio y no `docs/` entero, que arrastraría `docs/history/` al artefacto.
8. **Contenido de `impeccable` verificado contra el upstream**, no heredado del número que afirmaba el agente: el detector captura **61 issues deterministas**, no 46. Cuatro afirmaciones sin fuente corregidas con una nota que explica por qué no se fija un número sin fecha.

**Gates:** `cargo fmt --check` limpio · `clippy -p sddk-cli --all-targets -D warnings` limpio · **409 tests `dev`, 0 failed** · `shellcheck scripts/release.sh` limpio · manifest **394** ficheros, `--verify` OK · **12/12** guard de superficies · 3/3 guard de referencias · 10/10 índice de deuda · 15/15 changelog · 48/48 hook de push · puntero `PASS`.

**Lo que este slice NO cierra:** el **push** (`bc6e2cfd`, op-5) · `release.yml` **sigue enunciando la lista de superficies a mano** — es segura porque empaqueta un checkout limpio, pero es una quinta copia que depende de una propiedad del entorno y no del código; se puede derivar igual que la local, y **no se ha hecho** porque la ruta cloud no puede probarse en local y el guard no podría verificarla · INC-DEBT-051 (arreglo = contrato nuevo) · migración de los 25 receipts · RC 0.45.0 de PipelineK · las 19 superficies fuera de presupuesto de brevedad, que es lo que haría que `doctor --strict` volviera a estar verde.

**Primer paso preciso de la sesión siguiente:** OK del operador al push de `bc6e2cfd` y del cierre. Con eso, `bash scripts/release.sh` para publicar v2.5.3 (el CHANGELOG ya lo cubre) y `sddk dev install`.

---

**Estado (session-65d, 2026-10-01T21:20Z): DOS gates que respondían sin examinar nada, corregidos, publicados y **validados contra el binario instalado**. `sddk` en el PATH es **2.5.3** y `doctor --strict` **hoy falla con exit 1 sobre las 19 superficies** — que es exactamente lo que debe hacer y lo que no podía hacer antes. El bundle instalado sigue en 2.5.2 porque el 2.5.3 aún no está publicado como release.** **SIGUIENTE: ciclo de brevedad con SCOPE-CONTRACT; publicar v2.5.3 para poder instalar el bundle.**

**Hecho en session-65d:**

1. **INC-DEBT-053 (`ddfd2b51`, otro actor) ya publicado.** `ledger verify-chain` resolvía por defecto el stream `project:<id>`, que **no existe en ninguno de los 326 ledgers** de la máquina, seleccionaba cero eventos y contestaba `PASS` mientras `sddk ledger verify` sobre el mismo ledger ve 171. `debt report`/`debt gates` fabricaban un informe para un ciclo ajeno y sin hallazgos; como un informe vacío no incumple ningún predicado, `debt-severity-assigned` y `debt-priority-assigned` han sido **constantes**. Evaluación propia: **865 tests del lib verdes**, clippy limpio con `-D warnings`, entrada de deuda bien construida.
2. **PERO le faltaba un defecto que sus dos tests no podían alcanzar** (`d76cbb1c`). `verify_streams` reconstruía la etiqueta con `resolve_streams(None, ..)`, así que `verify-chain --stream cycle:p-demo/one` contestaba `stream: all streams of p-demo`: veredicto correcto sobre una salida que nombraba otra cosa. RED medido antes de corregir (`left: "all streams of p-demo"` / `right: "cycle:p-demo/one"`), GREEN después. La etiqueta viaja ahora desde quien la decide hasta quien la publica. Los dos tests que acompañan a ese refactor **no podían morder** el defecto: ambos pasaban por la ruta del default.
3. **INC-DEBT-054, mío (`cfe96856`): `doctor --strict` salía con exit 0 sin medir nada.** Medido sobre el binario publicado **v2.5.2** desde un directorio sin superficies: **0 checks de `surface.briefness` emitidos**, `all_present: true`, exit 0. Dos causas — los checks se anclaban a `current_dir()` mientras los de layout usan `framework_root`, y `resolve_active_framework_root` **ignora el cwd**, así que el caso normal de auditar un prefijo instalado es un cwd sin superficies; y cada enumeración iba dentro de `if let Ok(entries) = read_dir(..))`, que no distingue «no hay `agents/`» de «no se pudo leer».
4. **Ningún gate ejecutaba los criterios de ADR-016.** `grep -rn -- '--strict' .github/` → **cero coincidencias**: ni CI, ni `scripts/release.sh`. Los presupuestos (300/150/200) **siguen vigentes y no hay waiver**, pero su única aplicación automática eran dos tests que montaban una raíz **con** superficies, luego ninguno alcanzaba la ruta neutra. Un defecto con test, y el test ratificaba la otra mitad.
5. **Arreglo fail-closed.** `has_surface_dirs()` decide si un root puede medirse; se prefiere el cwd cuando contiene superficies (un desarrollador que acaba de editar un agent quiere que se mida *ese* agent) y se cae al framework root activo, que lleva las mismas carpetas en layout plano de bundle. Sin superficies se emite `surface.briefness.root` con `present: false` y un `detail` que dice *unverifiable*, no *satisfied*. Una medición vacía no es una medición aprobada.
6. **Dientes falsificados, no afirmados.** Reinyectar cada mitad del arreglo mata exactamente un test y ninguno más: volver a anclarse solo al cwd mata `cli_dev_doctor_brevity_mide_el_bundle_instalado_cuando_el_cwd_no_es_un_arbol`; no contar la violación mata `cli_dev_doctor_strict_no_pasa_sobre_una_ausencia_de_medicion`. El de la etiqueta se observo rojo antes de corregirlo.
7. **Deriva de punteros que `ddfd2b51` dejó.** `Cargo.lock` seguía en 2.5.2 para los 8 miembros del workspace (`0c512cf6`) y `manifest.toml` en 2.5.2 (`6c4e9b69`), ambos tras bumpear `Cargo.toml` a 2.5.3. Son la misma clase de disenso que el resto de esta sesión: dos autoridades diciendo versiones distintas del mismo artefacto.
8. **CHANGELOG 2.5.3 escrito (`9085402b`).** El gate de cobertura daba **PASS=0 FAIL=3** porque la versión estaba declarada sin sección; ahora **PASS=6 FAIL=0** cubriendo los tres `fix(cli)` desde `v2.5.2`. Sin esto, publicar 2.5.3 habría emitido un changelog que no menciona la mitad del trabajo que se instala.

**Gates:** 865 tests del lib verdes · clippy `-D warnings` limpio · 6/6 tests de `doctor` verdes con 2 nuevos · dientes 2/2 muertos · `test_changelog_coverage` PASS=6 FAIL=0 · `test_debt_index_coherence` PASS=10 FAIL=0 · YAML de `STATE.yaml` reparseado.

**Consecuencias aceptadas, no resueltas:** `--strict` **ya puede fallar y va a fallar** — el bundle público incumple 19 presupuestos (2 agents, 14 skills, 3 prompts); la fase `verify` de **todo** proyecto deja de poder autorizarse con los dos gates de deuda hasta que exista la detección; y `tests/test_release_state_pointer.sh` queda en **FAIL** en `current_sha NO esta en origin/main`, que es exactamente cierto y no se puede maquillar sin pushear. `scripts/reconcile_state_pointer.sh --check` dice PASS porque compara contra **main local**, el test contra **origin/main**: con 5 commits sin publicar las dos no pueden estar verdes a la vez, y eso es lo que el guard señala.

**Lo que este slice NO cierra:** el **push** (op-5; el hook **no** lo bloquea) · `sddk dev install` — el binario del PATH sigue siendo 2.5.2 y **no tiene ninguno de estos fixes** · publicar v2.5.3 · **adelgazar las 19 superficies** fuera de presupuesto, que es lo que haría que `--strict` volviera a estar verde · la detección de deuda que INC-DEBT-053 deja explícitamente sin implementar.

**Corrección de esta misma entrada:** se afirmó primero que el predicado (A) del hook era insatisfacible por no contener el rango un cambio de versión. **Es falso**, y se comprobó antes de darlo por bueno: `githooks/pre-push` define (A) como disyunción, y su segundo disjunct (variante 3 de INC-DEBT-040) admite cuando la versión del workspace en el tip supera al mayor tag publicado en el remoto — aquí **2.5.3 > v2.5.2**. El push se admite. La lección es la de siempre en este repo: un predicado leído a medias es un predicado que decide sobre información que no tiene.

**Siguiente paso preciso:** OK del operador al push de estos 6 commits. Después, `bash scripts/release.sh` para publicar v2.5.3 (el `CHANGELOG` ya lo cubre) y `sddk dev install`. Sin push, `doctor --strict` seguirá sin poder usarse como gate en ninguna parte y el puntero seguirá en FAIL.

---

**Estado (session-65c, 2026-10-01T17:00Z): v2.5.2 ESTÁ PUBLICADA, FIRMADA Y CERTIFICADA. El bloqueo por `musl-gcc` está RESUELTO y el puntero ya no dice lo contrario. La firma la produjo Actions, no `release.sh` — la identidad keyless sólo existe en un runner de CI, y el script local **aborta** en vez de publicar sin firmar. La certificación la conduce el harness de PipelineK, que es la autoridad de certificación por su propia política. El binario del PATH sigue siendo `sddk 2.4.2`: la release que lo arregla ya está fuera, pero la autoridad instalada aún no la ve.** **SIGUIENTE: `sddk dev install`.**

**Hecho en session-65c (release + certificación, sin tocar el ancla de confianza):**

1. **v2.5.2 publicada.** Tag ligero sobre `818d4ff9`, `gh workflow run release.yml --ref v2.5.2` → run **36890390356** success completo. 27 assets, `isDraft=false`, `isPrerelease=false`, `publishedAt 2026-10-01T16:22:15Z`. `release.sh --dry-run` había pasado los pasos 0–8 con binario `static-pie linked` real; el pipeline completo **abortó en el paso 8c** porque la identidad de firma no existe en este host.
2. **"PipelineK conduce, Actions firma"** (decisión del operador). El ancla `SDDK_COSIGN_IDENTITY` / `SDDK_COSIGN_ISSUER` de `install.sh` **no se toca**. Lo que faltaba no era firmar — `release.yml` ya lo hacía — sino que **nadie certificara después**.
3. **Certificación real desde el harness**: `pipelines/certify-sddk-release.pipeline.kts`, 8 etapas, ejecutada contra v2.5.2 con **8/8 `success`**: `cosign verify-blob` → **`Verified OK`** bajo los pins **leídos de `install.sh`**, `sha256sum -c` coincide, layout del bundle correcto, binario estático, instalación desde la URL pública y `dev doctor` con `all_present: true`. Receipt con `verdict: CERTIFIED`.
4. **Ocho defectos, todos encontrados ejecutando, ninguno leyendo.** Dos son de plataforma y afectan a cualquier pipeline del harness: **una etapa no corre en el workspace** (cada una tiene su sandbox, luego un workdir relativo entre etapas no puede funcionar) y **los raw strings no se dedentan** (un heredoc con la indentación del `.kts` nunca cierra y `cat` sale 0: etapa **verde** escribiendo fichero corrupto). Los otros seis: comando guardado en vez de ejecutado, pin comparado como patrón, `pins.env` sin comillas que perdía los backslashes al hacer `source`, la URL usada como ruta de destino, un `scripts/install.sh` que el bundle publicado no contiene, y un `SDDK_FRAMEWORK_DIR` que iba a **sobrescribir el bundle runtime del operador**.
5. **19 mutaciones, 19 muertas.** Las 3 que sobrevivieron la primera ronda eran la clase de defecto que la etapa existe para cerrar: **`--certificate-oidc-issuer=""` pasaba la suite entera**, porque todos los tests preguntaban si el flag estaba presente. Un pin vacío es cosign diciendo *"acepta cualquier firmante"*. Se escribieron los tests que faltaban, no se bajó el listón.
6. **`test_supply_chain_authenticity.sh --tag v2.5.2` = PASS=13 FAIL=0 SKIP=0**, con sus tres controles negativos. Addendum en INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY que **no cierra nada nuevo**: el INC sigue `closed` desde session-43 con testigo v2.2.27.
7. **Puntero reconciliado y era mentira en dos sitios**: `last_public_release_observed` decía `v2.4.2` y `workspace_version_at_current` decía *DECLARADA, NO PUBLICADA*. Ambas falsas; corregidas con evidencia observada. Los punteros previos quedan en `superseded_*`.
8. **Hallazgo fuera de alcance, reportado y NO arreglado**: `certify-candidate.pipeline.kts` y `promote-release.pipeline.kts` del harness **tienen el mismo defecto de workdir relativo entre etapas** y no pueden completarse. No se tocaron: no es el bug de ese fichero.

**Gates:** 28 tests estructurales verdes · 19/19 mutaciones killed · pipeline 8/8 etapas · `cosign Verified OK` · `test_supply_chain_authenticity.sh --tag v2.5.2` PASS=13 FAIL=0 · `test_debt_index_coherence` PASS=10 FAIL=0 · YAML de `STATE.yaml` reparseado.

**Lo que este slice NO cierra:** `sddk dev install` (**lo primero que toca la autoridad local**) · el **push** del commit del harness `fc249b8` (op-5, arrastraría 40 commits) · la **migración** de los 25 receipts (`apply` intacto, digest `b98e9a8d`, backup verificado, espera autorización) · el workdir relativo de los otros dos pipelines del harness · RC 0.45.0 de PipelineK · INC-DEBT-048 / 049 / 051.

**Siguiente paso preciso:** `sddk dev install` desde el bundle ya publicado y firmado — la release está fuera desde hace una hora y la autoridad instalada es la única que sigue ciega. Después, decisión del operador sobre el push del harness y, si autoriza, la migración.

---

**Estado (session-65, 2026-10-01T16:40Z): `sddk release plan` no funciona en ningún proyecto que no sea Rust (INC-DEBT-051, high/P1). La migración de identidad tiene **backup verificado y dry-run hecho**, pero **NO se ha ejecutado** y `apply` sigue intacto. El hallazgo grande no era el esperado: el script que iba a escribir la migración **era él mismo un INC-DEBT-050** — su espejo del normalizador no coincidía con el Rust en 6 formas, una de ellas la de Git más común. Release v2.5.0 sigue BLOQUEADO por musl-gcc.** **SIGUIENTE dentro de C3m:** C3m.0 (ADR de significado canónico de KMT) · C3m.1 · C3m.3 · C3m.4 · C3m.5.

**Hecho en session-65 (documental + scripts; sin tocar código de producción):**

1. **INC-DEBT-051 registrada.** `ensure_version_lockstep` (`crates/sddk-engine/src/version.rs`) hardcodea `root.join("Cargo.toml")` sin consultar adapters; la invocan `release plan` (`release_cmd.rs:668`) y `release apply` (`:847`). Ejecutado sobre PipelineK (Kotlin/Gradle): `VERSION LOCKSTEP ERROR: could not read …/Cargo.toml`. AGENTS.md §2.3 declara la política *"agnóstica de lenguaje/build/test runner"* y lista **Bazel** — Gradle y Bazel son el mismo caso, así que es una desviación de un principio declarado y no una limitación legítima. `dist` y `verify` fallan antes por argumentos/ruta y quedan **no evaluados**.
2. **El espejo de la migración tenía su propio INC-DEBT-050.** Reimplementaba `normalize_remote_url` con reglas parecidas pero distintas al Rust: rechazaba `git@host:owner/repo` (la forma de remote más común), trataba el puerto por defecto como global en vez de por esquema (`scp` no tiene), usaba `str.isdigit()` donde el Rust exige ASCII, aceptaba authority y puerto vacíos que el Rust rechaza, y **crashaba** con `UnboundLocalError` en IPv6. **Un `apply` habría escrito ids equivocados en ledgers reales.**
3. **Dos defectos silenciosos más:** `audit` no encontraba ningún receipt (`p-*` tomado como nombre literal → `glob` vacío **sin error** → imprimía `selfcheck: ok` con 0 receipts, indistinguible de "no hay nada que migrar"; ahora hay guarda que **falla con exit 4**), y `audit` siempre imprimía JSON (`set_defaults(func=audit)` pasaba el `Namespace` de argparse a un `bool`, y un `Namespace` siempre es truthy).
4. **El control de confianza no controlaba lo que decía:** el selfcheck pasaba el remote **crudo** a `stable_project_id`, así que **nunca ejercitaba `normalize_remote_url`**. Sustituido por un corpus dorado de **21 normalizaciones + 8 rechazos + 2 ids heredados**, **generado desde el binario real** e insertado por script (escribirlo a mano ya falló una vez: puse `:443` donde el Rust dice `/443`, y el selfcheck lo detectó).
5. **Falsificadores F60–F67: 7 OBSERVED.** **F62 se registra como NO-APLICA y no cuenta** — quitar `if not authority` es un no-op porque `if not host` ya cubre el caso; rama redundante en el espejo **y en el Rust**. Un falsificador que no puede fallar no es falsificador. El primer harness además **no mutaba el fichero** (`sed` sobre el texto ya corregido) y "observaba" un PASS vacío: segunda vez en la sesión que un falsador tiene que probarse a sí mismo.
6. **Trampa de verificación resuelta:** `Über` (U+00DC) y `über` (U+00FC) **se ven iguales** en pantalla y en `repr` pero dan `project_id` distinto. La distinción ASCII/Unicode del host era observable y **no estaba pineada**; se añaden dos casos no-ASCII y el test compara por codepoint.
7. **Addendum a INC-DEBT-050:** los **25 huérfanos se confirman y son estables**; el total de receipts sube 104→117 porque los gates crean adopts de prueba. Pero el **"16 project_id / 13 repos" de su frase de resumen era incorrecto desde que se escribió** — la lista de detalle ya enumeraba **15 sobre 15**, que es lo que reproduce la remedición.

**Gates:** selfcheck 21+8+2 OK · corpus diferencial **agree=23 diverge=0** · `tests/test_migrate_project_identity_mirror.py` **10/10** · **test falsificado** (rompiendo el espejo → `FAILED (failures=3)`) · `audit` 117 receipts / 25 huérfanos · `test_debt_index_coherence` **PASS=10 FAIL=0** · `git diff --check` limpio.

**Backup y dry-run (lo único que el operador autorizó):** backup verificado de **7033 ficheros** (535 MiB) en `/var/home/rubentxu/.sddk-migration-backup-20261001/20261001T145101Z`; plan de **15 `project_id`** con digest `b98e9a8d…`; las **tres vías de `apply` comprobadas rechazando** (confirm vacío, digest incorrecto, backup sin verificar) — las tres `exit 3` sin escribir nada.

**Lo que este slice NO cierra:** la **migración** (destructiva, `apply` sin ejecutar, espera autorización nueva) · el **arreglo de `release plan` para no-Rust** (contrato nuevo → SCOPE + ADR) · INC-DEBT-048 (normativa binaria) · INC-DEBT-049 (advertencia de historial).

**Riesgo residual declarado:** si alguien añade un caso al normalizador en Rust **sin regenerar el corpus**, `audit` seguirá verde sobre un espejo obsoleto, y `apply` re-verifica el plan con ese mismo espejo. Cerrarlo exige regenerar el corpus desde el Rust como **paso de integración**, no como disciplina.

---

**Estado (session-64, 2026-10-01T15:40Z): el camino `remote` de la identidad tiene golden pin — la próxima reasignación de `project_id` rompe un test en vez de fallar en silencio. Es el remedio de fondo de INC-DEBT-050; la migración de los 25 receipts huérfanos sigue ABIERTA y espera al operador. Release v2.5.0 sigue BLOQUEADO por musl-gcc.** **SIGUIENTE dentro de C3m:** C3m.0 (ADR de significado canónico de KMT) · C3m.1 · C3m.3 · C3m.4 · C3m.5.

**Hecho en session-64 (sólo tests, SemVer PATCH, sin tocar producción):**

1. **Tres tests nuevos** en `crates/sddk-domain/src/identity.rs`: `project_id_is_pinned_to_known_values` (tres formas de remote+scope), `remote_normalization_is_pinned_to_known_values` (casse mixta, `.git`, puertos, scp/ssh, credenciales) y `case_normalization_reassigned_real_project_ids_without_migration` (los **dos ids reales** de esta máquina, con sus valores exactos).
2. **El hueco era conceptual, no de cobertura.** `properties.rs:20` sólo afirma `f(x) == f(x)`, y eso **sigue siendo cierto si `f` se sustituye entera**. El defecto no tocaba lo que el test comprobaba: comprobaba una propiedad ortogonal. Y la asimetría era la causa — el seed de fallback **sí** tenía golden pin desde INC-DEBT-028 con el razonamiento escrito; el camino del remote, **el que recorre todo proyecto real**, no.
3. **Falsificadores F53–F55 OBSERVED.** F53 (quitar el `.to_lowercase()` del path) → el golden del normalizador falla con `left: "https://github.com/Acme/Widgets"` frente a `right: "…/acme/widgets"`. F54 (dominio `v1`→`v2`) y F55 (invertir el framing remote/scope) → fallan el golden del hash **y** el test histórico.
4. **Un falsificador mal diseñado se corrigió antes de ejecutarlo:** el F53 inicial mutaba `to_lowercase`→`to_ascii_lowercase`, que en ASCII da el mismo resultado — el pin no habría fallado y la prueba no habría probado nada.
5. **Las capas quedan pinadas por separado**, y eso sólo se ve al falsificar: con el normalizador mutado el golden del hash sigue verde (recibe el remote ya normalizado); con el hash mutado el del normalizador sigue verde. Cada pin protege la suya, y ese cruce era justo lo que hacía el defecto original.
6. **El comentario del golden dice lo que no hay que hacer si falla:** no copiar el valor nuevo. Copiarlo es exactamente lo que hizo D2, en silencio.

**Gates:** `sddk-domain --lib identity::` 30/0 (3 nuevos) · `sddk-domain --lib` 559/0 · fmt limpio · clippy `-D warnings` exit 0 · **workspace 5245 passed / 0 failed** (5242 + 3, cuadra con aritmética).

**Lo que este slice NO cierra:** la **migración de los 25 receipts** (destructiva, requiere al operador) y convertir la regla "tocar el normalizador es BREAKING CHANGE" en un gate automático de CI. El golden pin **impide el siguiente fork, no arregla el anterior.**

---

**Estado (session-63, 2026-10-01T14:20Z): el pin de identidad ya gobierna las CINCO vías del CLI — antes se escribía y no surtía efecto en `adopt status`, `cycle status` ni `config set`. INC-DEBT-049 queda con la parte del pin RESUELTA y la advertencia de historial huérfano ABIERTA. Release v2.5.0 sigue BLOQUEADO por musl-gcc, y ahora eso tiene consecuencia real: el binario instalado (`sddk 2.4.2`) no contiene el fix.**

**Lo que cambió en session-63 (autorizado por el operador, que además autorizó todos los gates humanos):**

1. **Aplicado** `sddk project pin --project-id p-63676b11dc0ef88f` — el remedio que session-62 dejó escrito y sin ejecutar. Y al aplicarlo **falló**, que es el hallazgo: `project resolve` reportaba `identity_source: pinned`, pero `adopt status` y `cycle status` seguían en `p-995939af668a53d8`.
2. **Causa raíz medida: hay CINCO resolvers de identidad independientes en `sddk-cli` y sólo DOS leían el pin.** `RuntimeContext::open` ✅, `run_project_resolve` ✅, `resolve_project_ids` ❌ (`config set`), inferencia de ciclo ❌ (`cycle status`/`next`), `plan_adoption` ❌ (`adopt`).
3. **La afirmación falsa estaba en el propio código:** el doc de `ProjectPin` decía *"every runtime context honor it"*, y el comentario de la inferencia enumeraba *"remote OR fallback_seed OR generate"* — omitiendo el pin. Nadie lo cazó porque los dos e2e del pin sólo invocan `project resolve`, el único resolver que ya funcionaba.
4. **Corrección:** una función canónica `resolve_identity_honoring_pin` decide la identidad de todo el CLI. `plan_adoption` es puro y sin disco, así que recibe el pin como dato (`AdoptionPlanInput.pinned_project_id`); un pin malformado **falla cerrado** en `validate_plan_input`. `.sddk/project-pin.json` a `.gitignore`: es identidad **por máquina** y versionarlo forzaría a todo checkout al `project_id` de quien commitea.
5. **5 tests nuevos**, uno por resolver que no tenía ninguna prueba con pin, **más uno de no-regresión** para el caso sin pin. **Falsificadores F49–F52 OBSERVED**, uno por resolver roto.
6. **Un falsificador descartado por ser una falsación:** el primer F49 se aplicó con 8 espacios de indentación sobre una línea de 4; el fichero no cambió y el e2e pasó "sin romper". Sólo cuenta tras verificar el fichero mutado.

**Gates:** `sddk-cli --lib` 859/0 · `project_pin_e2e` 4/0 · `adoption_identity` 3/0 · `sddk-engine -p sddk-cli` todo verde · fmt limpio · clippy `-D warnings` exit 0 · **workspace 5242 passed / 0 failed** (272 targets; cuadra con 5237 + los 5 tests nuevos, no con impresión).

**CONSECUENCIA DEL BLOQUEO DE RELEASE, ya no teórica:** el binario del PATH es `sddk 2.4.2` y **no contiene este fix**, así que en la CLI instalada el pin sigue sin surtir efecto y la autoridad sigue sin ver sus 65 ciclos hasta que se publique. Publicar v2.5.0 pasa de "tener la versión al día" a **desbloquear la autoridad operativa**.

7. **HALLAZGO QUE CORRIGE LA CAUSA DE TODO LO ANTERIOR — INC-DEBT-050 (critical/P1, open).** Al ejecutar el binario recién compilado con el pin, `adopt status` pasó a reportar `p-63676b11dc0ef88f` (el pin se honra) pero devolvió **`status: conflict`**. Inspeccionando el receipt encontré que **los dos receipts declaran el MISMO remote** y sólo se diferencian en la **caste del owner**: `…/Rubentxu/…` → `p-63676b11dc0ef88f` (65 ciclos) frente a `…/rubentxu/…` → `p-995939af668a53d8` (vacío), con **12 h de distancia el mismo día**. Reproducido exacto con el hash. **El remote NO cambió: cambió el código que lo normaliza** — commit `52182522` *"normalizar case del remote — case-change ya no forkea el ledger (D2)"*, **que sin migración forkeó precisamente lo que dice arreglar**. **MAGNITUD MEDIDA: 25 receipts de 104 (24%), 16 `project_id`, 13 repos remotos** de esta máquina. **Por qué nadie lo vio:** INC-DEBT-028 exige golden pin para el dominio del fallback seed, pero **el camino del remote no lo tiene** — `stable_project_id` no tiene ningún test que fije su salida. **Mi hipótesis anterior era errónea** y queda corregida en INC-DEBT-049: busqué "remote distinto" durante dos rondas de sondeo sin contrastar el receipt histórico, que estaba delante.

**SIGUIENTE dentro de C3m:** C3m.0 (ADR de significado canónico de KMT) · C3m.1 · C3m.3 · C3m.4 · C3m.5. **Pendientes de decisión:** INC-DEBT-050 (**critical**: migración de 25 receipts — destructiva, requiere operador; y el golden pin de fondo, que sí es una slice) · INC-DEBT-048 (binaria, norma) · INC-DEBT-049 parte abierta (SCOPE + ADR de contrato de estado) · `musl-gcc` (sudo del operador).

---

**Estado (session-62, 2026-10-01T13:00Z): C3m.2 corregido en código pero con DEUDA NORMATIVA ABIERTA (INC-DEBT-048). `AT-UAT-019` queda PASS PARCIAL, no PASS. Release v2.5.0 sigue BLOQUEADO por musl-gcc.** **SIGUIENTE dentro de C3m: C3m.0** (una sola definición canónica de KMT) · C3m.1 (invalidación incremental) · C3m.3 · C3m.4 · C3m.5. La decisión normativa de INC-DEBT-048 requiere autoridad, no un agente. **Y una segunda decisión de gobernanza:** INC-DEBT-049 (doble identidad de proyecto) espera autorización del operador antes de aplicar `sddk project pin`.

**Defecto medido (RED antes del fix):** el doc de `KnowledgeBasis::revise` afirmaba *"a new basis hash (because the `revised_at` participates in the hash)"*. La implementación hacía `derive_basis_hash(&new_basis.assertions)`, y esa función **sólo recibe `assertions`**: `revised_at` no tenía forma de participar. RED empírico: `revise(t=20)` con contenido idéntico daba el **mismo** `basis_hash` (`4de2152…` antes y después).

**Consecuencia que sobrevive a la spec:** `KMT::evaluate` compara los hashes **antes** que los timestamps, así que una revisión puramente temporal era **invisible al freshness** — devolvía `Fresh` sin mirar `revised_at`. La suite no lo cazaba porque el test existente (`..._revise_with_stale_time_is_rejected`) comprueba la **monotonía del tiempo**, que sí era correcta; la mitad del contrato que fallaba no tenía aserción.

**Hecho en session-62:**

1. `derive_basis_hash` → `derive_basis_hash_at(assertions, revised_at)`. `empty`, `insert` y `revise` pasan `Some(revised_at)`. Dominio **`v2`** con tag de versión: una identidad persistida v1 **falla cerrada** contra un basis v2 en vez de coincidir por accidente.
2. **4 tests nuevos** como *property-set*: el contrato del doc · dos revisiones distintas no colisionan · una basis sin tocar es distinguible de una revisada · el dominio v1 sigue reproducible y no colisiona con v2.
3. **Falsificadores F19 y F20 OBSERVED.** F19 (`revise` vuelve al hash legacy) → 2 FAIL. **F20 (que la derivación ignore el tiempo — el defecto raíz) → 4 FAIL**, los cuatro tests de identidad.
4. **Clippy encontró un defecto propio del refactor:** el wrapper `derive_basis_hash` quedó sin uso. No se dejó código muerto ni se silenció el lint: se eliminó.
5. **Impacto en datos, medido y no temido:** `grep basis_hash` y `grep KnowledgeBasis` en `sddk-storage` → **0 resultados**. `KnowledgeBasis` **no se persiste**, luego el cambio de dominio no invalida ninguna identidad almacenada. El tag `v2` es hoy prevención, no riesgo.

**Por qué NO se cierra (INC-DEBT-048, high/P1, open):**

- El cambio **contradice REQ-A3S1-021** de `arch-spec-A3-S1-knowledge-substrate.md` (`status: proposed`), que fija la derivación "from the sorted `(id, inner_basis_hash)` pairs", es decir **sólo del conjunto de assertions**. Código y spec discrepan ahora.
- **Matiz que corrige el diagnóstico previo:** el código **era consistente con la spec**; lo que mentía era el doc de `revise`. No era un descuadre docs/código sobre la derivación — los tres describían el mismo digest salvo el doc.
- **`AT-UAT-019` cita "el ADR de identidad", que no existe.** Verificado: no hay ADR de `KnowledgeBasis` (lo más cercano es ADR-0147, sobre otra cosa). Un criterio que remite a un documento inexistente no se puede cumplir ni incumplir honestamente.
- **Decisión normativa binaria, pendiente:** (a) actualizar REQ-A3S1-021 para incluir `revised_at`, o (b) revertir el cambio, corregir el doc de `revise` y resolver aparte que `KMT::evaluate` compara hash antes que tiempo. Cambiar una spec en `proposed` no corresponde a una slice de código.

**Gates:** `knowledge::` **24 passed / 0 failed** · `sddk-engine` completo **2376 passed / 0 failed / 11 ignored** (el cambio de dominio no rompió ningún consumidor) · **perfil completo del workspace 5237 passed / 0 failed / 23 ignored** · fmt limpio · clippy `-D warnings` exit 0 · `test_debt_index_coherence` PASS=10 FAIL=0 · `test_changelog_coverage` **PASS=14 FAIL=0**.

**Estado del release:** v2.5.0 **BLOQUEADO** (`rpm -q musl-gcc`: "el paquete musl-gcc no está instalado"). `release.sh --dry-run` se lanzó y **excedió 600 s** porque el dry-run ejecuta el perfil completo del workspace (pasos 0-8); es coste, no fallo del gate. Nada publicado, ningún tag, sin forzar glibc.

**Trazabilidad de la evidencia (no se re-presenta como prueba de este commit):** los gates de arriba se midieron sobre `7bbeee3d`, que es el commit de **código**. Este commit documental es posterior y **no toca `crates/`, `scripts/` ni shell tests**, así que el árbol ejecutable es idéntico al medido. Comprobable sin confiar en este texto: `git diff 7bbeee3d HEAD --name-only -- crates/ scripts/ tests/'*.sh'` debe salir **vacío** (el `RECEIPT.md` sí está bajo `tests/`, por eso el patrón es `tests/'*.sh'` y no `tests/`). Si no lo está, la evidencia de los gates **no** aplica a HEAD.

**Hallazgo de gobernanza posterior al push de C3m.2 — INC-DEBT-049 (high/P1, open): este repo tiene DOS identidades de proyecto vivas, y el CLI resuelve la que NO tiene historia.** `sddk adopt status` responde **`status: complete`** sobre `p-995939af668a53d8` (ledger de 380 KB, 6 referencias a eventos), mientras **65 ciclos** en `.sddk/cycles/` y **3.911.680 B** de ledger (173 referencias) viven bajo `p-63676b11dc0ef88f`, que el CLI no menciona. Causa con fecha y actor en el propio receipt de adopción: el **2026-09-30T19:29:34Z** (`actor: rubentxu`, `identity_source: remote`, remote `…/software-development-decision-kernel`, scope `.`) re-asignó el `project_id`, que es `hash(remote normalizado, scope)`. **Es un PASS falso** de la misma familia que C3l.7 e INC-DEBT-047, y golpea la premisa del proyecto: *la autoridad no ve su propia historia*. `sddk cycle status` arranca desde un storage vacío con un `complete` que invita a no mirar atrás. **Los datos NO se perdieron** — ambos storages están íntegros; se perdió el acceso. Agravante: los dos proyectos comparten `vault_path`, luego dos identidades escriben sobre el mismo vault (una autoridad canónica por concepto queda sin dueño). **Remedio local disponible y NO aplicado** (espera autorización del operador: cambiar la identidad autoritativa del repo es gobernanza, no una corrección de slice):

```bash
sddk project pin --root . --project-id p-63676b11dc0ef88f \
  --reason "remote renombrado: la identidad historica conserva 65 ciclos y 3.9 MB de ledger"
```

Lo que **sí** es defecto del repo es que ni `adopt status` ni `cycle status` declaren el historial existente bajo otra identidad — cero coincidencias en `crates/sddk-cli/src/`. No se corrige en una slice porque tocaría un contrato de estado que cambia lo que `sddk-cycle-resume` y `sddk-debt-verify` pueden asumir: necesita SCOPE y ADRs. Detalle completo, remedys y falsificadores F49–F52 en [`docs/debt/INC-DEBT-049-…md`](../debt/INC-DEBT-049-READOPTION-REASSIGNS-PROJECT-IDSILENTLY-ORPHANS-HISTORY.md).

---

**Estado (session-61, 2026-10-01T12:40Z): cerrado INC-DEBT-047 — el CHANGELOG declarado tenía que describir el trabajo, y ahora un gate lo exige antes del build. Release v2.5.0 sigue BLOQUEADO por musl-gcc, pero su contenido declarado ya es fiel.** **SIGUIENTE: levantar el bloqueo de musl y publicar v2.5.0, o bien C3m (si el operador lo prefiere antes del release).**

**Defecto encontrado y medido:** había **26 commits sin publicar** desde `v2.4.2` — incluida una slice `feat(architecture)` entera (C3l.7), X07, un `fix(roadmap)` y tres `test` — frente a una sección `## [2.5.0]` que listaba **dos** features, las que ya estaban cuando se commiteó el bump. `release-bump.sh --dry-run` confirma `no bump to derive: the workspace already declares the pending release (2.5.0)`: **el trabajo crecía detrás de una sección congelada sin ninguna señal**. Y `scripts/release.sh` no mencionaba `CHANGELOG` en ninguno de sus pasos.

**Por qué no era cosmético:** es **la misma forma de defecto que C3l.7, una capa más abajo** — un artefacto que no declara lo que es. El agravante: el release llevaba **tres sesiones detenido** en un paso físico (instalar `musl-gcc`), y en ese intervalo se acumularon dos slices enteras sin que nadie lo notara. Ningún guard lo delató porque no existía.

**Hecho en session-61:**

1. **`tests/test_changelog_coverage.sh`** (nuevo): la sección de la versión del workspace existe exactamente una vez, tiene contenido, y todo commit `feat`/`fix`/`test` desde el último tag está representado **por huella** — tipo + scope + las **4 primeras palabras del payload**, no mera presencia del tipo.
2. **Integrado como paso 2b de `release.sh`, antes del build**: un hueco detectado en el paso 9, después de `gh release create`, cuesta borrar un release; detectado aquí cuesta un commit. `--dry-run` lo salta por diseño.
3. **Falsificadores F17 y F18 OBSERVED.** **F18 es el que da valor al gate**: sustituir el payload por `feat(architecture): improvements to the architecture subsystem…` **conservando el scope** ⇒ exit 1. Sin la huella, un `feat` genérico habría satisfecho a cualquier otro `feat` del mismo scope.
4. **CHANGELOG de 2.5.0 actualizado** con los 8 commits reales, más una nota de que la release sigue bloqueada: una sección que describe un artefacto no publicado debe decirlo, no dejar que el lector lo infiera.
5. **Los dos tests de changelog no se solapan**: `test_changelog_merge.sh` prueba que el merge **no duplique** una cabecera; `test_changelog_coverage.sh` prueba que la sección **describa lo que se publica**. Son propiedades distintas y la segunda no la comprobaba nadie.
6. **Corrección de mi propio cierre de session-60:** escribí que "conformar el repo (eliminar waivers, implementar evaluadores) es **C5**". **Es INCORRECTO.** Leído `ROADMAP.md:124`, C5 es "evolución **condicionada a pruebas de valor** (P2/P3)" y su salida dice literalmente que "cada idea es DEFERRED hasta que el disparador y el SCOPE existan". Conformar el repo para cerrar AT-UAT-015 es **C3l.7 (que el UAT deja abierto) o C4**, no C5. Ninguno de los disparadores de C5 (X08, J7, J8, J9, R11) está activado.

**Gates:** `test_changelog_coverage` **PASS=11 FAIL=0** · `test_changelog_merge` PASS · `test_debt_index_coherence` **PASS=10 FAIL=0** · shellcheck limpio en el gate nuevo y en `release.sh` · `bash -n` limpio en ambos · `git diff --check` limpio. Perfil completo del workspace **no relanzado**: no se tocó código Rust (SUT = `CHANGELOG.md` + `release.sh` + gate shell), así que la evidencia de session-60 (5233 passed / 0 failed) sigue vigente.

**Límites:** el gate compara tipo, scope y 4 palabras, así que una reescritura profunda del subject puede dar un **falso negativo** (revisar de más), preferible al falso positivo (publicar de menos). Excluye `docs`/`chore` a propósito: listar cada sincronización de punteros es ruido. Compara contra el último **tag publicado**, no contra `origin/main..HEAD`: los commits que no entran en la release quedan fuera del contrato, que es lo correcto.

---

**Estado (session-60, 2026-10-01T12:05Z): C3l.7 CERRADA como slice — el architecture gate ya no puede certificar conformidad con deuda abierta. AT-UAT-015 sigue NOT PASS, y honestamente: el repo NO es conforme.** El release v2.5.0 sigue BLOQUEADO por el toolchain musl. **SIGUIENTE: C5** (conformar el repo: eliminar ARCH003/ARCH008 e implementar los 10 evaluadores), que es lo que realmente cierra AT-UAT-015.

**El supuesto del paquete para C3l.7 estaba obsoleto, y el defecto real era triple:**

1. **El test que certificaba el gate no lo ejecutaba.** `check_architecture_runs_against_repo` resolvía el binario en `target/{release,debug}/sddk` y hacía `return` si no estaba. El target dir de esta máquina es compartido (`/var/home/rubentxu/cargo-targets`), así que reportaba **`ok` en `0.00s`**. Auto-verde por `return`: el mismo patrón que C3l.4 corrigió en otro sitio, reintroducido aquí. **Un gate que no se ejecuta no puede certificar nada.**
2. **La afirmación era obsoleta.** Exigía `ARCH001` FAIL + exit 1. La edge `engine→storage` que la motivaba **ya no existe** (`sddk-engine/Cargo.toml` no depende de `sddk-storage`), y ARCH001 hoy es PASS. Con el binario presente, el test habría caído.
3. **El defecto estructural:** `RuleStatus = {Pass, Fail, Waived, NotApplicable}` no tiene `OPEN_DEBT`, y `Waived` y `NotApplicable` salían **ambos por el mismo camino a `exit 0`**. "Conformidad" y "no demostrado" eran literalmente el mismo código, que es como un repo con deuda arquitectónica abierta podía citarse como conforme.

**Hecho en session-60 (C3l.7):**

- **Nuevo `sddk_domain::rules::verdict`:** veredicto tipado y agregado `Conformant` / `OpenDebt` / `Waived` / `NotEvaluated`, con `is_conformant()` **true sólo para `Conformant`**. El claim de conformidad no está disponible por defecto: hay que ganárselo. Entrada vacía ⇒ `NotEvaluated`, para que un fichero de reglas vacío no pueda acuñar conformidad. Un `Fail` de error entre varios `Pass` **domina**: no se promedia.
- **No se añadió variante a `RuleStatus`** (21 consumidores): esto es una pregunta agregada, no un outcome por regla.
- **Exit codes:** `0` sólo `Conformant`; `1` `OpenDebt` (contrato histórico intacto); **`2` nuevo** para `Waived`/`NotEvaluated`, que un consumidor fail-closed distingue de una violación probada. El JSON `--out` lleva `verdict` además de `exit_status`.
- **El gate sobre el repo real:** antes `EXIT=0`; ahora `VERDICT: WAIVED (exit 2)` + *"This gate does NOT certify architectural conformance"*.
- **Test reparado y movido** de `src/dev/tests/` a `tests/`: `CARGO_BIN_EXE_sddk` **no** existe en un test unitario del crate, y esa resolución ambiental era parte del bug. Sin `skip` (si el binario falta, el test falla), root resuelto desde `CARGO_MANIFEST_DIR` (un test de integración corre en `crates/sddk-cli`, no en la raíz), y **3 fixtures con `schema_version: 1.0.0`** cuando el loader exige `1.2.0` — que el skip ocultaba.
- **Falsificadores F15 y F16 OBSERVED.** F16 es el decisivo: reponer el `skip` + el path inexistente devuelve **`finished in 0.00s`**, el patrón exacto que delató D1.
- **Clasificación deliberada de `WarningThenRatchet`** (una tercera severidad que no aparece en la lectura inicial del paquete): se trata como warning, porque bloquear sobre una violación que esa severidad existe para *congelar en el sitio* impediría que el código nuevo cumpla, que es su propósito declarado.

**Gates:** `verdict` 11/11 unitarios · `check_architecture_gate` 4/4 en **0.67s** (antes 0.00s sin ejecutar) · `context_fitness` 7/7 (contrato cross-crate de módulos root) · fmt limpio · clippy `-D warnings` exit 0 · **perfil completo 5233 passed / 0 failed / 23 ignored**.

**Límites:** cerrar la slice **no** cierra AT-UAT-015. Su criterio es "0 error no-waived o waiver vigente tipado", y el repo tiene 2 waivers vivos (ARCH003, ARCH008) y 10 evaluadores sin implementar ⇒ veredicto `WAIVED`. **Conformar el repo es C5.** `Waived` tampoco tipa el expiry en el estado (el expiry por ancestry ya existe en el evaluador, pero no viaja en el veredicto). Recibo: `tests/cycle-artifacts/p-63676b11dc0ef88f/session60-c3l7-architecture-gate-verdict/RECEIPT.md`.

---

**Estado (session-59, 2026-10-01T11:50Z): INC-DEBT-046 CERRADA — el puntero de estado es legible por máquina y tiene una sola clave autoritativa. Reconciliada además una divergencia de git con `origin/main`. El release v2.5.0 sigue BLOQUEADO por el toolchain musl (verificado en vivo hoy).** Todo commiteado y **pusheado**: `origin/main == HEAD 6a3e5f4c` (divergencia 0/0). **SIGUIENTE PASO: C3l.7** (architecture gate, AT-UAT-015), que cierra la vía C3l y desbloquea C3n.

**Hallazgo de entrada (no asumido):** `origin/main` contenía `86f2aad7` "chore(release): bump version" que la rama local **no** tenía. `f78a8bf2` era el ancestro común y **los dos bumps eran de contenido idéntico** (`Cargo.toml`/`CHANGELOG.md`/`manifest.toml` con diff vacío) — el remoto salió del step 1c de `release.sh`, el local del cierre de session-57. **Resuelto con `git rebase origin/main`:** historia lineal, `86f2aad7` ahora ancestro, Git descartó el bump redundante, y el árbol final es **idéntico** al HEAD de session-58 (`git diff --quiet` sin diferencias: cero bytes perdidos). No era un defecto del hook: eran dos ejecuciones del paso 1c sobre ramas hermanas.

**Hecho en session-59 (cierre de INC-DEBT-046):**

1. **`STATE.yaml` no lo parseaba ninguna máquina.** Una clave `development_head` (session-46b) con **3 espacios** de indentación hace que `yaml.safe_load` aborte sin leer el resto del documento. **No lo introdujo session-58**: se rompió al menos en session-46b y nadie lo vio porque estos punteros se leen a ojo. Una máquina recibía un error, no un estado.
2. **`development_head` estaba 9 veces duplicada.** En YAML la clave repetida no es error: **gana la última**. Una vez arreglada la indentación, el puntero vigente que leía una máquina era el de **session-45**, cuatro sesiones atrasado, mientras `CURRENT.md` sí era correcto.
3. **Cierre con las cuatro condiciones medidas:** puntero resuelto == sesión actual · `development_head` == 1 (era 9) · `superseded_development_head` == 1 clave con los **8 valores previos recuperables** · cero claves perdidas en `source:` (key sets idénticos). **Falsificador OBSERVED**: reintroducir el espacio reproduce el `ParserError` exacto.
4. **Error propio declarado:** mi primera consolidación **creó** una segunda clave `superseded_development_head` en vez de concatenar a la preexistente, con lo que la clave antigua ganaba y el índice salía vacío. Lo detecté al **recuperar** los valores (0 en vez de 8), no al escribir. Es el mismo modo de fallo que la INC documenta: una aserción declarada sin comprobación.
5. **Sin pérdida de historia:** las 8 sesiones (45/46/46b/48/54/55/56/57) conservan entrada completa en `SESSION-JOURNAL.md`; el campo consolidado queda como **índice**, no como fuente.

**Auditoría de deuda ejecutada (no heredada):** parseados los **79** ficheros `INC-*.md` → `closed 60 · resolved 11 · open 8`. Severidad máxima abierta era esta misma INC. Las 7 restantes: 6× `INC-AUDIT-S14` (medium/low, remediación diferida a C5 o marcadas "no es un bug" en su propio cuerpo) + `INC-DEBT-039` (low/P3, degradada en session-46, pendiente del primer run real). **Criterios verificados individualmente** — la regla de que "alerta sin verificar no es deuda" se cumple medida, no asumida.

**Estado del release (sin cambio, verificado hoy):** `x86_64-linux-musl-gcc` **ausente** ("el paquete musl-gcc no está instalado"); target Rust musl presente. `v2.5.0` sigue **BLOQUEADO** en el step 3/14, sin publicar. No se forzó glibc. Remedio (requiere sudo del operador): `rpm-ostree install --idempotent musl-gcc` + reboot, luego `bash scripts/release.sh` sin más cambios.

---

**Estado (session-58, 2026-10-01T11:05Z): C3l.6 COMPLETADO — X07 cruza la frontera de proceso/binario real (AT-UAT-013/014 PASS). AIW-S8 sigue SIN VERIFIED: la vía C3l necesita C3l.7. El release v2.5.0 sigue bloqueado por el mismo toolchain musl (sin cambio en §Bloqueo).** **SIGUIENTE PASO: C3l.7** (architecture gate, que cierra la vía C3l y desbloquea C3n), salvo que el operador levante antes el bloqueo de musl.

**Hecho en session-58 (C3l.6):**

1. **El defecto era de la frontera, no del aserto.** `aiw_s8_x07_second_binary_integration.rs` documenta *"a second consumer process"* y su `run_writer` devuelve un **path**: el test abre un **segundo `Storage` en el MISMO proceso**. Dos handles, un proceso. Comparten memoria, código compilado y PID — prueba que dos conexiones coexisten, no que un ejecutable aparte pueda leer lo que otro escribió.
2. **El consumidor es el binario `sddk` real** (`CARGO_BIN_EXE_sddk`), lanzado como proceso hijo, con el ledger redirigido por `SDDK_DATA_DIR`/`SDDK_STATE_HOME`. **Ningún binario nuevo** — el paquete lo prohíbe si una superficie CLI real cubre la lectura, y la cubre (`ledger events`, `ledger verify`, `cycle status`, `project resolve`). El test vive en `sddk-cli/tests/` porque es el único sitio con `CARGO_BIN_EXE_sddk`. Los 4 tests de storage **no se tocaron**.
3. **GREEN 6/6** (D0 proceso · D1 identidad · D2 ciclo/eventos · D3 schema guard · D4 bytes · D5 write fail-closed).
4. **El falsificador central NO mordía, y eso reveló un defecto de diseño mío.** F12 (sustituir el binario por un handle in-process) dio `4 passed; 0 failed` con el binario completamente ausente. Causa: la aserción de bytes compara *antes* y *después*, y un handle in-process **tampoco escribe** — sólo prueba "nadie escribió", no "otro proceso lo hizo". **Corrección: D0**, que afirma que el consumidor es un ejecutable distinto del binario de test y que su **PID difiere del PID del test**. Con D0, F12 muerde con el mensaje exacto del defecto original: `no boundary crossed`. Lección transferible: *byte-equality demuestra no-escritura, no ejecución-por-proceso.*
5. **Falsificadores F11–F14 todos OBSERVED.** F11 necesitó un segundo intento: la primera versión escribía con la conexión abierta y no mudou los bytes — el storage usa **WAL**, así que la escritura vive en `ledger.sqlite-wal` y sólo alcanza el fichero principal en el checkpoint. "No muerde" por razón mecánica, no por aserto débil; corregido cerrando el handle.
6. **Límite declarado (medido, no supuesto):** D4 afirma quietud del **fichero principal**, no del directorio. El binario abre el ledger en escritura (`SqliteLedgerFactory::open_ledger` → `Storage::open`; `RuntimeContext::open` además construye el engine con `Storage::open`), lo **crea si falta**, y los sidecars `-wal`/`-shm` aparecen y desaparecen. El contenido no cambia; el directorio sí se toca. No se maquilla como "read-only".

**Límites:** verifica X07, no certifica "dos CLIs de producción" ni el instalador/bundle; usa el binario de debug de cargo (el release está bloqueado), que es el mismo código. Recibo: `tests/cycle-artifacts/p-63676b11dc0ef88f/session58-c3l6-x07-second-binary/RECEIPT.md`.

---

**Estado (session-57, 2026-10-01T10:40Z): BLOCKED en `release.sh` step 3/14 — falta el compilador C `x86_64-linux-musl-gcc`. NO se publicó nada, y esa es la conducta correcta.** Detalle en [`docs/architecture/adrs/BLOCKER-MUSL-TOOLCHAIN-MISSING.md`](../architecture/adrs/BLOCKER-MUSL-TOOLCHAIN-MISSING.md).

**Trabajo verificado y commiteado, listo para salir en cuanto el bloqueo se levante:** `7360c32e`…`d3988a5e` (C3l.3+C3l.4+C3l.5) · `5a6f155f` (`test(push)`: repara el caso fail-closed que preguntaba al repo equivocado) · `f78a8bf2` (registra INC-DEBT-045, reindexa INC-DEBT-044) · `86f2aad7` (`chore(release): bump version` → workspace `2.5.0`, HEAD con el subject que exige el step 0).

**El bloqueo:** target Rust musl **instalado**, pero el compilador C `musl-gcc` **ausente** (`ring v0.17.14` lo necesita). Distro **Bazzite 44** (Fedora inmutable); `sudo` **requiere contraseña** ⇒ intervención del operador. Remedio: `rpm-ostree install --idempotent musl-gcc` + reboot, luego `bash scripts/release.sh` sin más cambios.

**NO se usó `SDDK_RELEASE_BUILD_TARGET` para forzar glibc.** El propio script lo prohíbe y lo califica de *"reintroduce INC-021"*: el asset se llama musl y `install.sh` lo reparte como musl, así que publicar un binario glibc con ese nombre **es la misma mentira que INC-DEBT-021 documentaba**.

**Gates ya verdes** (los que fallaron en intentos anteriores, resueltos): workspace green · los 8 shell contract tests, incluido `test_push_prevention_hook.sh` con `PASS=48 FAIL=0` y falsificador F10 OBSERVED · `test_vault_adr_mirror_coverage.sh` con 53 ADRs espejados e idempotente · `debt_index_coherence` PASS=10 FAIL=0.

**Tras el desbloqueo:** el release sale como `v2.5.0` sin cambios adicionales (el bump ya está commiteado y la versión es la correcta). Después, **C3l.6** (X07 segundo binario real), luego C3l.7 (architecture gate, que cierra la vía C3l y desbloquea C3n).

---

**Estado (session-56, 2026-10-01T10:05Z): C3l.5 COMPLETADO — X04 cruza la frontera multi-proceso real (AT-UAT-011/012 PASS). ADRs escritos para los 2 módulos root nuevos.** **SIGUIENTE PASO: C3l.6** (X07: segundo binario real). Después C3l.7 (architecture gate), que cierra la vía C3l y desbloquea C3n.

**Pre-flight ejecutado (no asumido).** Índice de deuda curado (44 de 77 ficheros ausentes del índice), así que parseé el frontmatter de los 77: **0 critical/high abiertas**; las 7 `open` son S14 ancient y sus propios criterios dicen *"no es un bug"*, *"no borrar"*, *"la recomendación queda anulada"*. **INC-DEBT-028** (único high/P1, `status: fixed` — valor **fuera del vocabulario canónico**, 1 caso de 77) verificado OBSERVED hoy: 3 invocaciones de `sddk project resolve` sobre repo sin remote → mismo `project_id`. **No era deuda**; normalizado `fixed → resolved` con la evidencia registrada.

**Hecho en session-56 (C3l.5):**

1. **El exit gate era inalcanzable por construcción:** `impl LeaseStore` existía **una sola vez**, para `InMemoryLeaseStore`. El doc del propio trait declaraba una intención inexistente. **Causa raíz:** el puerto vivía en `sddk-engine` cuando el patrón canónico del repo es puerto en `sddk-domain` + impl en `sddk-storage` (como `Ledger`).
2. **Puerto movido a `sddk-domain::ports`** (con re-export desde el engine: cero consumidores rotos) + nuevo `SqliteLeaseStore` con `BEGIN IMMEDIATE`, rollback del perdedor y escalera de `busy_timeout` heredada de INC-DEBT-029.
3. **Defecto real encontrado por el test nuevo:** `release` hacía `DELETE` y **destruía el contador de fencing** — tras una release el siguiente acquire reemitía token 1 ya usado, con lo que un holder obsoleto pasaba por vigente. Corregido: el fencing es monotono **por ciclo**, no por lease.
4. **GREEN 8/8** con ≥2 PIDs **afirmados** (`std::process::id()`, con test propio que falla si coincide) y reloj **real** (un `MockClock` haría G5 vacuo). Los 4 tests W0x antiguos intactos: prueban semántica intra-proceso, que sigue siendo válida.
5. **Falsificadores:** F6 → 1 FAIL · F8 → 5 FAIL · **F9 (estado en memoria) → 8/8 FAIL, decisivo**. **F7 (sin retry) → 0 FAIL: no mordió**, y queda **declarado** en el recibo en vez de disfrazado — con 6 procesos, `busy_timeout` solo absorbe la contención, así que la escalera de retry no queda probada como load-bearing a ese nivel.

**Regresión encontrada y corregida (por el perfil completo, no por testing scoped):** `no_new_root_level_context_module_without_adr` FALLÓ. Dos módulos root nuevos en `sddk-engine/src` (`dynamic_expansion.rs` de session-54 y `ext_outcome.rs` de session-55) sin ADR. **No la cazó el testing quirúrgico** porque `context_fitness` vive en `sddk-cli`: la regla "solo tests afectados" funciona para el SUT pero **deja pasar contratos cross-crate**. Resuelto por la vía que el propio test exige (ADR-0148 y ADR-0149), **no** inflando el baseline. Lección registrada en el recibo.

**Límites declarados:** esto **verifica X04**, no certifica "dos CLIs de producción" — los procesos son el binario de test re-ejecutado, no `sddk` como dos invocaciones. **X07 sigue NOT_VERIFIED**, así que `AIW-S8` no pasa a VERIFIED. `agent_leases` es tabla nueva y propia: un lease de agente puede existir para un ciclo que aún no está en el ledger, así que no reutiliza `cycle_leases` (que tiene FK a `cycles`).

---

**Estado (session-55, 2026-10-01T09:15Z): C3l.4 COMPLETADO — la ausencia de un provider externo nunca vuelve a reportarse como PASS (AT-UAT-009 PASS, AT-UAT-010 BLOCKED).** **SIGUIENTE PASO: C3l.5** (X04: dos CLI / concurrencia real multi-proceso `SQLITE_MULTI_PROCESS`). Después C3l.6 (X07 segundo binario real) y C3l.7 (architecture gate, que cierra C3l y desbloquea C3n).

**Hecho en session-55 (C3l.4):**

1. **RED medido con su control.** `aiw_s5_chronos_real.rs` usaba `None => return`; con `CHRONOS_MCP_BIN` ausente el run reportaba `3 passed; 0 failed; 0 ignored` en **`0.00s`** — dos de tres tests sin ejecutar. El contraste es la mitad del hallazgo: `aiw_s1_cognicode_real.rs`, **en el mismo entorno**, ya reportaba `3 ignored` porque usaba la convención correcta. El defecto estaba aislado a 2 sitios y la solución ya existía en el repo.
2. **Cinco estados tipados** (`ext_outcome`) con la invariante load-bearing: `is_pass()` es `true` **solo** para `PassObserved`, y **resolver un binario nunca devuelve un pass** — resolver es precondición, no observación. Sin esa distinción, "encontré el binario" se degrada en "el contrato se sostiene".
3. **Launcher con contrato de exit codes** (0 pass / 1 fail / 2 blocked / 3 not_run) + recibo con path/sha256/version/capabilities. Un provider resuelto cuyo perfil falla es `fail_observed`, **nunca** `blocked` — colapsarlos es cómo una regresión real se reporta como problema de entorno.
4. **GREEN en las dos direcciones:** run ordinario `2 passed; 2 ignored` (honesto); perfil EXT pedido sin provider `0 passed; 2 FAILED` (falla fuerte en vez de fingir).
5. **Falsificador M5 OBSERVED:** restaurado el patrón defectuoso, el run vuelve a `4 passed; 0 ignored` en 0.00s. Exit gate load-bearing. `grep "None => return"` en tests de integración externa → 1 hit, y es el comentario que documenta el defecto.
6. **Quick win del mismo commit:** `clientInfo` de Chronos informaba `"0.1.0"` congelado mientras el adapter CogniCode ya usaba `env!("CARGO_PKG_VERSION")`.
7. **Gates:** `ext_outcome` 4/0 · fmt limpio · clippy `-D warnings` exit 0 · shellcheck limpio · engine lib **1362/0/1** (era 1358) · suite completa 0 fallos.

**Límites declarados:** **AT-UAT-010 NO es un PASS** — `chronos-mcp` no está instalado; la semántica está implementada y falsificada pero la captura real sigue sin observarse. El pin enum↔launcher es parcial (el test lee el script, no prueba que lo emita). **`ProviderKind::Null` NO se toca**: el falso verde por *tipos* queda abierto y lo cierra la consolidación provider/capability (**C3m.3** + **C3m.5**), no este slice.

**Corrección de un error propio:** en el turno anterior afirmé que el ADR de significado canónico de provider/capability era **C3m.0**. Es incorrecto — **C3m.0 es el ADR de KMT** (Knowledge Merkle Tree). Lo es **C3m.3** (runtime provider-neutral provenance) y **C3m.5** (R0 bounded-context decision).

---

**Estado (session-54, 2026-10-01T08:20Z): C3l.3 COMPLETADO — la vertical de Dynamic Workflow Expansion existe como superficie de producción (AT-UAT-006/007/008 PASS).** Workspace `2.4.2` (= último tag publicado `v2.4.2`), `HEAD` con el fix commiteado. **SIGUIENTE PASO: C3l.4** (External test semantics: ausencia ≠ PASS — los tests Chronos pueden salir verdes vía `None => return`). Después C3l.5 (X04 concurrencia real `SQLITE_MULTI_PROCESS`) y C3l.6 (X07 segundo binario real).

**Hecho en session-54 (C3l.3):**

1. **El defecto no era una composición faltante, era una superficie.** `aiw_s4_dynamic_expansion.rs` llamaba `Engine::cycle_replan` directo: sin identidad de trigger (recibe `event_id` del caller), sin validación de authority sobre `plan_revisions`, sin `PlanRevisionV1` y sin noción de nodos nuevos. Y **W02 fijaba el defecto como contrato** (`replan_count == 2`, *"the contract is bounded counter, not dedup"*) — el mismo patrón que C3l.2 ya había prohibido para S7a. W02 reetiquetado como frontera medida, sin tocar su lógica.
2. **Hallazgo de divergencia (D3):** el append canónico deduplica por `event_id` (`INSERT OR IGNORE`), pero `update_cycle_with_event` ejecuta el `UPDATE cycles` incondicionalmente ⇒ un replay podía dejar el evento sin duplicar y `replan_count` re-incrementado. El guard de idempotencia corre **antes de cualquier mutación**, lo que cierra esa divergencia por construcción.
3. **Vertical implementada** (`sddk-engine::dynamic_expansion`): evidence gap → proposal de Secretary → decisión de orchestration → authority sobre `WritableSurface::PlanRevisions` → delta tipado → `PlanRevisionV1` N+1 parentado en el **tip real del ledger** → selección incremental (solo nodos ausentes del plan base) → recibo atómico. El linaje se reconstruye desde los eventos, así que sobrevive al restart.
4. **Evidencia:** RED por superficie ausente (`E0432`/`E0599`) → GREEN **12/12**. **4 falsificadores OBSERVED** sobre la producción mutada: guard de replay (5 FAIL), authority (2 FAIL), pin de base (1 FAIL), selección incremental (6 FAIL). W01..W11 intactos 7/7. fmt/clippy limpios.
5. **Matriz:** S4 `IMPLEMENTED_NOT_VERIFIED` → **IMPLEMENTED → re-verificable**. AT-UAT-006/007/008 PASS.

**Límites (declarados en el recibo y en el SCOPE-CONTRACT antes de escribir código):** lo registrado es la **selección y contabilidad** de nodos despachados, no la evaluación de operadores (DW-RUNTIME-003/004/005, fuera de scope del propio compiler) — por eso S4 **no** puede pasar a VERIFIED. La selección incremental usa el `base_ir` que declara el propio trigger: un trigger deshonesto podría sobre-despachar (mitigado, no cerrado). La revisión raíz es sintética y content-derived, porque `WorkflowManifest` no contiene un `WorkflowIR` (el substrate de plan-revision nunca estuvo unido al ciclo).

**Error de medición propio declarado:** el primer falsificador de authority gateó el check detrás de una env var que no se activaba — mutación **nula**, 12/12 PASS. Casi se concluye que el check no era load-bearing; repetido como eliminación literal, cayó.

**Higiene cerrada:** `docs/ROADMAP-ACCEPTANCE-TRUTHFULNESS.md` eliminado (duplicado byte-idéntico, sha256 `f06b9fb5…` en ambas; la canónica del paquete está commiteada). Una sola fuente.

**SIGUIENTE PASO (preciso):** **C3l.4** — semántica de tests externos: ausencia de evidencia no es PASS. Congelar el contrato de qué cuenta como `NotObserved` antes de tocar los tests Chronos.

---

**Estado (session-53, 2026-10-01T00:40Z): C3l.2 COMPLETADO — Producer→L0 wiring real (AT-UAT-004/005 PASS), release 2.4.2 en curso.** HEAD con el fix + docs. **SIGUIENTE PASO: C3l.3** (Dynamic Workflow Expansion E2E real — AT-UAT-006/007/008: proposal→authority→PlanRevision→execution + replay idempotente). Después C3l.4 (semántica EXT).

**Hecho en session-53 (C3l.2):**

1. **Fix:** `ProducerToL0Adapter` compone `Arc<SecretaryL0Engine>` (opción 2 del paquete); `with_engine(Arc, now_ms)` como ruta pública; `new()/with_now()` conservan engine fresco. Restricciones respetadas (sin authority, sin reglas hardcodeadas, Unknown silencioso, determinismo+cooldown).
2. **El test S7a antiguo fijaba el defecto como esperado** (`assert!(signals.is_empty())`) y reconstruía el evento a mano — prohibido por C3l.2. Reescrito como falsificador real: register rule → dispatch(real event) → señal esperada, sin reconstrucción.
3. **Exit gate como test propio:** `exit_gate_fresh_engine_cannot_fire_registered_rules` — engine vacío inyectado ⇒ 0 señales.
4. **Evidencia:** aiw_s7a 5/5 (RED antes: `with_engine` inexistente), gateway 133/0, fmt/clippy limpios. Matriz S7a → re-verificable; AT-UAT-004/005 PASS.

**Límites:** el cooldown ahora persiste entre dispatches del mismo adapter (engine persistente) — diseño pretendido; los tests de silencio usan engines frescos y siguen válidos.

**Hecho en session-52 (C3l.1):**

1. **Fix tipado `22459708`:** `ChallengeError` ya no se traga. `StrategyFailure { strategy_id, reason }` + `ReconciliationSummary::Incomplete { failures }`; `ConfirmedBaseline`/`AcceptedDebt` inalcanzables con fallos; señales reales dominan (falla+contradicción⇒Contradiction); `strategies_run` = ejecuciones completadas (Ok), no aplicables.
2. **Falsificadores C3l.1 en el suite para siempre** (`debverify_kernel/tests.rs::c3l1_falsifiers`, 6 tests): RED observado antes del fix (tipos ausentes), GREEN 34/34 después; engine **1358/0**; fmt/clippy limpios; sin consumidores exhaustivos del summary en producción (grep previo).
3. **Matriz:** R6 → IMPLEMENTED→re-verificable (C3n.2 re-ejecuta los falsificadores ya vivos en el suite); AT-UAT-002/003 PASS.

**Límites:** `ChallengeError` hoy solo tiene `MissingInput` — nuevos variantes de error requerirán mapeo a `reason` tipado (el match en `reconcile` es exhaustivo y fallará a compile time si se añade uno sin tratarlo: correcto por diseño).

**Hecho en session-51 (C3l.0, slice documental PURE — sin cambios de código):**

1. **Matriz congelada:** [ACCEPTANCE-TRUTHFULNESS-MATRIX.md](ACCEPTANCE-TRUTHFULNESS-MATRIX.md) — 21 filas (AIW-S0..S8 + S1b + R0..R11) con requirement → implementación → **frontera realmente ejercitada** (`boundary_class`) → test → evidencia → status PRE → status POST → trigger. Enlazada desde ROADMAP C3l.0 y AT-UAT-001. Rutas de evidencia citadas verificadas existentes.
2. **Re-clasificaciones (solo las mandadas por el paquete):** R6→IMPLEMENTED · S7a→NOT_VERIFIED · S4→IMPLEMENTED_NOT_VERIFIED · S5→IMPLEMENTED · S8→NOT_VERIFIED (claims multi-proceso) · R2/R4-snapshot/R5-invalidación/R8 parciales por C3m.x · claim «architecture conformant» NO VÁLIDO hasta C3l.7.
3. **Sin re-clasificar (regla):** AIW-S2/S3/S6/S7b/c/S1b, R1/R3/R9/R11 — sin defecto declarado. Receipts históricos intactos; AIW-S1/R7 mantienen VERIFIED por su SHA (la frontera MCP_EXTERNAL sí se cruzó).
4. Exit gate C3l.0 cumplido: la matriz responde, para cualquier hito, qué frontera se observó realmente sin leer el nombre del test.

**Higiene pendiente (decisión del operador):** `docs/ROADMAP-ACCEPTANCE-TRUTHFULNESS.md` (copia idéntica en raíz docs/ del roadmap ya commiteado dentro del paquete) sigue sin trackear — recomendado eliminarla (una sola fuente).

**SIGUIENTE PASO (preciso):** **C3l.1 DebVerify fail-closed** — hacer que `DebVerifyKernel::reconcile` respete `ChallengeError` con invariante `strategy_error ⇒ summary != ConfirmedBaseline` (`ReconciliationSummary::EvidenceGap/Incomplete`, sin scores ni booleanos ambiguos); falsificadores declarados en el paquete (una estrategia falla / todas / una falla y otra sin findings / una falla y otra con contradicción / `strategies_run` cuenta ejecuciones completadas). Después C3l.2.

---

Previous: **Estado (session-50, 2026-09-30T22:15Z): C3j objetivo 4 (`context expand`) implementado + INC-DEBT-044 resuelta + ROADMAP REENFOCADO (C3l/C3m/C3n adoptados).** Workspace `2.3.3`, release en curso (`feat → MINOR → 2.4.0`). **SIGUIENTE PASO: abrir C3l.0** (re-clasificación honesta del baseline, AT-UAT-001) — prioridad P0 del paquete `docs/sddk-roadmap-acceptance-truthfulness-2026-09-30/`; C3j continúa en paralelo.

**Hecho en session-50:**

1. **Reenfoque de roadmap (directiva del operador):** insertados **C3l/C3m/C3n** en ROADMAP.md con la regla de promoción (nada que reclame AIW/Context-First/runtime-enhanced/dynamic-expansion/Secretary/arquitectura-conforme puede ser CERTIFIED sin cerrar C3l/C3m/C3n aplicables); alta de **AT-UAT-001..026** con columna `Boundary`. El paquete no reemplaza C3j.
2. **`sddk context expand` (feat, `1ae2f6bf`)** — C3j objetivo 4: progressive disclosure mínima; contenido desde el **ledger**, no de la prosa de la capsule; read log durable por sesión con sha256; fail-closed tipado en todas las direcciones (ref desconocida LISTA las disponibles). **CTX-UAT-015 PASS** (`tests/uat_ctx_007_context_expand.sh`, 27 ok / 0 FAIL / exit 0); CTX-UAT-014 mitad observable PASS (el envelope del bootstrap no vuelca la capsule).
3. **INC-DEBT-044 (high/P1, resolved, `cb4ea598`)** — destapada por el UAT: las capsules de **ciclos reales** (id con barra) NUNCA se escribían a disco (`persist` tragaba el fallo); el bootstrap decía `compiled` con `capsules/` vacío. Nadie lo vio: todos los tests usaban ids sin barra. Fix: percent-encode en `file_name_for`; RED-first pinneado; engine 1352/0.
4. **Falsadores OBSERVED:** unitario 2/2 RED (prosa en vez de ledger + log suprimido); UAT 3 FAIL exit 1 contra binario mutado (exactamente contenido-ledger ×2 + read log); pre-feature RED contra `v2.3.3` publicado.
5. **Incidente de método declarado:** falsar sobre código sin commitear costó la implementación (recuperada del contexto de sesión y reverificada 30/0). **Lección: commitear el verde antes de mutar.**

**Límites:** CTX-UAT-014 parcial (presupuesto de tokens pendiente); `persist` fire-and-forget por trait (residual 044); CTX-UAT-007..012 fila-a-fila pendiente del operador; expand soporta work-item/decision/cycle (paths de recovery capsules no soportados, declarado).

**SIGUIENTE PASO (preciso):** (a) publicar 2.4.0 (bump derivado, flujo canónico, gates 9b/9c); (b) **abrir C3l.0**: congelar la matriz `requirement → implementation → boundary realmente ejercitada → test → evidence → status` para AIW-S0..S8 y R0..R11, re-clasificando solo claims afectados (VERIFIED→IMPLEMENTED/NOT_VERIFIED, PASS→NOT_RUN) sin reescribir evidencia histórica; primero C3l.1 (DebVerify fail-closed) y C3l.2 (Producer→Secretary L0) según el paquete.

---

Previous: **Estado (session-49, 2026-09-30T21:10Z): C3i CERRADO SIN UAT ABIERTAS + INC-DEBT-043 RESUELTA + RELEASE v2.3.3 PUBLICADO.** Workspace `2.3.3`, tag `v2.3.3` peel `f2e6efe0` == `origin/main`, `sddk 2.3.3` instalado. **SIGUIENTE PASO: abrir C3j** (desbloqueado por primera vez), empezando por el objetivo 3 paso 7 (hipermedia) y las filas CTX-UAT-007..012/015.

**Hecho en session-49 (WorkItem W1 = cerrar CTX-UAT-005 y MIG-UAT-001, las dos últimas UAT de C3i):**

1. **Las dos filas eran `NOT_RUN` por una premisa incorrecta, y la premisa se verificó antes de tocar código.** Decían «requiere dos versiones de skill conviviendo; sin release nuevo». El enunciado de origen (paquete hypermedia, `06-uat/UAT-MATRIX.md:18`) es *"caller legacy pasa cycle ID explícito → explicit vence inference"*: un contrato del **runtime**, no del texto de la skill, ejercitable contra un binario y un ledger. Y ya había release publicado. **Segunda vez en dos sesiones que una "puerta" resulta ser un script que faltaba** (la primera: CTX-UAT-002/003, session-48).
2. **HALLAZGO PRINCIPAL — INC-DEBT-043 (high/P1, resolved).** Al validar MIG-UAT-001 se observó que `context bootstrap --cycle <id inexistente>` devolvía `state: "explicit"` con ese id y **`binding_written: true`**: un envelope con forma de resolución y un **binding durable apuntando a un ciclo que no existe**. La superficie hermana `sddk cycle status --cycle` sí fallaba cerrado (`STORAGE_NOT_FOUND`) — y la skill documenta ambas en el **mismo** envelope `cli_context`. Clasificación honesta: **pre-existente desde `aff0a498`** (session-40), no regresión de session-46. **Por qué nadie lo vio:** las dos fixtures explícitas usaban ids que nunca se insertaban en el ledger, así que no podían distinguir «enlaza una referencia» de «enlaza una ficción».
3. **Resolución:** `ContextBootstrapError::CycleNotFound` + `Storage::cycle_exists` en la frontera del binding (paso 6), **después** de resolver la basis → exit 1, sin envelope, sin binding. El ciclo resuelto **por inferencia** queda exento (el resolver ya lo leyó de una lease viva). Blast radius acotado al binding: un ciclo con capsule durable sigue reconectando igual.
4. **CTX-UAT-005 PASS** (`tests/uat_ctx_006_skill_runtime_alignment.sh`, 7 secciones, 36 aserciones). Lo nuevo: los 3 estados en **un binario y un ledger**, y la ejecución real de las **acciones de recovery que la skill nombra** (desde 0 `cycle start`; desde N, elegir un candidate de la lista que emitió el runtime).
5. **MIG-UAT-001 PASS** (`tests/uat_ctx_005_explicit_cycle_migration.sh`, 7 secciones, 34 aserciones). Explícito vence a la ambigüedad, identidad/workspace idénticos, y **sin pérdida de contexto demostrado por comparación explícita** (la ruta ambigua no entrega capsule; la explícita sí).
6. **Falsadores observados:** unitario 2/2 RED con la variante de error presente (el check es load-bearing, no la firma). E2E: `uat_ctx_005` **8 FAIL** y `uat_ctx_006` **5 FAIL, 7/7 secciones, exit 1** contra binario mutado; ambos PASS tras revertir. Detector adicional: `uat_ctx_005` contra el **binario publicado v2.3.2** da 8 FAIL de los cuales **4 exactamente** en fail-closed — lo que acota el defecto: el runtime ya honraba «explícito vence»; faltaba negarse a una referencia rota.
7. **Dos errores de medición propios, declarados en el recibo:** (a) la primera corrida del falsador dio PASS contra el binario mutado porque **el build de la mutación había fallado** y se corrió contra el binario viejo — se llegó a «mis tests no son load-bearing» sin comprobar el build; (b) el `trap` de limpieza devolvía el exit del `rm` (64) en vez del veredicto, lo que habría reportado un exit inventado como resultado de UAT.
8. **Robustez derivada del falsador:** `uat_ctx_006` abortaba a media corrida si el runtime mutado no devolvía `candidates` (`KeyError` bajo `set -e`), ocultando el resto del log. Lecturas de JSON hechas tolerantes; un UAT que aborta no informa.

**Gates observados:** `context_cmd::tests` 25 passed / 0 failed · fmt limpio · clippy `-p sddk-cli --all-targets -D warnings` exit 0 · shellcheck limpio en ambos scripts · `test_debt_index_coherence.sh` PASS=10 FAIL=0 · `uat_ctx_005` 34 ok / `uat_ctx_006` 36 ok con exit 0. Perfil completo del workspace: ver journal de esta sesión.

**Deuda:** 0 nueva abierta. INC-DEBT-043 registrada y resuelta en el mismo bloque. Severa reciente: ninguna otra vigente (los 3 candidatos S14 siguen sin cumplir criterio; 041/042 y 038 cerradas).

**SIGUIENTE PASO (preciso, actualizado tras publicar):** ~~(a) release 2.3.3~~ **HECHO** — v2.3.3 publicado por CI (run 36760173483, 27 assets, gates 9b/9c OBSERVED, instalado y podado; recibo con addendum). **(b) abrir C3j** — desbloqueado por primera vez al no tener C3i ninguna UAT abierta: empezar por el objetivo 3 paso 7 (hipermedia) y las filas CTX-UAT-007..012/015 que siguen NOT_RUN, con el patrón de automatización ya establecido.

**Límites declarados (actualizados):** no se ejecutó perfil completo al redactar el recibo (sí antes del commit: `cargo test --workspace` 5182/0/19, fmt, clippy workspace); los 2 commits documentales de session-48 **ya salieron** con el bump real; no se probó si `--cycle` **de otro proyecto** pasa el `cycle_exists` (pregunta abierta, no defecto confirmado); la etiqueta `binary.bundle_coherence` ya no aparece en la salida de `dev doctor` de esta versión (se registra lo observable: `all_present: true`).

---

Previous: **Estado (session-48, 2026-09-30T17:34Z): C3i VERIFIED + RELEASE v2.3.2 PUBLICADO.** Workspace `2.3.2`, tag `v2.3.2` peel `4952e88c` == `origin/main`, `sddk 2.3.2` instalado. Árbol con 2 commits locales sin pushear (documental), ver "SIGUIENTE PASO".

**Hecho en session-48 (pre-flight: "deuda" sin criterios vigentes no es deuda; se verificó y la premisa caducó):**

1. **La "puerta" de CTX-UAT-002/003 no existía.** Estaban NOT_RUN desde session-40 por "gate humano: solo se puede dejar una lease escribiendo en el ledger real". FALSO: `sddk cycle start --lease-owner` y `cycle lock acquire` escriben en el ledger aislable del sandbox. El bloqueo era la ausencia del script.
2. **HALLAZGO PRINCIPAL: evidencia UAT caducada que nadie notó.** `tests/uat_ctx_002_context_bootstrap.sh` estaba **ROJO** contra el binario actual mientras la matriz lo declaraba **PASS** desde session-40: INC-DEBT-042 (session-46) cambió el contrato a exit 4 (`no_capsule_source`, degradación honesta) y el script asumía exit 0. **Causa raíz: ningún job de CI ejecuta los UAT de context** — dos sesiones sin detección. Un UAT que nadie ejecuta no es evidencia, es decoración.
3. **`tests/uat_ctx_004_cycle_inference.sh`** (nuevo): CTX-UAT-002 (dos ciclos, uno con lease → `resolved` al correcto, `context_source: compiled`, no menciona el otro) y CTX-UAT-003 (dos leases → `ambiguous`, 2 candidates con owner+expires_at_ms, **sin `cycle_id`**), más recovery (liberar lease → vuelve a `resolved`). Falsador RED→GREEN OBSERVED: neutralizar el guard de ambigüedad (`SDDK_UAT_FORCE_GUESS=1`) hace caer el script con 4 aserciones; revertido, PASS.
4. **CI ejecuta ahora los UAT** en el job espejo `shell-contracts` (con build release; timeout 15→25 min). Añade cobertura; no relaja ninguna allowlist.
5. **Deuda severa reciente: ninguna vigente.** Los 3 candidatos open con severidad (S14-TEST-PORTS-UNCONSUMED, NO-STRUCTURED-LOGGING, FORCE-VERSION-ERGONOMICS) no cumplen el criterio: generalidad especulativa re-severizada, observabilidad de amplio alcance, y ergonomía de session-14 cuyo pipeline ya deriva bien. 041/038 resueltas en session-47.
6. Contratos documentados al escribir el UAT: `cycle start` deriva el cycle_id del nombre y no acepta `--cycle`; `--timestamp` es RFC 3339; `lock release` exige `--fencing-token`.
7. **Release v2.3.2 PUBLICADO** por CI (run 36751152773 success, 27 assets, publishedAt 17:32:49Z). Gates 9b/9c OBSERVED: 27/27 assets HTTP 200; sha CDN == binario (`364adbe0…`); cosign Verified OK. Instalado: `sddk 2.3.2`, `current → 2.3.2`, doctor `all_present: true`, prune removed 2.3.1. **Orden respetado** (lección de session-47): push → verificar `HEAD == origin/main` → solo entonces taggear; la nota queda escrita en `githooks/pre-push` para el siguiente.

**SIGUIENTE PASO (preciso):** (a) pushear `441c8d46` (nota del hook) — el pre-push lo rechaza porque `githooks/` no está en la allowlist documental y el bump ya está en `origin/main`; sale con el próximo bump real, sin `--no-verify`; (b) cerrar **CTX-UAT-005** y **MIG-UAT-001** con el patrón de `uat_ctx_004` ya establecido, dejando C3i sin UAT abiertas y desbloqueando C3j.

---

Previous: **Estado (session-47, 2026-09-30T16:31Z): DEUDA SEVERA RESUELTA + RELEASE v2.3.1 PUBLICADO.** Workspace `2.3.1`, `HEAD = origin/main = accd4911` (1 commit documental pendiente de este cierre), árbol con cambios doc del cierre.

**Hecho en session-47 (criterio del operador: regresiones/deuda severa primero, sin abrir roadmap):**

1. **INC-DEBT-041 resolved** (shellcheck gate): ruta 1 del triaje. Re-medición honesta: ≈49 hallazgos en 15 ficheros (no 28/9: globo parcial en session-45); mayoría SC2016 intencional (grep de literales, patrón de contrato). ~30 directivas justificadas + SC2129 refactorizada (apply_banner) + GATE_END documentado como anchor espejo. **0 hallazgos OBSERVED**; sin tocar ci.yml ni bajar tolerancia. Commits `3c746a88` + `1e45f810`.
2. **INC-DEBT-038 resolved** (recibo mentiroso de `dev install --source`): opciones 2+3 del propio doc. `InstallReceipt.layout` opcional; `--source` escribe `layout:"flat"` + `bundle_version:null`; el doctor trata el recibo flat como coherencia N/A en verde (`all_present: true` E2E en prefix aislado). TDD RED→GREEN; doctor 9/9; dev_install 5/5. Commits `d1d59df6` + `a3751cc0`.
3. **Fix de herramienta:** `reconcile_state_pointer.sh` escribía versiones en prosa que su propio guard (check 3c) rechaza → `a3753be0`.
4. **Regresión pillada por el gate del release:** el test de migración v1→v2 esperaba la binding mentirosa que 038 elimina → actualizado a pinnear el recibo flat (`accd4911`).
5. **Release v2.3.1 por CI** (run 36742855897 success, 27 assets, no draft/prerelease, publishedAt 16:23:27Z). Incidente tag fantasma (primer tag antes del commit del test; draft inválido eliminado, re-tag al HEAD bueno; patrón v2.2.25). **9b OBSERVED:** peel == HEAD == origin/main, 27/27 assets HTTP 200 (un 500 transitorio de CDN, refresco ~2min). **9c OBSERVED:** sha CDN `a84e5980…` == binario; cosign Verified OK (nota: `sddk.bundle.json` no existe como asset; verificación por cert+sig). **Pasos 10-12 OBSERVED:** install.sh exit 0; `sddk 2.3.1`; current → 2.3.1; doctor `all_present: true`; prune removed 2.3.0.

**SIGUIENTE PASO (preciso):** evaluar S4+ de `docs/research/2026-09-30-sddk-cli-defects-evolution-plan.md` como hito propio del roadmap (pendiente desde C3k) o abrir C3i (hypermedia). Receipt: `tests/cycle-artifacts/p-63676b11dc0ef88f/session47-debt-041-038-release-v2.3.1/RECEIPT.md`.

---

Previous: **Estado (session-46b, 2026-09-30T15:10Z): C3k COMPLETO + RELEASE v2.3.0 PUBLICADO.** Workspace `2.3.0`, `HEAD = origin/main = 9d5c13d9`.

1. **W2c**: `sddk project pin/unpin` (`.sddk/project-pin.json`, schema 1, valida `p-*`; resolve y RuntimeContext honran el pin; e2e 2/2). Commit `9c3e027e`.
2. **INC-DEBT-040 variante (3)**: ruta (A-v2, tag-baseline) en `githooks/pre-push` alineada con `release_admission_check_v2`, fail-closed si ls-remote falla; matriz 48/48 con 3 expectativas AMENDED; debt → resolved. Commit `37c90b51`.
3. **Release v2.3.0** (minor: 3 feats): run 36732655082 success; 9b OBSERVED (18/18 assets, tag==HEAD); 9c OBSERVED (cosign Verified OK, sha CDN == binario); install + doctor all_present + prune. Higiene pillada por guards: MANIFEST stale (`94a7516d`), BUNDLE.toml fósil 2.2.32 (`f2fed84b`), drift de puntero (`7231a09f`).
4. Cierre C3k: ROADMAP COMPLETO, receipt `c3k-release-v2.3.0`, punteros reconciliados (`1e37b08e`).

---

**Estado (session-46b, 2026-09-30T12:39Z): RELEASE v2.2.37 PUBLICADO + hito C3k en roadmap.** Workspace `2.2.37`, `HEAD = origin/main = 89a45a9e` (docs C3k, 1 commit tras el tag), árbol limpio.

**Hecho en session-46b:**

1. **Release v2.2.37 publicado por CI** (run 36714821817 success, 27 assets, isDraft=false, isPrerelease=false): tag anotado `11d8d053` peel `1927d215` == commit bumpeado. Bump real 2.2.36→2.2.37 (`7f535fb9`) para satisfacer el predicado (A) del hook (mismo patrón INC-DEBT-040 de session-45: v2.2.34/35/36 quedan sin publicar, punteros ceremoniales).
2. **Gates 9b/9c OBSERVED** (autorizados por operador): tag anchoring via `git ls-remote`, 6/6 assets HTTP 200 de muestra, `cosign verify-blob` Verified OK (identity `release.yml@refs/tags/v2.2.37`), CDN sin staleness (sha servido `c2de8bd3...` == binario descargado).
3. **Instalación local OBSERVED:** `install.sh --version v2.2.37 --editor all` exit 0; `sddk --version` = 2.2.37; `framework/current → 2.2.37`; doctor 319 present + `all_present: true` (19 advisory `surface.briefness.*`); prune removed 2.2.33 kept 2.2.37.
4. **Hito C3k PROPOSED en ROADMAP.md** + `docs/research/2026-09-30-sddk-cli-defects-evolution-plan.md`: los 11 hallazgos del report de `agent-secretless` (`docs/receipts/sddk-2.2.33-defects.md`) confirmados contra el código fuente con file:line. Plan W1..W7 (sign-off integrity, identidad, discard linaje, gates evaluadores, UAT status/plan, render check, ingest multi-sesión). **W4 requiere decisión del operador** (evaluador material `sddk.cli` vs prompt corregido).
5. Recibo: `tests/cycle-artifacts/p-63676b11dc0ef88f/session-46-release-v2.2.37/RECEIPT.md`.

**SIGUIENTE PASO (preciso):** abrir C3k con W1 (D1 sign-off) y W2 (D2 identidad, high/P1) como primer slice; W4 bloqueado por decisión de modelo del operador. C3j sigue con paso 7 hipermedia y objetivo 6 (CTX-UAT-011..015, HYP-UAT-001..004 NOT_RUN).

---

Previous: **Estado (session-46, 2026-09-30T11:21Z):** Workspace **`2.2.35`** (bump pendiente a 2.2.36 para este bloque de código), `HEAD = a14540c5 = origin/main`, árbol CON cambios de session-46 sin commitear (C3j objetivo 3 paso 5 completo: compilación de capsule a nivel ciclo).

**Hecho en session-46 (delegación total del operador: "continua con el roadmap hasta el final sin parar"):**

1. **Puntero reconciliado** (`bash scripts/reconcile_state_pointer.sh`): el guard rojo de session-45 (puntero 56 commits detrás) quedó verde; el único FAIL restante es el drift conocido de `manifest.toml` (2.2.34 vs 2.2.35), que arregla el bump del próximo release (patrón establecido, no tocar a mano).
2. **DECISIÓN DE MODELO — ADR-0147** (`docs/architecture/adrs/ADR-0147-FRONTIER-SEMANTICS-AND-CYCLE-CAPSULE-INPUTS.md`): (D1) `frontier` solo se define para un run existente; ausencia de fila ≠ frontier vacío. (D2) el bootstrap compila capsule a nivel CICLO con facts reales del ledger. (D3) la ruta run-level queda pendiente del primer run real (INC-DEBT-039 re-scoped a low/P3).
3. **CTX-003 paso 5 IMPLEMENTADO (era el MUST bloqueado por INC-DEBT-039):** `CycleFacts` + port `CycleFactSource` + `CycleLedgerCapsuleInputs` en `sddk-engine` (`cold_start.rs`, 5 tests verdes 15/15) y `StorageCycleFactSource` + `compile_cycle_capsule` wired en el bootstrap (`context_cmd.rs`). Con ciclo activo: `status: complete`, `context_source: compiled`, capsule persistida bajo `cycle-<id>`, `basis_revision = capsule_id`. Sin ciclo: `no_capsule_source` exit 4 (degradación honesta intacta).
4. **BUG DE WIRING encontrado y corregido:** el bootstrap leía `resolved.active_leases` para inferir el ciclo, pero `resolve_cycle_context` devuelve ese campo SIEMPRE vacío por contrato (la lease única viaja en `cycle_id`; cero/ambiguas viajan como errores tipados). El código muerto degradaba a `NoActiveCycle` incluso con lease activa: ningún bootstrap habría compilado jamás. Fix: leer `resolved.cycle_id`. Pinneado por el test de integración nuevo.
5. **INC-DEBT-042 CERRADA** (high/P1, cerrada en session-46 con evidencia) e **INC-DEBT-039 RE-SCOPED** a low/P3 (solo queda la ruta run-level, disparador: primera fila real en `node_runs_v1`). Índice de deuda actualizado; guard `test_debt_index_coherence.sh` PASS=10 FAIL=0.

**Evidencia observada:** test de integración `bootstrap_with_active_cycle_compiles_capsule_from_ledger_facts` (ledger real en tempdir: ciclo + 3 work items done/active/paused + 2 decisiones + lease activa; la capsule durable lleva cycle ref, item cerrado en relevant, decisión aceptada en decisions.accepted, item pausado en must_read). Suites: `cargo test -p sddk-cli --lib` 847/0/1 · `cargo test -p sddk-engine --lib` 1351/0/1 · cold_start_tests 15/15 · clippy `-p sddk-cli -p sddk-engine --all-targets` limpio.

**Límite residual declarado:** el goal de la capsule es el `display_name` del `CycleManifest`; la entidad `Goal` de `sddk-domain` no tiene tabla de persistencia propia, así que usarla requeriría inventar facts. Queda para el ciclo que introduzca persistencia de goals.

**SIGUIENTE PASO (preciso):** (1) commitear este bloque como `feat(cli): context bootstrap compila capsule del ciclo activo desde el ledger (ADR-0147)`; (2) bump real 2.2.35 -> 2.2.36 con `bash scripts/release-bump.sh --force-version 2.2.36` (arregla también el drift de manifest.toml); (3) push + `bash scripts/release.sh` (gates 9b/9c autorizados por el operador en esta sesión); (4) install + doctor + cierre de ciclo C3j objetivo 3 (quedan paso 7 hipermedia y objetivo 6).

**UAT:** CTX-UAT-007..010 cubiertos por tests de integración observados (compiled/recovered/degradación/isolation). CTX-UAT-011..015 (paso 7 hipermedia, objetivo 6) y HYP-UAT-001..004 siguen **NOT_RUN**.

---

Previous: **Estado (session-44, 2026-09-30T06:44Z):** Workspace **`2.2.32`**, `HEAD (local) = ad6c6e63`, `origin/main = 5440b2e8` — **45 commits sin publicar**, árbol limpio. **El release v2.2.32 sigue SIN publicarse, y ahora hay una razón nueva y más fuerte: el push está bloqueado por un predicado insatisfacible (INC-DEBT-040, high/P1).**

**Lo que ocurrió, en orden.** El primer paso de session-43b (`git push origin main`) **se ejecutó y fue rechazado** por `githooks/pre-push` con `HOOK_EXIT=1`. La causa **no** es un bump pendiente: el bump ceremonial `061afe26` de session-43b **no bumpeó nada** (su padre ya era `2.2.32`). El bump real a `2.2.32` se commiteó en `3d4e457a`, que **ya es ancestro de `origin/main`**, así que en el rango `5440b2e8..ad6c6e63` **ningún commit cambia `[workspace.package] version`** — verificado commit por commit, salida vacía. El hook exige que el cambio ocurra **dentro del rango**; ocurrió 44 commits antes de `origin/main`. Es inobservable por construcción.

**Los dos gates no se contradicen: cada uno tiene razón.** `githooks/pre-push` mide contra el rango `origin/main..HEAD`; `release_admission_check_v2` mide contra el **último tag publicado** (`v2.2.27`) y responde `ACCEPT 2.2.27 -> 2.2.32`. Dos referencias distintas para la misma pregunta. Por eso no había waiver que negociar.

**El auto-desbloqueo tampoco existe:** `scripts/release-bump.sh` **se niega** a derivar bump (*"the workspace declares the pending release (2.2.32), no bump to derive"*) — se desactiva justo cuando hay un release pendiente declarado. Y `release.sh` no puede esquivarlo: su paso 1c hace ese mismo push y muere (línea 293), sin flag que lo salve.

**Falsación (clon aislado, sin red, remoto intacto), hook invocado directamente:** control sin bump `HOOK_EXIT=1`; con bump real `2.2.32 -> 2.2.33` vía `--force-version` `HOOK_EXIT=0` **y** `ACCEPT last-publish=2.2.27 -> 2.2.33`. **Corrección de método, declarada:** una primera medición dio `REJECT` post-bump y se registró como «el hook rechaza incluso un bump real» — **falso**, por medir el exit de `head` y no el del hook. Repetido con captura explícita. Cuarta vez que un artefacto de medición afirma algo falso.

**Consecuencia en cascada, declarada:** el rojo de `test_release_state_pointer.sh` (2 checks) **no es deuda independiente**, es efecto mecánico de este bloqueo. No se reparó ni se maquilló la tolerancia.

**DESVIACIÓN DE CONTRATO CORREGIDA (`ad6c6e63`):** `AGENTS.md` §2.1 describía el hook ceremonial **retirado** («rechaza push a main sin commit `chore(release): bump version»`) — literalmente la razón por la que session-43b creyó que el push estaba desbloqueado. Corregido a la formulación vigente, con `INC-DEBT-040` enlazada.

**DECISIÓN PENDIENTE DEL OPERADOR (bloquea la publicación):**
1. **Publicar como `v2.2.33`** con bump real (`--force-version`). Falsado que hook y admission aceptan. Coste: `v2.2.32` queda sin publicar para siempre.
2. **Publicar como `v2.2.32`** con `--no-verify`. Coste: etiqueta sin bump visible en el rango, changelog que no describe el árbol publicado.
3. **Corregir el predicado del hook** para comparar contra el último tag publicado, como ya hace `release_admission_check_v2`. Elimina la deuda en vez de rodearla, pero altera un gate de admisión: requiere su propia decisión y tests que falsifiquen el caso nuevo. No es emergency work.

**Deuda**: 1 nueva (`INC-DEBT-040`, high/P1) ⇒ **4 P1/critical reales** de 25 entradas (INC-DEBT-040, INC-DEBT-039, INC-DEBT-030, INC-DEBT-026). Sigue 1 P2 medium: INC-DEBT-038.

**Estado no tocado**: `c0-t01-pointer-mutation` sigue `PAUSED`, backup intacto. **El ledger real no fue escrito**: sólo lecturas (`sddk version`, `sddk adopt status`).

**Gates observados**: `release_admission_check_v2` ACCEPT · `sddk dev manifest --verify` `manifest OK` exit 0 · `gh auth status` OK (Rubentxu) · `cosign` `/usr/bin/cosign` y `jq` presentes (9c no abortará) · `githooks/pre-push` directo `HOOK_EXIT=1` · `test_adr_promotion_format.sh` `violations: 0` exit 0.

**Restante de C3j (sin cambio):** objetivo 3 **paso 5** (bloqueado por INC-DEBT-039, requiere decisión de modelo sobre `frontier` con `node_runs_v1` vacía) y **paso 7** (hipermedia), y objetivo 6. CTX-UAT-002/003 y 007..012/015 **NOT_RUN**. **C4/C6/C7: no abrir.**

**Commits de este bloque**: `ad6c6e63` (INC-DEBT-040 + corrección de AGENTS.md §2.1). Recibo: `receipts/session-44/UAT-EVIDENCE-2026-09-30T0644.yaml`.

Previous: **Estado (session-43b, 2026-09-29T22:32Z):** Workspace **`2.2.32`**, `HEAD (local) = 061afe26`, `origin/main = 5440b2e8` — **42 commits sin publicar**, árbol limpio. **Release v2.2.32 AUTORIZADO Y PREPARADO, PERO NO PUBLICADO.**

**Estado (session-43b, 2026-09-29T22:32Z):** Workspace **`2.2.32`**, `HEAD (local) = 061afe26`, `origin/main = 5440b2e8` — **42 commits sin publicar**, árbol limpio. **Release v2.2.32 AUTORIZADO Y PREPARADO, PERO NO PUBLICADO.**

> **Lo que NO ocurrió, declarado de entrada:** no se ejecutó `git push` ni `scripts/release.sh`. **No existe tag `v2.2.32`** y el último release público sigue siendo **`v2.2.27`**. La sesión se cerró en el punto exacto anterior al push. Lo que **sí** está hecho: perfil completo en verde y el commit `chore(release): bump version` (`061afe26`), que es lo que el `pre-push` hook exige para desbloquear el push.

**PRIMER PASO DE MAÑANA (verbatim, en este orden):**
```bash
git push origin main     # 42 commits; 061afe26 ya desbloquea el hook
bash scripts/release.sh  # 0-13, con gates 9b (assets) y 9c (autenticidad)
```
`cosign` está en `/usr/bin/cosign`, así que **9c no abortará**. Después, `test_release_state_pointer.sh` debe pasar **solo** al publicar: es el rojo declarado desde session-35 (`current_sha` va 39 commits por detrás, tolerancia 3) y se cierra con push + tag. **No maquillar la tolerancia.**

**El hallazgo de este bloque — el perfil completo destapó trabajo invisible.** `cargo test --workspace` dio `TEST_EXIT=101` con **4 FAILED** en `run_view_cli.rs`, y no eran regresión: los 4 tests **afirmaban `exit 0` y una `RunStateView` bien formada para run ids que no existen en ningún store**, es decir **fijaban la fabricación de INC-DEBT-039 como contrato**. Sin fuente real, "exit 0" sólo puede significar "invento". Un test así no falla cuando corriges el defecto: **falla al revés**, y como nadie lo ejecutó tras session-42, el rojo vivió una sesión sin que nadie lo notara. El test de texto pasó de *exigir* las dos cabeceras a **prohibirlas**. Segundo defecto, encontrado por el falsador y no por lectura: el payload de error **no era JSON válido** (`;` crudo en un `format!`); session-42 arregló el exit code pero nunca comprobó que la carga útil fuera parseable, y en canal legible por máquina no-JSON es indistinguible de un fallo de transporte. Corregido con `serde_json::json!`, válido por construcción.

**Falsación**: mutar `load_run_state_view` para devolver `RunStateView::for_test(Declared, …, vec![], vec![], vec![])` deja **4/4 RED**; revertir deja **4/4 GREEN**. Repetida tras el fix del JSON para descartar que el fix ablandara las aserciones. La primera mutación no compiló (campos privados) y se descartó como falsación inválida antes de darla por buena.

**Gates observados**: `cargo fmt --check` OK · `clippy --workspace --all-targets -D warnings` exit 0 · `cargo test --workspace` **5150 passed / 0 failed / 19 ignored**, 264 suites · `cargo metadata --locked` exit 0 (`Cargo.lock` no stale, donde se hundió session-22). **Lección de método:** la primera corrida dio `TEST_EXIT=0` pero el `tail -30` había truncado el log a 33 líneas y el agregado decía `PASSED=0`. **Un exit 0 sin log completo no es evidencia**, así que se reejecutó redirigiendo los 6780 líneas y sólo entonces se leyó el total.

**Versión**: el histórico pedía MINOR (6 feats, 12 fixes), pero `release-bump.sh` deriva del último tag (`v2.2.27`) y el workspace ya declaraba 2.2.32, así que se negaba a derivar. Se usó `--force-version 2.2.32` per `AGENTS.md §2.3` (el workspace version es el puntero ceremonial del release). El bump **fusionó** el bloque de CHANGELOG en la sección `## [2.2.32]` existente en vez de duplicarla.

**Commits de este bloque**: `34355f47` (fix run-view + tests falsados), `061afe26` (chore(release): bump version).

**Estado no tocado**: `c0-t01-pointer-mutation` sigue `PAUSED`, backup intacto. **El ledger real no fue escrito en session-42, session-43 ni session-43b.**

**Deuda**: 0 nueva. Siguen **3 P1/critical reales** de 24 entradas: INC-DEBT-039 (bloquea CTX-003 paso 5, requiere decisión de modelo), INC-DEBT-030, INC-DEBT-026.

**BLOQUEO DE CTX-003 paso 5 (sin cambio)**: INC-DEBT-039 no es trabajo de apply. Hay que responder qué significa `frontier` cuando `node_runs_v1` está vacía (ADR-075 / REQ-CurrentRunView-Shape) antes de escribir el adaptador. Implementarlo sin eso sería repetir, en el sitio opuesto, el defecto que session-42 eliminó.

**Candidato anotado y NO implementado** (session-43): check mecánico de coherencia índice↔documento de deuda. Este bloque aporta un cuarto ejemplo del mismo patrón — un artefacto (los tests) que afirmaba algo que el código ya no hacía.

**Estado (session-43, 2026-09-30T00:08Z):** Workspace **`2.2.32`**, `origin/main = 5440b2e8` — **40 commits sin publicar (medido en `90ec494b`)**. `development_head` declara `90ec494b`, que es el **árbol de código y evidencia que esta sesión certifica**; el commit documental de cierre va **encima** y no cambia código ni evidencia, así que `90ec494b` sigue siendo el SHA cuya evidencia es válida. **Auditoría de vigencia de deuda: de 3 P1 listadas, 2 no eran deuda real.** Se aplicó el criterio del operador —alerta de deuda sin verificar sus criterios no es deuda real— *antes* de elegir trabajo. (1) **INC-AUDIT-S14 CERRADA** (`51d3619b`, `4531744d`, `90ec494b`): cerró también la parte de **distribución**, que era lo único que quedaba. Su criterio de session-21 era "publicar con el workflow firmado Y observar que la firma verifica"; `v2.2.27` cumple ambas partes — `cosign` verifica binario y bundle bajo el pin **extraído del código** (`PASS=13 FAIL=0`), con subject real `...release.yml@refs/tags/v2.2.27`. **Lo que faltaba no era código sino ejecución:** los tests comprobaban que el pin fuera coherente, nunca que un release publicado verificara — deuda de proceso vestida de deuda técnica. De ahí las dos piezas nuevas: `tests/test_supply_chain_authenticity.sh` (produce la observación bajo demanda) y el **paso 9c de `release.sh`**, que verifica la autenticidad contra **los bytes que el CDN sirvió en 9b** (no una segunda descarga), **fail-closed** sin `cosign` y con opt-out explícito. (2) **INC-DEBT-034 reconciliada**: su documento ya decía `closed` desde session-33 pero el índice lo listaba `open` y repetía una divergencia de layout **que ya no existe** — `release.sh` y el gate sourcean `scripts/release-assets-contract.sh` (escenario 12, `PASS=13 FAIL=0`) y CI publica los nombres exactos del contrato. (3) **Entrada duplicada de S14 eliminada** del índice: afirmaba *"release.sh no firma nada"*, falso desde session-21. **Quedan 3 P1/critical reales de 24 entradas** (INC-DEBT-039, INC-DEBT-030, INC-DEBT-026). **El guard nuevo encontró dos bugs en sí mismo al ejecutarse**, ninguno visible por lectura: un `sed` codicioso que devolvía el pin más el resto de `cosign.rs` (⇒ `malformed subject in identity`, diagnóstico que no tiene que ver con la firma) y un `verified 0/2` que reportaba `ok`. Falsado en ambos ejes de la trust root (pin→`.*` ⇒ FAIL=1; issuer→`.*` ⇒ FAIL=4) y revertido. Lote de release **sin regresiones**: `test_release_public_gate.sh` 13/0, `test_release_admission.sh` 24/0, `test_install_asset_contract.sh`, `test_release_pipeline_consistency.sh`, `test_release_ci_contract.sh` verdes, `shellcheck` del guard limpio, `bash -n release.sh` OK (los 2× SC1091 son preexistentes, confirmado con `git stash`). **No se tocó Rust, así que no se ejecutó `cargo test`.** Recibo: `receipts/session-43/UAT-EVIDENCE-2026-09-30T0007.yaml`.

**Conocimiento negativo que queda de esta sesión:** la divergencia índice↔documento de deuda **no es cosmética** — produjo 2 de las 3 alertas P1 que motivaron la auditoría, y el índice es lo que se lee para priorizar, así que cuando difieren el índice miente y alguien gasta una sesión en deuda ya cerrada. Anotado como **candidato no implementado**: un check mecánico de coherencia índice↔documento.

**Bloqueo actual para CTX-003 paso 5 (sin cambio):** la **opción (a)** de INC-DEBT-039 (adaptador `RunStateViewInputs` sobre el ledger) sigue pendiente y **no debe escribirse todavía**. Antes hay que responder una pregunta de **modelo**, no de implementación: **qué es `frontier` cuando `node_runs_v1` está vacía** (ADR-075 / REQ-CurrentRunView-Shape). Implementar el adaptador sin eso sería repetir, en el sitio opuesto, el defecto que session-42 eliminó.

**Restante de C3j (sin cambio):** objetivo 3 **paso 5** (bloqueado) y **paso 7** (hipermedia), y objetivo 6. CTX-UAT-002/003 y CTX-UAT-007..012/015 **NOT_RUN**. **C4/C6/C7: no abrir.**

**Pendiente del operador:** recuperar el ledger real (`c0-t01-pointer-mutation` sigue `PAUSED`; backup intacto; **el ledger real no fue escrito en session-42 ni en session-43**), autorizar el push (**39 commits**), y autorizar el release. **Nota operativa nueva:** `release.sh` ahora tiene 9c, así que un operador sin `cosign` en el PATH verá **abortar** el release en vez de publicar sin comprobar. El opt-out `SDDK_SKIP_AUTHENTICITY_CHECK=1` existe y anuncia que la autenticidad NO se verificó.

**Estado (session-42, 2026-09-29T23:48Z):** Workspace **`2.2.32`**, `HEAD (local) = 0f74b21f`, `origin/main = 5440b2e8` — **35 commits sin publicar (medido en `0f74b21f`)**. **Deuda reconciliada y un defecto de producción corregido.** (1) **INC-DEBT-037 CERRADA** (`0636f875`): `SDDK_STATE_HOME` figuraba `critical/P1 open` desde session-15, pero la causa raíz ya estaba corregida en `e62da1bc` (session-35) y el documento nunca se actualizó. Verificado ahora: `inc_debt_037_state_home_precedence.rs` **6/6 PASS** y aislamiento e2e con binario release (`adopt apply` con `SDDK_STATE_HOME` propio produce `p-8d17246c…` ≠ `p-63676b11dc0ef88f`, o sea sobre otro ledger). `severity_at_detection` se conserva: la severidad crítica **era** correcta; cambia el status, no la memoria del incidente. (2) **INC-DEBT-039 REGISTRADA** (`docs/debt/INC-DEBT-039-RUN-STATE-VIEW-SCAFFOLD-READS-NO-LEDGER.md`, high/P1, **bloquea CTX-003 paso 5**): deuda **no declarada**, invisible hasta chocar con el objetivo. `sddk run-view` no leía el ledger — resolvía `origin` por el prefijo del `run_id` (`R-decl` ⇒ `Declared`) y pasaba `frontier`/`blockers`/`pending_decisions` como `vec![]` constantes; la spec (REQ-CurrentRunView-Shape.md:43) define `frontier` como vacío *"iff the run is terminal or no node is ready"*, así que un vector constante no distingue "nada listo" de "no se consultó", y como `ActionSurfaceView` se deriva de ahí la fabricación llegaba a la evaluación de políticas. (3) **Opción (b) IMPLEMENTADA** (`3055aeae`, `0f74b21f`): `load_run_state_view` es el seam único donde aterrizará la lectura real y hoy **falla cerrado** con `RUN_STATE_SOURCE_UNAVAILABLE`; además la fuente se decide **antes** que la policy (antes un nombre inexistente reportaba `POLICY_NOT_FOUND`, señalando el defecto equivocado). Falsador **RED 4/5 → GREEN 5/5**; el stdout del RED mostraba `"origin": "Declared"` y `"available_actions": ["Abort"]` para un run inexistente. E2E con binario release: **exit 4, stdout 0 bytes**, JSON tipado en stderr; antes **exit 0** con vista inventada. Gates: CLI lib **843 GREEN** (0 failed, 1 ignored), integración **187/187**, `context_fitness` **7/7**, `inc_debt_037_state_home_precedence` **6/6**, fmt/clippy limpios, build release OK. Recibo: `receipts/session-42/UAT-EVIDENCE-2026-09-29T2348.yaml`.

**Bloqueo actual para CTX-003 paso 5:** la **opción (a)** de INC-DEBT-039 (adaptador `RunStateViewInputs` sobre el ledger) sigue pendiente y **no debe escribirse todavía**. Antes hay que responder una pregunta de **modelo**, no de implementación: **qué es `frontier` cuando `node_runs_v1` está vacía** (medido en el ledger del operador: `node_runs_v1: 0`, `workflow_run_events_v1: 0`, `decision_records_v1: 0`, `event_snapshots_v1: 0`; de 81 `work_items_v1` sólo 2 con `cycle_id` real). Es decisión de semántica de spec (ADR-075 / REQ-CurrentRunView-Shape). Implementar el adaptador sin eso sería repetir, en el sitio opuesto, el defecto que esta sesión acaba de eliminar.

**Restante de C3j:** objetivo 3 **paso 5** (compilar `ContextCapsule` en el bootstrap, **bloqueado** por lo anterior) y **paso 7** (representación hipermedia), y objetivo 6 (hipermedia). CTX-UAT-002 y CTX-UAT-003 **NOT_RUN**; CTX-UAT-007..012 y 015 **NOT_RUN**. **No hay UAT para `run-view`**: lo cubren los 5 tests de `inc_debt_039_run_view_provenance.rs` más la verificación e2e. **C4/C6/C7: no abrir.**

**Pendiente del operador (sin cambio):** recuperar el ledger real (`c0-t01-pointer-mutation` sigue `PAUSED`; backup intacto sha `0ebc44c20ec06eb5…`; **el ledger real no fue escrito en session-42**), autorizar el push y autorizar el release (bump MINOR sugerido: feat+fix acumulados).

Previous: **Estado (session-41, 2026-09-29T21:06Z):** Workspace **`2.2.32`**, `HEAD (local) = 1ee19f21`, `origin/main = 5440b2e8` — **30 commits sin publicar (medido en `e05f67aa`)**. **C3j objetivo 5 IMPLEMENTED+VERIFIED:** `90612d86` — `FilesystemDeltaStore` (CTX-008, un `ContextDelta` por fichero `delta-<seq>.json`, secuencia monotónica, watermark, escritura atómica, replay ordenado) más `ContextBridge::facts()`. `8ff09e25` — `sddk context delta` con dos modos: `--publish` anexa un cambio material, sin él drena (rehidrata un `ContextBridge` y reproduce el stream). `1ee19f21` — **ADR-0146** registra el módulo raíz nuevo y su **no-autoridad sobre la base**. **El bug que hacía el handoff durable imposible, encontrado por los tests:** rehidratar en la base *actual* hace que todo delta ya consumido parezca stale y el bridge vuelva vacío (`applied: 0`, 4 tests RED a la vez). La rehidratación correcta **rebobina al origen del stream** (`store.origin_basis()`), no a la base actual. **Segundo defecto, encontrado por `git status`:** un `delta-2.json` en la raíz del repo de una ejecución fallida que escribía en el CWD; el helper de test ahora falla ruidosamente en vez de escribir donde sea. **Tres falsadores RED→GREEN observados y revertidos** (rebobinar a la base actual ⇒ 4 RED; corrupción silenciosa ⇒ 1 RED exacto; delta no-advisory rompiendo CDD-004 ⇒ 5 RED). CLI lib **843 GREEN**, engine lib **1351 GREEN**, service tests **19/19**, store **9/9**, `context_fitness` **7/7**, `cli` integración **187/187**, `agent_surface_golden` **3/3** sin drift (la superficie es por comando top-level), fmt/clippy limpios, `bash tests/uat_ctx_003_durable_deltas.sh` **7/7 PASS** con binario release. Recibo: `receipts/session-41/UAT-EVIDENCE-2026-09-29T2106.yaml`.

**Restante de C3j:** objetivo 3 **paso 5** (compilar `ContextCapsule` en el bootstrap) y **paso 7** (representación hipermedia), y objetivo 6 (hipermedia). CTX-UAT-002 y CTX-UAT-003 **NOT_RUN** (requieren leases reales ⇒ escribir en el ledger, gate humano); CTX-UAT-007..012 y 015 **NOT_RUN** (el objetivo 5 cubre CTX-008, pero las filas que le corresponden necesitan que el operador confirme su UAT exacta). **C4/C6/C7: no abrir.** INC-DEBT-038 sigue ABIERTO.

**Pendiente del operador (sin cambio):** recuperar el ledger real (`c0-t01-pointer-mutation` sigue `PAUSED`; backup intacto sha `0ebc44c20ec06eb5…`), autorizar el push y autorizar el release (bump MINOR sugerido: feat+fix acumulados).

Previous: **Estado (session-40, 2026-09-29T20:38Z):** Workspace **`2.2.32`**, `HEAD (local) = aff0a498`, `origin/main = 5440b2e8` — **23 commits sin publicar**. **C3j objetivo 3 IMPLEMENTED+VERIFIED:** `aff0a498` — `sddk context bootstrap`, la operación de alto nivel de CTX-003, compuesta sobre los resolvers canónicos (identidad, `resolve_xdg_paths`, `resolve_cycle_context`, `plan_adoption`/`apply_adoption`, stores durables) sin segunda autoridad. **CTX-003 cubierto en pasos 1, 2, 3, 4 y 6; pasos 5 (compile capsule) y 7 (hypermedia) PENDIENTES y declarados como tales.** CTX-004 con estados tipados `no_active_cycle`/`resolved`/`ambiguous`/`explicit`; CTX-005 replay no-op semántico; CTX-011 sin transcript en el binding. **Dos bugs reales encontrados por los tests:** el `seq` de `ContextBasis` se incrementaba en cada llamada (el replay no era idempotente) y `cycle_key` devolvía `cycle:<id>` con `:` — separador de componentes en `FilesystemCapsuleStore` — así que el read-reuse de capsule **nunca habría funcionado en producción**. **Dos falsadores RED→GREEN observados** (guarda de idempotencia elidida; `apply_adoption` elidido). `3e8b6031` — **ADR-0145** cierra deuda arquitectónica heredada: el guard `no_new_root_level_context_module_without_adr` estaba RED desde `f35c5e82` (verificado con `git stash` sobre HEAD). CLI lib **834 GREEN**, engine lib **1342 GREEN**, todas las suites `--test '*'` verdes, fmt/clippy limpios, `bash tests/uat_ctx_002_context_bootstrap.sh` **4/4 PASS** con binario release. Recibo: `receipts/session-40/UAT-EVIDENCE-2026-09-29T2038.yaml`.

**Restante de C3j:** objetivo 5 (delta durable observable entre procesos a nivel CLI, CTX-008) y objetivo 6 (primera representación hipermedia, = paso 7 de CTX-003). También el paso 5 de CTX-003 (compilar capsule en el bootstrap). CTX-UAT-002 y CTX-UAT-003 **NOT_RUN** (requieren leases reales ⇒ escribir en el ledger, gate humano); 007..012 y 015 **NOT_RUN**. **C4/C6/C7: no abrir.** INC-DEBT-038 sigue ABIERTO.

**Pendiente del operador (sin cambio):** recuperar el ledger real (`c0-t01-pointer-mutation` sigue `PAUSED`; backup intacto sha `0ebc44c20ec06eb5…`), autorizar el push y autorizar el release (bump MINOR sugerido: feat+fix acumulados).

Previous: **Estado (session-39, 2026-09-29T19:55Z):** Workspace **`2.2.32`**, `HEAD (local) = bef41df3`, `origin/main = 5440b2e8` — **diecinueve commits sin publicar**. **C3j slice 1 IMPLEMENTED+VERIFIED:** `f35c5e82` — `durable_session_binding` (CTX-002, JSON por sesión, write+rename atómico, corrupción tipada) y `durable_capsule_store` (CTX-001, `FilesystemCapsuleStore` implementa el trait `CapsuleStore` de `cold_start`; índice reconstruido desde disco en `open()`; **sin BD canónica nueva**, stop condition del bundle respetada). `42fad823` — e2e `durable_context_e2e` **5/5**: **CTX-UAT-006** (restart recupera la MISMA capsule; segundo `cold_start` = `FromRecovery`, no `Fresh`), **CTX-UAT-013** (reattach sin transcript, asertado por contenido — CTX-011), **CTX-UAT-014** (progressive disclosure preservado en el body persistido), session ≠ run en round-trip, y `ContextBridge` rechazando delta stale sobre binding recuperado (CTX-008). **Mutación falsadora observada:** elidir `rebuild_index()` en `open()` ⇒ CTX-UAT-006 **FAILED**; revertido ⇒ GREEN. Engine lib **1342 GREEN**; vecinos `cold_start_tests` 10/10, `context_capsule_tests` 10/10, bridge 6/6, binding 7/7; superficies 508/508 + 6/6; clippy/fmt limpios; **CTX-UAT-001 re-ejercitado con binario 2.2.32 reconstruido: 20/20**, recibo `4926c6bece9f8c9f…` byte-estable. Recibo: `receipts/session-39/UAT-EVIDENCE-2026-09-29T1954.yaml`.

**Restante de C3j:** objetivo 3 (`context bootstrap` application service + wiring CLI, reusando adoption + inferencia 0/1/N + `ContextBridge::bootstrap` + store durable), objetivo 5 (delta durable observable entre procesos a nivel CLI), objetivo 6 (primera representación hipermedia). CTX-UAT-007..012/015 **NOT_RUN** (esperan el wiring). **C4/C6/C7: no abrir.** INC-DEBT-038 sigue ABIERTO.

**Pendiente del operador (sin cambio):** recuperar el ledger real (`c0-t01-pointer-mutation` sigue `PAUSED`; backup intacto sha `0ebc44c20ec06eb5…`), autorizar el push y autorizar el release (bump MINOR sugerido: feat+fix acumulados).

Previous: **Estado (session-38, 2026-09-29T19:31Z):** Workspace **`2.2.32`**, `HEAD (local) = ceae92b9`, `origin/main = 5440b2e8` — **quince commits sin publicar**. **C3i COMPLETO (objetivos 1–5):** objetivo 5 IMPLEMENTED+VERIFIED (`c2a170b4`): pin `bootstrap_identity_is_unique_and_stable_across_restarts_and_refresh` en `crates/sddk-engine/tests/adoption_identity.rs` — 4 rederivaciones del plan con runtime metadata distinta (versión/timestamp/actor) producen idénticos `project_id`/`workspace_id`/ledger/receipt; re-apply y refresh solo convergen metadata; falsador observado (drift de `remote_url` ⇒ `p-9269c465 ≠ p-7b71d07a`, FAIL con diagnóstico correcto). **CTX-UAT-001 automatizado** (`e99631cb`): `bash tests/uat_ctx_001_adoption_convergence.sh --bin <sddk> [--repeats N]` — ejecutado 20/20 PASS con binario release 2.2.32 (recibo `467017c5…` byte-estable, identidad `p-8d17246c…` estable); ShellCheck limpio. Recibo: `receipts/session-38/UAT-EVIDENCE-2026-09-29T1930.yaml`. Verificación scoped: engine lib 1333 GREEN, adoption 13/13, adoption_identity 1/1, cli `--test cli adopt` 6/6, superficie adopt-convergence 6/6, fmt/clippy limpios.

**Restante de C3i:** solo el **cierre formal** — bump MINOR + `bash scripts/release.sh` con autorización del operador (15 commits acumulados incluyen feat+fix desde v2.2.27). **C3j/C6/C7: no abrir hasta ese cierre.** INC-DEBT-038 sigue ABIERTO (ciclo de diseño propio).

**Pendiente del operador (sin cambio):** recuperar el ledger real (`c0-t01-pointer-mutation` sigue `PAUSED`; backup intacto), autorizar el push y autorizar el release.

Previous: **Estado (session-37, 2026-09-29T19:13Z):** Workspace **`2.2.32`**, `HEAD (local) = 539a5fe2`, `origin/main = 5440b2e8` — **once commits sin publicar**. **Deuda sesión-36 ejecutada:** (1) fósil `BUNDLE.toml` regenerado a 2.2.32 con ancla correcta y **guard nuevo** `tests/test_dev_install_source_guard.sh` (RED contra el fósil, mutation-tested, shellcheck, ci.yml lo recoge) — `sddk dev install --source .` vuelve a pasar sus tres fail-closed (`edf148cb`). (2) **INC-DEBT-038** registrado: `--source` instala layout plano con recibo versionado ⇒ `bundle_coherence: missing`; 3 opciones de resolución, ciclo propio pendiente (`ee463133`). (3) **C3i objetivo 2 IMPLEMENTED+VERIFIED:** `adopt apply` sobre convergido era un refresh encubierto (recibo con timestamp nuevo en cada apply, observado); ahora es **no-op byte-estable**, `refresh` es el único verbo de runtime metadata, pin en motor + 6 checks de superficie (`a5987c63`, `539a5fe2`). CTX-UAT-001 **PASS** ×20 e2e aislado (recibo `receipts/session-37/`).

**Restante de C3i:** pin específico de identidad única (objetivo 5, observado estable ×20), script reutilizable de CTX-UAT-001. **C3j/C6/C7: no abrir.**

**Pendiente del operador (sin cambio):** recuperar el ledger real (`c0-t01-pointer-mutation` sigue `PAUSED`; backup intacto) y autorizar el push (once commits).

Previous: **Estado (session-36, 2026-09-29T18:28Z):** Workspace **`2.2.32`**, `HEAD (local) = efa18c64`, `origin/main = 5440b2e8` — **siete commits sin publicar**. **Adoptado el paquete hypermedia sin segundo roadmap:** C3i/C3j añadidos a `ROADMAP.md` (autoridad única), C6/C7 PROPOSED condicionado, C4/C5 intactos (`2c5a4760`). **C3i slice 1 IMPLEMENTED+VERIFIED** (`efa18c64`): `sddk-cycle-resume`/`mcw.md`/`cli-usage-contract.md` ahora enseñan la inferencia tipada real (0/1/N) que el runtime ya tenía; el pin obsoleto `runtime-active-cycle-discovery-unavailable` sustituido y 10 pins nuevos mutation-tested. Evidencia: `tests/test_workflow_contract.py` **508/508**; CTX-UAT-001..005 **PASS contra runtime real** aislado (`SDDK_STATE_HOME=/tmp/c3i`), MIG-UAT-001 NOT_RUN con razón; recibo en `docs/roadmap/receipts/session-35b/UAT-EVIDENCE-2026-09-29T1822.yaml`. Ledger real intacto (sha `58745880051db0a5…` re-verificado).

**HALLAZGO session-36:** (1) el `BUNDLE.toml` del checkout es un fósil v1.145.1 (`d572547b`, 2026-09-08) que rompe `dev install --source .`; `release.sh` genera el suyo en el stage, por eso los releases no lo sufren; pendiente regenerar/eliminar el fósil. (2) `dev install --source` con staging no versionado deja `sddk-install.json` (bundle 2.2.32) incoherente con `framework/current` (2.2.27) → `dev doctor` reporta `binary.bundle_coherence: missing`; las superficies del prefix SÍ llevan el contenido nuevo (verificado). (3) `dev doctor` briefness: 19 superficies exceden budget, **preexistente** (`mcw.md` ya iba con 393 líneas antes del +2 de esta sesión).

**Restante de C3i:** objetivo 2 (service `bootstrap`/`ensure` idempotente de alto nivel y que `init.md`/`orchestrator.md` lo consuman), objetivo 4 (no re-pedir adopción por sesión), objetivo 5 (identidad única), y automatizar CTX-UAT-001 a 20 reinicios. **C3j/C6/C7: no abrir.**

**Pendiente del operador (sin cambio):** recuperar el ledger real (`c0-t01-pointer-mutation` sigue `PAUSED`) y autorizar el push (siete commits).

Previous: **Estado (session-35, 2026-09-29T17:54Z):** Workspace **`2.2.32`**, `HEAD (local) = e62da1bc`, `origin/main = 5440b2e8` — **cuatro commits sin publicar**: `e01bc66c` (self-test del guard), `b0371571` (MIGRATION_21), `675256b6` (registro de INC-DEBT-037) y `e62da1bc` (precedencia de `SDDK_STATE_HOME`). Los tres últimos los observó esta sesión. Último release público sin cambios: **`v2.2.27`**. `STATE.yaml.current_sha` apunta a `e62da1bc` (HEAD local) y el trabajo sin publicar queda también en `development_head`. **El guard `test_release_state_pointer.sh` queda en ROJO y así se declara:** con 4 commits sin publicar es imposible satisfacer sus dos requisitos a la vez (`current_sha` contenido en `origin/main` **y** a ≤3 commits de `main`). Al inicio de la sesión iba 2 (verde); mis 3 commits lo llevaron a 4. No se maquilla bajando la tolerancia ni adelantando el puntero: se cierra al publicar con el bump.

**INCIDENTE DECLARADO — el ledger real fue mutado sin autorización y NO está recuperado.** Al verificar `MIGRATION_21` sobre una copia, se descubrió que `SDDK_STATE_HOME` no aislaba (solo lo leía `admission.rs`, nunca el resolver del ledger): la verificación escribió sobre `~/.local/state/sddk/projects/p-63676b11dc0ef88f/ledger.sqlite` (`user_version` 20→21, `c0-t01-pointer-mutation` `OPEN`→`PAUSED`, leases 31→30). **Sin pérdida de datos**; backup byte-exacto en `/tmp/ledger.pre-m21.backup.sqlite`. Detalle y conocimiento negativo en `SESSION-JOURNAL.md` adenda session-35; causa raiz en `docs/debt/INC-DEBT-037-SDDK-STATE-HOME-IGNORED-BY-LEDGER-PATHS.md` (critical/P1). **Desde entonces no se ha escrito en el ledger real**: verificado por hash (`58745880051db0a5…` idéntico antes y después de la prueba de aislamiento).

**Dos decisiones del operador, ambas bloqueantes:**
1. **Recuperación del ledger real.** `c0-t01-pointer-mutation` sigue en `PAUSED` cuando debería estar `OPEN`. Vías: `sddk cycle resume` (reversible, conserva v21) o restaurar el backup (byte-exacto, pero revierte a v20). **Ninguna ejecutada.**
2. **Push.** El hook pre-push exige bump real de `[workspace.package] version`; publicar implica release. **No autorizado.**

**Verde observado (lote acotado por el cambio):** `sddk-engine`+`sddk-cli` **155 suites / 3619 tests**, `sddk-storage` **35 suites / 279 tests**, `cargo fmt --check` limpio, `clippy -D warnings` limpio en los tres crates tocados. Prueba de aislamiento end-to-end: copia v20 genuina migra a v21 y devuelve `PAUSED` con el ledger real byte-idéntico. `cargo test --workspace` completo **no** se ejecutó como gate de este trabajo (perfil de `verify`/release, no de `apply`).

Previous: **Estado (session-34i, 2026-09-29T16:07Z):** Workspace **`2.2.32`**, puntero `9b491d9f`, `HEAD == origin/main == 3d4e457a`, árbol limpio. Último release público: **`v2.2.27`** (`5ee68265`, binario `a3b76113…`). **Nada publicado en 2.2.28–2.2.32**; esperan el flujo canónico con tu autorización.

**PERFIL COMPLETO OBSERVADO EN VERDE** (session-34f, evidencia en `docs/roadmap/receipts/session-34/UAT-EVIDENCE-2026-09-29T1552.yaml`): `cargo test --workspace --locked` → `TEST_EXIT=0`, 259 suites ok, 0 fallos (reejecutado tras tocar `lint.rs`); `cargo fmt --check` OK; `cargo clippy --workspace --all-targets -- -D warnings` OK; `tests/test_release_public_gate.sh` **13/13**; `tests/test_workflow_contract.py` **498/498**.

**HALLAZGO session-34f — tres gates rojos, ninguno causado por esta sesión, los tres arreglados:**
1. `test_workflow_contract.py` fallaba 1 de 499 contra un prompt **correcto**: anclaba el verify post-transición en `archive_verifies[1]`, pero el prompt verifica 3 veces antes del status post-transición, así que el índice señalaba un paso pre-transición. Anclado en `[-1]` y **mutation-tested** (borrar el verify post-transición sigue fallando; restaurar → 498/498).
2. `sddk lint` reportaba `agents/skills/*/SKILL.md` como `agents/SKILL.md`, **un fichero que no existe**: ambos walks reconstruían el path como `agents/{stem}.md` y descartaban el directorio real. Un diagnóstico que señala un path inexistente es inaccionable. Corregido con `strip_prefix(root)` + test.
3. El hint de `SDDK009/SDDK010` decía `sddk generate docs --root .`, pero sin `--in-repo` el comando escribe en XDG (ADR-0011) y nunca refresca el fichero del repo: **el gate era irrecuperable siguiendo su propia instrucción**. Corregido el hint. Efecto colateral: al regenerar bien, `docs/generated/workflow.md` perdió 4 estados y 1 fase que el doc commiteado tenía y `workflow/workflow.yaml` no declara — el doc llevaba tiempo generado desde un manifest obsoleto.
`lint` baja de **72 a 70** errores; los 70 restantes son deuda preexistente.

**HALLAZGO session-34h — me equivoqué en el diagnóstico anterior, y el arreglo cambió.** Reporté una "brecha del guard" diciendo que `test_release_state_pointer.sh` no comparaba la versión narrada con la real. **Falso**: el check 4 ya lo hace, y en `b3034111` puntero y `Cargo.toml` ambos decían 2.2.30, luego el PASS era correcto. El hueco real era otro, del mismo patrón que los tres gates anteriores: el campo `current_sha` llevaba un **comentario en prosa libre** afirmando "workspace 2.2.30" *después* de que el workspace pasara a 2.2.31, y ese texto no lo contrasta nadie por construcción. El contrato añadido es "el comentario no afirma versiones", no "el comentario es correcto" — parsear la prosa para extraer una versión reintroduce el mismo problema con un parser de texto libre encima. Mutation-tested con `workspace 9.9.9`; el propio puntero afirmaba 2.2.27, 2.2.30 y 2.2.31 a la vez y ahora no afirma ninguna.

**DEUDA ABIERTA (preexistente, NO tocada):** 9 errores `SDDK011/013/018` de `agents/skills/*/SKILL.md`, que usan frontmatter de *skill* (`name: core.*`, `user-invocable: false`) pero viven bajo `agents/`; vienen de `dc297ca2 feat(skills): materialize three M7.5 placeholder skills` y nadie los declaró. Corregirlo exige decidir si se mueven a `skills/` o se declaran como agentes, y ambas cambian la superficie del bundle — **decisión de layout, no bug de tooling**. Los otros 61 son `SDDK001` (referencias explícitas rotas, mayormente en paquetes históricos). `shellcheck` sigue rojo por severidad **info/style** (SC2012/2016/2129/2034) en 3 ficheros que no toqué; CI corre el mismo comando con `|| exit 1`. El fichero del guard modificado sí queda shellcheck-clean.

**ADR-0144 propuesto** (`docs/architecture/adrs/ADR-0144-JCODE-HOST-BOUNDARY.md`, `status: proposed`): resuelve el `decision_request` de C2c. El adapter público vive fuera (`@1jehuang/jcode-sdk` 1.1.0, con control mecánico de schema drift en ambas direcciones). Recomienda sidecar Node sobre el SDK y enumera **las obligaciones que la alternativa Rust exigiría**. La elección es del operador.

**C2 con sus limitaciones declaradas:** C2a T08-T11 PASS (cognicode-mcp 0.97.3, `8cced222`; no se ejercitó timeout sostenido), C2b T12-T14 PASS (chronos-mcp 0.1.4, `a3e9a9a6` + DRIFT-1 `278d1577`; no se ejercitó crash/restart en captura), C2c T15-T18 observados (`624cef9a`; los escenarios que requieren un turno real de modelo quedaron **NOT_RUN** por falta de quota/provider, así que **C2c no es un PASS completo**). C3g con addendum N=3 (`38ed1bd4`): baseline indicativa, no estadística.

**Siguiente acción exacta:** una decisión tuya, bloqueante para cerrar en verde: aceptar o rechazar **ADR-0144** (sidecar Node sobre el SDK público vs adapter Rust propio para C2c). El ADR recomienda Node y deja por escrito las obligaciones que Rust exigiría (filtro de unknown frames, tabla de eventos, test de paridad contra el SDK). En paralelo, dos decisiones de layout/severidad sin urgencia: el destino de `agents/skills/*/SKILL.md` y qué severidad de shellcheck tolera CI. Después, elegir en `docs/roadmap/ROADMAP.md` el siguiente WorkItem: anclar C0 (T01/T02) al SHA actual, o abrir el primer slice de C3 (Authority interleavings o contención SQLite).

Previous: **Estado (session-34, 2026-09-29T15:29Z):** Workspace **`2.2.30`**, puntero `d47a1766`, guard `test_release_state_pointer.sh` **PASS** (tras alinear manifest.toml y Cargo.lock, que mi bump manual con sed había dejado stale). Último release público: **`v2.2.27`** (`5ee68265`, binario `a3b76113…`). **2.2.28/29/30 preparados y commiteados, SIN publicar** (esperan flujo canónico con autorización del operador; la entrada CHANGELOG de 2.2.28 ya está commiteada en `d47a1766`). **C2 completo contra reales**: C2a T08-T11 PASS (cognicode-mcp 0.97.3, `8cced222`), C2b T12-T14 PASS con fix `4666a118` (chronos-mcp 0.1.4, `a3e9a9a6`), C2c T15-T18 observados con `@1jehuang/jcode-sdk` 1.1.0 (`624cef9a`, `decision_request` ADR sidecar Node vs adapter Rust). **C3g completo** (static p95 3.04s/172MB, runtime p95 4.64s/165MB, Verified ×3, `38ed1bd4`). DRIFT-1 fixeada (`278d1577`).

**Siguiente acción exacta:** elegir en `docs/roadmap/ROADMAP.md` el siguiente WorkItem: anclar C0 (T01/T02) al SHA actual, o abrir el primer slice de C3 (Authority interleavings o contención SQLite).

Previous: **Estado (session-33b, 2026-09-29T12:20Z):** Workspace **`2.2.20`** Último tag público: **`v2.2.20`** (`46608302ce99181c8d2fba4d382b54fe8be1ced8`, publicado `2026-09-29T12:11:52Z`, 27 assets, `isDraft=false`, `isPrerelease=false`, run `36565787996` 13/13 jobs, sha == HEAD) — **VERIFICADO**: cosign Verified OK (binario y bundle, identidad `release.yml@refs/tags/v2.2.20`), digest del binario instalado **idéntico** al publicado (`fc23bad5…`), instalación real `all_present: true`, `current -> framework/2.2.20`, `sddk dev update` 377 ficheros content-verified. **Suite completa de contratos shell: 22/22 en dos rondas** — primera vez observada con cero rojos. El último rojo (`test_vault_coherence_alignment.sh`) se resolvió de forma honesta: falso positivo del original reproducido con fixture (aprobaba informes sin veredicto), test reescrito con `evaluate_report()` pinada contra 6 fixtures herméticos, e informe real del trigger producido por el agente `sddk-coherence` con veredicto **`n/a`** (session-33 cerró por ruta estándar de release, no por la ruta vault; informe en `.sddk-cycle-artifacts/coherence/release-archive-vault-complete.md`). Defecto nuevo documentado: mezclar `sddk dev update` (binario 2.2.19, extrae en la raíz) con `install.sh` v2.2.20 (directorios por versión) rompe 69 enlaces; la reinstalación con `install.sh` lo restaura. Nota transitoria: `test_vault_mirror_auto.sh` falló 3 veces dentro del bucle de suite en una ventana post-install, 10/10 aislado y 2 rondas de suite limpias después; no reproducible fuera de esa ventana. Modo `on` / `declared:project`. `sddk adopt status` = **`complete`**.

**Siguiente acción exacta:** continuar el roadmap desde `docs/roadmap/ROADMAP.md` con el siguiente WorkItem READY. Si se toca la ruta de release: `git fetch --tags origin` antes de bumpear, y publicar solo vía `gh workflow run release.yml --ref <tag>`.

Previous: **Estado (session-33, 2026-09-29T11:48Z):** Workspace **`2.2.19`**, `HEAD == origin/main == 1cf5d535`, árbol limpio. Último tag público: **`v2.2.19`** (`1cf5d5352373a95ae4b4fd642cd42d5ad5338b97`, publicado `2026-09-29T11:44:52Z`, 27 assets, `isDraft=false`, `isPrerelease=false`) — **CERTIFICADO por seis vías independientes**: workflow `36562863987` 13/13 jobs en success con `sha` == HEAD; 11/11 URLs de distribución HTTP 200; `cosign verify-blob` **Verified OK** en binario y bundle con identidad `release.yml@refs/tags/v2.2.19`; digest del binario instalado **idéntico** al asset publicado (`4f5ec5b5…`); instalación real `all_present: true` con `current -> framework/2.2.19` y 0 enlaces rotos; `sddk dev update` con **377 ficheros** content-verified via `MANIFEST.sha256`. Gates: `cargo fmt --check` OK, clippy `-D warnings` OK, `cargo test --workspace` **0 fallos**, suite shell de contratos **21/22**. Modo `on` / `declared:project`. `sddk adopt status` = **`complete`**.

**HALLAZGO session-33 — el conocimiento negativo de session-32 queda EXPLICADO.** Session-32 escribió que la mutación "siempre `true`" no ponía los pins en rojo y que "el mecanismo por el que el pin no detectaría ese mutante sigue sin explicar". **La causa: el pin existente prueba la FUNCIÓN del detector, no el SITIO donde se la invoca.** El detector `tarball_wraps_all_members_under_one_dir` sigue siendo correcto bajo el mutante, así que el pin — que lo llama directamente — sigue verde. La forma del defecto es *"función correcta detrás de un `if` equivocado"*: se buscó un fallo en la lógica y la lógica estaba bien. Confirmado por ejecución. De ahí nace `tests/test_release_bundle_layout.sh`, que sí lo detecta porque parsea el bloque del push y exige que su condición nombre el flag que el detector asigna.

**DECISIÓN session-33 — el layout raíz del bundle es el contrato canónico** (cierra `INC-DEBT-034`). Lo fuerza `release.yml`, que extrae sin `--strip-components` y exige `framework/MANIFEST.sha256`; `AGENTS.md §8` está actualizado en consecuencia. El consumidor Rust además tolera las dos formas: detecta el wrapper en el listing del tarball y solo entonces aplica el strip. `tests/test_release_bundle_layout.sh` cruza los cuatro consumidores del contrato (productor, consumidor CI, consumidor Rust, documentación).

**HALLAZGO session-33 — una discrepancia de digest que es FALSA.** `~/.local/share/sddk/bin/sddk` es un binario **viejo de la v1.145.1 (2026-09-09)** que sigue en disco. Comprobar el digest del release vigente por esa ruta da un valor distinto del publicado y parece una distribución rota. El binario vigente es `~/.local/bin/sddk`, cuyo digest **sí coincide bit a bit**. Anotado en `STATE.yaml` para que no se relea como fallo.

**HALLAZGO session-33 — publicar desde la workstation es imposible por diseño, y está bien.** `scripts/release.sh` aborta en el paso 8c: la firma keyless exige que el OIDC provider emita un certificado para la identidad del proyecto (dentro de Actions), mientras que en local emitiría uno para la persona que abre el navegador — que los instaladores rechazan. La vía correcta es `gh workflow run release.yml --ref <tag>`.

**Notas de proceso (valen para cualquier sesión futura):** (1) `release-bump.sh` se apoya en el **último tag local**, que puede ir atrasado respecto al remoto: hacer `git fetch --tags origin` antes de calcular el bump; (2) el hook pre-push **rechaza** un rango que toque `crates/` o `tests/` sin bump real de `[workspace.package] version` — el asunto `chore(release): bump version` **no es autoridad**; (3) al actualizar una versión ya instalada, verificar con `install.sh`: la primera pasada puede escribir enlaces hacia un directorio de versión que aún no existe si el estado viene de la versión anterior.

**Deuda abierta:** `test_vault_coherence_alignment.sh` en rojo — **preexistente, no causado por esta sesión** (el test no se toca desde el import inicial `34d68c21`); exige un artefacto en `.sddk-cycle-artifacts/coherence/<trigger>.md` que genera el agente de coherencia, que no se ejecutó aquí. **NO se fabricó.** Además: `INC-DEBT-035` (decisión de seguridad del operador), `INC-DEBT-031`, `INC-DEBT-026`, `DEFAULT-GATE-DISCONNECTED`, `NO-STRUCTURED-LOGGING`, `TEST-PORTS-UNCONSUMED`, `RELEASE-FORCE-VERSION-ERGONOMICS`, y la asimetría del instalador con `SDDK_PREFIX` distinto del real.

**Siguiente acción exacta:** resolver `test_vault_coherence_alignment.sh` (ejecutar el agente de coherencia, o decidir explícitamente si el test degrada a WARN cuando el artefacto externo no existe), y continuar el roadmap desde `docs/roadmap/ROADMAP.md` con el siguiente WorkItem READY. Si se toca la ruta de release, empezar por `git fetch --tags origin`.

Previous: **Estado (session-32, 2026-09-28T23:05Z):** Workspace **`2.2.18`**, `origin/main` = **`58bea01f`** (= local, árbol limpio). Último tag público observado: **`v2.2.17`** (`447211e2`, publicado `2026-09-28T22:36:15Z`) — **NO certificado**: su smoke falló en el paso de actualización. Modo `on` / `declared:project`. `sddk adopt status` = **`complete`**. Estado local en esta sesión: 31/31 tests de `dev::update`, fmt limpio, clippy sin hallazgos en el fichero tocado.

**HALLAZGO session-32 — la ruta de instalación tenía 4 defectos encadenados, uno por release.** Ninguno se detectó en local: cada uno solo aparece al ejecutar el instalador contra un release real. La secuencia observada es la evidencia:

| tag | run | qué reveló |
|-----|-----|-----------|
| v2.2.15 | `36488888188` | smoke step 1 (**install**) pasó por 1ª vez; step 2 murió con 127 (binario en `$PREFIX/sddk`, vive en `$PREFIX/bin/sddk`) |
| v2.2.16 | `36490012790` | step 2 llegó a cosign: `accepts 1 arg(s), received 3` |
| v2.2.17 | `36492642837` | cosign verificó OK; `bundle is missing required MANIFEST.sha256` |

**HALLAZGO session-32 — el argv de cosign nunca funcionó en ningún release publicado.** `dev update` emitía `verify-blob --bundle <BLOB> --certificate-identity-regexp=… --certificate-oidc-issuer=…`: el blob ocupaba el hueco de `--bundle` y los flags de pinning quedaban como posicionales. cosign 2.4.3 rechazaba **antes de evaluar ningún pin**. El comentario del código daba por bueno un argv jamás ejecutado. Corregido en `5e2ff10d` con `cosign_argv()` + 3 pins de forma. Mutación falsada en dirección fallida (reinsertar el defecto → ROJO).

**HALLAZGO session-32 — productor y consumidor discrepan del layout del bundle (ABIERTO).** CI construye el bundle sin directorio envolvente (`release.yml:107`), `AGENTS.md` §8 paso 5 documenta la forma envuelta. El consumidor aplicaba `--strip-components=1` a ciegas: **borraba `MANIFEST.sha256`** y aplanaba `agents/*.md`. Corregido en `2f7d5064` detectando el layout antes de aplicar el strip (5 pins). **Pendiente la decisión de política:** alinear el productor al contrato, o fijar el layout raíz como contrato y corregir AGENTS.md + `release.sh`.

**⚠️ Conocimiento negativo obligatorio:** la evidencia de mutación del fix de layout (`2f7d5064`) **NO es concluyente y así consta en el propio commit**. El mutante "siempre `true`" no puso los pins en rojo pese a `cargo clean` y recompilación forzada; la lógica se validó aparte con un binario `rustc` autónomo (`root_level -> false`, `wrapped -> true`). **Nadie debe citar `2f7d5064` como mutación falsada en ambos sentidos.** El mecanismo por el que el pin no detectaría ese mutante sigue sin explicar y es el primer punto a investigar si la cadena vuelve a fallar.

**Deuda abierta:** `INC-DEBT-034` (layout productor vs consumidor), `INC-DEBT-036` (cadena de 4 defectos — cerrada en código, cierre formal pendiente), `INC-DEBT-035` (decisión de seguridad del operador), `INC-DEBT-032` (suite no hermética), `INC-DEBT-031`, `INC-DEBT-030` (refutado), `INC-DEBT-026`, `DEFAULT-GATE-DISCONNECTED`, `NO-STRUCTURED-LOGGING`, `TEST-PORTS-UNCONSUMED`, `RELEASE-FORCE-VERSION-ERGONOMICS`.

**Siguiente acción exacta:** `gh workflow run release-automation.yml --ref main` para publicar **v2.2.18** (fix de layout ya en `origin/main` = `58bea01f`). Si el smoke pasa el paso de actualización, se cierra la cadena de 4 defectos; después, verificación post-publicación (URLs 200, `cosign verify`, instalación e2e) y cierre formal de `INC-DEBT-036`. Al diagnosticar, usar **`gh run view <id> --log-failed`**: el log completo mezcla jobs y epistoló tres veces antes de llegar al mensaje real.

Previous: **Estado (session-31, 2026-09-28T16:41Z):** Workspace **`2.2.6`**, **no publicado**. Último tag público **`v2.0.1`**. `origin/main` = `ed0e3c47`; local = `b0cff81e` + bump. Modo `on` / `declared:project`. `sddk adopt status` = **`complete`**. Suite completa **5077 passed / 0 failed / 19 ignored** (exit 0) en el árbol de session-31.

**HALLAZGO session-31 — el blocker de firma NO era un blocker.** `INC-DEBT-030` daba por agotados los minutos de Actions (§2.5) y ofrecía solo dos salidas, ambas con decisión del operador. **Refutado por ejecución**: el repo es público, Actions está habilitado, y `gh workflow run ci.yml` arrancó el run `36450601924` real. La vía 1 (publicar desde Actions) es viable y es la correcta, porque produce el issuer `token.actions.githubusercontent.com` que los instaladores pinan. La vía 2 (`SDDK_SKIP_SIGNING=1`) sigue descartada: degrada el contrato de instalación para todos los usuarios.

**HALLAZGO session-31 — el mismo bug de `manifest_sha256` seguía vivo en el CI, donde se publica.** Session-30 corrigió `release.sh`; `.github/workflows/release.yml:199` conservaba `awk 'NR==1 {print $1}'`, o sea el digest del primer fichero listado en vez del manifest. Falsificado con el binario release real y `dev install` en las dos direcciones: el bundle que el workflow publicaba era **rechazado por su propio instalador** (*"Refusing to install a bundle whose declared manifest hash does not match"*). Y `dev manifest --verify` no lo detecta: `verify_manifest_anchor` solo se invoca desde el path de instalación (`install.rs:121`). Corregido en `eae22737` con dos tests: uno que extrae el patrón real del workflow (falsificado por mutación en ambos sentidos) y otro que prueba la consecuencia con `dev install` real.

**HALLAZGO session-31 — regresión de la sesión anterior, acotada por bisect.** `17d9b804` (guard de CHANGELOG) referenciaba `"$CHANGELOG.md"`, variable que `release-bump.sh` nunca define: con `set -euo pipefail` el bump abortaba. Bisect: `ed0e3c47`/`cf481d11`/`e6998f01` verdes, `17d9b804` FAILED, aislado también. Los tests que "cubrían" el cambio eran verdes porque **copian el bloque** del script con un nombre de variable que sí existe. Corregido en `a91c273f`.

**HALLAZGO session-31 — la suite no es hermética, y eso lo demo el CI.** Local verde y remoto rojo sobre el mismo código, con **tests distintos**: en CI fallan `cli_incidence_dka_orphan_review_phase_exists` y `cli_incidence_dka_managed_closure_vault_route_exists`, que exigen ficheros de `~/.sddk-knowledge/` con `env!("HOME")` (tiempo de compilación). No están versionados y no existen en un runner limpio. Registrado en `INC-DEBT-032` con alcance medido (2 tests). No corregido aquí: elegir entre versionar la INC, `#[ignore]` con motivo, o declarar el vault como input externo es política de gobernanza (`AGENTS.md` §2.7), no corrección técnica.

**Deuda abierta real: 8 INCs.** `INC-DEBT-030` (blocker **refutado**, acción = publicar por Actions), `INC-DEBT-032` (suite no-hermetica, nuevo), `INC-DEBT-031` (CHANGELOG duplicado, P3), `INC-DEBT-026`, `DEFAULT-GATE-DISCONNECTED`, `NO-STRUCTURED-LOGGING`, `TEST-PORTS-UNCONSUMED`, `RELEASE-FORCE-VERSION-ERGONOMICS`.

**Siguiente acción exacta:** publicar **v2.2.6**. Tres pasos: (1) `git push origin main` — el bump a 2.2.6 satisface la cláusula (A) del pre-push, que exige cambio real de `[workspace.package] version`; (2) `gh workflow run release-automation.yml`, que crea el tag desde `origin/main:manifest.toml` y despacha `release.yml` (donde el binario musl y `cosign` keyless ya están resueltos); (3) verificar los assets públicos e instalar end-to-end. **Irreversible** (`git.release=human_gate`), ya pre-aprobado por el operador en session-31. Antes de cerrar: reconciliar `STATE.yaml` (el guard ya no miente, pero afirma un SHA sin publicar — se resuelve publicando).

Previous: **Estado (session-28, 2026-09-28T10:10Z):** Workspace **`2.2.0`**, **no publicado**. Último tag público **`v2.0.1`**. Modo `on` / `declared:project`. `sddk adopt status` = **`complete`** (session-26 dijo "no adoptado": era falso).

**HALLAZGO session-28 — el bump de CI habría REGRESADO la versión del workspace.** `release-bump.sh` derivaba la siguiente versión siempre del **último tag**, así que un bump manual era invisible: con el workspace en 2.1.1 y el tag en v2.0.1 derivaba **v2.1.0**, una regresión. Y no era hipotético: el paso "Open release PR when a bump is pending" de `release-automation.yml` corre ese script y hace `gh pr merge --auto --squash`, o sea que el rollback se habría mergeado solo sobre main. **Reproducido antes de arreglar** en un clon aislado (2.1.1 → 2.1.0). Corregido derivando desde la más alta entre workspace y tag; se eliminó además la segunda lectura de `WORKSPACE_VERSION` para que el ancla del `sed` y la base de la derivación no puedan divergir. Cobertura nueva: `tests/test_release_bump_derivation.sh` (6 casos) — no existía ninguna para este script. Contra la lógica original da 4 pass / **2 fail** (los 2 rollbacks); contra el fix, **6/0**.

**HALLAZGO session-27 — la admisión de release rechazaba un release válido.** `release.sh` rutaba a **v1** (HEAD vs HEAD^) en vez de **v2** (último tag publicado). Como el bump vive N commits atrás de HEAD, HEAD y HEAD^ carrying la misma versión y el gate rechazaba un release correcto. El defecto ya estaba documentado en `release_admission.sh:15-19` y la v2 existía, probada con 22 checks, **nunca seleccionada**. Corregido en `a716953`; 3 checks de **conexión** añadidos (mutación a v1 → 2 fallan, `die`→`warn` → 1).

**Corrección session-27: el bloqueo musl NO bloquea la publicación.** `release-automation.yml` crea el tag desde `origin/main:manifest.toml` y despacha `release.yml`, que instala `musl-tools` con su propio sudo (líneas 53-57). La publicación es ejecutable por CI sin tocar esta máquina. Solo la ruta local `release.sh` necesita musl en el host.

**Deuda abierta real: 7 INCs.** `SUPPLY-CHAIN-AUTHENTICITY` (code-closed/distribution-open), `DEFAULT-GATE-DISCONNECTED` (nuevo, session-27), `NO-STRUCTURED-LOGGING` (medium), `TEST-PORTS-UNCONSUMED` (medium), tres `low`.

**Siguiente acción exacta:** publicar `2.2.0` es lo único que cierra `INC-AUDIT-S14` en distribución. **Irreversible** y lo decide el operador: `gh workflow run release-automation.yml`. Antes conviene confirmar que la derivación da `v2.2.0` (ya verificado) y que el gate de admisión acepta. `ADR-0143` sigue `proposed`.

Previous: **Estado (session-27, 2026-09-28T09:40Z):** `HEAD == origin/main == 9a642e7` (`chore(release): bump version a 2.1.1`). Workspace **`2.1.1`**, **no publicado**. Último tag público **`v2.0.1`**. **`sddk adopt status` = `complete`** (session-26 dijo "no adoptado": era falso).

**Bump `2.1.0` → `2.1.1`** (`9a642e7`) producido por `scripts/release-bump.sh --force-version`, no a mano. **2.1.0 nunca fue tag ni release**, así que publicar 2.1.1 no salta ninguna versión pública.

**Puntero de estado reconciliado** (`241ada6`): el guard daba FAIL por 4 commits de retraso (tolerancia 3) — tercera repetición del patrón session-18/22. Reconciliación mecánica que preserva la nota de evidencia previa.

**Deuda abierta real: 6 INCs**, no uno. `SUPPLY-CHAIN-AUTHENTICITY` (code-closed/distribution-open), `NO-STRUCTURED-LOGGING` (medium), `TEST-PORTS-UNCONSUMED` (medium), tres `low`. La nota de session-25 ("el único INC abierto") era de alcance más estrecho.

**Siguiente acción exacta:** leer el dry-run hasta el paso 8. Si pasa, **publicar 2.1.1** es el único camino que cierra `INC-AUDIT-S14` en distribución. Irreversible: requiere firma keyless desde Actions (`gh workflow run release-automation.yml`). `ADR-0143` sigue `proposed`.

Previous: **Estado (session-25 cierre, 2026-09-28T09:00Z):** `HEAD == origin/main == 3a142b8` (`chore(release): bump version a 2.1.0`). Workspace **`2.1.0`**, **no publicado**; `--dry-run` con árbol limpio dio **release admission ACCEPT 2.0.12 → 2.1.0** con 0 `✗`. Último tag público sigue **`v2.0.1` → `5ce4bca`**. Cierre de session-25: **`INC-DEBT-024` cerrado** (las tres mitigaciones, verificadas en ambos sentidos) y la deuda menor de la versión de cosign. Previous: session-24 (`d1973b1`) implementó el gate de issuer fail-closed.

**`INC-DEBT-024` — CERRADO en session-25.** Era el único INC abierto (20 de 21 cerrados). Cerrado por las tres vías, cada una con su falsificación:

1. **Gate de issuer fail-closed** (`08639ff`, session-24): `release.sh` lee el issuer del certificado que cosign acaba de acuñar (del bundle) y hace `die` si no es `DEFAULT_CERT_ISSUER`, si falta el certificado, o si no se puede leer. El issuer sale de la **URI SAN**, no del DN RFC4514 (el DN lleva el nombre de la CA Fulcio, no el proveedor OIDC).
2. **Pre-check de contexto** (`ef0d5a6`, session-25): sin `GITHUB_ACTIONS=true`, aborta **antes** de firmar. No es un mensaje más limpio: el gate de issuer corre *después* de que cosign firme, y para entonces ya han pasado el device flow interactivo (cuelgue en run desatendido) y, si el operador lo completa, un certificado de persona. El pre-check evita las dos. `SDDK_SKIP_SIGNING=1` sigue siendo la salida declarada para publicar sin firma; `SDDK_ALLOW_LOCAL_SIGNING=1` existe solo para falsificar el check y **no** es vía a un release bueno.
3. **`ADR-0143 §(a)` reescrita** (`f9beddf`): la opción 1 es "firmar antes de publicar **desde Actions**". Añadido un `§(a-bis)` que separa el código (ya no puede publicar con la identidad equivocada) de la política (quién ejecuta el release, y elegir entre las dos opciones).

**HALLAZGO que reencuadra las dos sesiones anteriores (session-25):** `release.yml` **no invoca `release.sh`** — firma en su propio job `sign`, con `id-token: write` y `cosign sign-blob --output-signature --output-certificate`. Por tanto el pre-check protege la **ruta local** (donde estaba el fallo, y era el no documentado) **sin tocar CI**, y el gate de issuer cubre `release.sh`, **no** la firma de producción. La identidad de la firma de producción ya la garantiza el diseño del workflow; lo que session-24/25 cerraban era el agujero local.

**Deuda menor también cerrada** (`314bc34`): `cosign-release: 'v2.4.3'` explícito en los dos pasos `cosign-installer`. El action estaba pineado por SHA, lo que da falsa sensación de control: lo que quedaba por defecto era la **versión**, que pertenece a un tercero. Un check de contrato exige el pin en todos esos pasos.

**Falsificación permanente (7 checks en `test_install_asset_contract.sh`).** Cada uno probado con su mutación: cambiar el pin del issuer → 2 fallan; `die`→`warn` en el gate de issuer → 1; eliminar el pre-check → 3; `die`→`warn` en el pre-check → 1; **mover el pre-check después del bucle de firma → 1 (el de orden)**; quitar el pin de cosign del job `sign` → 1, nombrando el job. El de orden importa: un pre-check correcto en la posición equivocada no evitaría el device flow.

**Verificación de session-25:** `cargo test --workspace --locked --no-fail-fast` = **5057 passed / 0 failed / 19 ignored**, 258 suites, exit 0 (idéntico al baseline, así que el bump no alteró nada). `fmt`, `clippy -D warnings`, `shellcheck -S error`, `diff --check`, `cargo metadata --locked`, YAML del workflow: limpios. Suite de shell + public gate: **12/12 PASS**.

**Lo que queda abierto, y NO es técnico.** `INC-AUDIT-S14-SUPPLY-CHAIN-AUTHENTICITY` está `code-closed, distribution-open`: cerrarlo exige **publicar un release firmado real**. La vía existe y está verificada (`gh workflow run release-automation.yml` crea el tag y despacha `release.yml`), pero crea un tag y un release público irreversibles, así que no se ejecutó. `ADR-0143` sigue `proposed`. Ambas salidas son del operador.

**Estado (session-24 cierre, 2026-09-28T08:11Z):** `HEAD == origin/main == 2f0482e` (`chore(release): bump version a 2.0.12`). Workspace **`2.0.12`**, **no publicado**. Último tag público sigue siendo **`v2.0.1` → `5ce4bca`**. Cierre de session-24: implementado el **identity gate de firma** (`08639ff`), que era la mitigación 1 de `INC-DEBT-024` y que session-23 dejó declarada y sin hacer. Previous: session-23 (`a0729b0`) escribió el INC y corrigió `ADR-0143` para exigir que la firma se obtenga desde Actions.

**Identity gate de firma — CERRADO en `08639ff`:** `release.sh` comprobaba la firma con `command -v cosign`, o sea con la presencia del binario, no con la identidad realmente emitida. Eso era justo el agujero: la identidad keyless depende de **dónde** firmas, y cosign está instalado igual en los dos sitios. Resultado posible: un release correctamente firmado que el propio `install.sh` de este repo rechaza, con un error que en el usuario se lee como manipulación. Ahora `release.sh` extrae el issuer del certificado que cosign acaba de acuñar (del bundle: `.cert` en v2, `verificationMaterial.x509CertificateChain` en el formato nuevo) y **hace `die` antes de publicar** si no es el esperado. El issuer se lee de la **URI SAN**, no del DN RFC4514 — el DN lleva el nombre de la CA Fulcio, no el proveedor OIDC. **Fail-closed en los tres casos**: issuer distinto, bundle sin certificado, bundle ilegible. Un control que no puede leer lo que verifica no ha verificado nada. Falsificado en ambos sentidos con 3 checks permanentes en `test_install_asset_contract.sh` (suite de shell del paso 1: **11/11 PASS**).

**CORRECCIÓN session-24 — hipótesis mía que era FALSA y que no debe quedar escrita:** afirmé que la firma en CI fallaría por incompatibilidad de versión de cosign, deducido del `cosign v3.1.3` instalado en esta máquina. **Falso, por dos errores.** (1) El comentario `cosign-installer@053f9b74 # v3.8.1` es la versión del **action**, no la de cosign; ese SHA **sí** existe y resuelve a `refs/tags/v3.8.1` del propio `cosign-installer` (confirmado con `git ls-remote`). (2) Su `cosign-release` por defecto es **`v2.4.3`** y el workflow **no lo overridea** — CI instala v2.4.3, no v3.1.3. En v2.4.3 `--output-signature` y `--output-certificate` **sí** existen (`sign_blob.go`, `SignBlobCmd(..., outputSignature, outputCertificate, ...)`). **El workflow de firma no estaba roto.** La incompatibilidad real es entre la v3 del host y la v2 de CI, y afecta a quien intente reproducir la firma en local, no a la publicación. Todo esto está escrito en el INC, no en un handoff.

**Deuda menor que abre session-24 (sin registrar como INC aparte):** la versión de cosign que instala CI es un **default implícito** del action. Si sigstore cambia ese default, la versión usada por la firma cambia sin que nada en este repo lo refleje. Fijar `cosign-release: 'v2.4.3'` explícito en el workflow lo convierte en decisión declarada y visible. **No se ha tocado el workflow**: es superficie de publicación y pide su propio slice con su propio bump.

**Lo que sigue abierto es una decisión de política, no de ingeniería** (`ADR-0143 §(a)`, status `proposed`): si la firma se obtiene **antes** de publicar (paso 8c, ya escrito) o se mantiene la rama `sign` del workflow con un guard de publicación. Con el gate de session-24, la opción 1 ya no puede publicar por error. La elige el operador.

**Estado (session-22 cierre, 2026-09-28T07:45Z):** C1 cerrado; C2 NOT_EVALUATED (bloqueada por `chronos-mcp`; `cognicode-mcp` presente); C3a-h PASS_OBSERVED; C4 — último release publicado sigue siendo **`v2.0.1` → `5ce4bca`**. Workspace **`2.0.10`**: 9 PATCH por delante del tag público, sin publicar. Dos cierres en esta sesión: el bloqueo del `Cargo.lock` stale (que rompía CI y release con `--locked`) quedó cerrado en `8136bbf`, y el flake `R-flake-inv10` del gate de concurrencia quedó cerrado en `982014e`. Previous: session-21 (206584b, a030eed, c9984b8) corrigió el pin de identidad cosign a `refs/tags/vX.Y.Z` — insatisfecible con el pin previo `@refs/heads/main` — y alineó el manejo del trio `.sig`/`.bundle.json`/`.pem` en los tres consumidores. Este puntero se revalida al comienzo de cada sesión con `bash tests/test_release_state_pointer.sh`.

**FLAKE `R-flake-inv10` — CERRADO en `982014e`:** el gate `inv10_grep_gate_no_mutex_on_workflow_state` afirmaba `elapsed < 20ms` con hijos triviales. Es un umbral absoluto sin margen: media la máquina y no el código. Reproducido en **4 de 6 corridas bajo carga** (22-31ms) mientras pasaba en 0.00ms en aislamiento. Subirlo a 200ms lo volvía verde pero **dejaba pasar la serialización completa** (`max_concurrency=1`, ~60ms), o sea que habría dejado de detectar la regresión que existe para detectar — se descartó. El wall-clock no separa los dos casos en una máquina desconocida: contención con hijos triviales (~30ms) y serialización genuina (~60ms) se solapan. Lo que sí separa es el **ratio** entre el elapsed total y el coste de una corrida serializada, que es adimensional. Falsificado en ambos sentidos: concurrente bajo `nproc` procesos en burn da **8/8 y 6/6**; `max_concurrency=1` **falla con ratio 100%**; `max_concurrency=2` **falla con ratio 50%**. Los hijos duermen 20ms porque con 4ms una máquina cargada tardaba 42ms solo en agendar 50 duermes concurrentes (21% del serial) — midiendo latencia de despertar, no contención del mutex.

**CORRECCIÓN session-23 — mi registro estaba equivocado, y esto lo corrige:** he estado repitiendo que "el trust root de la firma sigue sin definirse" como el bloqueo abierto. **Es falso.** Session-21 ya lo decidió e implementó: Fulcio keyless, issuer `https://token.actions.githubusercontent.com`, subject regex anclado a `release.yml` sobre un tag SemVer, con ambos extremos obligatorios y fail-closed en las dos rutas de consumo. La decisión existía; lo que faltaba era el documento que la gobernara. Eso es `ADR-0143` (status `proposed`), escrita en esta sesión: fija la política, sus consecuencias asumidas, y delimita con precisión lo que sigue abierto.

**Lo que de verdad queda abierto es una decisión de política, no de ingeniería** (`ADR-0143 §(a)`): si la firma se obtiene **antes** de publicar (paso 8c de `release.sh`, antes de `gh release create`, que ya está escrito) o se mantiene la rama `sign` del workflow con un guard de publicación. Hoy, si `sign` falla a mitad, el tag queda publicado sin assets firmados, es el `latest`, y es **ininstalable** por ser fail-closed. Ambas opciones tienen un coste real y la elige el operador.

**Deuda de proceso, cerrada:** el patrón de puntero desfasado que llevaba tres sesiones (session-18, 22, 23) ya no requiere intervención manual. `scripts/reconcile_state_pointer.sh` (`56eac21`) reconcilia los campos mecánicos respetando la misma tolerancia que el guard, y conserva las notas de evidencia en lugar de pisarlas. El fallo original —el bump viaja en un commit posterior al que mueve el puntero— sigue siendo real, pero ya no requiere que nadie lo recuerde.

**RECONCILIATION session-22 (deuda de proceso propia, segunda vez):** al abrir sesión, `STATE.yaml` declaraba `current_sha=aaed465` / `2.0.7` mientras el repo estaba en `62d4728` / `2.0.8` — **5 commits de deriva** (`1de1caa`, `206584b`, `a030eed`, `c9984b8`, `62d4728`) acumulados en session-21, que sí actualizó el journal pero saltó el puntero de autoridad. Corregido en el mismo ciclo; el guard pasó a `PASS (0 commit(s) de retraso)`. Patrón: session-18 y session-22 repiten exactamente la misma falla. Merece un guard que escriba el puntero, no solo que lo detecte.

**BLOQUEANTE session-22 — CERRADO en `8136bbf`:** `Cargo.lock` estaba en 2.0.7 mientras `Cargo.toml` estaba en 2.0.8 (el commit `62d4728` bumpeó solo el primero). Rompía `ci.yml:36` y `release.yml:63`, que construyen con `--locked` — un checkout limpio fallaba con **exit 101**, y publicar moría en el paso 63 antes de la firma. Cerrado bumpeando `2.0.8 → 2.0.9` con `Cargo.toml`, `manifest.toml` y `Cargo.lock` en el mismo rango (cláusula (A) del hook `pre-push`, **sin tocar la allowlist**). `cargo metadata --locked` pasó de 101 a 0. Además, `tests/test_release_state_pointer.sh` tiene ahora un check 6 que compara `Cargo.lock` contra `Cargo.toml`, falsificado en ambos sentidos: es lo que habría detectado `62d4728`.

Previous: **C4 v1.172.0 — publicado y formalizado** (release v1.172.0 en GH Releases con FC-1 v2 + chromium-skip fix; tag apuntado a `d89c2c0`; `CERTIFICATION-RECEIPT.yaml` schema §5 compliant).

| Campo | Valor observado o pendiente |
| --- | --- |
| Fuente de la fotografía | `main@0ccecae` (session-18) — **pusheado y verificado contra `origin/main`**. Último release **publicado**: tag `v2.0.1` → `5ce4bcacfb8ec4855cd6c543b59f6bda940b7872`. Rango de session-18: `971e0ea..0ccecae`. Reconciliación de puntero aplicada: `STATE.yaml` decía `5ce4bca`/`2.0.1` con 12 commits de retraso; corregido a `e001e39`/`2.0.5` sin reescribir historia, con `certification_claim` (citaba 1.171.2/v1.172.0, 3 majors obsoleto) y `next_action` (decía PAUSE desde session-13) también reconciliados. La fotografía de session-15 abajo se conserva como historia, sin alterar. |
| Workspace en esa fotografía (session-15) | `2.0.1` (Cargo.toml + manifest.toml, commit `5ce4bca`). Último tag SemVer publicado = `v2.0.1` — **coinciden por primera vez** (la serie `1.17x` del workspace había quedado por debajo del tag `v2.0.0` y era rechazada por la admisión v2 con `not-above-last-publish`; alineado en `5ce4bca`). **Session-18: workspace `2.0.5`**, ya no alineado — 4 PATCH por delante del tag. |
| Release pública comprobada en esa fotografía | `v2.0.1` (2026-09-27T19:26:59Z) — GH Releases Latest. Assets: `sddk`, `sddk-v2.0.1-...-musl.tar.gz`, `sddk.sha256`, `bundle.tar.gz(.sha256)`, `CHECKSUMS`, `sbom.json`, `gh-release-receipt.json`. Digest `sddk` publicado por API == sha256sum local `417a7163a286f75b…` (verificado independientemente del script). |
| Hito activo | C1 CERRADO; C2 NOT_EVALUATED (systemic, provider MCP bridges ausentes); C3a-h PASS_OBSERVED; **C4 v1.172.0 publicado + cert formalizado** (`docs/roadmap/receipts/c4-release-v1.172.0/CERTIFICATION-RECEIPT.yaml` schema §5 compliant). **FC-1..FC-8 todos IMPLEMENTED o DEFERRED-DUPLICATED** (session-13 close: FC-4 implementado como shell orchestrator). |
| Estado PRs abiertos | Ninguno. |
| Commits since v1.171.0 (workspace ahead → release v1.172.0) | `0974292 fix(test)` · `e49ff38 chore(release): bump 1.171.0 -> 1.171.1` · `3d24000 feat(cli): extend sddk uat batch with selective filters` · `3c93472 test(cli): add filter predicate coverage for uat batch` · `8a541c3 chore(release): bump 1.171.1 -> 1.171.2` · `550ef84 Revert "chore(release): bump 1.171.1 -> 1.171.2"` · `17f938b docs(roadmap): reconcile session-12 closeout at 1.171.2` · `e9cf84c Reapply "chore(release): bump 1.171.1 -> 1.171.2"` · `d89c2c0 chore(workspace): regenerate Cargo.lock at 1.171.2` · tag `v1.172.0` published · `00e7b56 docs(roadmap): reconcile release v1.172.0 publication` · `96da6db docs(roadmap): v1.172.0 CERTIFICATION-RECEIPT + UAT-EVIDENCE (T29/T31)` · `1a6f7ef docs(roadmap): state sync — v1.172.0 cert formalized (HEAD 96da6db)` · `2321efe docs(debt): formalize legacy 'body **status**: closed' to frontmatter` · `d5823bc docs(roadmap): enrich v1.172.0 cert with flake root-cause analysis` · `fdfe6fb feat(operations): FC-4 docs/operations/uat-replay.sh — pinned-release replay` · `72825fe docs(roadmap): mark FC-4 as IMPLEMENTED in FEATURE-CANDIDATES`. |
| Working tree uncommitted | Vacío. |
| Tests verified this session | Perfil COMPLETO sobre el árbol publicado como `v2.0.1`: **`cargo test --workspace --offline --no-fail-fast` = 5041 passed; 0 failed; 19 ignored (exit 0)**. `cargo clippy --workspace --all-targets -- -D warnings` → clean (0 diagnostics). `cargo fmt --check` → clean. El `release.sh` volvió a correr el perfil completo en step 1 (sin `--skip-tests`). El fix de traversal verificado **en el código publicado** vía `git show v2.0.1:crates/sddk-cli/src/dev/update.rs` (10× `ensure_safe_tarball_members`, 1× `no-same-owner`). |
| Real-provider binary availability | `cognicode` CLI v0.97.3 presente; `cognicode-mcp` PRESENTE (v0.97.3). `chronos-mcp` AUSENTE (no instalable). `jcode` v0.86.0 presente; `jcode-sdk` no publicado. **C2 sigue NOT_EVALUATED** — por `chronos-mcp` ausente. |
| Siguiente acción exacta | Todo el trabajo propio que quedaba está hecho. Las tres salidas restantes son del operador, y todas tocan política o algo irreversible: **(1) Publicar** (elegir entre firmar antes, opción 1 de `ADR-0143 §(a)`, o mantener la rama `sign` con un guard de publicación; publicar `2.0.11` con firma keyless cerraría `INC-AUDIT-S14` en distribución). **(2) Ratificar `ADR-0143`** y decidir sus huecos (b), (c) y (d). **(3) Decisión de C5** (change-scoped verification da su primer consumidor real al SPI de `test_ports.rs`, SPEC-043 §4). La entrada que requiere input externo sigue igual: **Chronos publica `chronos-mcp`** (sin binario, C2 no es ejecutable). Todo lo demás es mantenimiento P2/P3 ya registrado. |
| Evidencia requerida para mover puntero | Para C4 PROMOTION a `CERTIFIED_BASE`: ejecutar C2 con providers reales (cognicode-mcp + chronos-mcp + jcode-sdk instalados), re-run T01-T35 contra el SHA de release, ejecutar `tests/clean_machine_uat.sh --tag v1.172.0` en podman. |
| Bloqueos y decisiones | **C2 sigue cerrado honesto NOT_EVALUATED** (receipts c2a/c2b/c2c). **C3a-h cerrado PASS_OBSERVED**. **Release v1.172.0 publicado + cert formalizado** (commit `96da6db`). **Certificación v1.172.0: PASS_PARTIAL_OBSERVED** (formal: `CERTIFICATION-RECEIPT.yaml`). **Override SemVer LIFTED en v1.171.0 sigue aplicable retroactivamente**; v1.172.0 es SemVer-correct minor (1 feat detectado) — algoritmo canónico coincidió con override. **FC-3 RECHAZADO por duplicación con `sddk ledger export --cycle`**. **FC-1 v2 IMPLEMENTED** (extensión de `UatBatchArgs`, no nuevo subcomando). J7/J8/J9/X08/R11 siguen DEFERRED. **Riesgo honesto documentado**: v1.172.0 fue publicado con `--skip-tests`; el full profile re-ejecutado post-publish pasó 5048/5048, así que la evidencia durable es sólida, pero el contrato literal del release script (correr full profile inline) NO se cumplió para v1.172.0 — registrado como `R-flaw-concurrency-planning-substrate-flake` en cert. |
| Próxima revisión | Al inicio de **cada** sesión y después de cada commit/release relevante |

## Recuperación sin adivinar

1. Confirmar qué rama contiene el nuevo plan, si el PR está integrado y cuál es la versión real de main. Si no está integrado, el puntero de main no ha cambiado.
2. Leer [ROADMAP.md](ROADMAP.md) y [CERTIFICATIONS.md](CERTIFICATIONS.md); localizar el último recibo **observado** del hito activo y los casos [UAT](UAT-MATRIX.md) no ejecutados.
3. Contrastar el último bloque de [SESSION-JOURNAL.md](SESSION-JOURNAL.md) con `git log -5` y el estado operativo; si difieren, registrar reconciliación como **nueva** entrada, sin editar el pasado.
4. Solo entonces abrir/continuar el próximo WorkItem. Un resumen de sesión, un commit de docs o un dry-run no sustituyen un recibo de certificación.

## Estado de certificación (al cierre de session-12 / 2026-09-22T20:58Z)

- **C1 (Base)**: cerrada con full profile 4998/0/15 sobre e7968f8; certificados H02/H05+H06/cycle-c.
- **C2 (Integraciones reales)**: **NOT_EVALUATED_PROVIDER_MISSING (C2a, C2b) / NOT_EVALUATED_ADAPTER_MISSING (C2c)**. Cierre honesto documentado en `docs/roadmap/receipts/c2a/`, `c2b/`, `c2c/`. Status systemic; recovery requires operator decision.
- **C3a-h**: TODOS **PASS_OBSERVED** — Authority hardening (C3a), Storage adversarial (C3b-c), Performance baseline (C3d), Schema resilience (C3e), Migration re-application safety (C3f, closes C3e-F1 via ADR-0141), Performance budget harness (C3g), Supply-chain audit + remediation (C3h, 0 vulns).
- **C4 (Release y certificación de producto)**: **v1.172.0 publicado y formalizado** (2026-09-22T20:23:49Z). `docs/roadmap/receipts/c4-release-v1.172.0/CERTIFICATION-RECEIPT.yaml` schema §5 compliant creado en commit `96da6db` (status: PASS_PARTIAL_OBSERVED, 4 PASS_OBSERVED gates + 12 HISTORICAL_CARRY_OVER + 1 NOT_VERIFIED; T29 + T31 con 6 falsifiers cada uno, 0 triggered). 9 commits ahead of v1.171.0 tag. FC-1 v2 + chromium-skip fix incluidos. Pipeline completo (0-13) PASS vía `bash scripts/release.sh --skip-tests` con admisión v2. Binary sha256 verificado. Instalación local en `~/.local/share/sddk/framework/1.171.2/` OK. Poda de bundles viejos (`1.171.0`) ejecutada. Doctor check: `binary.bundle_coherence: present, all_present: true`. Distrib round-trip OK. **Full profile re-ejecutado post-publish = 5048 passed; 0 failed; 19 ignored** (flake `concurrency_planning_substrate` no triggered en re-run). Promotion a CERTIFIED_BASE blocked por C2 NOT_EVALUATED. Riesgo honesto documentado en cert: el contrato literal de correr full profile inline en release.sh NO se cumplió (--skip-tests usado), aunque el full profile post-publish sí pasó.
- **C5 (Evolución condicionada)**: pendientes P2/P3 (J7/J8/J9/X08/R11), ninguno activo.
- **FC-1 v2 IMPLEMENTED**: `sddk uat batch` extendido con `--scenario`, `--flag`, `--priority`, `--exclude-flaky` (commits `3d24000` + `3c93472`). 7 nuevos tests predicate. Sin duplicación de código: helpers puros (`batch_filter_matches`, `uat_priority_label`) reutilizables.
- **Chromium-skip fix**: test `stale_detects_geometry_change` ahora hace skip limpio cuando chromium no está instalado (commit `0974292`), no panic.
- **Session-12 audit findings** (operador: "CLOSED != CERTIFIED"):
  - RECEIPT.md original (session-11) era un release receipt, NO un CERTIFICATION-RECEIPT (schema §5). Reemplazado por `CERTIFICATION-RECEIPT.yaml` schema-compliant.
  - 40 INCs declaradas `status: closed` fueron auditadas; todas tienen evidencia de cierre (commit refs, test names, exit codes, matrix case counts). NO hay cierres paperwork.
  - FC-3 working tree RECHAZADO: duplica `sddk ledger export --cycle X --output <file>`. La extensión propuesta es extender `LedgerExportArgs` con `--format text|json|jsonl` y `--output opcional` (FUTURO).
  - FC-5 working tree NO implementado pero marcado DEFERRED-DUPLICATED con `sddk fork diff` y `sddk memory diff`.
  - **v1.172.0 cert formal**: PASS_PARTIAL_OBSERVED con T29+T31 PASS_OBSERVED (0 falsifiers triggered de 12 totales). Acepta flake pre-existente `concurrency_planning_substrate` como R-flaw (workspace-wide concurrency, no en isolation). 5048 tests passed post-publish = evidencia durable sólida.

**Distinción importante**: workspace v1.171.2 == binary-version 1.171.2 (alineado). Tag SemVer publicado = v1.172.0 (computed desde commits acumulados). El binario instalado en `~/.local/bin/sddk` corresponde a v1.172.0 (sha256 generado por pipeline). Próximo release: continuar con FC-* restantes.

## Estado real al cierre de session-29 (2026-09-28) — PENDIENTE DE DECISION DEL OPERADOR

> Esta seccion es mas nueva que la de arriba. El "Estado de certificacion"
> de arriba refleja session-12 (v1.172.0) y **ya no describe el estado real**.

### Git

- `HEAD` local = `99bc6526`, arbol limpio, **9 commits sin push**.
- `origin/main` = `a048ddf1`.
- Workspace `2.2.0` (declarado en `a9da3104`, ya en `origin/main`).
- Ultimo tag publicado = `v2.0.1`. **v2.2.0 nunca se publico** (no existe
  tag ni release), asi que el numero 2.2.0 solo existio como commit.

### Lo que hay sin publicar (9 commits)

Tres defectos reales de la ruta de release, corregidos y con mutaciones que
los detectan:

1. `7559a710` — `release-bump.sh` re-bumpeaba cuando el workspace ya declara
   la release. Con 2.2.0 y tag v2.0.1 derivaba v2.3.0, y
   `release-automation.yml` habria auto-mergeado un PR 2.2.0 -> 2.3.0.
   Matriz 7/7, 4 mutaciones.
2. `df55db4c` — cobertura del contrato de `release.sh` paso 2.5, que es la
   ruta que el fix anterior atraviesa. 3 checks conductuales + 2 estaticos,
   4 mutaciones (M4, M6, M7, M8).
3. `46666a3c` — `BUNDLE.toml` declaraba `manifest_sha256` con el hash de la
   **primera linea** del manifest, no con el del manifest. El campo tampoco
   se verificaba en ningun sitio. Mutacion M10.

Ademas: `993def7b` (journal), `5324a528`, `48561746`, `d33e5f40`,
`99bc6526` (documentales), y dos INCs nuevos:

- `INC-DEBT-025-MANIFEST-SHA-FROM-FIRST-LINE` (high/P1, code-fixed)
- `INC-DEBT-026-BUNDLE-CONTENT-NOT-IN-HISTORY` (medium/P2, open) — el bundle
  local mezcla `skills/` de `gentle-ai/sdd` (`~/.config/kilo/`, declarado
  `__managed_by: gentle-ai/sdd`). El asset publicado verifica 377/377, asi
  que no hay incidente de distribucion. Queda abierto determinar si el
  bootstrap de ese proyecto escribe en el bundle de este: seria una
  intrusion y romperia la regla de cero intrusion.

### Evidencia (OBSERVED, sobre el arbol actual)

- `cargo fmt --check` OK, `cargo clippy --workspace --all-targets -D warnings`
  OK, `cargo test --workspace` = **5057 passed / 0 failed / 19 ignored**,
  exit 0.
- Tests shell: 15 PASS / 2 FAIL. Los 2 fallos son preexistentes
  (`test_vault_coherence_alignment` falla tambien sobre HEAD sin mis
  cambios; `test_release_state_pointer` solo dice "aun no publicado").
- shellcheck limpio en los 4 ficheros tocados.
- 12 mutaciones nombradas, cada una falla el gate que la posee.

### EL BLOQUEO (decision del operador)

`githooks/pre-push` rechaza el push: `scripts/**` y `tests/**` no estan en la
allowlist docs-only y el rango no tiene cambio real de
`[workspace.package] version`. El hook no tiene bypass.

Precedente del propio repo para este caso exacto: `a7169537` (fix de codigo
en `scripts/release.sh` + `tests/`) se publico junto a un bump real,
`3a142b86` a 2.1.0. Mismo patron, misma resolucion.

**Correccion de session-29**: se recomendio 2.2.1 alegando que 2.2.0 se
publicaria con el doble bump. Es falso — los 3 fixes estan en el arbol 2.2.0
(8 commits de trabajo tras `a9da3104`, 4 ficheros, 308 inserciones). El
numero 2.2.0 es correcto y nunca se publico.

Dos salidas, ambas legítimas:

- **A)** bump real a **2.2.1** en commit propio, siguiendo el precedente
  `3a142b86`. El hook acepta y el push sale. Se salta un 2.2.0 que nunca
  existio como release.
- **B)** excepcion de hook para publicar **2.2.0** con los fixes dentro.

La eleccion entre A y B cambia el numero publico y la politica del hook, no
la calidad del codigo. **La decision es del operador y no se toma aqui.**

### Siguiente paso ejecutable

1. Operador elige A o B.
2. `git push origin main` (con bump propio si A).
3. `bash scripts/release.sh` (14 pasos) para publicar 2.2.0 o 2.2.1.
4. Post-publish: `sddk dev install` y `sddk dev doctor` (que hoy reporta
   `content.manifest: missing` por el bundle mezclado de INC-DEBT-026).

> **El conteo de commits sin publicar es auto-referencial:** cada commit que lo declara se cuenta a sí mismo. Medir con `git log --oneline origin/main..HEAD | wc -l`.
