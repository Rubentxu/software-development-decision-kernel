# Changelog

All notable changes to this project are documented in this file.

## [2.5.3] - 2026-10-01

### Security
  - fix(release): los pins legacy vuelven a estar inlineados, porque su FORMA es un contrato con el harness que los lee — la migración anterior los renombró a `SDDK_LEGACY_CERT_*`, que parecía más claro y rompió la certificación de v2.2.11–v2.5.2: son exactamente los releases que el ancla legacy existe para mantener verificables, y `certify-sddk-release.pipeline.kts` los extrae de `install.sh` por su forma de parameter expansion. Con el nombre nuevo, `grep` no encontraba nada en un `install.sh` de tag previo y el stage moría. Los defaults vuelven a estar inlineados en `${SDDK_COSIGN_IDENTITY:-…}`, que es lo que contiene un `install.sh` de v2.5.2, y el guard de deriva lee ahora la misma forma. **Un nombre es un contrato con lectores externos al fichero, y renombrarlo es breaking aunque el valor no cambie.** Segundo intento fallido en el mismo sitio: una constante con alias tampoco valía, porque devolvía una indirección que el extractor no sabe seguir. El falsador encontró además un **self-hit de documentación**: un comentario mío citaba el patrón de `grep`, el extractor coincidió con **el comentario** y devolvió un fragmento truncado — de ahí el `grep` que devolvía `SDDK_COSIGN_IDENTITY:-[^`. Con 10 mutaciones, 10 detectadas
  - feat(release): ancla de firma key-based firmada por PipelineK, con la clave privada en un KMS y sin depender de un runner de Actions — la identidad keyless solo existe dentro de un runner de Actions, y `release.sh` completaba los pasos 0–8b y **abortaba en el 8c** con *"the project's signing identity does not exist on this host"*: desde que Actions dejó de ser el CI **no se podía publicar ninguna release**. El comportamiento era correcto (firmar desde un portátil acuña un certificado de *persona*, que el instalador rechaza) y la consecuencia no lo era. Ahora el ancla es la clave pública del proyecto (`cosign verify-blob --key`), firmada por PipelineK contra una clave en KMS para que el material privado nunca aterrice en un host — la alternativa elegida sobre fichero local precisamente por esto (ADR-0151, con la propiedad que se cambia escrita en tabla: keyless nunca tuvo clave en reposo, key-based sí, y esa es la diferencia que se paga a cambio de no depender de un runner hospedado). **El invariante se reforzó, no se cambió**: donde antes se leía el *issuer* del certificado acuñado y se comparaba con una constante —lo que probaba «firmado por algo»—, ahora se **verifica el artefacto contra el ancla que los instaladores pinnean**, lo que prueba «firmado por nosotros». La ventana de transición mantiene el ancla keyless para que `install.sh --version v2.5.2` siga verificando, y `LEGACY_CERT_*` se retira explícitamente al cerrarla. Tres consumidores migrados (`cosign.rs` como autoridad canónica, `install.sh`, `update.rs`) y **falsificados con 8 mutaciones, las 8 detectadas**. La sexta salió de falsificar el propio guard tres veces seguidas: afirmaba vigilar el `bail!` del camino de firma y contaba **cualquiera de los 17 `bail!` del fichero** — habría seguido verde con el `bail` que protegía eliminado, que es exactamente «un PASS que no midió nada», compensado por el mismo defecto que este cambio cierra. El ancla queda bajo `assets/trust/` y no junto al crate porque `crates/` no es superficie de `MANIFEST_SURFACES`: el manifest se quedó en 394 entradas sin quejarse de un fichero que el bundle necesitaba
  - feat(release): el ancla viaja como **cuerpo base64 en una línea**, no como PEM — el guard de deriva compara la copia de `install.sh` contra `cosign.rs` con `sed`, que es *line-oriented*: un PEM multilínea se extrae como cadena vacía en **ambos** lados y el guard pasa sin haber comparado nunca una clave. Verificado contra cosign v3.1.3 que un cuerpo sin framing se rechaza (`PEM decoding failed`), y que el mismo error produce una clave ajena — que es lo que hacía **vacuo** un control negativo: ambas fallaban por formato, no por identidad. Con el framing reconstruido, una clave ajena falla con un error *distinto* (`transparency log certificate does not match`), y eso es lo que hace el negativo discriminante

### Features
  - feat(cycle): `sddk cycle list` enumera los ciclos que ningun comando nombraba — el ledger de `p-63676b11dc0ef88f` tiene 100 ciclos y **ninguna superficie los enumeraba**: `Storage::get_cycle(id)` exige saber el id y nada lo daba, `cycle status` resuelve por lease vivo —los 30 leases estan caducados, luego responde «no active cycle found», que es correcto para la pregunta que hace—, y `ledger events` solo alcanza los 77 que emitieron un hecho y ademas trunca en 50 de 590 sin decirlo. Los otros 23 no los nombraba nada, **17 de ellos `OPEN`**: un ciclo que el ledger afirma abierto y que ningun comando nombra no es un ciclo abierto. `Storage::list_cycles` lee **`cycles`, no `events_v1`**, y esa eleccion **es** el remedio: una enumeracion construida sobre el log de hechos reproducia el defecto que pretende arreglar, porque esos 23 no tienen hechos **por eso** son invisibles. Deliberadamente **no** es `CycleRecord`: ese tipo lleva `manifest: CycleManifest` y 2 filas de este proyecto tienen un `manifest_json` que no deserializa, luego producir uno afirma algo falso —`get_cycle` ya devuelve error en ellas—. La fila ilegible se lista **marcada** (`manifest_readable: false`) y no se tira: tirarla cambiaria «invisible» por «omitido en silencio» y el recuento dejaria de cuadrar sin explicacion. `status` y `phase` son `String` y no el enum del dominio porque `MIGRATION_21` existe justamente porque el conjunto almacenado y el enum pueden discrepar. La salida declara el recuento **antes** de las filas —es la linea que hace visible la truncacion, que es lo que `ledger events` no hace— y `unreadable_manifests`. Verificado con el perfil completo: **5374 passed, 0 failed**, clippy `-D warnings` exit 0, y el falsificador R6 contra el almacenamiento real **PASS=9 FAIL=0** con el sha256 del ledger igual antes y despues. `cli_golden` cayo y su fixture se regenero con el delta revisado linea a linea: **una linea**, el subcomando nuevo. STOP 1 del SCOPE respetado: `get_cycle` no se toca y sus 3 tests de caracterizacion siguen verdes sin reescribir
  - feat(release): el resultado de un apply ya no afirma un lockstep que nadie comprobó — `ReleaseOutcome.version_lockstep_passed` lo escribía **a mano**, después de una comprobación que solo garantiza que no hubo violación: `ensure_version_lockstep(...)?` y luego `let version_lockstep_passed = true;`. Sobre un proyecto Go o Bazel eso informaba «el lockstep pasó» sin que hubiera pasado nada, porque no hay manifiesto contra el que comparar. El campo **no lo leía nadie** en el workspace, de modo que el literal no decidía nada: solo aparecía en la salida. Ahora `ReleaseOutcome` lleva `version_authority: VersionAuthority`, el **tipo del engine**, y la ruta forge lo deriva de `ensure_version_lockstep_detailed`. Esto cierra **D2**, que el lote anterior dejó abierto y escrito. **La razón por la que no se renombra el otro campo, que se llama igual:** el de `LocalReleasePreconditions` es una **puerta** —`release.rs:205` aborta con `Precondition` si es `false`— y su valor llega al storage como la cadena de `ReleaseFailureEvidence::failed_precondition`, que tres tests de integración comparan literalmente. Renombrarlo cambia un contrato de datos durable por claridad en un nombre interno. Con el resultado tipado, los dos campos **dejan de llamarse igual**, y la homonimia que impedía el arreglo desaparece por construcción, no por una nota que lo explique. La puerta **no cambia de semántica** y sus tres tests pasaron sin reescribirlos
  - feat(release): una sola forma de serializar la autoridad, porque el plan y el resultado divergían — con `rename_all` sobre el enum, serde usa la forma **externa** y emitía `{"tag_is_the_only_authority": {...}}`: la clave era el propio nombre de la variante, y el mismo hecho salía con **dos esquemas distintos** en `release plan` y en `release apply`. Con `tag = "kind"` los dos emiten `{"kind": "...", ...}`, y un test del gateway lo fija. Al hacer eso el resultado pasó a usar el tipo del engine, la **copia que tenía en la CLI** se volvió una divergencia con fecha —`declared_in` frente a `candidates`, `undeclared_ecosystems` frente a `ecosystems`, dos comandos hablando de la misma cosa en dos idiomas— y desaparece: quien lea los dos ya no tiene que traducir. El render de texto de ambos sale ahora de la **misma función**, que es la única forma de que no se separen otra vez. Además el doc de `apply_release` seguía diciendo «the workspace Cargo.toml version», una frase que este trabajo dejó de hacer cierta, y como un doc no se ejecuta ningún test de comportamiento lo vigila: el que lo cubre es **estructural**, lee el fuente, y es el cuarto caso de esta serie
  - feat(release): el plan dice de dónde salió la versión, y no disimula cuando no hubo nada que comparar — `release plan` ahora lleva la autoridad, porque el plan de un proyecto Go era indistinguible del de un repo Rust. El lote 1 de ADR-0153 dejó disponible el resultado del contrato (`ensure_version_lockstep_detailed` → `VersionAuthority`) y el propio ADR lo nombraba como riesgo asumido, pero nada lo llevaba a quien decide si publicar: `ensure_version_lockstep` acaba en `map(|_| ())` y el `VersionAuthority` se tiraba. Medido **antes** del cambio sobre un proyecto Go con `go.mod` y sin `Cargo.toml`: **exit 0** y una salida de cinco campos que no distinguía un release comprobado de uno que no tenía nada que comprobar. Go y Bazel no declaran versión en ningún manifiesto, así que el tag no tiene contra qué contrastarse — eso no es un verde y no puede vestirse de uno. Ahora la salida lleva `version_authority` con `kind` (`cross_checked` / `tag_is_the_only_authority`), la versión, los manifiestos leídos y los ecosistemas que no la declaran; en texto dice además `nothing was cross-checked`. El campo es **aditivo**, y un test fija que los seis campos previos siguen igual. El lockstep de un repo Rust no cambia, incluido el texto del refusal: `cli_release_plan_refuses_on_version_mismatch` pasó **sin reescribirlo**. Falsificado end-to-end sobre un proyecto Go y uno Rust montados **fuera de este repositorio** (montarlos dentro falsearía la prueba), 4 mutaciones, las 4 detectadas. **El falsificador encontró un defecto en sí mismo en la primera pasada**: la mutación que quitaba el campo de la salida no compila —no es opcional—, el binario viejo sigue en su sitio, el comando sale con `exit 0` y la aserción lee el artefacto que **no** se mutó; se declaró satisfied midiendo lo contrario de lo que creía. Arreglado en el arnés, no en el código: toda mutación comprueba que su build terminó antes de que se le pregunte nada, y una que no compila se marca `SKIP`, nunca `PASS`. **Lo que NO se cierra, escrito y medido:** la ruta forge de `release_cmd.rs:847-848` sigue escribiendo `let version_lockstep_passed = true;` a mano tras comprobar con `?`, y eso llega a `ReleaseOutcome.version_lockstep_passed`. No se arregla aquí porque los **dos** campos con ese nombre **tienen que significar cosas distintas**: el de `LocalReleasePreconditions` es una puerta que `release.rs:205` lee para abortar, y pasarla a `was_cross_checked()` dejaría a Go y a Bazel **sin poder publicar jamás**; el de `ReleaseOutcome` solo se escribe y se serializa. Decidir cuál de los dos nombres cambia es contrato de `sddk-gateway` y lleva lote propio
  - feat(release): el contrato de donde declara su version un proyecto, y por que el arreglo obvio no funciona (ADR-0153, lote 1) — `ensure_version_lockstep` abria `Cargo.toml` a pelo, de modo que Kotlin, Gradle, Maven, npm y Bazel abortaban, contra el principio de AGENTS.md §2.3. Lo que hacía imposible el arreglo fácil es que **Go y Bazel no declaran versión en ningún manifiesto**: no hay dónde ponerla, luego un registro de «manifiesto → versión» no puede cubrirlos. De ahí las **dos clases de fuente** de ADR-0153, con `TagIsTheOnlyAuthority` como resultado distinto de `CrossChecked` y permitido solo para los ecosistemas cuya entrada lo dice —Rust nunca lo toma, y hay un test que lo fija. El lector es genérico **por formato**, con el conjunto de formatos cerrado: el identificador del ecosistema no aparece en ninguna función de resolución, y un test estructural lo comprueba porque uno de comportamiento no puede — reintroducir `if spec.ecosystem == "rust"` dejaba la suite **verde**. Rescate acotado: la declaración explícita `.sddk/version-source.json` recupera `NotDeclared` y `NoSource`, y **no** `Unreadable`, `Unparsable` ni `Divergent`. Un polyglot con versiones divergentes no se puede publicar hasta que alguien escriba esa declaración: bloqueo deliberado. 50 tests, **12/12** mutaciones, y `release plan` verificado con exit 0 sobre un repo Gradle real
  - feat(identity): alias de identidad de proyecto, lote 3 de 3 — el cableado — el alias se resuelve en el **único** sitio (`resolve_identity_honoring_pin`) y se aplica a **ambas** ramas, la derivada y la pinada: si no, un pin obsoleto —el caso que más necesita redirigir— no alcanzaría el redirect, y el orden de las dos ramas no es intercambiable. Nuevo `sddk project alias --from --to --reason`, con `--reason` **obligatorio** y rechazo de un `--to` que ya tiene ledger. `ResolvedProjectIdentity` expone `alias_hops` y `project resolve` declara la redirección. Se eliminó un **segundo** resolutor que quedaba en `run_project_resolve`. 10 tests, **7/7** mutaciones; la séptima, `resolve_bypasses_the_wiring`, **escapó** porque los tests vivían a ambos lados de la costura y ninguno la cruzaba — se partió `run_project_resolve_with(args, table)` para poder cruzarla
  - feat(identity): alias de identidad de proyecto, lote 2 de 3 — el store — `crates/sddk-cli/src/project_alias.rs` sobre JSON en `$XDG_STATE_HOME/sddk/project-aliases.json`, **no** SQLite: apartarse del SCOPE, y el motivo está escrito, porque `cat` sobre el fichero es la auditoría, y esconderlo detrás de una base de datos sería justo la clase de fallo «redirección silenciosa» que este trabajo viene a cerrar. La tabla es **append-only y sin `remove`**, así que retirar un alias se hace **reconstruyendo la tabla por el store**, nunca editando el JSON por debajo. El store además **rechaza el ciclo al declararlo**, más estricto que el diseño original. 15 tests, **6/6** mutaciones
  - feat(identity): alias de identidad de proyecto, lote 1 de 3 — la capa de dominio — `ProjectAlias`, `AliasTable` append-only, y `AliasResolution` con `hops` observables. `IdentityError::AliasCycle` y `::AliasChainTooLong` son **variantes distintas**, no un mismo error con dos mensajes: el ciclo es un defecto de la entrada y la cadena larga un límite del contrato, y colapsarlos hacía que el autociclo —que sí termina— se diagnosticara como el otro caso. `MAX_ALIAS_HOPS = 16`. 11 tests, **6/6** mutaciones, y la primera pasada **encontró un defecto en el propio test**: `no_cycle_detection` pasaba, pero por el motivo equivocado
  - feat(bundle): sexta superficie `docs/impeccable-reference`, y con ella el tar deja de tener una lista propia — `MANIFEST_SURFACES` pasa a `["agents","skills","prompts/sddk","assets","specs","docs/impeccable-reference"]` con su `impeccable_reference_count`, su brazo de conteo y los dos `tar` de producción. Existe porque `agents/impeccable-primary.md` **sí viaja** en el bundle y citaba dos ficheros que el bundle no llevaba: una promesa que solo resolvía quien clonaba el repo, la misma clase que acababa de cerrar con `specs`. Se podría haber enviado `docs/` entero, pero eso arrastraría `docs/history/` al artefacto; se eligió el subdirectorio, que es exactamente lo que el guard deja escrito
  - feat(bundle): el staging del bundle se deriva de `MANIFEST.sha256` — `scripts/release.sh`aba las superficies con una lista escrita a mano y las volvía a nombrar en el `tar`, y esa quinta copia del contrato ya había producido dos defectos medidos (ver la entrada de `fix(bundle)`). Ahora `awk '{print $2}' MANIFEST.sha256 | xargs -d '\n' cp --parents -t` decide el contenido, más un contrato fail-closed de que el conjunto del staging es exactamente el del manifest + `BUNDLE.toml`. Las dos rutas de producción ya no enuncian el mismo hecho igual, y eso es correcto: `release.sh` deriva porque corre en un árbol de trabajo donde **sí** hay debris; `release.yml` lista porque empaqueta un checkout limpio, donde la lista explícita es legible y auditable
  - feat(bundle): specs viaja en el bundle, con un guard que ata las cuatro copias del contrato — `specs/` pasa a ser quinta superficie de `MANIFEST_SURFACES` (`["agents","skills","prompts/sddk","assets","specs"]`), con su `specs_count` en `ContentsSection`, su brazo de conteo y los dos `tar` de producción actualizados. Cierra el hueco que dejó el commit anterior: las 6 citas de las specs E14 resolvían para quien clonaba el repo, no para quien instala el artefacto, así que el bundle contenía una orden de leer una spec que no tenía. **El riesgo residual de INC-DEBT-052 quedaba anotado como *no implementado* y esta era exactamente la operación que lo activaba**: añadir una superficie sin campo aborta la generación, y añadirla en una sola de las cuatro copias produce un bundle que la omite mientras su manifest afirma cubrirla. `tests/test_bundle_surface_coverage.py` (8 tests) ata ahora las cuatro copias, incluida la deriva **entre las dos rutas de producción**, que el recibo original ni enumeraba. Falsificado con 6 mutaciones, todas detectadas. **Lo que el guard NO cubre, declarado en vez de omitido:** ata las copias por *nombre*, no por *significado* — un renombrado coherente de la superficie y del campo cambiaría el significado sin que nada se queje

### Fixes
  - fix(cli): ledger watch declara cuantos eventos existian, no solo cuantos emitio — con `--max-events 5` sobre 591 eventos decia `emitted 5` y no decia nada de los 586, luego un operador no puede distinguir "se acabo el ledger" de "pare yo a los 5". **Declarar que ha emitido N no es declarar que habia M**, y es la frase que hace falta: la sesion-69f llamo a este comando "el modelo del comportamiento correcto" y la afirmacion era cierta —escribe `[watch] emitted 5 events, exiting`— y por eso llevo a la conclusion equivocada. Es INC-DEBT-060 / F63 en una **tercera** superficie, y F63 **por construccion**: `Storage::list_events_after` (`sddk-storage/src/lib.rs:1022`) recorre todos los streams con `u32::MAX` y luego hace `.take(limit)`, tirando el largo **en cada poll**, desde un `canonical_events()` que ya habia cargado el ledger entero. El total se cuenta con el **cursor inicial** y no con el que acaba la corrida: el primero responde "cuanto habia" y el segundo "cuanto queda", y el segundo no se puede comparar con `emitted`. El filtro sale del bucle a una funcion libre, `apply_watch_filters`, llamada desde los dos sitios, porque copiar los dos `retain` seria una segunda regla que declara que es un evento de esta consulta y puede divergir sin que nada lo note. **`COUNT(*)` se descarto siendo 51,8x mas barato** (0,077 ms frente a 4,015 ms sobre 591 filas, medido): habria exigido probar que su predicado SQL equivale al `retain` en Rust para `--cycle` y `--frame`, y esa prueba no esta hecha. Barato y posiblemente falso no es una mejora. Aditivo: `sddk-cli` 1449 passed, 0 failed, sin reescribir un verde
  - fix(cli): el arreglo habia partido el doc de run_ledger_watch — al insertar `apply_watch_filters` entre el comentario de la funcion y la funcion, esta se quedo **sin documentacion** y la nueva heredo su lista de `Exit conditions`. Lo detecto `clippy` con `doc_lazy_continuation`, que yo lei como un problema de formato y era un problema de **acoplamiento**: un comentario que documenta la funcion equivocada es peor que ninguno. El bloque nuevo va antes del doc original, y se comprueban las dos condiciones: `fmt` limpio y `clippy -D warnings` en 0
  - fix(vault): la frase de alcance de `vault export` ya no puede mentir, porque el conjunto de campos omitidos se **deriva** en vez de mantenerse a mano — lote 2 de `cl-vault-node-projection`, y existe porque el lote 1 se **falsificó a sí mismo`. Su remedio para «la página no dice qué omite» fue una constante `OMITTED_NODE_FIELDS` escrita a mano, con el razonamiento de que una omisión declarada es mejor que una silenciosa. Añadirle `"tags"` — un campo que la proyección **sí** transporta — dejó **todos los tests en verde** y hizo que la página dijera: *«carries 7 of 8 fields … Not carried: body, tags»*. Dos afirmaciones en la misma línea, contradiciéndose (7 + 2 ≠ 8), y la segunda **falsa**. Eso es **peor que el silencio que sustituyó**: una página que miente no es una página incompleta. Una lista escrita a mano, introducida para arreglar una lista escrita a mano, es el mismo defecto con sombrero nuevo — y el test que debía cazarla comparaba la proyección contra el **tipo** y nunca contra la **lista**. La constante **desaparece**: la frase se deriva serializando un nodo dos veces —una por la proyección, otra por el tipo— y nombrando la diferencia, que es lo mismo que haría a mano quien la leyera. No queda ninguna entrada que alguien pueda editar para volver a decir algo falso. `body` sigue sin transportarse y el motivo sigue junto al campo, ahora como doc del tipo y no como dato. **R5 comprueba que la frase sea aritméticamente cerrada y cierta**: el recuento transportado, el total y el número nombrado tienen que cerrar, y nada nombrado puede estar presente. Falsificado con la misma mutación que destapó el defecto —la lista derivada sustituida por una fija que incluye `tags`—: **R5 cae** con el 2 contra 1, ambos conjuntos impresos y la frase citada, **y R1 seguía en verde**, que es la prueba de que el test nuevo aporta algo que el anterior no daba. Y `clippy -D warnings` hizo su parte: al quedar la constante sin uso la reporta como `dead_code` y obliga a borrarla, que es exactamente lo que debe pasar con una segunda fuente de verdad
  - fix(vault): el HTML de `vault export` dice que campos del nodo lleva y cuales omite, y deja de reconstruirlos a mano — la cuarta superficie de la misma clase, y la primera encontrada **por criterio** en vez de por analogía de un defecto anterior. `VaultNode` deriva `Serialize` y tiene **ocho** campos; `export_node` era un `serde_json::json!` con **seis**, que descartaba `tags` y `body` **sin declarar nada**. Medido contra el binario real con un vault que lleva `tags` en el frontmatter: `status` viaja y tiene columna propia en la tabla, `tags` no viaja y no tiene columna — mismo tipo de metadato, dos reglas, ninguna derivable. **La asimetría es lo que lo hace defecto y no decisión.** Ahora la proyección se deriva del nodo y la página declara su alcance antes de mostrar un solo dato, con **ambos recuentos derivados de la serialización** y no escritos a mano: una constante ahí sería el mismo defecto un nivel más abajo. `body` **sigue sin viajar**, con el motivo escrito en el sitio donde se decide, y R4 es el guard contra el arreglo obvio y erróneo — declararlo todo incrustando cada documento y dejando una página honesta e inútil. **Y el falsificador encontró un defecto en el remedio, que es lo que más valor tiene aquí:** O2 afirmaba que `From<&VaultNode>` hacía que añadir un campo a `VaultNode` fuera **error de compilación**. **Es falso, y medido:** se añadió `mutant_field`, se actualizó el parser para satisfacerlo, y `cargo build` **pasó**. Un `From` entre dos tipos **distintos** no es exhaustivo por ningún lado, y el mismo doc de `GraphExport` afirmaba lo mismo desde el ciclo anterior: **corregido en los dos sitios**, con la medición escrita al lado. El acoplamiento real lo sostiene un test que fija `transportados == declarados − omitidos` con los dos conjuntos **derivados del tipo**, no escritos a mano. Ese test tuvo dos versiones previas **incorrectas**: la primera repetía la lista de ocho campos como literal —el defecto bajo prueba con otro sombrero— y la segunda construía un `VaultNode` literal, con lo que la mutación hacía que **el fichero de test dejara de compilar** y la aserción útil nunca llegara a ejecutarse: un guard que dispara por el motivo equivocado tiene la misma forma que uno que no dispara. La sonda es ahora un nodo **parseado de un fixture real**, y con la mutación activa R2 cae nombrando `mutant_field` y R1 reporta «7 de 9»
  - fix(vault): el HTML que exporta `vault export` declara lo mismo que `vault graph`, y ya no puede volver a separarse — el riesgo 3 del recibo del ciclo anterior decía que la réplica HTML «no se contradice con el grafo» pero que **no se había verificado** si era intencional. Medido: **no lo era**, y era una **tercera superficie** de la misma clase de defecto. `GraphExport` (`export.rs:90-95`) no era una vista parcial de `GraphView`: era una **estructura distinta con tres campos escrita a mano**, luego las dos listas de campos podían separarse sin que nada lo notara. Sobre el mismo vault de dos ciclos disjuntos, el JSON incrustado en la página declaraba `cyclic` y nada más: sin `cycle_count`, sin `multiple_cycles`, y con `topological_order` ausente **sin decir por qué** — el defecto de `vault graph` intacto, en una superficie que nadie miraba. Y el STOP 4 del ciclo anterior **se cumplió literalmente con el defecto entero presente**, porque decía «si `vault export` deja de cuadrar con `vault graph`» y el HTML no afirma nada *falso* sobre el grafo: sencillamente no declara lo mismo. **Una condición que se puede cumplir con el defecto ahí no es un guard**, y queda anotado como tal en el SCOPE. El arreglo invierte el control: el mapeo pasa de un literal en el sitio de llamada a un `impl From<&GraphView>`, de modo que **añadir un campo a `GraphView` sin decidir qué dice la página es un error de compilación** y no una divergencia silenciosa — un sitio de llamada no es donde mira un compilador. Los `skip_serializing_if` de `GraphExport` **no son cosméticos**: replican los de `GraphView` porque las dos superficies tienen que coincidir también en **qué claves están presentes**, no solo en sus valores; sin ellos esta estructura emite `"topological_order": null` donde `GraphView` omite la clave, y un consumidor que compare los dos documentos ve una diferencia donde no la hay. El HTML visible —tabla de nodos, estilos, título— **no se toca**, y el test existente del módulo **no se reescribe**: sus siete aserciones siguen verdad, que es la comprobación de que el cambio es aditivo. **Y el primer R1 estaba mal, no el producto:** afirmaba que la clave `cycle_count` tiene que estar **presente**, y falló contra una implementación correcta. La forma saturada **omite** `cycle_count` cuando la respuesta es «2 o más», porque `None` **es** la codificación de «2 o más» y `multiple_cycles: true` es lo que la hace distinguible de «ausente»; exigir la clave would've reintroducido exactamente la ambigüedad que el ciclo quita, y además habría forzado un `cycle_count: null` que `vault graph` **no** emite — una divergencia de signo contrario. Se corrigió **el guard**: la propiedad comprobada es la **forma** —un entero exacto cuando es 0 o 1, y `multiple_cycles: true` sin recuento cuando es 2 o más—, no la presencia. El script de medición tenía **la misma aserción equivocada** y la corrigió el mismo diagnóstico: primero en el test, después en el script, y en los dos casos el producto estaba bien desde el principio
  - fix(release): el remedio de un gate de release ya no puede salir con exito sin haber medido nada — `tests/test_vault_adr_mirror_coverage.sh` es gate del pipeline (`release.sh:217,234`) y hoy esta **rojo en HEAD**, con 3 ADR `accepted` sin reflejar: ADR-0151, ADR-0152 y ADR-0153, promovidos en estas ultimas sesiones. Lo que el gate imprime como remedio —`Run: python3 scripts/mirror_adrs_to_vault.py`— **no podia correr bien en ninguna maquina que no sea esta**: `REPO_ROOT` estaba hardcodeado a `/home/rubentxu/Proyectos/agentesIA/sddk-framework`, una ruta que aqui solo funciona porque es un symlink al disco real. Y el fallo no habria sido visible: `Path.glob` sobre un directorio inexistente devuelve un iterador **vacio**, luego el script imprimia `created: 0, skipped: 0` y salia con **0** — el mismo caso de «un PASS que no midio nada», pero en el guion que se ejecuta **cuando algo ya ha ido mal**. Medido antes de tocarlo, no supuesto: `glob` sobre ruta ausente devuelve `[]`, luego `created:0 skipped:0, exit 0, sin decir nada`. El arreglo tiene tres partes y las tres son el mismo principio: **`REPO_ROOT` se deriva de `__file__`** —con lo que el guion funciona en cualquier clon, no solo en el de esta maquina—, y el `main` **falla cerrado** en los tres estados vacios que antes pasaban: no existe el directorio de ADRs, existe pero no tiene ADRs, y hay ADRs pero ninguno `accepted`. Cada uno dice **por que** en stderr y devuelve 1. El recuento de `accepted` se **declara** en la salida junto al repo que lo produjo, porque un `created: 0` sin decir cuantos ADRs se consideraron es indistinguible de un `created: 0` porque ya estaban todos. Y la correccion **no cambia** lo que el guion hace cuando si hay trabajo: sigue siendo **aditivo y nunca reescribe** —`if target.exists(): skipped`—, verificado con las marcas de tiempo de los 57 ficheros del vault antes y despues de una segunda pasada, byte a byte intactas. Falsificado con **5 escenarios** sobre un arbol de ADRs sintetico mas el repo real: los tres vacios fallan cerrado nombrando la causa, el repo real sale 0 declarando los **57** `accepted` que un recuento a mano confirma, y la idempotencia crea 0. El gate que lozilla **vuelve a PASS** con los 57 ADRs reflejados
  - fix(vault-graph): `vault graph` declara cuantos ciclos hay y por que falta el orden topologico — el caso aciclico **parece** correcto, y por eso el defecto se escondia: `node_count` 30 coincide con el real y el orden sale completo, luego nada falla. El defecto aparece al medir el caso que la funcion **no** promete: `find_sample_cycle` (`graph.rs:88-94`) devuelve el **primer** ciclo que encuentra y para, y `GraphView` no tenia ningun campo de recuento. Con **2 ciclos disjuntos** la salida daba `sample_cycle` con uno y callaba sobre cuantos hay, y el `topological_order` **desaparecia** sin decir por que — una linea que no esta se lee como «no se computo», que es otra afirmacion distinta de «no existe porque el grafo es ciclico». **La solucion obvia se refuto midiendo, y esa es la parte que manda**: contar ciclos de forma ingenua no es lento, es **incorrecto antes que lento** — un anillo de 1000 nodos tiene **1** ciclo y el recuento ingenuo devuelve **1000** rotaciones de el (**558 ms**), y un bouquet de 500 cuenta 1000 porque cada ciclo se recorre en las dos direcciones. Un campo que parece una verdad y no lo es es peor que ningun campo, luego la forma elegida **satura**: `cycle_count` es `0` o `1` exactos y `None` cuando hay mas, con `multiple_cycles` al lado para que `None` sea distinguible de «ausente», y `topological_order_absent_because: "cyclic"`. **Todos son campos anadidos, STOP 3 vacio**: el consumidor existente (`cli.rs:8579`) no se reescribe. Lo que responde es **la unica pregunta que un `sample_cycle` le debe a quien lo lee** —¿hay uno o hay mas?— y cuesta **una pasada extra**: se quitan las aristas del ciclo de muestra y se vuelve a buscar, misma complejidad que la pasada que lo encontro, y ningun contador que pueda mentir. `vault show` **se midio y NO es de esta clase**: su `backlinks` no tiene cota, es el array completo. Medir tambien descarta, y por eso los dos comandos que las sesiones anteriores dejaban como «NO MEDIDOS» estan ahora **medidos** —uno es defecto y el otro no, que era exactamente la distincion que falto con `ledger watch`. El falsificador cubre **6 escenarios** sobre tres vaults (aciclico, un ciclo, dos ciclos) mas que `vault export` no puede contradecir a `vault graph`, y **encontro dos bugs suyos** antes de poder medir el producto: `vault export` tiene un guard fail-closed (ADR-0082, `writer.rs:76`) que **canonicaliza** el directorio de datos del proyecto y rechaza si no existe, y el arnes escribia en la raiz temporal; y el segundo leia el `project_id` con un glob sobre el arbol de **data**, que `vault graph` **no crea** —crea el de **state**—, luego la busqueda no encontraba nada. Los dos fallaban «por la razon equivocada», que es un FAIL que no media nada
  - fix(vault): `vault search` declara cuanto dejo fuera, y `--limit 0` ya no significa cero — misma clase de defecto que INC-DEBT-060 F63, en otra superficie. Medido sobre el indice real de esta maquina: imprimia **20 de 75 documentos** sin declarar ningun total, con **exit 0**; `--limit 0` devolvia `no hits`; y el JSON era un array desnudo. **El total aqui NO viene gratis, y eso cambia el diseno**: en `ledger events` venia en memoria porque el corte es posterior al `take`, mientras que `search_index` corta en SQL (`LIMIT ?2`, `search.rs:174`), luego declararlo exige una segunda consulta. La alternativa de pedir `LIMIT n+1` y deducir «hay al menos uno mas» se descarto **midiendo**: el `COUNT` resulta **mas barato que la propia busqueda** (0,119 ms contra 0,376 ms con 75 docs) y el sobrecoste **baja** con la escala, de +31,7 % a +7,8 % y a **+5,2 %** con 7500 documentos, porque `ORDER BY rank LIMIT 20` tiene que ordenar todos los matchs mientras que `COUNT ... WHERE MATCH` solo los recorre. Por eso se paga el total **exacto**: el dato completo sale mas barato que el parcial, y un total que a veces es exacto y a veces no obliga a quien lo lee a descubrir cual de los dos tiene. **`search_index` NO cambia de firma** — es API publica de `sddk-vault` con **8 tests unitarios** que la usan directamente, y el camino corto de devolver `(hits, total)` los rompia a todos—; el total se obtiene con `count_matches`, una funcion nueva, porque anadir no rompe nada. **`--limit 0` es la cuarta vez que este binario contradice su propia convencion**: `ledger export --limit 0`, `ledger events --limit 0` y `ledger watch --max-events 0` significan todos, y aqui `LIMIT 0` en SQL no es caso especial y devuelve cero filas. El impacto se midio **antes** de tocar nada —consumidores del JSON en el repo, **1**, y su asercion de fondo no se mueve—, que es donde el ciclo de F63 fallo y rompio el build. Verificado: **5384 passed, 0 failed**, clippy `-D warnings` exit 0, `fmt --check` limpio, y el falsificador O1-O4 contra el indice real **PASS=9 FAIL=0** con `declared=61` contrastado contra `sql=61` y el sha256 del indice intacto. **Dos fallos del arnes, ninguno del producto**, y el segundo importa mas que el primero: el fixture de R1 no cassaba —FTS5 hace coincidencia de token exacto sin stemming, asi que `crypto` no encuentra `cryptography`— y eso dejaba **R6 vacio**, con su rama «con coincidencias» ejercitando en realidad la de «sin coincidencias» y pasando igual
  - fix(cli): `ledger events` declara cuanto dejo fuera, y `--limit 0` ya no significa cero — INC-DEBT-060, falsificador **F63**. Medido sobre el ledger real de `p-63676b11dc0ef88f` en copia byte-identica: 590 eventos, 114 streams, y el comando imprimia **50**, nombraba **19 de los 114 ciclos**, no declaraba ningun total y salia con **exit 0**. La ventana por defecto son las secuencias 12 a 20: las 50 mas recientes, con las 540 anteriores invisibles y nada en pantalla que lo diga — un `ledger events` sin `--limit` decia «el ledger» y no decia «una ventana». **El total ya estaba en mano y se tiraba**, y por eso el lote no toca storage: `Storage::list_events` (`lib.rs:974`) llama a `canonical_events`, que recorre TODOS los streams con `u32::MAX` y devuelve el vector entero, y el truncamiento ocurre despues en memoria con `.take(args.limit)` (`ledger.rs:410`); `total_events` es `all.len()` **antes** del `take`. Ninguna consulta nueva, ninguna API de storage, ninguna migracion. El texto declara `events: {shown} of {total} (truncated|complete)` **en ambos casos**, porque una declaracion que solo aparece al trucar no se puede leer en un log donde no truca, y «no truca» es justo lo que hay que poder comprobar. **El JSON pasa de array a `{events, total_events, shown, truncated}`**: no es un campo que faltara anadir, es que un array no tiene donde llevar un total. Y `--limit 0` pasa a significar «todos», igual que `ledger export --limit 0` (`ledger.rs:442-445`) y que `ledger watch --max-events 0`, cuando antes significaba CERO con lista vacia y exit 0 — la convencion invertida a un comando de distancia. **CAMBIO DE FORMA, DECLARADO, con el impacto mal medido al principio:** son **dos** consumidores en el repo y no uno, porque se busco con un grep sobre una lista de ficheros elegida a mano en vez de sobre el arbol, y el segundo (`aiw_s8_x07_real_binary_boundary.rs:292`) salio por el perfil completo del workspace, o sea **despues de romper el build**. Y `skills/` ni se miro, siendo superficie del bundle distribuido: `skills/sddk-cycle-resume/SKILL.md:62` ejecuta este comando con `--format json`; examinado y **no es rotura** —la skill no parsea el array, pide al agente que reconstruya la cadena causal leyendola, y una envoltura que dice «10 de 590» es mas informativa—, pero pudo no serlo. En los dos tests **la asercion que significa algo sobrevive intacta**: cuantos eventos y cuales. Quinta vez en este ciclo que medir con el instrumento equivocado produce un numero falso, y la quinta vez el numero iba a un documento. Verificado con el perfil completo: **5378 passed, 0 failed**, clippy `-D warnings` exit 0, `fmt --check` limpio, y el falsificador O1-O4 contra el almacenamiento real **PASS=7 FAIL=0** con `declared=590` contrastado contra `sql=590` y el sha256 del ledger identico antes y despues
  - fix(identity): la identidad se resuelve UNA vez y entra ya resuelta en el engine — `AdoptionPlanInput` deja de llevar `remote_url`, `pinned_project_id`, `scope` y `fallback_seed`, y lleva `identity: ResolvedProjectIdentity`; `plan_adoption` deja de llamar a `resolve_project_identity` **por completo**. Es eliminación y no un campo opcional a propósito: con la identidad ya resuelta, derivar por dentro es imposible porque el input ya no tiene de qué derivar, luego «un solo punto de decisión» de ADR-0152 queda cierto **por construcción** y no por nota; con un campo opcional alguien reañade la llamada y ningún test de comportamiento lo nota, porque un reader que hardcodea una derivación se comporta igual mientras el alias no esté declarado. La forma corta —pasar el id resuelto por `pinned_project_id`— no exige tocar nada y **no funciona**: el engine lo trataría como pin, `identity_source` no viajaría y `alias_origin` se perdería un nivel más adentro, con una forma que *parece* correcta. **Cuatro superficies** resuelven por el resolver canónico, y la cuarta la encontró **el falsificador** buscando un segundo resolutor con las otras tres ya arregladas: `sddk generate docs` escribía la documentación generada bajo el data dir del id **retirado** mientras `project resolve` nombraba el superviviente. Es la cuarta afirmación de convergencia que este trabajo producía y era falsa; no salió leyendo el SCOPE ni midiendo el arranque, salió **contando puntos de llamada** — y por eso el guard nuevo cuenta llamadas con paréntesis y exige **exactamente una**: prohibir el nombre dejaría fuera la llamada legítima del resolver canónico, y contarlo sin paréntesis contaba también la línea `use`. De paso, un defecto **preexistente**: `find_persisted_fallback_seed` solo encontraba recibos con `identity_source == Fallback`, luego un recibo escrito **bajo un pin** era invisible y `generate docs` caía en silencio al fallback in-repo en todo checkout pinneado y sin remote. La forma clásica de arreglarlo —derivar la semilla de la ruta canónica— es **demasiado**: convierte cualquier directorio en un proyecto y deja muerto ese fallback, y lo cazó `real_cli_exit_status_tracks_lint_errors_and_stale_checks` con `SDDK009`. El arreglo correcto era **una cláusula en el predicado**: el pin sobrescribe el `project_id`, no la semilla. Los dos tests del pin que vivían en el engine se midieron antes de moverlos: uno **ya estaba cubierto** e2e donde ahora se decide, y el otro era un **hueco real**, rellenado con `a_malformed_pin_fails_closed_instead_of_deriving` en el mismo cambio. **5361 tests del workspace a 0 fallos**, fmt y clippy limpios. Detalle y lo que **no** limpia en `docs/debt/INC-DEBT-059-…md`
  - fix(ci): el gate de criterios de ADR-0153 existia pero nadie lo ejecutaba — `tests/test_adr_0153_criteria.sh` se creó en session-68 al aceptar ADR-0153 y **nunca se cableó al runner**, así que el único guard que mide los siete criterios **uno a uno** —el que impide que un PASS agregado tape un criterio rojo— no corría en ningún release. `tests/test_gate_coverage.py` lo dice en voz alta y llevaba rojo desde entonces (`SIN runner y SIN motivo: 1`). Dos fallos encadenados, y el segundo habría sido el que miente: no estaba en la lista de `release.sh`, **y no era ejecutable** (`-rw-r--r--`), con lo que el runner hace `if [ -x "$t" ]` con `warn … skipping` en la negativa. Añadirlo a la lista sin el `chmod` habría convertido un FAIL ruidoso en un **skip silencioso**: cableado en apariencia, ejecutado nunca. Medido después: `PASS=7 FAIL=0` con los siete criterios por separado, y el gate de cobertura en `con runner: 36 · SIN runner y SIN motivo: 0`
  - declarar el fix del gate, y no citar la cabecera que el gate cuenta — el paso 2b exige una entrada por cada commit `feat`/`fix`/`test` desde el último tag, y su propio `fix(changelog)` anterior no tenía la suya: el gate lo detectó y con razón. **Y esa entrada casi lo volvió a romper por el motivo contrario al que lo había roto antes.** Cuenta la cadena del encabezado con `grep -cF` sobre el **fichero entero**, no sobre las cabeceras, luego una entrada que **cita su propio encabezado** cuenta como una segunda y falla con «expected exactly one header». Por eso esta no la cita: la describe. Es un caso que solo se ve al redactar, y por eso queda escrito en la propia línea
  - fix(release): el error de «no hay manifiesto» no decia cual era la salida — al recorrer los criterios de cierre de INC-DEBT-051 se midio F58 de extremo a extremo y el error **cumplia la mitad** de lo que exigia: listaba los trece manifiestos que busca, pero no mencionaba `.sddk/version-source.json`, que es justamente la salida para un proyecto cuyo ecosistema no declara version en ningun sitio. Quien lo recibia tenia una lista de rutas y ningun camino adelante, y la declaracion existia sin aparecer en el mensaje. El mensaje la nombra ahora, y un test lo mide: antes el test solo afirmaba la primera mitad, dos mitades de un mismo casi que se confundian con el todo. Es la clase de hueco que aparece cuando un criterio se marca cumplido por parecer resuelto en lugar de por medirse
  - fix(changelog): el gate de cobertura estaba rojo desde el lote 1, y mi entrada no casaba con su huella — `tests/test_changelog_coverage.sh` es el paso 2b de `release.sh` y abortaba con **PASS=29 FAIL=8**. Solo uno de los ocho fallos era del lote que lo ejecutaba; los otros siete eran trabajo del mismo objetivo —los tres lotes del alias de identidad, la auditoría, las dos entradas de deuda del cierre por alias y el lote 1 del contrato de versión— **sin declarar** en la sección de esta versión. Eso significa que **v2.5.3 tenía dos bloqueos, no uno**, y el segundo era invisible porque 2b solo corre en el paso 2 del release, que está parado en el 8c por la firma. **Esta propia entrada casi lo vuelve a romper, y el motivo está escrito aquí a propósito:** el gate cuenta la cadena del encabezado con `grep -cF` sobre el fichero entero, no sobre las cabeceras, luego una entrada que **cita su propio encabezado** cuenta como una segunda y el gate falla con «expected exactly one header». No se cita la cabecera: se describe. El fallo propio era de forma, no de contenido: la huella del gate son las **cuatro primeras palabras del payload**, en minúsculas y **sin normalizar acentos** —su `norm()` solo colapsa mayúsculas y espacios—, y la entrada empezaba por el nombre del comando en vez de por el sujeto del commit. Las tres lotes de `feat(identity)` comparten huella, luego una sola línea habría cubierto tres commits y el gate lo habría aceptado sin distinguir nada. **PASS=38 FAIL=0**, la primera vez que este gate queda en verde
  - fix(debt): el audit no veia la tabla de aliases, y comparaba literales — `migrate_project_identity.py` resolvia por el camino viejo, así que sobre un storage con alias **no redirects** y, peor, comparaba cadenas literales: dos `project_id` que resuelven al mismo destino se contaban como divergencia. Ahora carga la tabla, aplica la cadena **después** de derivar, compara **formas resueltas** en vez de literales, valida ciclo y cadena larga, y falla cerrado con exit 4. `roots()` honra `SDDK_STATE_HOME`. **5/5** falsificaciones, y dos las hice mal la primera vez: escribí el fichero corrupto fuera de la ruta que el código lee, luego el guard no podía verlo
  - fix(debt): la migracion de project_id no existe, porque la identidad esta horneada en el fact log — el `project_id` se deriva de un hash de contenido dentro de un **fact log append-only**, luego ninguna reescritura del storage lo preserva: cambiarlo exige reescribir historia, y el registro de deuda que proponía esa reescritura **no tenía cómo ejecutarse**. INC-DEBT-049 y INC-DEBT-050 son **una sola deuda**, no dos, y se cierran por otra vía: un **alias de resolución** que no toca el fact log, no pierde recibos y **converge** —el pin por checkout, la alternativa, no converge
  - fix(debt): el apply de migracion de project_id era inejecutable y corrompia ledgers — el apply que acompañaba a la deuda anterior escribía fuera de la ruta canónica, así que los artefactos no estaban donde el código los leía, y comparaba identificadores por igualdad literal: aplicado «con éxito» sobre un storage ya redirigido, producía **divergencias inventadas** y dejaba recibos nuevos. El defecto no era la idea sino su ejecución, y por eso el cierre por alias no reutiliza este camino
  - fix(adoption): comparar el remoto como identidad normalizada, no como cadena cruda — `adopt status` respondía `conflict` sobre el storage **ya convergido** de este repo: mismo `project_id`, mismo workspace, mismos paths, y el release 2.5.3 lo decía con el detalle *"receipt identity differs from plan; refresh only accepts runtime metadata drift"* — un contrato absurdo aplicado a un caso que no es runtime drift, sino el plan olvidando quién es. Eran **tres sitios**, no uno: `plan_adoption` (rama `Some(pinned)`) construía la identidad con `remote_url: None`, y tanto `same_identity` como `inspect_ledger` comparaban el remoto **en crudo**. Los dos últimos son independientes —arreglar el segundo sin el tercero solo traslada el conflicto—, de ahí una única función `remote_urls_match` para ambos. La comparación cruda contradecía una decisión ya tomada: el dominio normaliza owner/repo a minúsculas **antes** de hashear el `project_id` (test golden `case_change_in_owner_or_repo_resolves_to_same_project_id`), y el recibo y la fila `projects` guardan `Rubentxu/…` porque se acuñaron el 2026-09-30, antes de `52182522`. El pin sobreescribe **solo** `project_id`; `identity_source` se conserva en `Pinned` porque `context_cmd.rs:1066-1069` lo lee para reenviar el pin. **Verificado end-to-end**, mismo repo, mismo pin, mismo storage y solo cambiando el binario: release 2.5.3 → `conflict`; el binario con el arreglo → `complete`. **El falsador encontró la mitad negativa que faltaba**: sustituir la comparación por una que devuelve `true` siempre que ambos lados normalicen dejó los tests positivos **EN VERDE**, porque fijaban «el mismo repo con otro case ya no es conflicto» pero no «un repo distinto sigue siendo conflicto» — un guard que declara siempre coincidencia era aceptable. Los negativos usan **pin** a propósito: sin él un remoto distinto acuña otro `project_id` y el veredicto es `Absent`, no `Conflict`, luego discriminaban por el guard equivocado. INC-DEBT-049 no se cierra: su parte abierta (declarar historial bajo otra identidad) sigue necessitando SCOPE + ADR
  - fix(debt): el guard de coherencia llevaba rojo desde session-65b y nadie lo ejecutaba — `scripts/check_debt_index_coherence.sh` salía con **exit 1** sobre este repo y llevaba así desde session-65b, por dos causas encadenadas. (1) INC-DEBT-052 declaraba su estado como `**status:** resolved` en markdown bold, un dialecto que el guard no lee: lee frontmatter YAML y la prosa `**Estado:**`, y aquel no era ninguno de los dos. (2) El guard no estaba referenciado por ningún runner: `ci.yml:46` hace `shellcheck` —que es lint, no ejecución— y `release.sh` corría el **test de fixtures**, que monta un árbol desechable por caso. Sus 10 casos PASABAN, y ese verde leía como cobertura. Es la forma del INC-DEBT-033 un nivel más hondo: el propio header del guard advierte que un test que no puede mover el sujeto bajo test no puede falsificarlo, y el suite entero pasaba porque pasaba contra fixtures. **El método casi se diagnostica al revés**: la primera hipótesis fue que el guard era ciego al dialecto, y estuvo a punto de «ablandarse» para aceptar la entrada. Lo refutó la evidencia — lo que se había ejecutado era el TEST, no el GUARD, y un `PASS=10 FAIL=0` leído como «el guard funciona» es justo el falso positivo que este repo lleva slices persiguiendo. **Corrección en la dirección correcta: el documento se ajustó al contrato, no el contrato al documento.** `resolved` sigue siendo `resolved`; lo que cambia es que ahora es legible por máquina, y el guard queda **más estricto** después, no antes: falsificado: ablandar el guard para aceptar `**status:**` rompe el test. 2 mutaciones, 2 detectadas
  - fix(bundle): el arreglo del staging **produjo dos defectos nuevos que ningún gate vio** — (1) **`MANIFEST.sha256` dejó de viajar en el bundle**: el manifest no puede listarse a sí mismo —un fichero no puede contener su propio digest—, luego derivar el staging de él descarta el fichero que el bundle más necesita, y tanto `release.yml:230` (`bundle lacks MANIFEST.sha256`) como `update.rs` lo tratan como **required**: el release se habría roto en la ruta cloud y en cada instalación. (2) **El prefijo del tarball se duplicó en los 396 miembros**: el `--xform` transforma el nombre del miembro, y al recibir el directorio ya envuelto le prepende el mismo prefijo — medido, **608 miembros** con `software-development-decision-kernel/software-development-decision-kernel/…`. Que los produjera el arreglo de (a) y (b) es lo incómodo del caso: la corrección se verificó con 12/12 del guard de superficies y 9/9 mutaciones, y **nadie empaquetó nada**. Un guard que compara código no puede observar un artefacto mal construido; los dos nacen de verificar el staging y nunca el tarball. `tests/test_release_bundle_step5.sh` stage → `tar` → extraer → preguntar al bundle extraído lo que preguntarían `install.sh` y `update.rs`: presencia de `MANIFEST.sha256` y `BUNDLE.toml`, prefijo único, ancla igual al manifest que viajó, las 394 entradas con su digest verificado, y ningún artefacto gitignored a bordo. 3 mutaciones, las 3 detectadas, una por defecto
  - fix(release): el lockstep leia la version de una DEPENDENCIA, y abortaba en dos formas de `Cargo.toml` validas — el parser de `ensure_version_lockstep` no usaba un parser TOML: recorria el fichero linea a linea y entraba en «modo workspace» con `starts_with("[workspace")`, que **tambien coincide con `[workspace.dependencies]`**. Con esa tabla antes de `[workspace.package]` —orden legal, y el que emite el propio Cargo cuando las dependencias se declaran primero— leia la clave `version` de una dependencia y la devolvia como version del proyecto. RED medido contra el codigo real: `left: "9.9.9" / right: "1.42.5"`. **El fallo era silencioso**: no abortaba, devolvia un veredicto seguro sobre un numero que describe otra cosa — un tag `v9.9.9` habria **autorizado un release** sobre la version de una dependencia, y el tag correcto `v1.42.5` habria sido rechazado. Ademas abortaba con `version = '1.2.3'` (comilla simple, TOML valido que el parser solo despejaba en `"`) y en repos de un solo crate, donde la version vive en `[package]` y el mensaje de error mencionaba un `[workspace]` inexistente. Sustituido por el parser TOML real (`toml`, la misma dependencia que `sddk-cli` ya usaba) con la precedencia **escrita y total** — `[workspace.package]`, despues `[workspace]`, despues `[package]`— en vez de «la primera clave `version` que aparezca». Añadir un formato nuevo es ahora una entrada mas en el sitio que ya define la precedencia, no un `if` mas en el parser: la lista escrita a mano es exactamente lo que produjo los defectos (a) y (b) de INC-DEBT-056 con las superficies del bundle. 4 mutaciones falsificadoras, las 4 detectadas; una de ellas degrada el error de parseo a tabla vacia, que convierte un error tipado en «no hay version» — la misma suplantacion que `prompts_count = 0`. **INC-DEBT-051 NO se cierra**: sigue leyendo `Cargo.toml` y un proyecto Kotlin, Gradle, Maven, npm o Bazel sigue abortando. Lo que se arregla es un defecto distinto de la misma linea, que hacia el contrato futuro mas dificil de verificar. El caso real se reprodujo hoy: `sddk release plan --tag v0.45.0` sobre `pipeline-kotlin` sigue dando `VERSION LOCKSTEP ERROR`
  - fix(bundle): el guard de dev-install-source era ciego a los contadores — `test_dev_install_source_guard.sh` verificaba version, rango binario, ancla y `schema_version` del `BUNDLE.toml` commiteado, y **ningun** contador. Por eso `skills_count = 244` con 245 entradas reales, y la ausencia total de `impeccable_reference_count`, Bufaron dos revisiones sin que nadie lo notara: el fichero puede ser coherente en su cabecera y mentir en su cuerpo. Es la misma clase que el `prompts_count = 0` de INC-DEBT-052, y el defecto ya existia antes de que esta sesion lo encontrara **por casualidad** al regenerar el fichero. Anadido el caso 5, con la tabla de superficies escrita a mano (no derivada: los nombres de campo no corresponden a las rutas, `prompts/sddk` -> `prompts_count`, y derivar uno del otro es justo lo que produjo el cero silencioso) y con las dos direcciones comprobadas — un contador que discrepa del manifest es un bundle que miente sobre su contenido, y una superficie sin contador es un contador que solo puede mentir. La tabla se ata ademas a `MANIFEST_SURFACES`: una superficie nueva en Rust falla el guard hasta que se escriba su linea, en vez de quedar sin verificar en silencio. **6 mutaciones falsificadoras, las 6 detectadas:** contador fosil, contador ausente, contador a cero, superficie retirada de la tabla, superficie de mas en la tabla, y superficie nueva en `MANIFEST_SURFACES` sin linea. La de contador a cero es la que mas importa: es exactamente el defecto de INC-DEBT-052, y hasta hace un momento pasaba
  - test(bundle): las dos rutas de produccion nunca se compararon, y la razon para no compararlas era falsa — la decision que session-65h dejo abierta («`release.yml` no puede probarse en local») se sostiene en que `act` v0.2.89 y podman 5.8.7 estan en la maquina: el job `framework-bundle` **se ejecuta de verdad**, con un evento `workflow_dispatch` que fija `refs/tags/v2.5.3` para satisfacer el contrato de tag-anchoring, y construye el tarball (solo falla `upload-artifact` despues, por un bug de `act` al copiar la action). Con ambas rutas ejecutables, la comparacion que faltaba es de una vez: **las dos publican el mismo bundle** — 396 miembros, byte-identicos, mismo ancla, sin debris. La divergencia de layout (envuelto contra plano) es intencionada y esta documentada en `release.yml:135-139`, y el consumidor la resuelve con `tarball_wraps_all_members_under_one_dir`. **Dos cosas que la prueba detecto y que no eran del codigo:** el primer montaje usaba el arbol de trabajo en vez de un checkout limpio, y por eso la ruta cloud «filtro» `agents/.atl/` y `assets/*.bak` — eso NO es un defecto de `release.yml`, porque `actions/checkout` no puede contener ficheros no trackeados, y `git archive` lo confirma (72/245/44/17/14/2 = 394, sin debris); un FAIL del harness que parece un defecto del producto es la forma mas cara de perder el tiempo. Y `mktemp -d` crea con modo 700 mientras el contenedor alcanza el arbol con otro mapeo de uid, asi que todo falla con `Permission denied` aunque el host muestre `drwxr-xr-x`: el scratch tiene que vivir dentro del repo. `tests/test_release_routes_parity.sh` lo deja reproducible en un comando, con 4 mutaciones falsificadoras, todas detectadas. **Con esto la decision se toma con evidencia: NO derivar `release.yml` del manifest**, porque su lista explicita esta verificada contra la ruta local y cambiarla seria una simplificacion sin evidencia a su favor, mientras el coste — una ruta que no puede probarse en el host donde corre el gate — sigue sin pagarse
  - fix(bundle): el gate del ancla de manifest solo miraba la ruta cloud — `tests/test_release_ci_manifest_anchor.sh` existía para converger los productores de `manifest_sha256` y tomaba `WF=release.yml`, con `scripts/release.sh` apareciendo **una vez, en un comentario**. El resultado es que la divergencia que el gate nombra en su propio texto era invisible para él, y existía: `release.sh` seguía escribiendo hex desnudo mientras las tres rutas cloud escriben `sha256:`. `verify_manifest_anchor` normaliza ambos formatos, luego no era un defecto de corrección sino de convergencia — pero convergencia es exactamente lo que el gate declara hacer. Extendido a los dos productores, con control negativo que exige que un productor en hex desnudo sea rechazado, y **en rojo sobre el estado real**: la primera ejecución de la versión ampliada falló porque `release.sh` divergía, que es la única forma en que un gate demuestra que está mirando algo
  - fix(bundle): el staging del bundle era una quinta copia del contrato, y produjo dos defectos medidos — al añadir la sexta superficie se añadió a la lista del `tar` y se olvidó en la del `cp -r`: la fase 5 aislada devuelve `tar: specs: No se puede efectuar stat` y `exit=2`, luego con `set -euo pipefail` el release se abortaba —ruidoso, pero **incompleto**. Y `cp -r <superficie>` copia lo que hay en disco **incluido lo que `.gitignore` excluye**: contando ficheros reales del tar contra entradas del manifest, `agents` daba 73/72 y `assets` 18/17, y los dos sobrantes eran `agents/.atl/.skill-registry.cache.json` (`.gitignore:26`) y `assets/agent-models.yaml.bak` (`.gitignore:17`) — de modo que **`manifest_sha256` en `BUNDLE.toml` no describía el propio tarball**, con dos ficheros sin digest que la instalación no puede verificar. **El guard de la entrada anterior no vio ninguno de los dos, y no por casualidad:** comparaba la lista del `tar` contra `MANIFEST_SURFACES`, así que (a) pasaba —metía `specs` en la lista— y (b) era invisible por construcción, porque un fichero que correctamente no está en ninguna lista no puede aparecer en una comparación de listas. Un guard de listas solo ve divergencia entre declaraciones; no ve que una declaración deje de ser la que se obedece, ni lo que se publica sin declarar. Las dos cosas aparecieron al **ejecutar** el staging y contar ficheros (INC-DEBT-056)
  - fix(bundle): el guard de superficies no veía el contenido que se publica — reescrito para atar la propiedad en vez de la lista: el staging se deriva del manifest, el `tar` empaqueta el árbol entero en vez de reenumerarlo, el manifest casa con `git ls-files` **en ambas direcciones**, y un **canario** untracked colocado bajo una superficie real debe **no** llegar al staging. Ese último es el único test que puede ver (b): ningún test de listas puede observar un fichero que correctamente no figura en ninguna. 12 tests, 9 mutaciones falsificadoras observadas. La falsificación encontró **dos puntos ciegos en el guard nuevo mismo** y los dos eran de la misma familia: `assertNotIn("agents") + len == 1` lo satisfacía un subconjunto (`software-development-decision-kernel/agents` es un miembro, solo que el inesperado), y `test_staged_tree_matches_the_manifest_exactly` comparaba el manifest **consigo mismo** —borrar una entrada la borraba de las dos mitades y el gate seguía verde mientras el fichero dejaba de publicarse en silencio—. Sexta vez que un falsador encuentra en sí mismo lo que la inspección no
  - fix(surfaces): las cuatro citas de basename ambiguo, investigadas una a una — tres eran mecánicas y una no. `deep-research-methodology-hub` **sí existía en un único sitio**, `skills/deep-research/sub/…`: a la cita le faltaba el segmento `sub/`, y el `skills/deep-*/` que el propio agente declaraba tampoco existía. Al arreglarlo apareció un **número que mentía**: el agente afirmaba *21 bundled skills* y hay **22** con `SKILL.md` — la misma clase que `prompts_count = 0`, un contador que no describe lo que publica. `skill-registry` citaba `docs/skill-style-guide.md`, que **no existe en este repo por diseño**; `skill-creator` y `skill-improver` declaran esa ausencia y su fallback, `skill-registry` no — y además no tenía copia propia, así que se le copió la guía, **byte-idéntica** a las otras dos (un solo digest en las tres entradas del manifest). La cuarta, `docs/impeccable-reference/`, **no es ambigua: es ausente**. Nunca existió en ninguna rama (`git log --all --diff-filter=A` vacío) y la skill `impeccable` tampoco está en este repo, porque el agente es un wrapper de una skill externa que el usuario instala en `<your-impeccable-skill-path>/`. No hay destino al que corregir, luego queda registrada y es decisión del operador: retirarla o escribirla. Línea base del guard **15 → 2**, con el motivo de cada una
  - fix(cli): verify-chain y debt gates dejaban de examinar nada y contestaban igual — `sddk ledger verify-chain` resolvía por defecto el stream `project:<id>`, un identificador que **no existe en ninguno de los 326 ledgers** de la máquina, seleccionaba cero eventos y contestaba `PASS` mientras `sddk ledger verify` sobre el mismo ledger veía 171; `debt report` y `debt gates` fabricaban un informe para un ciclo ajeno y sin hallazgos, y como un informe vacío no incumple ningún predicado los gates `debt-severity-assigned` y `debt-priority-assigned` han sido constantes, no ciegos (INC-DEBT-053)
  - fix(cli): un stream nombrado se respondía con la etiqueta del conjunto entero — `verify_streams` reconstruía la etiqueta con `resolve_streams(None, ..)`, así que `verify-chain --stream cycle:p-demo/one` contestaba `stream: all streams of p-demo`: veredicto correcto sobre una salida que nombraba otra cosa, y el doc-comment del struct prometía lo contrario. La etiqueta viaja ahora desde quien la decide hasta quien la publica (addendum de INC-DEBT-053)
  - fix(cli): `doctor --strict` pasaba sin medir nada, y ningún gate lo ejecutaba — los checks de `surface.briefness` se anclaban a `current_dir()` y cada enumeración iba dentro de `if let Ok(read_dir(..))`, así que desde un directorio sin superficies emitía **0 checks** y salía con `all_present: true` y exit 0; medido sobre el binario publicado v2.5.2. Ningún workflow de CI lo ejecutaba (`grep -rn -- '--strict' .github/` → 0 coincidencias), luego los criterios de ADR-016 sólo corrían desde dos tests que montaban una raíz con superficies y nunca alcanzaban la ruta vacua. Ahora se mide el árbol que contiene las superficies —el cwd, o el framework root activo, que las lleva en layout plano— y sin ellas se falla cerrado nombrando que el presupuesto es *inverificable* (INC-DEBT-054)
  - fix(surfaces): cua-test-orchestrator hacia el trabajo ella misma en vez de delegar — la skill orquestaba tres subagentes que **nunca se escribieron** (`cua-test-scenarist`, `cua-test-runner`, `cua-test-judge`, más un `agents/cua-test-orchestrator.body.md` inexistente): `git log --all` da cero commits para cualquiera de los cuatro. No era una referencia muerta, era una **orden de cargar** ficheros ausentes, y `ui-audit-protocol` dependía del `JudgeVerdictEnvelope` de un judge que no existe. Y nada de eso necesitaba un segundo agente: el único modelo distinto es Fara, y Fara es un endpoint HTTP, no un despacho, así que los tres papeles pasan a ser pasos del mismo agente. Se conservan las partes que no eran decoración —sin automatización de navegador, Fara solo por HTTP con `temperature: 0` y `max_tokens: 200`, solo assets estáticos, envelopes y nombres intactos porque `ui-audit-protocol` los reutiliza— y se documenta **por qué** `max_tokens` es 200: es la configuración contra la que se calibró la rúbrica, y subirlo es un experimento distinto y no validado. Regla nueva, que es la que este diseño existe para hacer posible: una respuesta vacía o truncada de Fara es `unresolved`, **nunca `pass`** — un criterio que nadie evaluó no está satisfecho, y sin esa regla el fallo por defecto de un modelo pequeño que devuelve nada es reportarlo como aprobado
  - fix(surfaces): las specs E14 existen, estaban en el vault; al repo y a las citas — las 6 citas de `specs/E14-uat-guided-pipeline/...` no abrían desde el repo, y la conclusión obvia —«hay que escribirlas o quitar la promesa»— era **falsa**: las cinco specs existen en el knowledge vault, nunca estuvieron en el repo, y una de las seis citas ya decía «full spec in knowledge vault». Cita no dangling en sustancia, sino una ruta que no resuelve desde ningún sitio alcanzable. Copiado el **directorio completo** (14 ficheros, 1624 líneas) y no solo los tres citados: 5 de 13 habría dejado un conjunto parcial, que es peor que ninguno porque parecería autoritativo. `diff -rq` contra el vault: 0 diferencias. Línea base del guard de referencias **15 → 4**, y control negativo re-ejecutado. **Límite que esto NO resuelve:** `MANIFEST_SURFACES` es `["agents","skills","prompts/sddk","assets"]`, luego `specs/` no viaja en el bundle — las citas resuelven para quien clona el repo, no para quien instala el artefacto. Cerrar eso es un cambio de contrato de distribución
  - fix(uat): el veredicto de sesion salia READY sin haber ejecutado nada — la regla del veredicto estaba escrita **tres veces**, y las dos copias sin plan contra el que cruzar contaban `Fail`/`Blocked`/`NotRun` sobre `results`: con la lista vacía salían tres ceros y caían en el `else` → `READY`. El guard de integridad que ya rechazaba una sesión `executor: human` fabricada es `if executor == Human`, luego una sesión `executor: fara` con `results: []` se aceptaba, se persistía como lista en el control plane, y `total = results.len().max(1)` enmascaraba el vacío en el denominador de cobertura. Peor que los otros dos del género: `verify-chain` y `doctor --strict` contestaban `PASS`/exit 0 —integridad—, y este contesta `READY`, que es afirmación de aptitud para publicar. RED medido `left: "READY" / right: "NOT_READY"`. Ahora hay una sola autoridad, `UatVerdict::from_counts` / `from_results`, y las tres copias delegan: `from_counts` es **la regla que `aggregate_report` ya aplicaba** —la única que contaba `Partial`—, sin cambio de comportamiento donde ya se usaba, y `from_results` añade una sola cosa, `results` vacío → `NOT_READY`. La clase de `Partial` **no se decide aquí**: ADR-012 §6 no la menciona, `aggregate_report` la trata como riesgo y las otras dos ni la contaban, así que se adopta la autoridad previa y se registra el hueco de contrato (INC-DEBT-055)

### Documentation
  - docs(adr): ADR-0153 a `accepted`, con los siete criterios medidos uno a uno y uno de ellos reescrito — la aceptación no se declara por suma, así que cada criterio se ejecutó por separado: `bash tests/test_adr_0153_criteria.sh` reporta el veredicto de cada uno y **exige que pasen todos** los tests de un criterio que tiene varios, para que un verde agregado no pueda tapar uno rojo. Resultado **PASS=7 FAIL=0**. El **criterio 1 estaba redactado de una forma que ninguna implementación correcta podía cumplir**: decía «`Cargo.toml` no aparece en `version.rs`», y los tests de paridad de Rust tienen que *construir* un `Cargo.toml` para comprobar que el lockstep no ha cambiado. Medido: trece apariciones, **cero** en código de producción —seis fixtures, dos asserts sobre el mensaje y cinco comentarios que cuentan la historia—. Se reescribe a la propiedad que sí tiene dientes, «el código que resuelve no nombra ningún manifiesto», y se hace cumplir con un test **estructural** que recorta el `REGISTRY` —que sí debe nombrarlos, porque es donde vive el dato— y que además está **falsificado**: inyectar un `root.join("Cargo.toml")` en el lector lo hace fallar. Con esto **INC-DEBT-051 queda resuelta** y sus cuatro falsificadores F56–F59 están medidos contra el binario. Reconciliación de redacción, escrita y no omitida: F56 y F59 hablan de «adapter» y el contrato elegido **no tiene adapters** —es un registro de ecosistemas, y añadir uno es solo datos—, así que cambia el sustantivo y no la exigencia de que la comprobación sea auditable
  - docs(adr): ADR-0150 da por escrito el presupuesto de brevedad que el código aplicaba sin contrato — el «ADR-016 surface-brevity» que citaban `doctor.rs` y el CHANGELOG **nunca fue commiteado**; los tres umbrales (300/150/200) estaban en una constante y en una línea de changelog, sin derivación. Ratificados con su medición (78 skills en presupuesto, media 758 tokens; techo a 1,26× la mediana) y con **vía de waiver** en vez del «sin excepciones nominales» que afirmaba el changelog: un gate sin salida se salta. Añade el contrato de corte —qué se queda en la superficie de invocación y qué va a `references/`— y el caso de las superficies que son especificaciones (ADR-0150)
  - docs(adr): enmienda a ADR-0150, el remedio es proporcional al exceso — ejecutar los puntos 1-7 sobre el material más barato del repo (`uat-discovery`, 14 líneas por encima) encontró que el contrato solo ofrecía dos salidas, partir o waiver, y ninguna sirve para un exceso pequeño. Medido: quitar solo duplicación genuina recupera 4 de las 14. Se añade la escalera de tres peldaños —adelgazar, partir, waiver—; el tercero es el que hace coherente el segundo (ADR-0150)
  - docs(debt): INC-DEBT-051 verificada vigente, y el PRE-FLIGHT que no se emitió — los cuatro criterios se sostienen, incluido que **no existe seam de adapter** para la versión de un proyecto no-Rust. Se corrige el registro: hay un tercer call site (`release_cmd.rs:947`) que la entrada no nombraba y que **no** es un tercer defecto, porque `sddk-gateway/src/release.rs:205` rechaza con `ReleaseError::Precondition`. Los tres fallan cerrado

### Tests
  - test(cli): R1-R5, ledger watch debe declarar cuantos eventos existian — cinco RED antes de tocar nada, y los cinco caian por la razon correcta. R1 el texto declara el total **aunque no quede nada fuera**, porque una declaracion que solo aparece al truncar no se puede leer de un log donde no ocurrio; R2 el JSON lleva `total_events` y `pending` y conserva `__watch_complete`; R3 `emitted + pending == total_events`; R4 con `--cycle` el total es el de **ese** ciclo; R5 con `--from-sequence` el total cuenta solo lo posterior al cursor. **R4 y R5 son los que sostienen el ciclo**: R1 y R2 se satisfacen declarando *cualquier* numero, y son el unico guard contra la solucion que se descarto, que es exactamente `COUNT(*)` sin filtro. El total de referencia **no esta escrito a mano**: se lee de `ledger events`, que ya declara, asi que los tests comparan dos comandos entre si y no contra un literal que se pudriria con el fixture. R5 se escribio **dos veces** —la primera media tres eventos con `sequence: 1`, porque las secuencias son por stream y cada `cycle start` abre su frame, y no habia cursor que partiera el ledger en dos: murio por su fixture, no por el defecto, y un RED que mide su fixture no mide nada—
  - test(cli): R6, el filtro de watch acota lo que se emite, no solo lo que se cuenta — **caracterizacion, no RED, y se declara como tal** porque llamarlo RED seria falso: el filtro del bucle ya funciona. R4 mira el **numero** que produce el filtro y R6 mira los **eventos**. Una mutacion que quita `apply_watch_filters` del bucle y lo deja en el recuento emite los eventos de todos los ciclos declarando el total de uno: R1 a R5 siguen verdes, la aritmetica cierra y el total es correcto, y solo miente el flujo emitido. Un guard con una sola direccion no es un guard
  - test(cli): R6 con fixture que puede discriminar, tras fallar M5 del falsificador — el falsificador **vetó** su propio guard: quitar el filtro del bucle mientras el recuento lo mantiene emitía los eventos de todos los ciclos declarando el total de uno, y los seis tests seguían verdes. El defecto estaba en el **guard**, no en el producto. La causa: el fixture elegía el ciclo con **menos** eventos, y en un fixture de ciclos de un solo evento eso es el **primero** del ledger, luego una corrida sin filtrar emitía justo ese y parecía correcta. R6 ahora apunta al **último** ciclo escrito y sube el tope por encima de su propia cuenta, con lo que la diferencia aparece como líneas de más y no como una primera línea errónea. Con la corrección, el falsificador detecta **5 de 5** mutaciones
  - test(vault): la proyeccion de nodos de `vault export` medida, tres RED, y un guard que dos veces no media lo que decia — cierra la pregunta por **criterio** que los ciclos anteriores no se hacian: *¿qué más declara el mismo hecho, y cada uno lo declara igual?*. De **5 superficies candidatas, 1 es defecto, 2 ya estaban cerradas y 2 se descartan**, y la reducción va escrita porque «5 candidatas» es el número que viaja a un documento y se convierte en trabajo que nadie necesitaba. Los dos descartes son por **homonimia**: `sddk-domain` tiene **otro** `GraphView`, y `ActiveGraphView` parece uno más — leídos, uno es una vista **prestada y filtrada** sin `Serialize` y el otro envuelve una proyección canónica y **falla sin ella**, luego ninguno declara datos propios y no son de esta clase. Es la sexta vez que el número de candidatos se reduce al leerlos. **R2 es el que lleva el peso, y no por estilo: R1, R3 y R4 los cumple quien escriba un párrafo en el HTML, y R2 pregunta si el acoplamiento puede hacerse fallar.** Su respuesta medida es que **no puede en compilación**, y su segunda versión tenía además un defecto propio —usaba un literal de `VaultNode`, con lo que la mutación rompía la compilación del propio test antes de que ninguna aserción corriera—, y ambas correcciones están escritas **en el propio test**, que es donde se van a volver a leer. **R4 es de caracterización y pasa hoy**: declara que `body` no viaja, y es el guard contra el arreglo obvio y erróneo. **El árbol quedó ROJO a propósito** entre ese commit y el siguiente, y los tres RED caen **por aserción y no por un fixture roto**
  - test(vault): la replica HTML de `vault export` medida, tres RED, antes de tocar nada — recoge el riesgo 3 del recibo de `cl-vault-graph`, que decía que la réplica HTML no se contradice con el grafo pero que **no se había verificado** si era intencional. Los tres RED caen **por aserción y no por un fixture roto**: el vault es un directorio real de markdown con frontmatter, parseado con `parse_vault`, y la aserción ve la ausencia que describe. El texto del fallo deja la comparación escrita, que es la evidencia: el HTML incrustaba `{"cyclic":true,"sample_cycle":[…],"topological_order":null}` y `vault graph` emitía `{"cyclic":true,"edge_count":5,"multiple_cycles":true,"node_count":5,"sample_cycle":[…],"topological_order_absent_because":"cyclic"}`. **R3 es el que lleva el peso y no es decorativo**: R1 y R2 son comprobaciones de **presencia**, y una comprobación de presencia pasa en cuanto el campo existe, pase lo que pase el valor; R3 compara **valores** campo a campo, y es lo que convierte el arreglo en una forma que las dos superficies no pueden volver a dejar. Por eso el fixture tiene **dos ciclos disjuntos** y no uno: con exactamente uno, la forma saturada y la de «un ciclo» coinciden, y pasaría contra un `GraphExport` que no declara nada. El test extrae el JSON **escrito a mano** en vez de usar un helper, porque lo que se comprueba es si un **consumidor** puede recuperar esos campos del artefacto publicado, y un helper que conociera la forma de la estructura no probaría nada del fichero. Se añade `serde_json` a `dev-dependencies` de `sddk-vault` por eso. **El árbol quedó ROJO a propósito** entre este commit y el siguiente
  - test(cli): `vault graph` medido, tres RED y una caracterizacion, antes de tocar nada — cierra el «NO MEDIDO» que dos slices anteriores dejaron abierto, y el hallazgo es que **medir tambien descarta**: `vault show` queda **fuera de la clase** (su `backlinks` no tiene cota, es el array completo) mientras que `vault graph` es un **defecto real** que solo aparece en el caso que la funcion no promete — con dos ciclos disjuntos, `sample_cycle` devuelve **uno** sin declarar cuantos hay y el `topological_order` desaparece sin decir por que—. Los tres RED de `crates/sddk-cli/tests/vault_graph_declaration.rs` caen **por asercion y no por un fixture roto**: el vault es un directorio real de markdown con frontmatter, indexado con `vault index`, y R1 y R2 ven la ausencia que describen. **El cuarto test es de caracterizacion y NO es RED**, y es el que mas valor tiene de los cuatro: `r4_acyclic_output_is_unchanged` afirma que el caso aciclico **no cambia**, luego **pasa hoy** y seguira pasando despues —llamarlo RED seria falso, y es el cuarto guard mal escrito de la serie que se declara como el guard que es. Esa caracterizacion es la que hace que O3 sea falsable: sin ella, un arreglo que declarase recuento de ciclos tambien en el grafo aciclico pasaria inadvertido. Y la medicion que mas condiciona el diseno no la hizo ningun test: **cuenta de ciclos ingenua sobre tres formas de grafo** (cadena de 50/200/1000, bouquet de 50/200/1000), que demuestra que el recuento exacto no es lento sino **incorrecto** —la cadena de 1000 tiene **un** ciclo y cuenta 1000 rotaciones (**558 ms**), y el bouquet de 500 cuenta 1000 porque cada ciclo se recorre en dos direcciones—, luego el SCOPE prohibe expresamente prometerlo (STOP 1). **El arbol quedo ROJO a proposito** entre este commit y el siguiente
  - test(cli): `vault search` medido, cinco RED y un guard, antes de tocar nada — abre el ciclo `p-63676b11dc0ef88f/vault-declaration`, con SCOPE-CONTRACT y PRE-FLIGHT (`Readiness: READY`) en `docs/roadmap/receipts/cl-vault-declaration/` y **el coste del `COUNT` medido antes de decidir el diseno**. **El arbol quedo ROJO a proposito** entre este commit y el siguiente, y el recuento exacto es **5 RED y 1 verde**, no seis rojos: `never_returns_empty_output_for_a_search_that_ran` **pasa hoy** —comprueba que la salida no este vacia, y hoy imprime `no hits` o lineas de hit— y llamarlo RED seria falso, asi que se declara como el guard que es, contra que el arreglo introduzca silencio. Los cinco RED caen **por asercion y no por un fixture roto**: el vault es un directorio real de markdown con frontmatter, indexado con `vault index`, y las aserciones ven la ausencia que describen. **`--limit 0` es la cuarta vez que el binario contradice su propia convencion**, y aqui la causa es concreta: `LIMIT 0` en SQL no es un caso especial, luego devuelve cero filas, mientras que `ledger export`, `ledger events` y `ledger watch` ya lo tratan como «todos». Y lo que este test **no** afirma: que `vault graph` y `vault show` esten bien. Tambien proyectan datos, no se han medido, y el SCOPE lo dice como «no medido» y no como «correcto» — esa distincion es la que falto con `ledger watch`
  - test(cli): F63 medido, cuatro RED antes de tocar `ledger events` — abre el ciclo `p-63676b11dc0ef88f/ledger-declaration`, el remedio del falsificador F63 de INC-DEBT-060, con SCOPE-CONTRACT y PRE-FLIGHT (`Readiness: READY`) en `docs/roadmap/receipts/cl-ledger-declaration/` y la medicion **rehecha en esta sesion y no heredada**. **El arbol quedo ROJO a proposito** entre este commit y el siguiente: los cuatro tests de `crates/sddk-cli/tests/ledger_events_declaration.rs` caen **por asercion, no por un fixture roto** —el andamiaje `adopt apply` + `cycle start` produce eventos de verdad y los cuatro ven la ausencia que describen—, asi que no hay que arreglar nada al heredarlo. **R4 es el que mas duele y no estaba en el enunciado de F63:** `--limit 0` imprimia **cero** eventos, mientras `ledger export --limit 0` imprime todos (`ledger.rs:442-445`) — convencion invertida dentro del mismo binario, con lista vacia y exit 0, y hoy el test ve exactamente `[]`. **R3 no es un campo que falte:** el JSON era un array desnudo, y un array no tiene donde llevar un total, luego el arreglo cambia la forma en vez de anadir una clave. Cuarto guard mal escrito de la serie, declarado otra vez: la primera version del falsificador de F63 imprimio **0 eventos** contra un comando que imprime 50, porque asumio una forma `sequence: N` en vez de la real —una linea por evento—
  - test(cycle): lote 1 del remedio de INC-DEBT-060, con 3 RED y 3 de caracterizacion — **El arbol queda ROJO a proposito y esta release no es publicable como esta.** Al mapear la superficie aparecio un segundo defecto, mas grave que el que abria la incidencia y que no estaba escrito: **81 de los 179 ciclos de `p-63676b11dc0ef88f` tienen un `manifest_json` que no deserializa en `CycleManifest`**, luego `get_cycle` les devuelve **error**, no registro. No es que no se puedan nombrar: es que **no se pueden leer**. `CycleManifest` exige once campos sin `serde(default)`, asi que un `{}` falla con *missing field `schema_version`* y `json_from_sql_error` lo propaga. Tres tests de caracterizacion lo fijan por escrito, con precondicion de que un manifiesto completo —construido con `CycleManifest::new`, no con JSON a mano— **se lee bien** por la misma llamada: sin ella, un FAIL probaria que `get_cycle` esta roto para todos los ciclos, que es otro defecto. Fijarlo antes de que la enumeracion lo esquive es lo que convierte un cambio posterior en decision y no en deriva (STOP 1). Los tres RED de `cycle_list_e2e.rs` caen con `unrecognized subcommand 'list'` y el andamiaje de `adopt apply` + `cycle start` funcionando: caer por el fixture habria sido un FAIL que no media nada. **Son de CLI y no de storage a proposito**: un test que llama a `list_cycles` no compila, y un RED comprado rompiendo el build tumba el crate entero; R3 queda para el lote 2 por ese motivo. Uno de los RED exige que dos ciclos coexistentes sean **nombrados los dos** sin convertirse en el error de ambiguedad de la resolucion por lease — que es exactamente la diferencia por la que 91 ciclos `OPEN` son invisibles hoy— y otro exige que el recuento se **declare** y cuadre, porque el modo de fallo que este ciclo existe para evitar es el de `ledger events` devolviendo 50 de 590 sin decirlo. Ademas se corrige un error del propio SCOPE: R4 estaba etiquetado como RED, y un test que afirma el comportamiento actual **pasa**
  - test(identity): los tests que cruzan la costura adopt <-> store de alias, en ROJO — `adoption_contract.rs` y `project_pin_e2e.rs` tienen **cero** ocurrencias de «alias», medido: los tests vivían a ambos lados de la costura y **ninguno la cruzaba**. Es la misma forma que la mutación `resolve_bypasses_the_wiring` del lote 3 de ADR-0152 — escapó por idéntica razón, y encontrarla costó partir `run_project_resolve` para que un test *pudiera* cruzarla; aquí la costura existe y nadie la cruzó nunca. El fichero lleva su propio doc explicando por qué existe, porque el del vecino lo declara de otra cosa. **Lote 1: tests y nada más**, escritos para caer *antes* de tocar producción, que es lo que permite que el lote siguiente sea «hacerlos verdes» en vez de «comprobar a posteriori si algo se movió». Los cuatro caían, pero **tres por el motivo equivocado**: `--scope` es obligatorio en `adopt apply` y el helper exigía éxito, luego el fallo era del andamiaje y el mensaje de la propiedad no se imprimía; y en la segunda pasada `adopt status` sale 1 y `context bootstrap` sale 4, así que exigir éxito las hacía medir el **código de salida** en vez de lo que reportan. De ahí el helper `run_reporting`, que devuelve stdout sea cual sea el status: la propiedad es lo que el comando **reporta**, y el código de salida se sigue de ahí. Es la regla de session-67b aplicada a un arnés propio — un FAIL que no midió nada es peor que un SKIP, porque además tapa el defecto con un mensaje que no lo describe. Los tests borran `SDDK_DATA_DIR` explícitamente: `CliSandbox` hereda el resto del entorno, y un `SDDK_DATA_DIR` exportado en la shell de quien lanza los tests mandaría toda escritura al data dir **real**; la hermeticidad del sandbox se afirma, no se supone
  - test(release): el falsificador del ancla tenía una sexta copia de las superficies — el falsificador end-to-end del ancla de manifest —la prueba que demuestra que `sddk dev install` acepta un bundle con ancla correcta y rechaza uno con ancla erróna **por esa causa**— stageaba solo las cuatro superficies originales y luego pegaba el `MANIFEST.sha256` completo encima. Al incorporarse `specs` y `docs/impeccable-reference` como superficies, ambos bundles quedaron con 16 ficheros listados ausentes y `dev install` los rechazó a los dos. Eso es grave porque el falsificador **había dejado de discriminar**: el bundle con ancla correcta y el de ancla erróna se rechazaban *idénticos*, por un error incidental, de modo que el check negativo pasaba «por la razón equivocada» y el sentido entero del fichero se perdía sin que su veredicto lo dijera. Lo detectó la tercera aserción —*«el rechazo nombra el ancla del manifest, no un error incidental»*—, que existe exactamente para eso. Arreglado derivando el staging del `MANIFEST.sha256`, igual que hizo `release.sh` en INC-DEBT-056, más una copia explícita del manifest (que no puede listarse a sí mismo, y sin el cual `dev install` no tiene contra qué verificar). **Resultado: PASS 3/3** con las tres comprobaciones discriminando de verdad: A instala con exit 0, B es rechazado nombrando `manifest anchor check` y el digest concreto que esperaba. Tercera copia de la lista de superficies encontrada y corregida en esta sesión, tras el guard del manifest (quinta) y la del propio `release.sh`
  - test(cli): ETXTBSY al ejecutar el binario recien instalado hacia fallar la suite la mitad de las veces — `cli_dev_install_default_layout_is_executable_and_verify_passes` fallaba en el perfil de workspace y no por culpa del producto: `execve` devolvía `ExecutableFileBusy` (errno 26) al lanzar el binario que `dev install` acababa de escribir. Medido dos veces por separado — fallo en la run 2 de 3 en ambas, resto en verde — un flake del 50% en un gate que `release.sh` ejecuta en su paso 1, o sea que **v2.5.3 podía negarse a publicarse la mitad de las veces**. **La producción ya declara esa condición transitoria y el test no:** `dev::common::atomic_write` nombra el código (`const ETXTBSY: i32 = 26`) y reintenta el `rename` hasta 100 veces porque el destino puede estar ocupado; la aserción pedía el mismo fichero por la misma puerta sin compartir ese contrato. Descartado antes que nada: **colisión de rutas** — `CliFixture::new` usa un `tempdir()` único y el test ya toma `dev_install_serial_lock()`, luego los installs de dev están serializados dentro del proceso; es una carrera del kernel, no dos tests peleándose por un path. **El reintento está acotado y verificado, no es una tolerancia:** solo reintenta cuando `raw_os_error() == Some(26)` y cualquier otro errno vuelve de inmediato; comprobado empíricamente en este sistema de ficheros que ejecutar un binario sin `+x` devuelve errno **13 (EACCES)** y no 26, luego un binario de verdad no ejecutable no puede entrar por ese brazo; la aserción de modo `0o755` corre **antes** del exec, así que el permiso equivocado se detecta antes y más claro; y el bucle está limitado a 20 intentos con espera creciente, de modo que una condición permanente agota el bucle y falla igual. Tolera una carrera, no una instalación rota. **6/6 runs en verde** después del cambio
  - test(domain): el guard del manifest tenia una quinta copia de las superficies — `manifest_contains_only_tracked_files` fallaba en el perfil de workspace con *«MANIFEST.sha256 contains 16 untracked files»*, y **ninguno de los 16 lo era**: `git ls-files specs/` devuelve 14 y `git ls-files docs/impeccable-reference/` devuelve 2, casando exactamente con el manifest, con `git status` limpio. La causa era el propio test: filtraba `git ls-files` por un pathspec con las cuatro superficies originales escritas a mano (`-- prompts/sddk skills agents assets`) mientras la autoridad `MANIFEST_SURFACES` (`crates/sddk-cli/src/dev/common.rs:23`) declara **seis** desde `2bc0511c`. Al filtrar por cuatro, todo lo demás salía como «no trackeado» por definición. **El commit que añadió las superficies se llamaba «con guard de las cuatro copias»** y `test_bundle_surface_coverage.py` las fijaba: esta era una quinta que nadie vio — la clase exacta que este repo lleva sesiones persiguiendo, una lista escrita a mano que nadie actualiza cuando cambia la lista que la define. **El arreglo no es añadir las dos superficies al pathspec**: eso dejaría la copia en pie y reintroduciría el defecto en la siguiente superficie. La propiedad del test es «toda entrada del manifest está trackeada por git», y filtrar era una optimización que se convierte en copia, así que se pide a git directamente con `--full-name` y sin pathspec — aquí no queda lista que mantener. La dirección inversa (trackeado pero ausente del manifest) es otra propiedad y la cubre `tests/test_bundle_surface_coverage.py`. **Falsificado**: añadida a mano la línea de un fichero no trackeado (`agents/.atl/.skill-registry.cache.json`), el test lo nombra y falla — un guard que daba falsos positivos no puede pasar a dar pases vacuos por habérsele quitado el pathspec, y esto comprueba que no
  - test(engine): auditar INC-DEBT-048 y refutar una de sus dos afirmaciones — la entrada de deuda sostenía dos cosas; una se reproduce y la otra no. Vigentes, verificadas contra el árbol y el ledger reales: la spec sigue en `status: proposed`; REQ-A3S1-021 sigue fijando la derivación «from the sorted `(id, inner_basis_hash)` pairs» mientras el código deriva con `derive_basis_hash_at(&assertions, Some(revised_at))`; `AT-UAT-019` sigue remitiendo a un ADR de identidad inexistente; e impacto en datos **cero**, medido (`basis_hash` e `IntelligenceLoopReceipt` no aparecen en `sddk-storage`, y el ledger vivo no tiene ninguna tabla relacionada). **No reproducible:** el documento afirma que `KMT::evaluate` devuelve `Fresh` sin mirar `revised_at` en una revisión puramente temporal. Es *condicional* a la derivación que REQ-A3S1-021 describe; bajo la derivación en vigor (v2, que mezcla `revised_at` en el digest) el hash difiere, la primera comparación falla y se alcanza la rama de timestamps — observado: `Unknown { FutureEvidence }`. **Hay una contradicción, no dos**, y la que queda es normativa. **Radio de impacto corregido:** el criterio de cierre (a) trata cambiar REQ-A3S1-021 como acto local sobre una spec `proposed`, y no lo es — `KnowledgeBasis::basis_hash()` es entrada de la derivación de `IntelligenceLoopReceiptId`, que fija **ADR-0126, `accepted` desde 2026-09-17**. El impacto en datos sigue siendo cero; el de gobernanza no
  - test(gates): ningun test puede existir sin runner o sin motivo escrito — el hallazgo de que un guard llevaba rojo sin ejecutarse no era un defecto de ese guard: era que **la superficie de gates es una lista escrita a mano** y un test nuevo no entra en ella solo. Barrido: **13 de 36 tests sin runner**, no los 7 que dio la primera medición. La diferencia es la lección — un test nombrado en un **comentario** no está gated, y la nota de exclusión de `release.sh` nombra dos tests precisamente porque NO se ejecutan, así que un `in` a pelo los puntuaba como cubiertos. Contar prosa como cobertura es el mismo error que contar una declaración como obediencia. **11 cableados** por ser herméticos (medidos: 36–299 ms), 6 shell y 5 python. **6 fuera, con motivo escrito**, y el caso que decide: `test_h05_isolation.sh` **pasa sin medir** — sin el rlib release imprime `skip:` y aun así reporta `PASS=1 FAIL=0`, luego cablearlo habría devuelto un verde vacío, la misma forma que INC-DEBT-054; un PASS que no midió nada es peor que un gate ausente, porque además tapa el defecto. El guard nuevo se protege con dos reglas que impiden que la lista de excepciones se pudre (una excepción a un fichero inexistente es FAIL; una excepción a un test que ya tiene runner es FAIL) y su propia contabilidad se corrigió al falsarlo: reportaba «excepcionados: 0» con seis excepciones vivas porque la derivaba por resta — un guard que miente sobre sus propias cifras no puede usarse para justificar por qué el resto pasa. 3 mutaciones, 3 detectadas
  - test(uat): validar que las citas de la matriz UAT resuelvan a documentos reales — `docs/roadmap/UAT-MATRIX.md` es la tabla que un agente lee para saber qué está aceptado, y muchos exit criteria remiten a una autoridad, pero **ningún runner validaba que esas referencias resolvieran**. El caso que lo-rota: `AT-UAT-019` dice «Comportamiento coincide con el ADR de identidad» y ese ADR no existe, luego el criterio no puede pasar ni fallar y la fila quedó en PASS PARCIAL de forma permanente. Tres propiedades, todas verificables: toda autoridad citada por ID (`ADR-NNNN`, `REQ-…`, `INC-DEBT-NNN`) resuelve; cada fila tiene tantas celdas como la cabecera de **su** tabla; los IDs de fila son únicos. **Lo que NO comprueba, y conviene decirlo:** una autoridad citada solo en prosa no tiene ID que resolver, y un guard que fingiera cubrirla estaría midiendo algo que no mide — la prosa se reporta como **aviso**, no como veredicto, y hoy emite exactamente uno: `AT-UAT-019`. **Instrumentación corregida antes que el guard:** la primera medicion reporto «28 filas con 5 celdas y 44 con 4» y de ahí casi se declara un defecto de columnas que **no existe** — la matriz tiene dos tablas con cabeceras distintas y el script las mezclaba por índice fijo; la propiedad 2 existe para que ningún parser repita ese error. 3 mutaciones, 3 detectadas, y una **tuvo que repetirse**: la primera no llegó a aplicarse (la columna es `C0 / T1`, no `C1 / T1`) y el guard dio PASS sobre un fichero intacto
  - test(surfaces): 25 referencias rotas que ninguna comprobación detectaba — una cita es prosa, no un enlace: nada la resuelve. Barrido de `agents`, `skills` y `prompts/sddk`: 394 referencias comprobadas, 25 pares que no abren, en 13 superficies y cinco familias. `git log --all` da **cero commits** para los ficheros ausentes: nunca se escribieron, no se perdieron. El guard congela la línea base, así que falla si alguien añade una cita rota nueva **y falla igual si arregla una sin actualizar la línea base** (INC-DEBT-054)
  - fix(surfaces): studio-orchestrator citaba prompts/studio-agents/, que no existe — seis rutas corregidas; los agentes están en agents/studio-*.md, verificados uno a uno contra el fichero real antes de tocar nada. La línea base del guard baja de 25 a 19 referencias rotas
  - fix(surfaces): nueve citas mas apuntaban a un destino que existe en un unico sitio — `test-pyramid-builder` (2 assets de `test-pyramid`) y `cua-test-orchestrator` (`ui-audit-protocol`), verificadas con `find` antes de sustituir. **Solo se corrigen las que son mecánicas**: las tres cuya coincidencia de nombre era ambigua quedan sin tocar, porque «apuntar a lo más parecido» no es arreglar una cita. La línea base del guard baja de 19 a 16
  - test(surfaces): la linea base era 15, no 16 — un falso positivo mio — el recuento de referencias rotas arrastraba un par que no abria por una razón distinta de la que se registraba: `references/rust-testing.md` **sí** existe, relativo a `skills/test-pyramid/`, y se buscaba desde la raíz. El guard detectó que la línea base declaraba un par más de los que realmente no abrían, o sea que el número que este commit congela era del autor y no del árbol; una línea base con falsos positivos es peor que una línea base pequeña, porque hay que auditar cada entrada para saber si es deuda o ruido. Corregido a 15, verificado por recuento independiente

## [2.5.2] - 2026-10-01

### Features
  - feat(scripts): herramienta fail-closed de migracion de project_id — `audit`/`plan`/`backup`/`apply`, con `apply` exigiendo digest del plan y backup verificado; su espejo del normalizador no coincidía con el Rust en 6 formas (entre ellas `git@host:owner/repo`, la más común de Git) y habría escrito ids equivocados en ledgers reales. `tests/test_migrate_project_identity_mirror.py` lo fija con 10 tests, falsificado él mismo (INC-DEBT-050)
  - feat(architecture): el gate de conformidad distingue deuda abierta de conformidad — veredicto tipado `Conformant`/`OpenDebt`/`Waived`/`NotEvaluated`; exit 0 queda reservado a conformidad probada (C3l.7)
  - feat(lease): LeaseStore durable y multi-proceso; X04 cruza la frontera real (C3l.5)
  - feat(aiw): vertical real de expansion dinamica con identidad estable de trigger (C3l.3)

### Fixes
  - fix(manifest): los conteos de superficie de BUNDLE.toml describen el manifest que se publica — `prompts_count` valía 0 en todos los bundles escritos porque `MANIFEST_SURFACES` llama `prompts/sddk` a la superficie y el `match` buscaba `prompts`; ahora los conteos se leen del manifest recién escrito y una superficie sin campo aborta en vez de escribir 0 (INC-DEBT-052)
  - fix(uat): el gate de derivacion del bump devuelve un valor, no la salida de su limpieza — `rm -rf` escribía en el stdout de la función y su mensaje se concatenaba al tag, así que el gate comparaba contra una cadena contaminada (PASS=0 FAIL=7, verificado también en el baseline)
  - fix(cli): el pin de identidad pasa a gobernar las cinco vías del CLI — `sddk project pin` se escribía y no surtía efecto en `adopt status`, `cycle status` ni `config set`; el doc afirmaba "every runtime context honors it" y sólo 2 de 5 resolvers lo honraban (INC-DEBT-049, parte resuelta)
  - fix(knowledge): revise() produce una identidad nueva, como su contrato afirma — antes devolvía el mismo `basis_hash` con contenido idéntico y una revisión temporal era invisible al freshness (C3m.2, parcial — ver INC-DEBT-048)
  - fix(release): el CHANGELOG declarado tiene que describir el trabajo que se publica — nuevo gate de cobertura integrado como paso 2b del pipeline (INC-DEBT-047)
  - fix(release): el gate de cobertura se cazó a sí mismo en el commit que lo implementa
  - fix(roadmap): el puntero de estado tiene una sola clave autoritativa y parsea — `STATE.yaml` no era parseable por máquina y `development_head` estaba 9 veces duplicada (INC-DEBT-046)
  - fix(uat): la ausencia de un provider externo nunca vuelve a reportarse como PASS (C3l.4)

### Tests
  - test(domain): el camino remote de la identidad queda con golden pin — `stable_project_id` y `normalize_remote_url` fijan valores absolutos; sin eso, el commit que normalizó la casse reasignó 25 de 104 adopciones sin migración (INC-DEBT-050, remedio de fondo)
  - test(architecture): el gate se ejecuta de verdad y deja de certificar conformidad — el test que lo certificaba hacia `skip` y reportaba `ok` en 0.00s sin ejecutar el gate
  - test(x07): el segundo consumidor cruza la frontera de proceso real, no un segundo handle (C3l.6)
  - test(push): el caso fail-closed media el repo equivocado y nunca verifico nada (INC-DEBT-045)

### Other
  - docs(debt): registra INC-DEBT-052, el bundle declaraba cero prompts y publicaba 44
  - docs(roadmap): cierra C3l.7 con AT-UAT-015 honestamente en NOT PASS
  - docs(roadmap): cierra C3l.6 con X07 en la frontera de proceso y AT-UAT-013/014 PASS
  - docs(roadmap): session-59 cierra INC-DEBT-046 y reconcilia la divergencia con origin/main
  - docs(release): registra el bloqueo del toolchain musl que detiene v2.5.0
  - docs(debt): registra INC-DEBT-046, el puntero de estado no era parseable por maquina
  - docs(debt): registra INC-DEBT-045 y reindexa INC-DEBT-044
  - docs(c3l): cierre de C3l.5 con recibo, matriz X04 y punteros reconciliados
  - docs(arch): ADR-0148 y ADR-0149 para los modulos root nuevos de sddk-engine
  - chore(debt): normaliza el status de INC-DEBT-028 tras verificar sus criterios
  - docs(c3l): cierre de C3l.4 con recibo, matriz S5 y punteros reconciliados
  - chore(c3l): el recibo del EXT gate es artefacto de run, no evidencia
  - docs(c3l): cierre de C3l.3 con recibo, matriz S4 y punteros reconciliados
  - docs(c3l): reconciliar puntero de estado (session-53)
  - docs(c3l): session-53 cerrada con v2.4.2 publicada y verificada

> **Nota de estado:** esta release sigue **BLOQUEADA** — falta el toolchain
> `x86_64-linux-musl-gcc` (`scripts/release.sh` aborta en el step 3/14). La
> entrada describe el contenido declarado del artefacto, no una publicación.
> Detalle en `docs/architecture/adrs/BLOCKER-MUSL-TOOLCHAIN-MISSING.md`.

## [2.4.2] - 2026-09-30

### Fixes
  - fix(gateway): ProducerToL0Adapter evalua contra un engine inyectado — las reglas registradas disparan por la ruta publica (C3l.2)

### Other
  - docs(c3l): C3l.2 cerrada con falsificador y exit-gate en el suite; S7a re-verificable; AT-UAT-004/005 PASS
  - docs(c3l): reconciliar puntero de estado a f8d1a009 (session-52)
  - docs(c3l): session-52 cerrada con v2.4.1 publicada y verificada

## [2.4.1] - 2026-09-30

### Fixes
  - fix(engine): DebVerify reconcile respeta ChallengeError — strategy_error ⇒ summary != ConfirmedBaseline (C3l.1)

### Other
  - docs(c3l): C3l.1 cerrada con falsificadores en el suite; R6 re-verificable; AT-UAT-002/003 PASS
  - docs(c3l): C3l.0 — matriz de acceptance truthfulness congelada (AT-UAT-001 PASS)
  - docs(c3j): session-50 cerrada con v2.4.0 publicado y verificado

## [2.4.0] - 2026-09-30

### Features
  - feat(cli): sddk context expand — progressive disclosure minima de la capsule (C3j objetivo 4)

### Fixes
  - fix(engine): las capsules de ciclos reales (id con barra) se escribian en un subdirectorio inexistente y persist tragaba el fallo

### Other
  - docs(c3j): session-50 — context expand verificado, INC-DEBT-044, y adopcion de C3l/C3m/C3n
  - docs(c3i): session-49 cerrada con v2.3.3 publicado y verificado

## [2.3.3] - 2026-09-30

### Fixes
  - fix(cli): un --cycle que no existe falla cerrado en vez de enlazar la sesión a una ficción

### Other
  - docs(c3i): cerrar CTX-UAT-005 y MIG-UAT-001 con evidencia; C3i sin UAT abiertas
  - test(c3i): automatizar MIG-UAT-001 y CTX-UAT-005, cuyas premisas NOT_RUN habían caducado
  - docs(roadmap): session-48 cerrada con v2.3.2 publicado y verificado
  - docs(hook): dejar escrito el orden push -> HEAD==origin/main -> tag

## [2.3.2] - 2026-09-30

### Fixes
  - fix(uat): bootstrap exit 4 (no_capsule_source) es contrato valido, no fallo

### Other
  - ci: ejecutar los UAT de context en el job espejo (causa raiz del UAT caducado)
  - docs(roadmap): CTX-UAT-002/003 PASS con evidencia y C3i a VERIFIED
  - test(c3i): automatizar CTX-UAT-002 y CTX-UAT-003 (era 'gate humano' inexistente)
  - docs(roadmap): puntero de estado reconciliado al commit documental del cierre
  - docs(roadmap): cierre session-47 — deuda 041/038 resuelta, release v2.3.1 publicado

## [2.3.1] - 2026-09-30

### Fixes
  - fix(cli): INC-DEBT-038 — recibo honesto para dev install --source (layout flat)
  - fix(debt): INC-DEBT-041 resuelta — gate shellcheck en 0 hallazgos a severidad style

### Other
  - docs(roadmap): puntero reconciliado a a3751cc0 tras deuda 041/038
  - docs(debt): INC-DEBT-038 -> resolved (session-47, opciones 2+3)
  - docs(debt): INC-DEBT-041 -> resolved (session-47, ruta 1 con re-medición)
  - docs(roadmap): cierre C3k — release v2.3.0 publicado, INC-DEBT-040 resuelta

## [2.3.0] - 2026-09-30

### Features
  - feat(cli): project pin persistido para identidad estable del checkout
  - feat(cli): warning fail-loud cuando admission crea un ledger nuevo (D2)
  - feat(cli): linaje obligatorio en backlog discard y render --check (D3/D7)

### Fixes
  - fix(release): ruta tag-baseline en el pre-push (INC-DEBT-040 variante 3)
  - fix(engine,docs): gates del release = los declarados; receipts son artifacts (S3.1/S3.2/W4, ADR-0082)
  - fix(cli,storage): uat validate discrimina session/report; control-plane acumula sesiones (S3.3/S3.4)
  - fix(cli): uat plan valida --from contra git tags; uat status ancla en --root (D4/D5)
  - fix(domain): normalizar case del path del remote — case-change ya no forkea el ledger (D2)
  - fix(uat): sign-off fail-closed — plan con escenarios, evidencia real y actor honesto (D1)

### Other
  - docs(roadmap): comentario de current_sha sin versiones en prosa
  - docs(roadmap): reconciliar current_sha al trunk (15 commits de drift)
  - chore(bundle): regenerar BUNDLE.toml fosil 2.2.32 -> 2.2.37
  - chore(manifest): regenerar MANIFEST.sha256 tras el fix de prompts/sddk/phases/release.md
  - chore(cli): silenciar coverage local muerto tras la re-agregacion S3.4
  - docs(spec): documentar env vars de runtime en arch-spec-049 (D2/W2d)
  - docs(roadmap): quitar version de prosa del comentario del puntero (check 5 del guard)
  - docs(roadmap): cierre session-46b — release v2.2.37 publicado y plan C3k
  - docs(roadmap): hito C3k para defectos CLI 2.2.33 reportados desde agent-secretless

## [2.2.37] - 2026-09-30

### Features
  - feat(cli): context bootstrap compila la capsule del ciclo activo desde el ledger (ADR-0147)
  - feat(debt): abrir INC-DEBT-042 por el falso complete de context bootstrap

### Fixes
  - fix(cli): context bootstrap deja de reportar complete sin compilar capsule
  - fix(debt): el guard de coherencia se auto-desactivaba con su propia prosa
  - fix(debt): reabrir INC-DEBT-040, el fix de session-45 fue por ocurrencia
  - fix(debt): degradar INC-DEBT-039 de high/P1 a medium/P2 tras auditar su criterio
  - fix(debt): falsificar el diagnostico del shellcheck y abrir INC-DEBT-041

### Other
  - test(ci): el regex del anchor admite la forma cwd-relative del job standalone
  - docs(adr): status en minuscula en ADR-0147 (convencion ADR-0001 3.4)
  - docs(roadmap): reconciliar puntero de estado tras el bump a 2.2.36
  - docs(roadmap): rectificar la migracion a 2.2.35 que no hacia falta
  - docs(roadmap): doble check de los goals previos con evidencia mas fuerte
  - docs(roadmap): observar los goals de sesiones anteriores en vez de declararlos no-observados
  - docs(roadmap): cierre de session-45 con el release v2.2.33 publicado

## [2.2.36] - 2026-09-30

### Features
  - feat(cli): context bootstrap compila la capsule del ciclo activo desde el ledger (ADR-0147)
  - feat(debt): abrir INC-DEBT-042 por el falso complete de context bootstrap

### Fixes
  - fix(cli): context bootstrap deja de reportar complete sin compilar capsule
  - fix(debt): el guard de coherencia se auto-desactivaba con su propia prosa
  - fix(debt): reabrir INC-DEBT-040, el fix de session-45 fue por ocurrencia
  - fix(debt): degradar INC-DEBT-039 de high/P1 a medium/P2 tras auditar su criterio
  - fix(debt): falsificar el diagnostico del shellcheck y abrir INC-DEBT-041

### Other
  - docs(roadmap): rectificar la migracion a 2.2.35 que no hacia falta
  - docs(roadmap): doble check de los goals previos con evidencia mas fuerte
  - docs(roadmap): observar los goals de sesiones anteriores en vez de declararlos no-observados
  - docs(roadmap): cierre de session-45 con el release v2.2.33 publicado

## [2.2.35] - 2026-09-30

### Fixes
  - fix(cli): `context bootstrap` deja de reportar `complete` sin compilar capsule (INC-DEBT-042)

### Other
  - docs(roadmap): rectificar que el bump a 2.2.34 nunca llego a publicarse

## [2.2.34] - 2026-09-30

### Fixes
  - fix(debt): reabrir INC-DEBT-040, el fix de session-45 fue por ocurrencia
  - fix(debt): degradar INC-DEBT-039 de high/P1 a medium/P2 tras auditar su criterio
  - fix(debt): falsificar el diagnostico del shellcheck y abrir INC-DEBT-041

### Other
  - feat(debt): abrir INC-DEBT-042, falso success de context bootstrap
  - docs(roadmap): cierre de session-45 con el release v2.2.33 publicado

## [2.2.33] - 2026-09-30

### Features
  - feat(release): paso 9c verifica la autenticidad del release publicado
  - feat(cli): sddk context delta publica y drena el stream durable
  - feat(engine): DeltaStore durable con stream persistente entre procesos
  - feat(cli): sddk context bootstrap — operacion de contexto durable (CTX-003)
  - feat(c3j): stores durables de binding y capsule detras de los seams existentes
  - feat(c3i): alinear sddk-cycle-resume/mcw/contract con la inferencia real de ciclos

### Fixes
  - fix(cli): run-view falla cerrado con JSON valido y tests que lo comprueban
  - fix(cli): aclara el mensaje de RUN_STATE_SOURCE_UNAVAILABLE
  - fix(cli): sddk run view falla cerrado en vez de fabricar una RunStateView
  - fix(roadmap): conteo de commits sin publicar a 29 y nota sobre su autorreferencia
  - fix(roadmap): reconciliar el conteo de commits sin publicar (28, no 25)
  - fix(engine): bootstrap de adopcion repetido es no-op byte-estable (C3i obj 2)
  - fix(bundle): regenerar el BUNDLE.toml fosil y anadir guard del arbol
  - fix(engine): SDDK_STATE_HOME gobierna la ruta real del ledger
  - fix(storage): migracion 21 habilita PAUSED en bases de datos existentes
  - fix(guard): el puntero no puede afirmar versiones en prosa libre
  - fix(lint): reportar la ruta real de agents anidados y el hint que si regenera
  - fix(engine): C2B-DRIFT-1 - adapter fija CHRONOS_DB_PATH ademas de CHRONOS_STORE_PATH
  - fix(engine): adapter chronos soporta execution_query (renombre 0.1.4) con fallback legacy

### Other
  - test(debt): guard mecanico de coherencia indice<->documento de deuda
  - docs(debt): reconcilia el indice con dos documentos ya cerrados
  - docs(roadmap): close-out session-44 con recibo, journal y punteros
  - docs(debt): registra INC-DEBT-040 y corrige la descripcion del hook
  - docs(roadmap): verify the real ledger was not mutated by the session close
  - docs(roadmap): close-out session-43b con recibo, journal y punteros
  - docs(roadmap): close-out session-43 con recibo, journal y punteros
  - docs(debt): cierra INC-AUDIT-S14 y reconcilia el indice de deuda
  - test(release): guard end-to-end de autenticidad de un release real
  - docs(roadmap): close-out session-42 con recibo, journal y punteros
  - docs(debt): cierra INC-DEBT-037 y registra INC-DEBT-039
  - docs(roadmap): registrar la autorreferencia del conteo de commits
  - docs(roadmap): fijar el conteo de commits sin publicar al SHA medido
  - docs(roadmap): recibo, journal y punteros de session-41 (C3j objetivo 5)
  - docs(adr): ADR-0146 acepta durable_delta_store como modulo raiz
  - docs(roadmap): receipt y punteros de session-40 (C3j objetivo 3)
  - docs(adr): ADR-0145 acepta los modulos raiz de contexto durable
  - docs(roadmap): avanzar punteros a session-39 con C3j slice 1 verificado
  - docs(roadmap): registrar C3j slice 1 con evidencia session-39
  - test(c3j): e2e de contexto durable CTX-UAT-006/013/014 a nivel sustrato
  - docs(roadmap): avanzar punteros a session-38 con C3i completo
  - docs(roadmap): cerrar C3i objetivo 5 con evidencia session-38
  - test(c3i): automatizar CTX-UAT-001 como script reutilizable en tests/
  - test(c3i): pinear identidad unica del bootstrap estable entre reinicios y refresh (C3i obj 5)
  - docs(roadmap): cerrar session-37 con convergencia de adopcion verificada
  - test(c3i): pinear el contrato de convergencia de adopcion en las superficies
  - docs(debt): registrar INC-DEBT-038 (dev install --source instala layout plano con recibo versionado)
  - docs(roadmap): cerrar session-36 con el estado del slice C3i y sus hallazgos
  - docs(roadmap): adoptar delta hypermedia C3i/C3j/C6/C7 sin abrir segundo roadmap
  - docs(roadmap): registrar session-35, el incidente del ledger y el rojo del guard
  - docs(debt): registrar que SDDK_STATE_HOME no aísla el ledger
  - test(uat): self-test de mutacion para el guard del puntero de estado
  - docs(roadmap): puntero a 3d810c48 con la suite re-verificada
  - docs(journal): adenda session-34i con la re-verificacion de la suite y el falso cuelgue
  - docs(roadmap): CURRENT alineado con 2.2.32 y el diagnostico corregido del guard
  - docs(roadmap): puntero a eff37cee tras el guard endurecido
  - docs(journal): adenda session-34h con la correccion del diagnostico del guard
  - docs(journal): adenda session-34g con el bump forzado y la brecha del guard
  - docs(roadmap): puntero a 6f909de2 / 2.2.31 tras el push
  - docs(roadmap): punteros de sesion-34f alineados con la evidencia observada
  - docs(receipts): evidencia del perfil completo session-34 y hunt de gates rojos
  - docs(adr): ADR-0144 propuesto - boundary de integracion con el host JCode
  - docs(journal): adenda session-34e (guard en verde, 2.2.30 alineado y commiteado)
  - docs(roadmap): punteros a d47a1766 / 2.2.30 con el guard de estado en PASS
  - docs(journal): adenda session-34d (DRIFT-1 fix, leccion del bump manual, stash de release)
  - docs(roadmap): CURRENT a 2.2.29/cfa477cf con C2 completo y C3g cerrado
  - docs(roadmap): puntero a d83bc120 / 2.2.29 (sin publicar) con cadena de la sesion
  - docs(journal): adenda session-34c (C2c con SDK publico, C3g completo)
  - docs(uat): C3g addendum - presupuesto estatico y runtime con providers reales
  - docs(uat): C2c re-ejecutado con el SDK público de jcode (T15-T18, decision_request ADR)
  - docs(journal): adenda session-34b (C2a/C2b re-ejecutados, drift chronos, bump 2.2.28 sin publicar)
  - docs(uat): C2b re-ejecutado contra chronos-mcp 0.1.4 real (T12-T14 PASS con fix de adapter)
  - docs(uat): C2a re-ejecutado contra cognicode-mcp real 0.97.3 (T08-T11 PASS observados)
  - docs(roadmap): punteros a v2.2.27/5ee68265 y adenda de cierre session-34

## [2.2.32] - 2026-09-29

### Fixes
  - fix(guard): el puntero no puede afirmar versiones en prosa libre
  - fix(lint): reportar la ruta real de agents anidados y el hint que si regenera
  - fix(engine): C2B-DRIFT-1 - adapter fija CHRONOS_DB_PATH ademas de CHRONOS_STORE_PATH
  - fix(engine): adapter chronos soporta execution_query (renombre 0.1.4) con fallback legacy

### Other
  - docs(roadmap): puntero a eff37cee tras el guard endurecido
  - docs(journal): adenda session-34h con la correccion del diagnostico del guard
  - docs(journal): adenda session-34g con el bump forzado y la brecha del guard
  - docs(roadmap): puntero a 6f909de2 / 2.2.31 tras el push
  - docs(roadmap): punteros de sesion-34f alineados con la evidencia observada
  - docs(receipts): evidencia del perfil completo session-34 y hunt de gates rojos
  - docs(adr): ADR-0144 propuesto - boundary de integracion con el host JCode
  - docs(journal): adenda session-34e (guard en verde, 2.2.30 alineado y commiteado)
  - docs(roadmap): punteros a d47a1766 / 2.2.30 con el guard de estado en PASS
  - docs(journal): adenda session-34d (DRIFT-1 fix, leccion del bump manual, stash de release)
  - docs(roadmap): CURRENT a 2.2.29/cfa477cf con C2 completo y C3g cerrado
  - docs(roadmap): puntero a d83bc120 / 2.2.29 (sin publicar) con cadena de la sesion
  - docs(journal): adenda session-34c (C2c con SDK publico, C3g completo)
  - docs(uat): C3g addendum - presupuesto estatico y runtime con providers reales
  - docs(uat): C2c re-ejecutado con el SDK público de jcode (T15-T18, decision_request ADR)
  - docs(journal): adenda session-34b (C2a/C2b re-ejecutados, drift chronos, bump 2.2.28 sin publicar)
  - docs(uat): C2b re-ejecutado contra chronos-mcp 0.1.4 real (T12-T14 PASS con fix de adapter)
  - docs(uat): C2a re-ejecutado contra cognicode-mcp real 0.97.3 (T08-T11 PASS observados)
  - docs(roadmap): punteros a v2.2.27/5ee68265 y adenda de cierre session-34


### Features
  - feat(release): paso 9c verifica la autenticidad del release publicado
  - feat(cli): sddk context delta publica y drena el stream durable
  - feat(engine): DeltaStore durable con stream persistente entre procesos
  - feat(cli): sddk context bootstrap — operacion de contexto durable (CTX-003)
  - feat(c3j): stores durables de binding y capsule detras de los seams existentes
  - feat(c3i): alinear sddk-cycle-resume/mcw/contract con la inferencia real de ciclos

### Fixes
  - fix(cli): aclara el mensaje de RUN_STATE_SOURCE_UNAVAILABLE
  - fix(cli): sddk run view falla cerrado en vez de fabricar una RunStateView
  - fix(roadmap): conteo de commits sin publicar a 29 y nota sobre su autorreferencia
  - fix(roadmap): reconciliar el conteo de commits sin publicar (28, no 25)
  - fix(engine): bootstrap de adopcion repetido es no-op byte-estable (C3i obj 2)
  - fix(bundle): regenerar el BUNDLE.toml fosil y anadir guard del arbol
  - fix(engine): SDDK_STATE_HOME gobierna la ruta real del ledger
  - fix(storage): migracion 21 habilita PAUSED en bases de datos existentes
  - fix(guard): el puntero no puede afirmar versiones en prosa libre
  - fix(lint): reportar la ruta real de agents anidados y el hint que si regenera
  - fix(engine): C2B-DRIFT-1 - adapter fija CHRONOS_DB_PATH ademas de CHRONOS_STORE_PATH
  - fix(engine): adapter chronos soporta execution_query (renombre 0.1.4) con fallback legacy

### Other
  - docs(roadmap): close-out session-43 con recibo, journal y punteros
  - docs(debt): cierra INC-AUDIT-S14 y reconcilia el indice de deuda
  - test(release): guard end-to-end de autenticidad de un release real
  - docs(roadmap): close-out session-42 con recibo, journal y punteros
  - docs(debt): cierra INC-DEBT-037 y registra INC-DEBT-039
  - docs(roadmap): registrar la autorreferencia del conteo de commits
  - docs(roadmap): fijar el conteo de commits sin publicar al SHA medido
  - docs(roadmap): recibo, journal y punteros de session-41 (C3j objetivo 5)
  - docs(adr): ADR-0146 acepta durable_delta_store como modulo raiz
  - docs(roadmap): receipt y punteros de session-40 (C3j objetivo 3)
  - docs(adr): ADR-0145 acepta los modulos raiz de contexto durable
  - docs(roadmap): avanzar punteros a session-39 con C3j slice 1 verificado
  - docs(roadmap): registrar C3j slice 1 con evidencia session-39
  - test(c3j): e2e de contexto durable CTX-UAT-006/013/014 a nivel sustrato
  - docs(roadmap): avanzar punteros a session-38 con C3i completo
  - docs(roadmap): cerrar C3i objetivo 5 con evidencia session-38
  - test(c3i): automatizar CTX-UAT-001 como script reutilizable en tests/
  - test(c3i): pinear identidad unica del bootstrap estable entre reinicios y refresh (C3i obj 5)
  - docs(roadmap): cerrar session-37 con convergencia de adopcion verificada
  - test(c3i): pinear el contrato de convergencia de adopcion en las superficies
  - docs(debt): registrar INC-DEBT-038 (dev install --source instala layout plano con recibo versionado)
  - docs(roadmap): cerrar session-36 con el estado del slice C3i y sus hallazgos
  - docs(roadmap): adoptar delta hypermedia C3i/C3j/C6/C7 sin abrir segundo roadmap
  - docs(roadmap): registrar session-35, el incidente del ledger y el rojo del guard
  - docs(debt): registrar que SDDK_STATE_HOME no aísla el ledger
  - test(uat): self-test de mutacion para el guard del puntero de estado
  - docs(roadmap): puntero a 3d810c48 con la suite re-verificada
  - docs(journal): adenda session-34i con la re-verificacion de la suite y el falso cuelgue
  - docs(roadmap): CURRENT alineado con 2.2.32 y el diagnostico corregido del guard
  - docs(roadmap): puntero a eff37cee tras el guard endurecido
  - docs(journal): adenda session-34h con la correccion del diagnostico del guard
  - docs(journal): adenda session-34g con el bump forzado y la brecha del guard
  - docs(roadmap): puntero a 6f909de2 / 2.2.31 tras el push
  - docs(roadmap): punteros de sesion-34f alineados con la evidencia observada
  - docs(receipts): evidencia del perfil completo session-34 y hunt de gates rojos
  - docs(adr): ADR-0144 propuesto - boundary de integracion con el host JCode
  - docs(journal): adenda session-34e (guard en verde, 2.2.30 alineado y commiteado)
  - docs(roadmap): punteros a d47a1766 / 2.2.30 con el guard de estado en PASS
  - docs(journal): adenda session-34d (DRIFT-1 fix, leccion del bump manual, stash de release)
  - docs(roadmap): CURRENT a 2.2.29/cfa477cf con C2 completo y C3g cerrado
  - docs(roadmap): puntero a d83bc120 / 2.2.29 (sin publicar) con cadena de la sesion
  - docs(journal): adenda session-34c (C2c con SDK publico, C3g completo)
  - docs(uat): C3g addendum - presupuesto estatico y runtime con providers reales
  - docs(uat): C2c re-ejecutado con el SDK público de jcode (T15-T18, decision_request ADR)
  - docs(journal): adenda session-34b (C2a/C2b re-ejecutados, drift chronos, bump 2.2.28 sin publicar)
  - docs(uat): C2b re-ejecutado contra chronos-mcp 0.1.4 real (T12-T14 PASS con fix de adapter)
  - docs(uat): C2a re-ejecutado contra cognicode-mcp real 0.97.3 (T08-T11 PASS observados)
  - docs(roadmap): punteros a v2.2.27/5ee68265 y adenda de cierre session-34


## [2.2.31] - 2026-09-29

### Fixes
  - fix(lint): reportar la ruta real de agents anidados y el hint que si regenera
  - fix(engine): C2B-DRIFT-1 - adapter fija CHRONOS_DB_PATH ademas de CHRONOS_STORE_PATH
  - fix(engine): adapter chronos soporta execution_query (renombre 0.1.4) con fallback legacy

### Other
  - docs(roadmap): punteros de sesion-34f alineados con la evidencia observada
  - docs(receipts): evidencia del perfil completo session-34 y hunt de gates rojos
  - docs(adr): ADR-0144 propuesto - boundary de integracion con el host JCode
  - docs(journal): adenda session-34e (guard en verde, 2.2.30 alineado y commiteado)
  - docs(roadmap): punteros a d47a1766 / 2.2.30 con el guard de estado en PASS
  - docs(journal): adenda session-34d (DRIFT-1 fix, leccion del bump manual, stash de release)
  - docs(roadmap): CURRENT a 2.2.29/cfa477cf con C2 completo y C3g cerrado
  - docs(roadmap): puntero a d83bc120 / 2.2.29 (sin publicar) con cadena de la sesion
  - docs(journal): adenda session-34c (C2c con SDK publico, C3g completo)
  - docs(uat): C3g addendum - presupuesto estatico y runtime con providers reales
  - docs(uat): C2c re-ejecutado con el SDK público de jcode (T15-T18, decision_request ADR)
  - docs(journal): adenda session-34b (C2a/C2b re-ejecutados, drift chronos, bump 2.2.28 sin publicar)
  - docs(uat): C2b re-ejecutado contra chronos-mcp 0.1.4 real (T12-T14 PASS con fix de adapter)
  - docs(uat): C2a re-ejecutado contra cognicode-mcp real 0.97.3 (T08-T11 PASS observados)
  - docs(roadmap): punteros a v2.2.27/5ee68265 y adenda de cierre session-34

## [2.2.28] - 2026-09-29

### Fixes
  - fix(engine): adapter chronos soporta execution_query (renombre 0.1.4) con fallback legacy

### Other
  - docs(uat): C2b re-ejecutado contra chronos-mcp 0.1.4 real (T12-T14 PASS con fix de adapter)
  - docs(uat): C2a re-ejecutado contra cognicode-mcp real 0.97.3 (T08-T11 PASS observados)
  - docs(roadmap): punteros a v2.2.27/5ee68265 y adenda de cierre session-34

## [2.2.27] - 2026-09-29

### Fixes
  - fix(ci): el smoke test aserta el layout versionado (contrato nuevo del tar con BUNDLE.toml)

## [2.2.26] - 2026-09-29

### Fixes
  - fix(ci): la asercion post-tar busca BUNDLE.toml a raiz (el tar del workflow es root-level)

## [2.2.25] - 2026-09-29

### Fixes
  - fix(ci): el bundle tarball standalone lleva BUNDLE.toml (layout versionado en dev update)

### Other
  - docs(roadmap): reconciliar puntero mecanico a 2fb5f738 / 2.2.24

## [2.2.24] - 2026-09-29

### Fixes
  - fix(cli): dev update en layout legacy hace merge, no swap destructivo

### Other
  - docs(uat): recibo del fix dev update legacy merge (RED->GREEN + falsaciones E2E)
  - docs(uat): recibo del fix de layout del instalador (RED->GREEN observado)

## [2.2.23] - 2026-09-29

### Fixes
  - fix(cli): dev update instala en version-dir y apunta current ahi

## [2.2.22] - 2026-09-29

### Fixes
  - fix(changelog): dedup de cabeceras fantasma e items arrastrados por merges (INC-DEBT-031)

### Other
  - docs(uat): recibo C3 re-anclado a b3160ae9 con lote T19-T28 observado hoy
  - docs(uat): recibo C0 re-anclado a 28ea2910 con T01/T02 observados hoy
  - docs(state): last_public_release_observed a v2.2.21 con evidencia de verificacion
  - docs(state): current_sha reconciliado a eaf43061 (docs-only)
  - docs(roadmap): punteros a v2.2.21 publicado y verificado

## [2.2.21] - 2026-09-29

### Fixes
  - fix(tests): test_h05_isolation resuelve el target dir real via cargo metadata

### Other
  - docs(roadmap): punteros a session-34 con C1 re-anclada a 89f60190
  - docs(roadmap): entrada session-34 con el lote C1 re-anclado y el guard H05 reparado
  - docs(uat): recibo C1 re-anclado a 89f60190 con lote T03-T07 observado hoy
  - docs(roadmap): puntero CURRENT a session-33b con suite 22/22
  - docs(roadmap): cerrar la extension session-33b con la suite en 22/22
  - docs(state): registrar v2.2.20 como release observado y verificado

## [2.2.20] - 2026-09-29

### Other
  - docs(coherence): informar el trigger release-archive-vault-complete con veredicto n/a
  - test(coherence): pinar la logica de veredicto con fixtures hermeticos
  - docs(roadmap): cerrar session-33 con el release v2.2.19 verificado

## [2.2.19] - 2026-09-29

### Fixes
  - fix(test): certificar los invariantes de REQ-DKA-004 sin leer el vault

### Other
  - docs(state): reconciliar el puntero con el release v2.2.18 verificado
  - test(release): fijar el contrato del layout del bundle y explicar el pin ciego de session-32
  - docs(debt): cerrar INC-DEBT-036 con la verificacion de v2.2.18 en red

## [2.2.18] - 2026-09-28

### Fixes
  - fix(cli): detectar el layout del tarball antes de aplicar --strip-components

## [2.2.17] - 2026-09-28

### Fixes
  - fix(cli): argv de cosign verify-blob con --bundle sin valor ni blob en su hueco

## [2.2.16] - 2026-09-28

### Fixes
  - fix(ci): el smoke step 2 buscaba el binario en $PREFIX/sddk y vive en $PREFIX/bin/sddk

## [2.2.15] - 2026-09-28

### Fixes
  - fix(ci): el pin de zcode en el smoke exige ficheros nativos, no symlinks

## [2.2.14] - 2026-09-28

### Fixes
  - fix(ci): el smoke exigia un symlink de zcode que ADR-0081 elimino en sept

## [2.2.13] - 2026-09-28

### Fixes
  - fix(cli): el CLI ignoraba SDDK_FRAMEWORK_DIR y el smoke de CI no podia pasar

## [2.2.12] - 2026-09-28

### Features

### Fixes
  - fix(cli): identidad cosign y bandera de certificado hoja alineadas con Fulcio
  - fix(install): la verificacion de firma no podia pasar nunca

### Other

## [2.2.11] - 2026-09-28

### Features

### Fixes
  - fix(ci): la sonda de version se ejecutaba sin bit de exec y fallaba en silencio

### Other

## [2.2.10] - 2026-09-28

### Features
  - feat(ci): un solo publicador canónico con 9 payloads, firmas y smoke test

### Fixes
  - fix(ci): los unified no-x86_64 ejecutaban el binario del target y morian con 126
  - fix(release): aceptar las firmas canonicas sin abrir el gate a extras
  - fix(ci): el job de firma descargaba a un directorio y firmaba otro
  - fix(ci): el staging de assets contaminaba el release con el bundle del repo

### Other
  - docs(journal): cierre de la segunda pasada de session-31
  - docs(state): reconciliar el puntero a b88b5d79 / 2.2.8
  - docs(debt): el gate de 9 assets rechaza las firmas que el instalador exige
  - docs(state): registrar la publicacion de v2.2.6 y su resultado negativo

## [2.2.9] - 2026-09-28

### Fixes

### Other


### Features

### Fixes

### Other


## [2.2.8] - 2026-09-28

### Fixes

### Other

## [2.2.7] - 2026-09-28

### Features

### Fixes

### Other

## [2.2.6] - 2026-09-28

### Features

### Fixes
  - fix(release): $CHANGELOG no existe, el bump abortaba a mitad
  - fix(ci): el workflow de release publicaba un ancla de manifest equivocada
  - fix(release): no duplicar la cabecera de CHANGELOG al re-declarar una version
  - fix(cli): no rechazar los bundles publicados al verificar el ancla del manifest
  - fix(cli): validar el ancla manifest_sha256 de BUNDLE.toml al instalar

### Other
  - docs(debt): registrar la regresion del bump y la suite no-hermetica
  - docs(debt): registrar INC-DEBT-031, CHANGELOG con 4 versiones duplicadas y una fantasma
  - test(release): cubrir permanentemente el guard de estaticidad musl
  - docs(state): reconciliar el puntero a acbf498f / 2.2.5
  - docs(journal): entrada de session-30 con evidencia observada del pipeline
  - docs(debt): registrar INC-DEBT-030, el publish local bloqueado en 8c/14 por identidad de firma
  - docs(debt): cerrar INC-DEBT-025 y sincronizar el indice con 027
  - docs(roadmap): alinea el puntero de estado a la version 2.2.4

## [2.2.5] - 2026-09-28

### Fixes
  - fix(cli): validar el ancla `manifest_sha256` de BUNDLE.toml al instalar

  `contents.manifest_sha256` se escribia (`dev manifest --bundle`) y se
  parseaba, pero ningun codigo comparaba el valor declarado con el sha256 real
  del `MANIFEST.sha256` incluido. El ancla era decorativa: reescribir el
  manifest dentro de un bundle no rompia nada. `verify_manifest_anchor` cierra
  el ciclo y `dev install` lo invoca despues de `verify_bundle_compat` y antes
  de escribir nada en disco. Fail-closed, con caso permisivo para bundles ya
  publicados que no declaran el campo. Cierra INC-DEBT-025 (parte 2).

### Known blockers
  - El publish local de 2.2.4/2.2.5 aborta en 8c/14: falta la identidad de
    firma del proyecto en este host. Es el guard de INC-DEBT-024, no un
    defecto. Sin estado parcial: no se publico tag ni assets. Ver
    INC-DEBT-030. Publicar desde GitHub Actions, o usar
    `SDDK_SKIP_SIGNING=1` asumiendo el cambio de contrato de instalacion.

## [2.2.4] - 2026-09-28

### Features

### Fixes
  - fix(release): el guard de estaticidad rechazaba el binario musl correcto

### Other
  - docs(roadmap): reconcilia el puntero de estado a 0fbe9971 / 2.2.3

## [2.2.3] - 2026-09-28

### Features

### Fixes
  - fix(storage): retry ATOM-PER-ROW writes past DatabaseBusy

### Other
  - docs(debt): resuelve INC-DEBT-027 con ruta rootless de musl
  - docs(journal): cierre de session-30 — identidad corregida, publish bloqueado por musl

## [2.2.2] - 2026-09-28

### Features

### Fixes
  - fix(identity): derivar el fallback_seed del path canonico, no de un UUID aleatorio

### Other
  - docs(state): reconciliar puntero al estado publicado
  - docs(debt): registrar INC-DEBT-027 (musl toolchain ausente) y cerrar session-29

## [2.2.1] - 2026-09-28

### Features

### Fixes
  - fix(release): manifest_sha256 era el hash de la primera linea, no el del manifest
  - fix(release): no re-bumpear cuando el workspace ya declara la release

### Other
  - docs(state): reconciliar puntero a 78876492
  - docs(roadmap): dejar el estado real de session-29 en CURRENT.md
  - docs(journal): tipografia en la correccion sobre la version de release
  - docs(journal): corregir que publicar 2.2.0 no lleva el doble bump
  - docs(debt): resolver el alcance de INC-DEBT-026 contra el asset publicado
  - docs(journal): cobertura del paso 2.5, cinco mutaciones y dos falsos verdes
  - test(release): cubrir el contrato de release.sh paso 2.5
  - docs(state): reconciliar el puntero al estado real del repo
  - docs(journal): el segundo defecto de release-bump y la leccion de proceso
  - docs(journal): registrar el incidente de doble bump de session-28

## [2.2.0] - 2026-09-28

### Features

### Fixes
  - fix(release): derivar la version desde el workspace, no solo desde el tag

### Other
  - docs(roadmap): constancia de dos caracteres CJK en el mensaje de 6228ad12
  - docs(roadmap): el bloqueante musl no bloquea la publicacion por CI
  - docs(roadmap): el bloqueante musl requiere root, con el intento fallido documentado
  - docs(roadmap): resultado observado del dry-run y el bloqueante musl
  - docs(roadmap): cierre de session-27 y el INC de la clase 'gate desconectado'

## [2.1.1] - 2026-09-28

### Features

### Fixes

### Other

## [2.1.0] - 2026-09-28

### Features
  - feat(ops): script que reconcilia el puntero de estado, no solo lo detecta

### Fixes
  - fix(release): usar la admision v2 (contra el ultimo tag publicado)
  - fix(ci): fijar cosign v2.4.3 explicito en ambos pasos del workflow
  - fix(release): negarse a firmar fuera de CI antes de intentarlo
  - fix(release): verificar la identidad de firma antes de publicar (INC-DEBT-024)
  - fix(uat): sustituir el umbral wall-clock del gate inv10 por un ratio
  - fix(supply-chain): corrige el pin cosign a refs/tags y vuelve deterministas los guards
  - fix(supply-chain): publica el .pem para que la firma detached sea verificable
  - fix(supply-chain): alinea la firma del CI con la verificacion del instalador
  - fix(supply-chain): exige firma cosign y pinea identidad e issuer
  - fix(release): release.sh construye un binario musl real y verifica el linkage
  - fix(tests): el guard del puntero era insatisfacible por construccion
  - fix(roadmap): reconcilia STATE.yaml desfasado desde session-15 y lo ata a git
  - fix(test): el e2e de instalacion llevaba la firma como verificada sin existir
  - fix(install): el instalador pedia un asset que el release no publica (404)

### Other
  - docs(roadmap): reconciliar puntero de estado a 2543da2
  - docs(journal): falsificacion de ledger verify y censo real de ciclos
  - docs(roadmap): dejar constancia de un token corrupto en el mensaje de 07e7249
  - docs(roadmap): reverificar bajo carga el fix de flake de session-22
  - docs(roadmap): cierre de session-25 — INC-DEBT-024 cerrado, 2.1.0 admission ACCEPT
  - docs(adr): cerrar el hueco de implementacion de ADR-0143 §(a) (INC-DEBT-024)
  - docs(roadmap): receipt de verificacion de session-24 (5057/0/19, 258 suites)
  - docs(roadmap): sincronizar punteros al cierre de session-24 (identity gate de firma)
  - docs(debt): INC-DEBT-024 — la firma keyless en local no puede satisfacer el pin
  - docs(roadmap): puntero a 91e75c2 via el reconciliador propio
  - docs(adr): ADR-0143 — trust root de la firma, y corrijo mi propio registro
  - docs(roadmap): registra el cierre de la deuda del puntero (reconciliador)
  - docs(roadmap): cierre de session-23 con el flake R-flake-inv10 resuelto
  - docs(roadmap): evidencia final de session-22 a 2.0.9 con --locked
  - docs(roadmap): registra el cierre del lock y la leccion de proceso
  - docs(roadmap): puntero a 8136bbf y cierre del bloqueo del lock
  - test(release): el guard de puntero tiene que ver Cargo.lock, no solo Cargo.toml
  - docs(roadmap): tercera via para el lock, probada y estrecha
  - docs(roadmap): el Cargo.lock stale rompe CI y release, no era cosmético
  - docs(roadmap): reconcilia puntero a session-22 con la evidencia observada
  - docs(roadmap): reconcilia puntero y registra session-20
  - docs(roadmap): journal session-19 — premise falsada, INC-021 cerrada
  - docs(debt): cierra INC-021 con la evidencia y la premisa falsada
  - test(release): el guard de pipelines pasa a verde con la evidencia de session-19
  - docs(roadmap): CURRENT.md a session-18 y corrige erratas propias
  - docs(roadmap): journal session-18 y cierre de la ventana de auto-referencia
  - docs(roadmap): journal session-17 + actualiza el indice de deuda
  - docs(release): declara la autoridad de cada pipeline y corrige la causa raiz de INC-021
  - test(release): guard de coherencia entre los dos pipelines de release
  - docs(roadmap): journal session-16 — instalacion rota y premisa de firma falsada
  - docs(debt): registra los hallazgos de session-16 (021 musl, 022 asset 404, correccion del S14)
  - docs(roadmap): registra la leccion del pre-push hook sobre Cargo.lock
  - docs(roadmap): receipt v2.0.1 + punteros post-release + journal session-15

## [1.175.0] - 2026-09-27

### Fixes

### Other
  - docs(debt): indexa los 6 INCs de session-14 + corrige la deriva de FC-2
  - docs(roadmap): punteros post-release v2.0.0 + veredicto sobre C2

## [1.174.0] - 2026-09-27

### Features

### Fixes
  - fix(cli): cierra la primitiva de escritura arbitraria en sddk dev update
  - fix(engine): retira las referencias colgantes a los spikes borrados

### Other
  - docs(roadmap): receipt de la release v2.0.0 + revisión del INC de test_ports
  - docs(roadmap): cierre de session-14 — push verificado, v1.173.0 pendiente de publicar
  - refactor(cli)!: retira 1.403 LOC de spikes muertos de la API pública

## [1.173.0] - 2026-09-27

### Breaking changes (internal)

`refactor(cli)!` removed four public modules that had **zero consumers**:
`sddk_cli::spike_axs3`, `sddk_cli::spike_axs4`, `sddk_cli::spike_axs5`,
`sddk_engine::spike_sp06` (1.403 LOC of completed experiments whose
findings are preserved in `docs/architecture/spikes/`).

This is **not** breaking for users of the released binary or bundle. SDDK
is not published to crates.io and no workspace crate depended on the
removed modules. The `!` marker is honest at the git level and is
retained for git archaeology; the version bump is a **minor**, not a
major, because the observable contract for consumers — the CLI, the
bundle format, and the install path — is unchanged.

`scripts/release-bump.sh` derives `major` from any `!` or
`BREAKING CHANGE` in the range, so it proposed `2.0.0`. That was
overridden deliberately with `--force-version 1.173.0`, following the
same override path used for `v1.170.3`. A permanent `2.0.0` in this
changelog would have asserted a consumer-facing API break that never
happened.

### Features
  - feat(operations): FC-4 docs/operations/uat-replay.sh — pinned-release replay

### Fixes
  - fix(cli): el gate de clippy de sddk release es -D warnings, no -D errors
  - fix(gateway): evidence.bundle.write now really writes the bundle

### Other
  - docs(roadmap): punteros de session-14 — SHAs finales + claim de verificación corregido
  - docs(debt): 3 INCs de la auditoría session-14 + corrige premisa stale de C2
  - docs(roadmap): session-13 handoff — full context for tomorrow's session-14
  - docs(roadmap): session-13 closeout — FC-4 implemented + cert RCA + state sync at HEAD 72825fe
  - docs(roadmap): mark FC-4 as IMPLEMENTED in FEATURE-CANDIDATES
  - docs(roadmap): enrich v1.172.0 cert with flake root-cause analysis
  - docs(debt): formalize legacy 'body **status**: closed' to frontmatter
  - docs(roadmap): state sync — v1.172.0 cert formalized (HEAD 96da6db)
  - docs(roadmap): v1.172.0 CERTIFICATION-RECEIPT + UAT-EVIDENCE (T29/T31)
  - docs(roadmap): reconcile release v1.172.0 publication

## [1.171.1] - 2026-09-22

### Fixes
  - fix(test): skip stale_detects_geometry_change cleanly when chromium missing

### Other
  - docs(roadmap): session-12 audit reconcile v1.171.0 to PASS_PARTIAL_OBSERVED
  - docs(roadmap): RECEIPT + UAT-EVIDENCE + CURRENT/STATE for v1.171.0

## [1.171.0] - 2026-09-22

### Features
  - feat(vault): sddk vault show <node-id> — FC-6

### Other
  - test(cli): simplify FC-6 doc comment to satisfy clippy lint
  - test(cli): apply cargo fmt to FC-6 integration test


## [1.170.7] - 2026-09-22

### Other
  - test(cli): simplify FC-6 doc comment to satisfy clippy::doc_lazy_continuation


## [1.170.6] - 2026-09-22

### Fixes
  - fix(release-bump): anchor sed to workspace version, not last tag


## [1.170.5] - 2026-09-22

### Features
  - feat(vault): sddk vault show <node-id>

### Other
  - test(cli): integration test for vault show end-to-end

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [1.170.1] - 2026-09-22

### Features
  - feat(uat): sddk uat validate --format json
  - feat(uat): sddk uat status --format json
  - feat(cli): sddk dev doctor --format json

### Other
  - docs(roadmap): registra FC-7/FC-8 implementadas y marca FC-2 done
  - docs(roadmap): SESSION-JOURNAL + STATE.yaml \u2014 uat status JSON shipped
  - docs(roadmap): SESSION-JOURNAL + STATE.yaml \u2014 FC-2 delivered, next v1.171.0
  - docs(adr): ADR-0142 release script SemVer-correctness via release-bump.sh
  - docs(roadmap): STATE.yaml reconciled \u2014 v1.170.0 CERTIFIED

## [1.169.11] - 2026-09-14

No binary release. Test-only refactor delivered behind a `chore(release)`
bump so the pre-push hook accepts the push to `main`.

### Other
  - refactor(cli): drop `enforce_admission_or_block[_in]` legacy wrappers (commit `c3459db`). Four `cli_wrapper_*` tests now call `enforce_admission_or_block_ctx` directly with `ApprovalLoopContext::none()`. The legacy wrappers and their `#[allow(dead_code)]` markers are removed. CLI test surface: 1110/1110 ok; workspace: 3877/3877 ok.

## [1.169.10] - 2026-09-14

> **Note**: the entry date was retroactively reconstructed from the GitHub
> release timestamp (2026-09-14T09:41:35Z) because the `release-bump.sh`
> step that normally writes the CHANGELOG entry was skipped during the
> v1.169.x ceremonial bumps. This is the first release whose entry was
> hand-curated post-hoc.

### Fixes
  - fix(cli): record `authority.admission.decided` into the current project only (commit `3499d40`). The M2 audit event was being silently dropped on bootstrapped projects because `record_admission_decision` broadcast across every project dir in the global state, and the `SqliteEventStore::append` upsert violated the NOT NULL constraints (`display_name`, `scope`, `created_at`) that Storage migrations add to the same `projects` table. New `ensure_project_row` helper opens via `Storage` and upserts with the full schema.
  - fix(stderr): eliminate ~20 lines of `FOREIGN KEY constraint failed` noise per denied command. Same root cause: the cross-project broadcast iterated dirs without bootstrapped `projects` rows and emitted an `eprintln` per FK fail.

### Other
  - test(cli): pin M2 approval loop end-to-end with `sddk approval grant` (commit `d5f5d85`). New file `crates/sddk-cli/tests/cli_approval_loop_e2e.rs` exercises S1 (list) → S2 (grant) → ledger invariants against the published binary via `CARGO_BIN_EXE_sddk`.
  - chore(release): bump version 1.169.9 → 1.169.10 (commit `ee61deb`).

## [1.169.8] - 2026-09-14

M2 approval loop end-to-end (R-4-002 / WU-C4-7). ApprovalLoopContext is
the canonical caller surface for `admit_governed`. Granted approvals are
durable `Facts.approval_refs`; the engine skips the approval gate when
the matching fact is present. In stage `LowMedium` (M1), `RequireApproval`
is advisory on High surfaces (the deny path still records
`authority.admission.decided`); it will become blocking on High surfaces
in M4 (`EnforcementStage::All`).

### Features
  - feat(engine): `Facts.approval_refs` + skip approval gate when the matching fact is present (commit `96fc9ff`).
  - feat(cli): M2 approval loop end-to-end — `admit_governed` + grant lookup + request emission (commit `6546419`).
  - feat(cli): choke point único de enforcement en `admission.rs` (commit `80f6179`).
  - feat(engine): `emit_admission_decision` (`authority.admission.decided`) (commit `a100a3c`).
  - feat(engine): explain real vía `admit_with_explanation` (commit `dd0058c`).
  - feat(cli): `arch_lint` freeze `deny-new-dependency` con allowlist inicial (commit `38b132a`).

### Fixes
  - fix(cli): actualiza `C4_LEGACY_ALLOWLIST_M1` por line shifts del M2 approval loop (commit `c962c65`).

### Other
  - test(cli): M2 approval loop adds 2 `admission.decided` events to ledger (commit `8e5f532`).
  - test(cli): E2E Deny del authority engine sin efectos externos (commit `995399f`).
  - fix(cli): higiene clippy `-D warnings` del milestone M1 C4 (commit `2bf6ae9`).
  - chore(release): bump version 1.169.7 → 1.169.8 (commit `b3f132d`).

## [1.169.0 - 1.169.7] - 2026-09-12 → 2026-09-13

> **Synthetic entry covering seven ceremonial bumps without a binary
> release.** The C1.5 ledger cutover cycle closed across these bumps
> (`c15-ledger-removal-2026-09-13`); the bumps themselves were pushed to
> `main` via the pre-push hook's `chore(release)` predicate, but
> `release-bump.sh` was not invoked (no binary release). The entries
> below were reconstructed from `git log` post-hoc.

### Features
  - feat(storage): MIGRATION_20 elimina físicamente `ledger_events` (commit `4edebb1`, C1.5/WU-C15-4).
  - feat(domain): universal evidence cutover — `PlanningEvidenceKind` a read-only compat (commit `6bbc104`, WU-C2).
  - feat(storage): read-only legacy window con allowlist (commit `3662b87`, WU-C1.4).
  - feat(storage): hard-disable legacy domain event writes (commit `de9bbbf`, WU-C1.3).
  - feat(storage): redirect domain events to canonical event log (commit `319e285`, WU-C1.2).
  - feat(engine): cycle/run lifecycle cutover — runtime truth a Run/Authority (commit `14c32f3`, WU-C3, cierra C1-C3).

### Refactors
  - refactor(storage): WU-C15-3 elimina read layer legacy de `ledger_events` (commit `708ad88`).
  - refactor(domain): elimina `Ledger::load_all_ledger_events` del port (commit `3c62e03`, WU-C15-2, R-15-003).
  - refactor(cli): lectores fork/graph/telemetry canonical-only (commit `ed7e2be`, WU-C15-1, R-15-005a).
  - refactor(cli): allowlist C1 fuera, ratchet DDL-absence dentro (commit `bc5266d`, C1.5/WU-C15-7).
  - refactor(engine): `WritableSurface::LedgerEvents` out (commit `0f2e339`, C1.5/WU-C15-6).
  - refactor(storage): `release_lease_with_event` + wrappers legacy out (commit `c9d231a`, C1.5/WU-C15-5).

### Fixes
  - fix(storage): comentario stale `verify_cross_ledger_consistency` out (commit `c7f029c`, C1.5).
  - fix: verde en clippy `-D warnings` tras cutovers C1-C3 (commit `79c7c92`).
  - fix(test): regenera snapshots de help y limpia unused-mut post cycle 2/4 (commit `20a8451`).

### Other
  - test(storage): fixtures finales canónicos + draft release notes (commit `641fb9c`, C1.5/WU-C15-8).
  - test: alinea cli/sqlite_storage con cutovers C2-C3 (`MIGRATION_19`, `remediation=OPEN`, commit `76fbabf`).
  - test: limpia `allow(deprecated)` muertos tras removal C1.5 (commit `0acb24e`).
  - test(storage): export/recovery fixture para `ledger_events` (commit `c31f4ef`, WU-C1.1, gate destructivo del cutover).
  - style: `cargo fmt` tras cutovers C1-C3 (commit `6553ab1`).
  - docs(debt): INC-DEBT-023 — lints advisory sin ciclo de expansión (commit `22970ba`, requisito cycle-7b ADR-0047).
  - chore(release): bumps ceremoniales 1.169.0 → 1.169.7 (commits `8d88a7e`, `f60b974`, `5f45cf2`, `f76fe36`, `8121a5b`, `aaf5052`, `7890648`, `b5145fc`, `afcc737`, `30ff93a`, `53d583c`, `5ed8688`, plus Cargo.lock lockstep commits).

## [1.168.31] - 2026-09-12

### Other
  - docs(arch): update M9 row to reflect v1.168.29 ARCH-LINT-AX-S5 promotion state

## [1.168.30] - 2026-09-12

### Other
  - test(shell): cross-crate pin for scripts/release-receipt.sh vs engine::infer_actor_kind

## [1.168.29] - 2026-09-12

### Features
  - feat(lint): promote 3 of 4 AX-S5 asset_* lints to default: deny (ARCH-LINT-AX-S5)

## [1.168.28] - 2026-09-12

### Features
  - feat(spec): derive related-command depth-2 from depth-1 table (AX-S3 D2)

### Other
  - docs: session handoff 2026-09-12 (ARCH-HEX-001 closure v1.168.27)

## [1.168.27] - 2026-09-12

### Features
  - feat(authority): script-side GH Releases actor tracking (ARCH-HEX-001 slice 3)

## [1.89.6] - 2026-09-07

### Fixes
  - fix(storage): evitar self-deadlock de conexión en load_node_run y stream_node_runs
  - fix(scripts): aislar TMPDIR en release.sh para un gate de tests determinista

### Other
  - docs(release): documentar aislamiento de TMPDIR en el gate de tests

## [1.70.0] - 2026-09-02

### Features
  - feat(engine): frontier_for_state surfaces pause/resume/supersede when Paused
  - feat(cli): add CycleCommand::Pause and CycleCommand::Resume with args and handlers
  - feat(engine): implement cycle_pause and cycle_resume primitives
  - feat(cycle): add CycleStatus::Paused variant and PauseReason closed-set

### Other
  - docs: ADR-cycle-pause draft + BACKLOG Primitive 1 status + CHANGELOG entry
  - test(cli): clap rejects invalid PauseReason at parse-time

## [Unreleased]

### Added
  - feat(domain): `CycleStatus::Paused` variant + `PauseReason` closed set (`priority_revoked`, `context_switch`, `dependency_waiting`) + `CycleManifest` fields (`pause_at`, `review_at`, `last_pause_reason`). `assert_variant_count_eq!` bumped to 11.
  - feat(engine): `Engine::cycle_pause` releases lease atomically, emits `cycle.pause.requested` + `cycle.pause.applied` events, writes `pause-receipt.json`. `Engine::cycle_resume` re-acquires fresh lease with incremented fencing token.
  - feat(cli): `sddk cycle pause --reason <value> --lease-owner <owner> --fencing-token <token>` pauses a cycle. `sddk cycle resume --lease-owner <owner>` resumes a paused cycle and prints new fencing token.
  - feat(cli): `PauseReasonArg` clap ValueEnum (`priority-revoked`, `context-switch`, `dependency-waiting`).
  - feat(engine): `frontier_for_state` now surfaces `cycle.pause` (no-op advisory), `cycle.resume`, and `cycle.supersede` when cycle status is `Paused`.
  - test(engine): 9 new tests in `crates/sddk-engine/tests/cycle_pause.rs` cover pause/resume scenarios.
  - test(cli): 3 new tests in `crates/sddk-cli/tests/cycle_pause_args.rs` cover clap parse-time rejection of invalid `--reason` values.

## [1.85.0] - 2026-09-04

### Added
  - docs(adr): **ADR-072-PLANNING-LEDGER-DOMAIN-MODEL** accepted — 3 architectural locks: Shape C hybrid persistence (SQL topology + CAS evidence bodies), WritableSurface extended with 4 new variants (PlanItem [Human, Agent], DependencyEdge [System], EvidenceAttachment [Human, Agent], DecisionRecord [Human]), WorkItemStatus 6-variant closed set (`Draft`, `Active`, `Paused`, `Done`, `Superseded`, `Cancelled`) with compile-time `assert_variant_count_eq! 6`.
  - feat(domain): nuevo módulo `crates/sddk-domain/src/planning/mod.rs` (~681 líneas) con 6 domain types — `WorkItemV1`, `DependencyEdgeV1` (con `DependencyEdgeKind` 2-variant: `Blocks`, `BlocksOnClosure`), `WorkItemStatus` (6-variant con tabla de transiciones compile-time enforced), `EvidenceAttachmentV1` (con `PlanningEvidenceKind` 5-variant), `DecisionRecordV1` (con `DecisionKind` 4-variant: `Accept`, `Reject`, `Defer`, `Escalate`), `PlanningProvenanceChainV1`. Closes AC-PLN-01..06.
  - feat(engine): `WritableSurface` enum extendido de 8 → 12 variantes con `assert_variant_count_eq! 12`; `WRITABLE_SURFACE_MATRIX` con 4 nuevas entradas admitiendo actor kinds por superficie. Closes AC-PLN-10.
  - feat(engine): 6 nuevos `emit_planning_work_item_*` (drafted, activated, paused, resumed, completed, superseded) usando `Type::Custom(<id>)` con `schema_version: 1` per ADR-071. Todos cablean `with_correlation_from_context` para propagar `causation_id`+`correlation_id`. Closes AC-PLN-09.
  - feat(storage): `MIGRATION_14` con 2 tablas SQL (work_items_v1, work_item_dependencies_v1) — Shape C topology portion. Migration aditiva (no DROP/ALTER sobre tablas existentes).
  - feat(storage): `FilesystemCas` en `crates/sddk-storage/src/cas.rs` (~177 líneas) — Shape C evidence-bodies portion. CAS root default `~/.local/share/sddk/cas/<sha[0:2]>/<sha[2:4]>/<sha>.json`.
  - feat(domain,storage): `CasPort` trait en `crates/sddk-domain/src/ports.rs` (`put`/`get`/`exists`).
  - test(domain): 14 nuevos unit tests en `planning/mod.rs` — round-trip WorkItemV1/DependencyEdgeV1/EvidenceAttachmentV1/DecisionRecordV1, state machine invariants (7 transiciones válidas, terminales rechazan todo, identity hash determinista). 5 nuevos unit tests en `storage/cas.rs` — FilesystemCas put/get round-trip + hash collision.
  - test(domain): preserved 8 actor_authority_baseline + 12 event_authority + 9 event_correlation_widening + 4 cross_ledger_consistency + 3 event_replay_equality + 10 event_correlation_wiring + 4 approval_envelope_wins_test + 25 DW-IR-005 determinism — todas UNCHANGED y PASS.

### Changed
  - authority.rs: 4 nuevas variantes WritableSurface + 4 nuevas filas en WRITABLE_SURFACE_MATRIX (12 totales).
  - event_bus/emit.rs: +398 líneas con 6 nuevos emitters ADR-071 compliant.
  - storage/migrations.rs: `LATEST_SCHEMA_VERSION` bumped 13 → 14; MIGRATION_14 aditiva.

### Tests
  - test(workspace): `cargo test --workspace` 2143/0/9 PASS (vs 2124 baseline, +19 nuevos). `cargo clippy --workspace --all-targets -- -D errors` exit 0. `cargo fmt --check` exit 0.
  - verify verdict: **PASS_WITH_WARNINGS** — 8/12 ACs PASS, 4/12 PARTIAL (AC-PLN-09/10/11/12 — cobertura de integration tests). 4 MEDIUM + 4 LOW deviations registradas en debt-report.
  - debt-verify verdict: PASS_WITH_WARNINGS — 0 BLOCKER/CRITICAL/HIGH; FIND-PLN-004/005/006 (test pyramid gap) owner PLN-LEDGER-002 (next cycle).

### Carryovers preservados
  - FIND-000001..000005 (DW-OPERATORS-001 H6) — unchanged
  - FIND-000006 (ARCH-HEX-001 LOW unused ActorKind import at approval.rs:10) — unchanged
  - FIND-000007 (EVT-LEDGER-001 LOW unused EntityRef import at event_replay_equality.rs:8) — unchanged
  - FIND-000008 (EVT-LEDGER-001 LOW unused mut storage at cross_ledger_consistency.rs:174) — unchanged
  - INC-HX-AUTH-003 — unchanged (resolved)
  - INC-HX-AUTH-004 — unchanged (partial 5/7 paths; paths 6-7 OPEN owner RX-SECRETARY-001/002)

### Known issues (carry to PLN-LEDGER-002/003)
  - **FIND-PLN-004/005/006 (MEDIUM/P1)**: ~48 integration tests pendientes (engine integration for 6 emit functions + storage integration for MIGRATION_14 + WritableSurface validation tests). Owner PLN-LEDGER-002.
  - **FIND-PLN-001 (MEDIUM/P1)**: Apply fase bundle 10-14 concerns en 1 mega-commit. AGENTS.md §2.1 violation. Owner process-followup-cycle.
  - **FIND-PLN-007 (LOW/P2)**: `PlanningProvenanceChainV1::verify_references` es stub documentado. Owner PLN-LEDGER-003.
  - **FIND-PLN-008 (LOW/P2)**: `WorkItemV1::compute_identity` usa serialización completa (incluye created_at volátil). Owner domain-identity-cleanup.

## [1.84.0] - 2026-09-04

### Added
  - docs(adr): **ADR-071-EVENT-SCHEMA-VERSIONING** accepted — codifica la regla "los nuevos eventos del workspace no pueden añadirse a `std_registry`; en su lugar se modelan como `Type::Custom(<id>)` con `schema_id` versionado en `EventEnvelopeV1`." Closes AC-EVT-01/12, formaliza el ciclo de extensión sin tocar el canónico.
  - feat(domain): `ActorRef` struct (5 campos) emitido junto a `ActorKind` (3 variantes locked) como campo **aditivo** `actor_ref: Option<ActorRef>` en 4 carriers canónicos: `LedgerEvent`, `GateReceipt`, `EventContext`, `JournalEntry`. El corpus legacy conserva `actor: String` vía `#[serde(default)]`. Closes AC-02.
  - feat(domain,engine): `EventContext` ahora soporta `causation_id` y `correlation_id` (UUIDv7). Los 9 sitios de `emit.rs` cablean `with_correlation_from_context` para propagar ambos IDs en cada evento emitido. Closes AC-03.
  - feat(domain,storage): primitivo `Snapshot` + `SnapshotPort` trait + `MIGRATION_11` + tabla `event_snapshots_v1` para replay equality. `Storage::verify_cross_ledger_consistency` valida invariante AC-06 contra el store.
  - feat(cli): `sddk ledger replay --from-snapshot --verify-hashes` y `sddk ledger verify-cross-ledger` para auditoría de causalidad sin código ad-hoc.
  - fix(engine,cli): AC-08 (`gate_receipt` requiere autoridad System) y AC-09 (`knowledge_ingest` requiere autoridad Human) — engine gate + CLI knowledge_ingest validan `AuthorityContext` contra `WritableSurface::GateReceipts` / `WritableSurface::KnowledgeGraphVault`.
  - fix(domain): AC-06 `ApprovalState` envelope-wins precedence — `event.actor.id` toma precedencia sobre el payload `ApprovalDecisionInput.actor` (con fallback legacy `payload.actor`). AC-10 reforzado en projection.
  - test(domain,engine,storage): 4 suites nuevas — `event_replay_equality` (3 tests, AC-05), `event_correlation_widening` (9 tests, AC-02/03), `cross_ledger_consistency` (4 tests, AC-06), `event_correlation_wiring` (10 tests, AC-03 wiring at 9 emit sites), `approval_envelope_wins_test` (4 tests, AC-10). Total nuevos: 30 tests.
  - test(cli): `CliFixture::run` ahora fija `USER=user:test-cli-actor` para que los tests de knowledge-import operen con actor Human por defecto (no Agent).

### Fixed
  - fix(debt): **INC-HX-AUTH-003** (P1) — frontmatter flipped `status: open → status: resolved`; provenance de eventos ahora confiable vía `actor_ref: Option<ActorRef>` aditivo en 4 carriers canónicos.

### Changed
  - docs(debt): **INC-HX-AUTH-004** paths 4-5 cerrados (gate_receipt → EVT-LEDGER-001, knowledge_ingest → EVT-LEDGER-001). Paths 1-3 cerrados por ARCH-HEX-001. Paths 6-7 siguen OPEN owner RX-SECRETARY-001/002.

### Tests
  - test(workspace): `cargo test --workspace` 2124/2124 PASS (115 grupos). `cargo clippy -D errors` exit 0. `cargo fmt --check` exit 0. `cargo build --release -p sddk-cli` exit 0.
  - verify round 1 → FAIL (2 BLOCKER + 1 CRITICAL + 2 HIGH); 5 commits de corrección (dac6b59, ab52600, 3baba1b, 64f8516, 34c6150); verify round 2 → PASS.

## [1.83.0] - 2026-09-04

### Added
  - docs(adr): **ADR-070-ENGINE-AUTHORITY-ENFORCEMENT** accepted — enforces the matrix declared by ADR-069 at runtime via `AuthorityContext` (`for_cli` canonical + `#[cfg(test)] for_test`), `infer_actor_kind` extraction (locked v1.81.x prefix heuristic, replaces `cycle.rs:1217-1223`), and `WRITABLE_SURFACE_MATRIX` const table (8 surfaces × admitted `ActorKind`). New `EngineError::AuthorityContextRejected` variant for fail-closed validation. Closes AC-ARCH-HEX-01/02/04/05/07, REQ-LYR-*, REQ-WS-*.
  - feat(engine): new `crates/sddk-engine/src/authority.rs` module (~200 lines) declaring `WritableSurface` enum (8 variants matching ADR-069 §3), `AuthorityContext` struct + `for_cli`/`for_test` constructors + `validate(surface)` method, `infer_actor_kind(actor_id: &str) -> ActorKind` helper, and `WRITABLE_SURFACE_MATRIX` const table.
  - feat(engine): `apply_transition`, `cycle_pause`, `cycle_resume`, `cycle_supersede` each accept `auth: &AuthorityContext` as required parameter; each calls `auth.validate(WritableSurface)` before any ledger mutation (fail-closed). `cycle_pause` and `cycle_supersede` additionally verify `auth.lease_owner` matches.
  - feat(engine): `ApprovalDecisionInput` and `ApprovalRequestedInput` each gain `actor_kind: ActorKind` field. `emit_approval_decision` (event_bus/emit.rs:259) replaces hardcoded `kind: ActorKind::Human` with caller-supplied input and validates `actor_kind ∈ {Human, System}` for decision events. `emit_approval_requested` (emit.rs:191) replaces hardcoded `kind: ActorKind::Agent` with caller-supplied input and validates `actor_kind ∈ {Agent}` for request events.
  - feat(cli): `cycle.rs:1217-1223` prefix heuristic replaced with `sddk_engine::authority::infer_actor_kind` call; `AuthorityContext::for_cli(...)` constructed from CLI env. `approval.rs` populates `actor_kind` on `ApprovalDecisionInput` via the same helper. Closes DRY violation flagged in HX-AUTHORITY-001 audit note.

### Fixed
  - fix(engine): **INC-HX-AUTH-002 closed** (P0/critical, ARCH-HEX-001 owner) — forced-Human default in `emit_approval_decision` removed; caller-supplied `actor_kind` propagates to `EventEnvelopeV1.actor.kind`. Provenance is now trustworthy for H1's PLN-LEDGER-001 work.
  - fix(engine): **INC-HX-AUTH-001 closed** (P1) — 8 writable surfaces now have runtime authority validation via `WRITABLE_SURFACE_MATRIX`.

### Changed
  - docs(debt): **INC-HX-AUTH-004** annotated "4 of 7 paths closed by ARCH-HEX-001" (paths 1-4: approval grant/deny, approval_request, cycle_transition, cycle_pause/resume). 3 paths deferred: gate_receipt creation → EVT-LEDGER-001, knowledge_ingest → EVT-LEDGER-001, secretary_closed_set → RX-SECRETARY-001/002 + EVT-LEDGER-001.

### Tests
  - test(engine): new `crates/sddk-engine/tests/event_authority.rs` (12 tests) — caller-supplied `actor_kind` survives round-trip in `emit_approval_decision` and `emit_approval_requested` across Human/Agent/System variants; 2 validator rejection tests; 1 prefix-heuristic consistency test; 3 writable-surface-matrix coverage tests. `cargo test -p sddk-engine --test event_authority` 12/12 pass.
  - test(domain): 8 tests in `crates/sddk-domain/tests/actor_authority_baseline_tests.rs` continue to PASS unchanged — `ActorKind` closed set, `ActorRef` 5 fields, CLI prefix mapping, `LedgerEvent`/`GateReceipt`/`JournalEntry` provenance loss all preserved.
  - test(workspace): `cargo test --workspace --offline` green; 1 unrelated pre-existing flake in `dev::rdi_tests` (same as DW-IR-005 / HX-AUTHORITY-001, unrelated to this cycle).

### Documentation
  - docs(adr): **ADR-070** cross-references ADR-069 (matrix), ADR-068 (determinism), INC-HX-AUTH-001/-002/-004, plus downstream EVT-LEDGER-001 + RX-SECRETARY-001/002 + DW-OPERATORS-001.
  - docs(debt): INC-HX-AUTH-001, INC-HX-AUTH-002 frontmatter `status: closed` + Lifecycle row dated 2026-09-04. INC-HX-AUTH-004 sub-lifecycle annotation with 4 closed + 3 deferred paths.

Cycle path: A-min (CODE-CHANGE, not governance-only — exit gate demands "enforced by tests/checks"). 5 commits en rango `d43b120..f08317f` (1 docs(adr) + 3 feat/fix/test + 1 style hygiene post full profile). 10 ACs / 21 REQs / 20 Given-When-Then scenarios. Verify verdict pending.

## [1.82.0] - 2026-09-04

### Added
  - docs(adr): **ADR-069-EXPLICIT-AUTHORITY-MATRIX** accepted — locks the canonical 4-actor taxonomy (Human, Agent, System, Secretary ≡ `Agent{role=secretary}`), the writable-surface matrix (8 surfaces), the approval-point matrix (≥12 points), the provenance baseline (`EventEnvelopeV1.actor: ActorRef` 5-field contract + CLI prefix-string mapping locked as v1.81.x contract), and the no-parallel-authority invariant. All code-level enforcement explicitly deferred to ARCH-HEX-001 (order 80), EVT-LEDGER-001 (order 90), and RX-SECRETARY-001/002 (order 300/310). Closes AC-HX-AUTH-01..06, REQ-AUTH-TAX-01..04, REQ-AUTH-WS-01..04, REQ-AUTH-AP-01..04, REQ-AUTH-PR-01..03, REQ-AUTH-NPA-01..04, REQ-AUTH-REC-01..02.
  - docs(adr): **ADR-0072-AMENDMENT-1** accepted — renames Secretary's runtime identity from the prose-level `secretary role` to `Agent{role=secretary, behavior_id, closed_set_version}` for taxonomy consistency with ADR-069. Does NOT alter ADR-0072's budget-composition decision.
  - docs(adr): **ADR-0073-AMENDMENT-1** accepted — binds Secretary's closed-set L1 admission predicate (prose-level) to `actor.kind == Agent && actor.role == "secretary"`. The 8 auto-resolvable event classes and the orchestrator-/runtime-exclusive prohibitions remain. Stage 1 enforcement deferred to RX-SECRETARY-001/002 + EVT-LEDGER-001.

### Tests
  - test(domain): new `crates/sddk-domain/tests/actor_authority_baseline_tests.rs` (8 tests) — regression baselines for current ActorKind closed-set (3 variants), ActorRef 5-field contract, CLI prefix-string mapping (`user:*` → Human, `agent:*` → Agent, fallback → System), and provenance loss in `LedgerEvent`, `GateReceipt`, `JournalEntry`. Locks current behavior so downstream ARCH-HEX-001 + EVT-LEDGER-001 cycles can flip it with confidence. Closes AC-HX-AUTH-01, AC-HX-AUTH-04, AC-HX-AUTH-07, REQ-AUTH-TAX-01..03, REQ-AUTH-PR-01..03, REQ-AUTH-TST-01..02.

### Documentation
  - docs(debt): **INC-HX-AUTH-001** (writable-state, P2, owner ARCH-HEX-001) — 8 mutable surfaces lack formal authority declaration.
  - docs(debt): **INC-HX-AUTH-002** (approval-authority, **P0/critical**, owner ARCH-HEX-001) — `event_bus/emit.rs:259` hardcodes `kind: ActorKind::Human` regardless of caller. The most critical current policy violation.
  - docs(debt): **INC-HX-AUTH-003** (provenance, P1, owner EVT-LEDGER-001) — `LedgerEvent`/`GateReceipt`/`JournalEntry`/`EventContext` carry `actor: String` instead of `ActorRef`.
  - docs(debt): **INC-HX-AUTH-004** (no-parallel-authority, P1, owners ARCH-HEX-001 + RX-SECRETARY-001/002 + EVT-LEDGER-001) — 7 dual-writer paths (approval, approval_request, cycle_transition, cycle_pause, gate_receipt, knowledge_ingest, secretary_closed_set).

Cycle path: A-min (governance-only). 3 commits en rango `27e37e7..078782d` (1 docs(adr) + 1 test(uat) + 1 docs(debt)). 8 REQs (TAX/WS/AP/PR/NPA/REC/TST/INC) / 24 Given-When-Then scenarios / 8 acceptance criteria covered. Verify verdict pending.

## [1.81.0] - 2026-09-04

### Added
  - feat(domain): `LedgerEventInput` now derives `Serialize`/`Deserialize` and round-trips byte-exactly via JSON (`crates/sddk-domain/src/models/ledger.rs`). Closes REQ-IRDT-RT-06 / AC-IRDT-06.
  - test(domain): `test_stability_roundtrip_serde_with_operator_contracts` exercises `NormalizedPlanV1` round-trip with a populated `operator_contracts` field (≥2 entries); asserts `plan_identity()` stability AND per-operator projection equality. Closes REQ-IRDT-RT-02 / AC-IRDT-02.
  - test(domain): `roundtrip_preserves_content_hash` in `event_envelope.rs` proves `EventEnvelopeV1` JSON round-trip preserves `compute_content_hash()`. Closes REQ-IRDT-RT-05 / AC-IRDT-05.
  - test(domain): `crates/sddk-domain/tests/invalid_plan_rejection.rs` (5 tests) asserts invalid plans are rejected with structured errors and zero panics — covers malformed JSON, missing required field, extra field under strict schema, empty lineage, and the documented NoOp mutation. Closes REQ-IRDT-IP-01, IP-02, IP-04 / AC-IRDT-11, AC-IRDT-13.
  - test(domain): `crates/sddk-domain/tests/hashmap_audit.rs` (CI guard) walks `crates/sddk-domain/src/*.rs` and asserts NO `HashMap` field appears in any IR canonical form. Closes REQ-IRDT-HS-05 / AC-IRDT-10.
  - test(domain): `crates/sddk-domain/tests/serde_json_feature_guard.rs` verifies `serde_json` is built WITHOUT `preserve_order` / `indexmap` features. Closes REQ-IRDT-HS-03 / AC-IRDT-09.
  - test(domain): `schema_dialect_unknown_serializes_to_valid_json` in `operator_contract_error_tests.rs` constructs `OperatorContractError::SchemaDialectUnknown { dialect: ... }` directly and proves its serialization safety. Closes FIND-000005 carryover / REQ-IRDT-IP-03 / AC-IRDT-12.
  - test(engine): new `crates/sddk-engine/tests/build_operator_ir_permutation_equivalence.rs` (3 tests) — load-bearing for the IR→runtime frontier: (1) two IRs with identical `compute_content_hash()` but different `BTreeMap` insertion order produce structurally equivalent runtime trees (structural comparison via downcast + debug-format); (2) `Sequence` runtime children preserve IR declaration order; (3) `Choice` branches iteration is BTreeMap sorted-key order. Closes REQ-IRDT-HS-04, REQ-IRDT-DC-02..DC-04 / AC-IRDT-08, AC-IRDT-14..AC-IRDT-16.
  - test(engine): `build_operator_eval_failed_for_missing_operator_id` (extension to `build_operator_tests.rs`) — proves `build_operator` rejects malformed IR with structured `OperatorError::EvalFailed` error rather than panicking. Closes REQ-IRDT-IP-02.
  - docs: ADR-068-DETERMINISTIC-IR-RUNTIME-BOUNDARY accepted — establishes the IR→runtime `build_operator()` frontier at `crates/sddk-engine/src/operator.rs::build_operator` (line 1726) as the canonical second "compiler boundary" of the H0 determinism program. Documents the 4-invariant exit gate, the testable surfaces, and the carryover close-out from DW-IR-004 (FIND-000005) plus deferrals (FIND-000001, FIND-000002 to H6/DW-OPERATORS-001).

### Tests
  - test(workspace): 109 new tests pass across `sddk-domain` (lib + 6 new integration files) and `sddk-engine` (1 new integration file). 19 REQs / 22 Given-When-Then scenarios / 16 acceptance criteria covered. Cycle path: A-min. Verify verdict pending.

Cycle path: A-min. 5 commits en rango `ee6a5e2..2c9770e` (1 docs(adr) + 1 feat(domain) + 1 test(domain) + 1 test(engine) + 1 chore(release) bump).

## [1.80.0] - 2026-09-04

### Added
  - feat(domain): new `operator_contract` module (`crates/sddk-domain/src/operator_contract.rs`) with typed I/O contracts for all 12 Operator variants. `OperatorInputSchema` and `OperatorOutputSchema` replace the v1.29.0 untyped placeholder `outputs["items"]: serde_json::Value::Array`. `SchemaDialect` closed enum (JsonSchemaDraft07). `OperatorContractError` closed enum with exactly 8 variants. `OPERATOR_CONTRACT_SCHEMA_VERSION = 1`. 15 domain lib tests pass.
  - feat(domain): `NormalizedPlanV1` now carries `operator_contracts: BTreeMap<OperatorId, OperatorContractProjectionV1>` — the canonical map from each operator to its typed I/O contract. Populated by `from_workflow_ir` via `default_input_schema`/`default_output_schema`.
  - feat(engine): `Map::aggregate_collect_all` now produces `item_results: [{operator_id, outputs}, ...]` instead of the old `results`/`failures` split. `Map` struct gains `body_id: OperatorId` field. `validate_output` method validates Map output against default `OperatorOutputSchema`. All 31 map tests pass.
  - test(engine): new `crates/sddk-engine/tests/operator_contract_tests.rs` with 9 tests covering REQ-OPTEST-001 grep CI guard, per-variant schema coverage, and output validation.
  - test(domain): new `crates/sddk-domain/tests/operator_contract_schema_tests.rs` (10 tests), `operator_contract_error_tests.rs` (9 tests), `operator_contract_lineage_tests.rs` (7 tests) covering schema structural properties, error variant exhaustiveness, and variant-to-schema totality.
  - docs: ADR-067-TYPED-OPERATOR-IO accepted with Decisions 1-4 covering dialect selection, schema structure, Map output redesign, and error taxonomy.

## [1.69.0] - 2026-09-02

### Added
  - feat(domain): `StorageError::recovery()` ahora devuelve hints accionables keyed by entity — `cycle` (`sddk cycle start --scope .` / `sddk cycle rebuild --cycle <id>`), `gate receipt` (`sddk cycle evaluate-gate`), catch-all (`ensure the record exists`), y `LeaseConflict` (`sddk cycle lock inspect --cycle <id>` / `sddk cycle lock release --cycle <id>` con cycle_id rellenado). Generaliza GAP-UX-1 (v1.66.6 / cycle-51) más allá de `EngineError`. Audit grep `create the record|fix the reference` en `crates/sddk-cli/src/` → 0 hits.
  - feat(engine): `EngineError::recovery()` cita la invocación exacta `sddk cycle evaluate-gate --gate <name> --transition <id>` con parámetros rellenados desde el error context (gate name, transition id, receipt id). 26 variantes actualizadas en `crates/sddk-engine/src/lib.rs:1682`.
  - feat(cli): `FrontierEntryOutput` JSON gains `from_phase` / `to_phase` / `closes_cycle` surfaced desde `FrontierEntry.from.phase` / `FrontierEntry.to.phase` / `FrontierEntry.to.status` (OVG-02/03 del debt report de v1.68.0). Agents que leen `sddk cycle next --format json` ahora reciben la fase source/target del cycle y un boolean `closes_cycle`.
  - refactor(cli): `swap_current_to(framework_dir, target)` extraído a `crates/sddk-cli/src/dev/mod.rs:47` — colapsa 3 duplication sites (update.rs:130 post-prune, update.rs:279 dev-link, use_cmd.rs:59 install path). Atomicity y dev-link semantics sin cambios.

### Fixed
  - fix(cli): WARN-001 leak cerrado — `validate_cycle_project` ahora construye `sddk_domain::StorageError::NotFound { entity: "cycle", .. }` para cycle-ids malformados en lugar del `sddk_storage::StorageError::NotFound` genérico. 4 call sites affected (`cycle lock acquire` / `renew` / `release` / `status`). Commit `9d7f548` revierte solo la rama `CycleProjectMismatch` (preserva el error code `STORAGE_CYCLE_PROJECT_MISMATCH` que 4 integration tests assertean); la migración de `NotFound` se preserva.
  - chore(cli): prefijo `INC-DEBT-020:` removido de 4 production comments en `crates/sddk-cli/src/dev/update.rs` (la INC se cerró en v1.68.0). Las 3 referencias restantes viven dentro de `#[cfg(test)]` blocks.

### Tests
  - test(domain): nuevo test `gate_receipt_projection_parity_across_impls` en `crates/sddk-domain/src/ports.rs` assertea `GATE_RECEIPT_FIELD_COUNT = 14` across las 3 Ledger impls (Storage SQLite, InMemoryLedger testkit, FakeLedger engine stub). Drift en el SELECT de 13/14 columnas ahora lo caza `cargo test`.
  - test(cli): `swap_current_to_dev_link_mode` + `swap_current_to_version_dir` cubren las dos ramas del helper.
  - test(cli): `s_dev_link_preserved_without_prune_flags_keeps_dev_link_target` upgraded a ejercitar el helper real end-to-end (cierra la tautología `S-DEV-LINK-PRESERVED` cycle-53 PARTIAL).
  - test(cli): `cycle_next_json_output_has_stable_shape` actualizado con los 3 nuevos fields (`from_phase` / `to_phase` / `closes_cycle`).
  - test(workspace): `cargo test --workspace --locked` 1781 tests passed, 0 failed.

### Documentation
  - docs(release): notas v1.69.0 (`docs/releases/v1.69.0.md`) documentan la generalización de GAP-UX-1, los nuevos fields de `FrontierEntryOutput`, la refactorización de `swap_current_to`, y la prueba live stderr del WARN-001 leak closure.
  - docs(release): `docs/RELEASING.md` sin cambios — el flujo canónico de 13 pasos sigue verde tras cycle-54.

Cycle path: B-direct. 6 commits en rango `8789f05..9d7f548` (4 feat/refactor/fix + 2 in-cycle corrections: WARN-001 closure y CycleProjectMismatch revert). Verify verdict: `PASS_WITH_WARNINGS` (WARN-001 closed in-cycle; sin debt evidence gate porque el path es B-direct).

## [1.68.0] - 2026-09-02

### Added
  - feat(cli): nuevo subcomando `sddk cycle next` — frontier advisor que deriva la frontera del ciclo desde el grafo de transiciones DECLARADAS en `workflow.yaml`. Salida humana (lista de transiciones con `requires_met` y hints) o `--json` (envelope `{cycle, node, phase, frontier: [{transition_id, to, requires_met, unmet_gates, unmet_requirements, hint}]}`). Cierra slice 2 del Epic SD. La proof D1 (zero hardcoded sequences) la demuestra el diamond fixture test (`frontier_for_state_diamond_topology`, engine/lib.rs:2050-2080): si `frontier_for_state` hardcodease el path A-min, el diamond de 4 transiciones con 2 ramas paralelas fallaría al exigir 2 entradas con ambos ids.
  - feat(engine): `frontier_for_state(workflow, state, cycle_id, ledger)` como derivación pura. Estado siempre viene de `engine.replay_cycle` → `replay_state` (nunca de artefactos pre-calculados).
  - feat(storage): `Ledger::list_gate_receipts_for(cycle_id, transition_id, plan_hash)` (trait method nuevo, 3 impls: `Storage` SQLite con SELECT 13-columnas, `InMemoryLedger` testkit, `FakeLedger` engine stub). Soporta el path de "blocked transitions" del advisor sin tocar la autoridad de escritura.

### Fixed
  - fix(cli): `sddk dev update --prune-only --keep N` y `--prune` ahora re-apuntan el symlink `current` a la versión más nueva retenida (INC-DEBT-020). Antes, el prune dejaba el symlink colgando a una versión ya eliminada y `sddk dev doctor` fallaba con `binary.bundle_coherence: ABSENT` aunque la versión actual del bundle estuviera intacta. Nuevo helper `repoint_current_to_newest(framework_dir, newest_version)` en `update.rs:487` ejecutado desde ambas ramas prune (update.rs:315, 344). Modo dev-link sin flags de prune preserva su comportamiento previo (guard `!prune && !prune_only` en update.rs:282).
  - fix(engine): `frontier_for_state` clasifica `Requirement::Structured { kind }` con kind desconocido como `unmet_requirements` (cierra OVG-01 HIGH del debt report). Antes: catch-all silenciaba como satisfecho divergiendo de `evaluate_plan` y `apply_transition` que rechazan; ahora: convergencia semántica entre advisor (`cycle next`) y autoridad (`cycle transition`).

### Tests
  - test(engine): 8 tests `cargo test -p sddk-engine frontier` cubren diamond topology, terminal cycle, gate satisfied, missing gate, replay derivation, unknown kind (OVG-01), y state derivation from ledger replay. Diamond fixture test prueba D1 binding.
  - test(cli): 33 tests `cargo test -p sddk-cli --lib cycle` cubren S-NEXT-COMMAND, S-NEXT-JSON, S-NEXT-GATES, S-NEXT-TERMINAL, S-NEXT-NO-WORKFLOW, S-NEXT-INFERENCE, zero-arg inference con lease único.
  - test(cli): 9 tests `cargo test -p sddk-cli --lib dev::update` cubren las 3 ramas de prune (PRUNE-ONLY-REPONTS, modo normal preserve, dev-link sin prune preserve) más 2 regresiones del helper (`repoint_current_skips_nonexistent_version_dir`, `repoint_current_to_newest_symlink_swap_is_atomic`).
  - test(workspace): `cargo test --workspace --locked` 1781 tests passed (1 flaky infra-noise `dev::rdi_tests` bajo paralelismo, pasa en `--test-threads=1`).

### Documentation
  - docs(release): notas v1.68.0 (`docs/releases/v1.68.0.md`) documentan cycle next advisor + INC-DEBT-020 fix + transparencia verify FAIL→corrección→gate-closure.
  - docs(release): `docs/RELEASING.md` step 10 (Prune) y step 11 (Final state) añadidos en cycle-53; el fix INC-DEBT-020 hace que step 10 ya no requiera el "manual fix" histórico.
  - docs(agents): `AGENTS.md §5` checklist añade item de prune post-release con referencia al fix.

Cycle path: A-min (smoke depth). 11 commits en rango `2aa5e36..669d7dc` (8 feat/fix + 1 corrección clippy + 1 docs + 1 in-cycle OVG-01 remediation). Verify verdict: PASS_WITH_WARNINGS (1 PARTIAL `S-DEV-LINK-PRESERVED` tautológica documentada con plan de cycle-54). Debt verdict: PASS_WITH_WARNINGS (1 HIGH OVG-01 remediado in-cycle, 5 backlog follow-ups con fingerprints).

## [1.67.0] - 2026-09-02

### Added
  - feat(cycle): inferencia de contexto para subcomandos de `sddk cycle`. Cuando el estado es inequívoco (marcador de proyecto + único lease activo), los comandos `status`, `transition`, `rebuild`, `artifacts-dir`, `lock acquire/renew/release`, `supersede`, `replan`, `lock-status`, `evaluate-gate` ya no requieren repetir `--root/--scope/--cycle`. Resolver único: `resolve_cycle_context` en `crates/sddk-cli/src/cycle.rs:140` con 11 call sites. Cierra slice 1 del Epic SD (state-driven CLI).
  - feat(domain): nuevo método de trait `Ledger::list_active_cycle_leases_for_project` (read-only SQL) implementado en `Storage`, `InMemoryLedger` (testkit), `FakeLedger` (engine). Soporta el path de inferencia sin tocar la autoridad de escritura.
  - feat(cli): nuevo opt-out `--no-infer` para automatización y tests que prefieren la forma "siempre explícita". Sin args explícitos + `--no-infer` → `InferenceError::ExplicitRequired` listando `--root/--scope/--cycle`.
  - feat(cli): errores tipados de inferencia (`InferenceError::{NoProjectContext, ExplicitRequired, NoActiveCycle, AmbiguousCycle}`) con hints concretos: candidatos con cycle_ids + ready-to-paste `sddk cycle status --cycle <id>` para ambigüedad; hint de `sddk cycle start` para "no active lease"; hint a `sddk project resolve` / `sddk init` para "no project context".

### Changed
  - refactor(cli): `RuntimeArgs.root: PathBuf` → `Option<PathBuf>` y `RuntimeArgs.scope: String` → `Option<String>` (deferred defaults via `unwrap_or(Path::new("."))` / `unwrap_or(".")` en `RuntimeContext::open`). Compatibilidad hacia atrás con callers pre-inferencia (ledger.rs, plan.rs, recover.rs, run.rs, ship.rs, status.rs, vault_cmd.rs).

### Tests
  - test(cli): 9 tests en `crates/sddk-cli/src/cycle.rs` cubren S1, S2a, S2b, S3, S3b, S4, S5a, S5b, S6. Verificados por `cargo test -p sddk-cli --lib cycle` (9/9 pass) y `cargo test --workspace --locked` (593 passed; 1 pre-existing infra-noise fail bajo paralelismo, pasa en solitario).

### Documentation
  - docs(release): notas v1.67.0 en `docs/releases/v1.67.0.md` documentan el slice 1 del Epic SD con escenarios S1-S7, live smoke de `sddk cycle status` zero-arg, evidencia de verify, y limitaciones conocidas.
  - docs(roadmap): referencia a `docs/history/legacy-packages/sddk-decision-kernel-architecture/02-roadmap/RESEARCH/state-driven-cli/RESEARCH.md` como fuente canónica del Epic SD.

Cycle path: B-direct (cycle-family + storage + testkit + domain-trait + propagación mecánica a 7 callers + 9 tests + release notes). 4 commits en rango `fc5d7b3..a34c882` (e24598a feat, a34c882 test) + bump 1.67.0. Verify verdict: PASS_WITH_WARNINGS (1 pre-existing flaky infra-noise test + 1 missing runtime cycle record — filesystem-artifacts-only closure; `archive-manifest.md` es ground-truth durable del cierre, igual que en cycle-51).

## [1.66.6] - 2026-09-02

### Fixed
  - fix(engine): `cycle_supersede` libera el lease atómicamente en la misma transacción que `cycle.supersede.applied` (antes: `release_lease_on_phase_change=false`). Cierra GAP-BUG-1 (P2/medium).
  - fix(engine): `supersede-receipt.json` ahora escribe `lease_owner` y `fencing_token` desde los argumentos del caller (antes: usaba `actor` y hardcodeaba `0`). Cierra GAP-BUG-2 y GAP-BUG-3 (P2/medium).
  - fix(cli): `cycle supersede` llama `validate_cycle_project` antes de cualquier acceso a storage, convirtiendo `STORAGE_NOT_FOUND` para bare slugs en un error tipado apuntando a la forma canónica `<project_id>/<slug>`. Cierra GAP-UX-1 (P3/low).

### Added
  - feat(domain): `Ledger::cycle_exists` para validar existencia de successor antes de cualquier mutación de estado (patrón INC-DEBT-017). Cierra GAP-V-2 (P2/medium).
  - feat(cli): clap `#[arg(conflicts_with)]` en `--successor` para que la combinación `--successor` + `--reason` sea rechazada en parse-time. Cierra GAP-V-1 (P3/low).
  - feat(engine): rechazo de `--evidence-refs "[]"` vacío antes de cualquier mutación de estado (SPEC §2 línea 87: MUST). Cierra GAP-V-3 (P2/medium).

### Tests
  - test(engine): 6 nuevos tests en `crates/sddk-engine/tests/cycle_supersede.rs` cubren los gaps BUG-1/2/3, UX-1, V-2/3, y la invariante de digest del ledger (`supersede_releases_lease_atomically`, `supersede_receipt_lease_owner_matches_caller`, `supersede_receipt_fencing_token_matches_caller`, `supersede_rejects_nonexistent_successor`, `supersede_rejects_empty_evidence_refs`, `supersede_preserves_ledger_event_hashes`).
  - test(cli): 1 nuevo test en `crates/sddk-cli/tests/first_class_commands.rs` cubre el rechazo de `--successor + --reason` en parse-time (`supersede_cli_conflicts_with_rejected_at_parse_time`).

### Documentation
  - docs(adr): `ADR-0079-cycle-supersede` flip status → `accepted`. Cierra GAP-DOC-1.a.
  - docs(spec): `SPEC-SUPERSEDE-001` promovido a `docs/history/legacy-packages/sddk-decision-kernel-architecture/04-specs/`. Cierra GAP-DOC-1.b.
  - docs(agents): `AGENTS.md §9` cycle supersede workflow añadido. Cierra GAP-DOC-1.c.
  - docs(release): notas de release para v1.66.6 (`docs/releases/v1.66.6.md`) documentan los 11 gaps cerrados. Cierra GAP-DOC-1.d.

Cycle path: A-full (engine + CLI + domain feat + 7 tests + 4 docs promotions + release notes). 7 commits en rango `b4ea945..a797e09`. Verify verdict: PASS_WITH_WARNINGS. Debt verdict: PASS_WITH_WARNINGS (2 pre-existing MEDIUM findings F-1 time-coupling + F-2 N+3-vs-N+2 spec drift, deferred a cycle-52; sin nuevas findings).

## [1.66.5] - 2026-09-02

### Documentation
  - docs(agents): corrige la narrativa de `AGENTS.md §8` sobre el cierre formal del ciclo. La nota que decía "actualmente roto" se elimina; el cierre CLI ya es operativo desde v1.66.1 (`validate_cycle_project`) y v1.66.2 (`Storage::cycle_exists`, INC-DEBT-017). Preserva el rol durable de `archive-manifest.md` como ground-truth del cierre (líneas 208 + 213).
  - docs(roadmap): cierra el GAP-6 dentro de `docs/history/legacy-packages/sddk-decision-kernel-architecture/02-roadmap/ROADMAP.md` bajo un heading `### GAP-6 — Closed by v1.66.1 + v1.66.2 (cycle-50 bis not needed)`. El texto original queda preservado dentro de un blockquote `> **Original (archived for audit):**` para audit trail. Rationale del cycle-57 + INC-DEBT-017 nombrados.
  - docs(backlog): añade un candidato BSG (`## Candidate BSG — CLI bare-slug cycle-id acceptance (deferred)`) en `docs/history/legacy-packages/sddk-decision-kernel-architecture/02-roadmap/BACKLOG.md`, posicionado tras el bloque `Out of scope (v1)` del Epic LF. Owner: orchestrator. Priority: P3. Incluye referencia sha256 al exploration-report para audit. Symptoma vivo documentado en el F4 gotcha de `AGENTS.md §8`.
  - docs(roadmap): añade el Epic LF (Ledger Forensic) + candidatos cycle-55/56 (`pausa de ciclo` y `backlog como objetos del ledger`) a `ROADMAP.md`. Documenta la pausa de ciclo como objeto del ledger y el backlog como fuente viva de candidatos. 0 Rust, 0 Cargo, 0 tests; solo prosa.
  - docs(debt): flip de frontmatter `status: open → resolved` + `resolved_by` + `## Closure Evidence` para los 2 INC carry-over de v1.66.3 (`INC-CYCLE-13-APPLY-TEST-COUNT-MISREPORT`, `INC-CYCLE-13-LOC-OVERAGE`) que ya habían sido resueltos por cycle-13. Coherencia con el contrato de cierres INC definido en `docs/debt/INC-TEMPLATE.md`. Commit `bb263bd`.

Cycle path: A-min (docs-delivery). Diff composition: 3 markdown files / +34 / -7 (rama `fix/gap6-cycle-lock-repair`) — no Rust, no Cargo, no tests, no prompts/skills. Verify verdict: PASS_WITH_WARNINGS (W1 cosmetic, W2 release-time gate). Debt verdict: PASS (zero findings). Sin cambios en runtime, binario, bundle, fixtures ni public APIs.

## [1.66.4] - 2026-09-02

### Refactored
  - refactor(domain): extrae `build_corpus_envelope(seq, event_type, payload)` a un helper dentro de `#[cfg(test)] mod tests` (`crates/sddk-domain/src/event_registry/validator.rs`). Los 28 lines de construcción inline de `EventEnvelopeV1` colapsan a 1 call site; el helper aplica `+1` internamente para preservar el contrato anti-double-increment; byte-equivalence verificada en `corpus_replay_through_validator` (evt-corpus-1..18, sequence 1..18). Cierra `INC-CYCLE-14-CORPUS-FIXTURE-DUPLICATION` (LOW/P3, duplication cluster).

### Documentation
  - docs(rustdoc): `severity_for_event_type` en `crates/sddk-domain/src/projections/journal.rs` gana una sección `# Severity policy cross-reference` que explica la consolidación de 7 filas vs 8 categorías (evidence.* + uat.*) + exclusión pack/runtime. La tabla locked de 7 ramas (líneas 67-93) queda intacta; el test `journal_projection_severity_table_locked` sigue verde. Cierra `INC-CYCLE-14-SEVERITY-SPEC-DRIFT` (LOW/P3, coupling cluster, SPEC-027).
  - docs(rustdoc): 3 helpers de `crates/sddk-engine/src/event_bus/correlation.rs` (`with_correlation_from_context`, `with_causation`, `trace_causation_chain`) ganan una sección rustdoc `# Production wiring` que documenta el diferido a M6 SPEC-028. 0 callers de producción (solo el re-export en `event_bus/mod.rs:11`); 6 tests de los helpers pasan. Cierra `INC-CYCLE-14-HELPER-DOC-GAP` (LOW/P3, doc-quality cluster).
  - docs(rustdoc): `crates/sddk-engine/tests/adoption.rs` líneas 44-46 — el comentario impreciso `// durability-required:` se reformula a `// File-based for fixture consistency`. El cuerpo del test (líneas 48-61) queda inalterado; `same_basename_different_remotes_and_scopes_do_not_collide` sigue verde. Cierra `INC-CYCLE-13-DURABILITY-COMMENT-ACCURACY` (LOW/P3, doc-quality cluster).
  - docs(debt): housekeeping — flip de frontmatter `status: open → resolved` + `resolved_by` + `## Closure Evidence` para los 2 INC carry-over de v1.66.3 (`INC-CYCLE-13-APPLY-TEST-COUNT-MISREPORT`, `INC-CYCLE-13-LOC-OVERAGE`); commit `0d6dda4`. Coherencia con el contrato de cierres INC definido en `docs/debt/INC-TEMPLATE.md`.
  - docs(release): notas post-release para v1.66.4 (`docs/releases/v1.66.4.md`) mencionan los 4 INC ids resueltos en código + 2 INC carry-over housekeeping + la estrategia de release.

## [1.66.3] - 2026-09-01

### Fixed
  - fix(cli): `sddk dev test count-workspace` ahora parsea el output de texto de `cargo test` (regex `^test result:\s+ok\.\s+(\d+)\s+passed`) en lugar de los eventos JSON que `cargo 1.91 --message-format=json --no-run` no emite. Reemplaza el parser roto `CargoMessage`/`CargoTarget`/`CargoTestResult` con una función pura `parse_cargo_test_output` y 8 unit tests (empty, dedup, FAILED-excluded, leading whitespace, ground-truth 1739-line scale, multi-binary, single-binary, no-running-lines). Live binary reporta correctamente `total_workspace_tests: 1747 / test_binaries: 79` (antes: 0). Cierra `INC-CYCLE-13-APPLY-TEST-COUNT-MISREPORT` (P2/medium).

### Refactored
  - refactor(engine): extrae `mk_*` builders de `crates/sddk-engine/tests/port_contracts.rs` a `crates/sddk-engine/tests/common/` para que el archivo de contratos no acumule LOC adicional cuando se añadan escenarios futuros. ADR-0048 supersede el budget per-file por budget total-de-módulo.

### Documentation
  - docs(apply): `apply-progress.yaml` ahora exige `sddk dev test count-workspace` como fuente única de `total_workspace_tests` (prohibido recalcular o estimar). Coherencia byte-for-byte entre apply report y live binary stdout.
  - docs(debt): cierra `INC-DEBT-017` — el helper storage-layer `cycle_exists` + 4 pre-checks en acquire/renew/release/status (`fix/storage-cycle-lease-pre-existence-check`, v1.66.2) eliminan el drift que `fix/gap6-foreign-cycle-typed-error` (v1.66.1) dejaba fuera de scope. Contrato completo de errores tipados para `sddk cycle lock`: `foreign project → STORAGE_CYCLE_PROJECT_MISMATCH`, `own-project-missing → STORAGE_NOT_FOUND` (no más FK leak engañoso con `STORAGE_DATABASE`).

## [1.63.0] - 2026-08-31

### Added
  - feat(uat): artefacto unificado `sddk-<TAG>-<ASSET>.tar.gz` por release (nuevo job `unified-artifact` en `release.yml` que combina binario + bundle + BUNDLE.toml + manifest_sha256). `scripts/install.sh` lo prefiere cuando existe; cae al path legacy (binario + bundle por separado) si no.
  - feat(uat): schema `BUNDLE.toml` v2 (`schema_version`, `bundle.version`, `bundle.binary_min_version`, `bundle.binary_max_version`, `contents.manifest_sha256`) en `sddk-cli` (`crate::dev::bundle_manifest`). 6 tests nuevos: round-trip, exact-match, rejects-older, accepts-range, missing-file, unsupported-schema. Verifica compat semver-aware (pre-release `-rc.N` rankea bajo).
  - feat(uat): `InstallReceipt` extendido a `schema_version = 2` con `bundle_version`, `bundle_sha256`, `bundle_path`, `coherence_checked`. `sddk dev install --source` lo escribe tras verificar `BUNDLE.toml` contra el binario (fail-closed: nunca escribe receipt parcialmente).
  - feat(uat): doctor check `binary.bundle_coherence` v2 (3 condiciones: `receipt.version == CARGO_PKG_VERSION`, `bundle_version` matchea dir activo o BUNDLE.toml version, `verify_bundle_compat` dentro del rango declarado). `sddk dev doctor --prefix <P>` permite layout split-prefix (binario y bundle en directorios distintos).
  - feat(uat): `sddk dev manifest --bundle` regenera `BUNDLE.toml` con `manifest_sha256` del MANIFEST presente.
  - feat(install): `scripts/install.sh` rediseñado atómico (stage → apply → rollback on failure, trap cleanup de TMP_DIR). Detecta el tarball unificado cuando existe; legacy split-asset path preserva compatibilidad.

### Fixed
  - fix(install): INSTALL_BIN layout rustup-aware (`$PREFIX/bin/sddk` por defecto, `$PREFIX/sddk` cuando prefix termina en `/bin`). Antes asumía siempre el segundo y fallaba en el layout moderno.
  - fix(install): download_optional() ahora soporta URLs `file://` (testing local contra mirror).
  - fix(uat): 3 tests existentes (`uninstall_removes_prefix_and_editor_symlinks`, `verify_detects_tampered_bundle`, `cli_dev_install_accepts_committed_manifest`) actualizados para incluir `BUNDLE.toml` en sus fixtures (helper `write_test_bundle_manifest`) — falla esperada por la preflight v2 que rechaza installs sin BUNDLE.toml coherente.

## [1.62.0] - 2026-08-31

### Added
  - feat(workflow): transición `phase.build.remediate` (`REMEDIATING/build` → `OPEN/build` con gate `remediation-complete`) espejada de `phase.verify.remediate` para evitar deadlocks cuando `release.recover` mueve un ciclo a `REMEDIATING/build`. Cubre el gap identificado en [[ADR-0077]].

### Fixed
  - fix(uat): test `cli_phase_build_remediate_rejects_wrong_phase` marcado `#[ignore]` con motivo documentado (el workflow no expone ninguna transición hacia `REMEDIATING/verify`, por lo que el setup del test no puede ejercitar el rechazo bajo prueba). Seguimiento en cycle-45.

## [1.61.0] - 2026-08-30

### Added
  - feat(release): ruta de recuperación fail-closed desde RELEASE_PENDING (transición `release.recover` RELEASE_PENDING/release → REMEDIATING/build; nuevo artefacto `release-failure-evidence`; gate explícita `release-recovery-authorized`; fail-closed e idempotente).
  - docs(sddk): documento de investigación sobre la fase review huérfana en A-full (discrepancia R0–R6 entre `workflow.yaml` y el prompt layer; recomendación Option 1: remover `Phase::Review` del runtime).

## [1.60.0] - 2026-08-30

### Added
  - feat(spec): SPEC-042-secretary-runtime (Stage 0, docs-only) con §Substrate dependency verbatim de SPEC-028 §Contract; gate de promoción `SPEC-028-promoted` cierra Stage 1+ hasta Built.
  - feat(docs): epic `SECRETARY-A` en BACKLOG + amendment `ROADMAP §Phase 6` con priorización comparativa; Stage 1 marcado `proposed / blocked-by-SPEC-028-Built`.
  - feat(release): implementa `release-revalidation` para ciclos `RELEASE_PENDING` (recovery canónico tras push fallido por pre-push hook: rerun verify+debt-verify contra la candidate SHA, validar SHAs, persistir `release-revalidation-<sha>.json` con sha256 sidecar; permite transición `release.complete` sin re-ejecutar el ciclo completo).

### Fixed
  - fix(release): corrige invariants de release-revalidation (atomicidad del sidecar, validación de schema_version, rechazo si verdict≠passed).
  - fix(release): usa comando `local --locked` en vez de `--release` para verify (paridad con el gate duro del CI local).
  - fix(docs): corrige defectos RDI del ciclo `p-52b95ef55999f9de` (gate term + wikilinks + lesson).
  - fix(clippy): reemplaza `vec!` por arrays en regression tests.

### Changed
  - docs(adr): ADR-0072-secretary-budgets (compone ADR-0068+0070) + ADR-0073-secretary-authority (closed-set L1 = 8 event classes + Receipt rule + autoridad prohibida `release/gate/lease/receipt`).

### Tests
  - test(release): añade cobertura para release-revalidation.
  - test(release): añade tests de propiedades (b)(d) y dispatch de revalidate.

## [1.59.1] - 2026-08-30

### Fixed
  - fix(release): corrige manifiesto distribuido (refresh de 2 stale digests en MANIFEST.sha256 para `prompts/sddk/phases/archive.md` y `skills/_shared/cli-usage-contract.md`; nuevo regression test `cli_dev_install_accepts_committed_manifest`; instrucción de preflight en `docs/RELEASING.md`).

## [1.59.0] - 2026-08-30

### Added
  - feat(uat): implement release-distribution-integrity for cycle p-52b95ef55999f9de
  - feat(uat): harden release-distribution-integrity para cycle p-52b95ef55999f9de

### Fixed
  - fix(uat): corregir defectos RDI del ciclo verify
  - fix(rdi): usar source explícito de DistArgs para verificación y staging
  - fix(rdi): usar create_bundle_without_manifest para el test install_fails_on_absent_manifest_source

## [1.58.5] - 2026-08-29

### Other
  - feat(uat): agrega gate de ShellCheck y documenta alcance nulo de Ruff
  - fix(uat): corregir gate ShellCheck y cobertura de verify A-lite p-52b95ef
  - fix(shellcheck): limpia violations pre-existentes en 5 scripts cubiertos por el gate
  - fix(backlog): cierra bullets Phase C #1/#2/#3 y actualiza descripcion del gate fail-hard

## [1.58.4] - 2026-08-29

### Fixes
  - fix(archive): fija timestamp de cierre (`prompts/sddk/phases/archive.md` post-transition `updated_at` becomes manifest `closed_at`; placeholder rejection enforced)

## [1.58.2] - 2026-08-29

### Fixes
  - fix(archive): fija evidencia final de ledger

## [1.58.1] - 2026-08-29

### Other
  - docs(roadmap): reprioriza cierre de test-tooling

## [1.57.0] - 2026-08-28

### Features
  - feat(hooks): pre-push que exige commit de release para main

### Other
  - chore(debt): cierra INC push con prevencion mecanica probada

## [1.56.0] - 2026-08-28

### Features
  - feat(ci): wire sddk lint into just ci as local gate
  - feat(lint): SDDK023-SDDK027 diagnostics with fixture tests

### Other
  - chore(debt): registra INC por 4a violacion de push en apply
  - style(lint): aplica cargo fmt
  - chore(uat): parity-gated deletions of 7 shell files

## [1.55.0] - 2026-08-28

### Features
  - feat(lint): add SDDK020/021/022 lint diagnostics with fixture tests
  - feat(sddk-testkit): add CliSandbox builder with XDG env isolation

### Other
  - chore(uat): retira 6 tests shell con paridad y rewirea registros
  - style: fmt fixes from Train 2 implementation
  - test(sddk-cli): port first_class_help substring to Rust
  - test(sddk-cli): port cycle_inventory_contract to Rust

## [1.54.0] - 2026-08-28

### Features
  - feat(vault): reconcile wikilinks VAULT003 and materialise 4 vault nodes
  - feat(lint): add SDDK015-SDDK019 diagnostics for instruction-layer contracts

### Fixes
  - fix(lint): endurece diagnósticos de matriz con parser lineal y añade tests SDDK015-019
  - fix(uat): repara 27 violaciones de lint pre-existentes y registra facades en permissions

### Other
  - chore(uat): delete parity-proven shell wrappers and rewrite references

## [1.53.0] - 2026-08-28

### Features
  - feat(uat): residual closure — 6 items implementados

### Fixes
  - fix(uat): registra facades en agent-models y corrige argv de sddk-plan

## [1.52.0] - 2026-08-28

### Features
  - feat(uat): apply-push hardening — binding NO-PUSH contract + drift check + test gates

### Fixes
  - fix(ci): tabula receta just y registra test JS en fixtures

### Other
  - chore(debt): cierra INC apply-push con evidencia de verificación
  - test(workflow): actualiza expectativa del Step 1.7 al marcador advisory

## [1.51.0] - 2026-08-28

### Features
  - feat(uat): instruction-layer contract matrix and sizing advisory routing

### Other
  - docs(roadmap): publica ADR-0069/ADR-042 aceptados, INC sin número inventado y reprioriza roadmap

## [Unreleased] — cycle-43

### Fixed
- cycle-43 INC-DEBT-016: dm02 hang resolved via two-part fix. (1) `spawn_pending_and_ready` now matches `NodeRunState::Running` so Sequence intermediate state is re-evaluated. (2) `Sequence::evaluate` pushes a marker Attempt to `ctx.node_run.attempts` after each child so `completed_steps` advances. dm02_execute_completes_all_nodes: EXIT 124 → EXIT 0 (0.00s). dm02_stress_harness: 0/3 → 3/3 PASS. Workspace tests: BLOCKED → 1419 passed.

## [Unreleased] — cycle-41

### Fixed
- cycle-41 INC-DEBT-015: 36 unique sddk-engine clippy warnings reduced to 0 (~70 resolved). Fixed bogus clippy::missing_docs lint name (T1), applied machine clippy fixes across lib + tests (T2), suppressed needless_range_loop where clippy suggestion would change semantics (T3).

## [Unreleased] — cycle-40

### Fixed
- cycle-40 INC-DEBT-014: 85 unique sddk-engine clippy warnings reduced to 36 (~49 resolved). Deleted 17 unused test helpers + structs (T1), removed 28 unused imports (T2), resolved 5 Arc not Send+Sync in test helpers (T3), applied 3 derivable impls (T4), annotated 17 missing-docs (T5).

## [Unreleased] — cycle-39

### Fixed
- cycle-39 INC-DEBT-013 reqwest client cache drift closed at v1.48.7.

## [Unreleased] — cycle-38

### Fixed
- W1 (INC-DEBT-012): `resolve_alias_for` helper (reconcile.rs:258-281) was extracted in cycle-37 T2 but never wired — now called by all 3 adapters (json/claude/codex). Clippy warning resolved.
- F1 (INC-DEBT-012): `ParsedAgentForTest` had 3 unused fields (`description`, `tools`, `body`) — trimmed to 1 field (`aliases`). Clippy warning resolved. Spec correction: post-trim is 1 field, not 2; `name` is read from filename stem externally.

### Added
- `resolve_alias_for_first_match_wins`: direct unit test for `resolve_alias_for` helper with 3 sub-cases (no-match, canonical-only, alias-match). Anti-tautology: proves helper logic independently of adapter call sites.

## [Unreleased] — cycle-37

### Added
- Per-file frontmatter `aliases:` field parsed by `load_agent_sources` in all 3 adapter families (json/claude/codex). Closes INC-DEBT-011.
- `renames_builder()` builds alias → canonical name map from bundle agents with `aliases:` frontmatter. Scope-filtered to `is_framework_namespaced` agents. First-loaded alphabetical wins on collision (INV-11).
- `ReconcileContext.renames` field wired in all 3 adapter reconcile loops (json/claude/codex). Alias-driven name diffs now detected when config entry uses an alias instead of canonical name.
- Apply handlers activated: `apply_rename_in_agents_map` (json), `apply_rename_claude_file` (claude), `apply_rename_codex_file` (codex) now consume `ctx.renames` via alias-aware existing entry lookup.

### Fixed
- `apply_rename_in_agents_map`: now updates the entry's internal `name` field to match the new map key after rename (INC-DEBT-011).

## [Unreleased] — cycle-36

### Added
- Apply handlers for `FieldDiff { field_name: "name", ... }` in all 3 adapter families (json/claude/codex). Wires the consumer side of the rename-detection story started in cycle-35. **Dormant in production today** — all adapters set `existing.name = lookup_key`, so the rename diff is never emitted. Detection mechanism (rename map) deferred to future cycle. Closes INC-DEBT-010.

## [Unreleased] — cycle-35

### Fixed
- ExistingEntry.name design gap: `diff_existing_target` now compares `existing.name` vs `target.name` and emits a `FieldDiff { field_name: "name", ... }` when they differ. Closes INC-DEBT-009.

## [Unreleased] — cycle-33

### Changed
- `EditorCapabilities` in `sddk-cli`: removed `PartialEq, Eq` derives. Function pointer fields have unpredictable equality semantics. No workspace consumers of `EditorCapabilities::eq` were found (verified in cycle-33 explore). If you compare `EditorCapabilities` values in your code, refactor to compare individual fields or use a custom comparator.

### Fixed
- 7 pre-existing clippy errors in `crates/sddk-cli/` (closes INC-DEBT-007)
- 1 latent `unpredictable_function_pointer_comparisons` warning on `EditorCapabilities` derive

See: `docs/debt/INC-DEBT-007-preexisting-clippy-sddk-cli.md`

## [1.37.0] - 2026-08-22

Cierra el ciclo `kernel-cycle-14-m2-event-foundation` (path A-min). M2 del
event-foundation: `EventSchemaRegistry` + `CanonicalEventValidator` + tabla
de severidad `JournalProjection` + helpers públicos de correlación/causation.
Anti-AC preservados: 0 cambios en `sddk-storage/src/**`; 0 nuevas migraciones;
`EventEnvelopeV1` serialized shape byte-identical (REQ-M14-003 binding);
`emit_*` signatures byte-identical; `MANIFEST.sha256` byte-identical.

Amendment REQ-M14-004 (orquestador 2026-08-22): helpers `with_correlation_from_context`
+ `with_causation` + `trace_causation_chain` existen como API pública
probada; el wiring de producción queda diferido a M6 SPEC-028 para preservar
`emit_*` signatures. Resolución de conflicto self-contradiction (precedent
cycle-11 D2).

Verify 12/12 scenarios + 6/6 anti-ACs COMPLIANT (1094/0/6 cargo tests; 0
clippy warnings; 0 fmt warnings). Debt-verify PASS_WITH_WARNINGS: 0
introduced blockers; 2 medium/P2 (LOC overage 773 vs ≤220 per-file budget,
apply pre-verify push violation) + 3 low/P3 (helper doc-gap, corpus fixture
duplication, severity spec drift) — todas con INC filed y fingerprint
poblado.

### Added
  - feat(events): `EventSchemaRegistry` con 18 tipos registrados (incl.
    `lease.released` que faltaba) + macro `schema_struct!` (5-arg, 18
    invocaciones) que reduce ~216 LOC de boilerplate y aprieta el trait
    boundary (`EventSchema` no se puede olvidar `info()` ni
    `validate_payload()`). Cierra REQ-M14-001/002.
  - feat(events): `CanonicalEventValidator` con regex de tipo por segmento
    (acepta legacy 2-segmentos + actual 3-segmentos) + corpus replay test
    que valida que los 18 envelopes registrados son replay-safe.
    Endurecido a 18/18 casos en commit e1ded59 (regex fix para tipos
    legacy). Cierra REQ-M14-003.
  - feat(events): `JournalProjection` con tabla de severidad 7-row
    (crítica → baja; categorías `pack`/`runtime` colapsadas a Medium por
    default, decisión documentada in-line projections.rs:451-452). Cierra
    REQ-M14-005.
  - feat(events): helpers públicos de correlación/causation
    (`with_correlation_from_context`, `with_causation`,
    `trace_causation_chain`) en `event_bus.rs` con 3 tests nombrados
    `PASS`. Wiring de producción diferido a M6 SPEC-028 (amendment
    REQ-M14-004). Cierra REQ-M14-004.

### Fixed
  - fix(events): regex de formato acepta tipos legacy de 2 segmentos
    (commit e1ded59) — `corpus_replay_through_validator` endurecido de
    11/12 a 18/18.
  - fix(events): registrar `lease.released` + corpus replay +
    corregir INC — `LeaseReleasedSchema` añadido al registry en b6fc6d0
    (event_registry.rs:311).
  - fix(events): clippy `unreachable_patterns` en test — sustituido
    match+panic por `assert!(matches!(...))` en b6fc6d0.

### Housekeeping
  - chore(debt): 5 findings filed (medium/P2: FIND-0001 LOC-overage
    cluster CL-05, FIND-0002 apply-push-violation cluster CL-03; low/P3:
    FIND-0003 helper-doc-gap cluster CL-06, FIND-0004 corpus-fixture-
    duplication cluster CL-04, FIND-0005 severity-spec-drift cluster
    CL-07). Cada finding con fingerprint poblado y remediation_cycle
    declarado.
  - chore(debt): `INC-CYCLE-14-APPLY-PUSH-VIOLATION.md` — 3ª ocurrencia
    de la clase release-gate ordering; remediation target
    `kernel-cycle-15-apply-push-discipline`.
  - chore(debt): `INC-CYCLE-14-LOC-OVERAGE.md` — 3ª ocurrencia de la
    clase (port_contracts/gate_evaluator preceden); semantics distinta
    por fingerprint por ciclo; cluster `CL-LOC-OVERAGE`.

## [1.36.4] - 2026-08-22

Cierra el ciclo `kernel-cycle-13-m1-hexagonal-ports` (path A-min). Hexagonal
ports M1 — contrato de equivalencia byte-a-byte entre `InMemoryLedger` y
`SqliteLedgerFactory` para los 6 puertos de almacenamiento del engine (Ledger,
EventStore, GraphStore, ForkStore, ProjectionStore, ControlPlane) + 2
cross-checks de byte-equivalencia. Anti-AC preservados: 0 cambios en
`crates/sddk-engine/src/**`, `crates/sddk-domain/src/**`,
`crates/sddk-storage/src/**`; `MANIFEST.sha256` byte-identical; 0 test fn
borrados o relajados. Waiver WV-0015 ARCH003 composition-root avanzado al SHA
verificado (`granted_until_sha: 522e5b9...`); dev-deps `sddk-storage` y
`sddk-testkit` en `sddk-engine/Cargo.toml` anotados con `# WHY:` comments que
nombran el archivo consumidor.

Verify 8/8 scenarios + 5/5 anti-ACs COMPLIANT (1076/0/6 cargo tests; 0 clippy
warnings; 0 fmt warnings); debt-verify PASS_WITH_WARNINGS (3 LOW/P3 introduced:
ControlPlane concrete-only vs port-level coverage, byte_equiv partial-check
manifest subset, port_contracts local builders duplicating testkit API).
LOC adjudication (ADR-0048 total-module-sum): impl 0/80-120, boilerplate
5/20-30, fixtures 423/100-150 — overage en fixtures (+273) consolidado en
INC-CYCLE-13-LOC-OVERAGE (medium/P2, cluster=over-engineering/test-fixture-density).
INC envelope reporting defects desde apply/verify ya catalogadas en
INC-CYCLE-13-APPLY-TEST-COUNT-MISREPORT.

### Added
  - test(engine): `crates/sddk-engine/tests/port_contracts.rs` — suite de
    contratos (9 `#[test]`, ≥8 required) cubriendo los 6 puertos del engine
    con `InMemoryLedger` + `SqliteLedgerFactory::open_in_memory()`. 2
    cross-checks `byte_equiv_*` prueban equivalencia observable
    (event_count, cycle_record) entre adaptadores. Cierra REQ-M13-002.
  - test(engine): migrar `adoption::plan_is_write_free_*` a
    `Fixture::new_in_memory()` — preserva aserciones byte-equivalente sobre
    `plan.identity.project_id` y `plan.paths.ledger`. 23 tests
    durability-required conservan `Storage::open(&path)` con comentarios
    `// durability-required:` (reopen-same-path pattern). Cierra REQ-M13-001.

### Changed
  - chore(arch): waiver WV-0015 ARCH003 composition-root avanzado a
    `granted_until_sha: 522e5b9...` (era `f0db2bd...`). Cierra REQ-M13-003.

### Housekeeping
  - chore(engine): dev-deps `sddk-storage` (kept) y `sddk-testkit` (added) en
    `crates/sddk-engine/Cargo.toml` con comentarios `# WHY:` que nombran el
    archivo consumidor. Cierra REQ-M13-004 anti-AC hygiene.
  - chore(debt): 3 LOW/P3 introduced (FIND-013001 ControlPlane concrete-only,
    FIND-013002 byte_equiv_cycle_record partial check, FIND-013003 port_contracts
    local builders duplicating testkit API). Remediation target: backlog.
  - chore(debt): `INC-CYCLE-13-LOC-OVERAGE.md` — LOC exception entry (impl 0,
    boilerplate 5, fixtures 423/100-150, total 428/200-300; overage driven by
    test fixtures with shared builders ~60 LOC).

## [1.36.3] - 2026-08-22

Cierra el ciclo `kernel-cycle-12-workflow-contract-reconciliation` (path A-min).
Workflow contract reconciliation — fija el contrato de autoridad local de release
en los 3 archivos de la autoridad (`agents/sddk-release.md`,
`skills/sddk-release/SKILL.md`, `prompts/sddk/phases/release.md`): annotated tag
es MANDATORY y peels al SHA verificado de `main`; la verificación local
(`HEAD == origin/main` post-push) es la ruta de publicación obligatoria; la
distribución externa post-tag (GitHub Releases, CI/CD, assets) es opcional y
nunca es autoridad de cierre. Cierra el drift del allowlist de vocabulario MCW
y abre el contrato del knowledge pipeline (`scan -> verify -> import`) en el
orchestrator con conditioning explícito a `reviewed plan` y `knowledge_approved`.

0 Rust LOC; verify 14/14 scenarios + 11/11 anti-ACs COMPLIANT (296/0/0 test
contract; 1067/0 cargo tests; 0 clippy warnings); debt-verify PASS (2
introduced LOW/P3 — cosmetic orphan label + structural release-authority
coupling; pre-existing forward debt preserved); INC-CYCLE-11-PYTEST-CONTRACT-P1
cerrada (17 xfail cerradas por cycle-12; registry vaciado).

### Fixed
  - fix(agents): refs artifact store y autoridad release local (`agents/sddk-propose.md:27` + `agents/sddk-debt-verify.md:44` citan `sddk artifact store`; `agents/sddk-release.md:30-32` declara autoridad local obligatoria). Cierra REGRESSION I items 1-2.
  - fix(prompts): `sddk-verify` v2.3 + narrativa MCW + orden scan-verify-import — `skills/sddk-verify/SKILL.md` versionado a 2.3 con 8 literales del contrato de verificación (status incluye cycle, transiciones A-full/min/lite/b-direct, failed gate outcome, failed transition state, conditional lease flags); MCW § A-lite (L158) y § A-min (L162) narrativos coherentes con `workflow.yaml`; orchestrator Step 2 declara orden canónico `scan -> verify -> import` con conditioning a `reviewed plan` y `knowledge_approved`. Cierra REGRESSION J (8 literales) + D (5 assertions).

### Tests
  - test(workflow): cerrar 17 xfail — XFAIL registry vaciado (lines 38-61) + threshold `transition_artifacts` 15 -> 5 (lines 245-249) + INC `INC-CYCLE-11-PYTEST-CONTRACT-P1.md` cerrada + MANIFEST.sha256 regenerado tras edits de contenido. Cierra 1:1 todos los xfail pre-existentes: 2 (I-cluster artifact store) + 8 (J-cluster sddk-verify literals) + 5 (C-cluster release patterns x 3 files) + 1 (D-cluster scan-verify-import ordering) + 4 nuevas positive assertions en D-cluster (`contains with_knowledge`, `contains knowledge_approved`, `contains reviewed plan`, `import conditioned to both`). Suite 296 PASS / 0 FAIL / 0 XFAIL.

### Documentation
  - docs(debt): `INC-CYCLE-11-PYTEST-CONTRACT-P1.md` cerrada por cycle-12 (status: closed, resolved-by: cycle-12, lifecycle row appended).

### Housekeeping
  - chore(debt): 2 LOW/P3 introduced (FIND-0001: hardcoded orphan DEBT label en `tests/test_workflow_contract.py:1116`; FIND-0002: drift-prone release-authority duplication across 3 sources). Remediation target: backlog/opportunistic.

## [1.36.2] - 2026-08-22

Cierra el ciclo `kernel-cycle-11-a-full-coherence-gate-ordering` (path A-min).
Bug fix de docs/prompts/python — corrige el orden de las coherencias en el
bloque § A-full de `mcw.md` (steps 1.3-1.6 reordenados) y hace explícitas
las dependencias en `prompts/sddk/workflows/sddk-a-full.yaml` (4 `depends_on:`
añadidos a los coherence gates). 0 Rust LOC; 36/36 forward ACs + 9/9 anti-ACs
COMPLIANT; 7 low/P3 introduced (cosmetic/structural/opportunistic debt per
ADR-0047 + docs/debt/SEVERITY.md); 1 high/P1 introducida consolidada como
INC-CYCLE-11-PYTEST-CONTRACT-P1.md (17 xfail pre-existentes en
test_workflow_contract.py — captura estable de regresiones, fix en backlog).

### Fixed
  - fix(prompts): reordenar secciones cuerpo MCW A-full — steps 1.3-1.6 colocados tras los productores (`mcw.md` Phase 1 § A-full steps 1.1/1.2/1.7/1.8 byte-identical; Phase 2/3/4 byte-identical). Cierra REGRESSION O.
  - fix(prompts): `depends_on` explícitos en coherencias fase 2 A-full — `coherence-propose-spec` (step 1.4) → `spec-and-design-parallel`; `coherence-spec-design-tasks` (step 1.6) → `tasks`; `coherence-apply-verify` (step 2.2) → `apply`; `coherence-debt-release` (step 2.5) → `debt-verify`. Cierra REGRESSION L.

### Tests
  - test(workflow): tests de orden de coherencia + xfail P1 explícito (REGRESSION L/O/P/Q/R + COHO-001..005). 276 PASS / 0 FAIL / 17 XFAIL. C1-C8 closed.
  - test(workflow): superficie completa tests de coherencia — añade parser loop `COHERENCE_GATES` que itera los 4 coherence gates y verifica `depends_on` no vacío. Extiende COHO-002-2 con assertion explícita del step 1.6.

### Documentation
  - docs(debt): `INC-CYCLE-11-PYTEST-CONTRACT-P1.md` — captura durable de las 17 regresiones xfail del test contract (5 clusters: I propose/debt artifact store, J verify CLI contract, B transition artifact refs, C release authority, D knowledge pipeline ordering).
  - docs(handoff): `HANDOFF-2026-08-22-sddk-framework.md` — handoff de cycle-11.

### Housekeeping
  - chore(debt): entrada deuda P1/P2 cycle-11 + manifiesto (test_workflow_contract.py regresiones + manifiesta refresh discipline).

## [1.34.0] - 2026-08-21

### Refactor
  - refactor(ucl): migrate sddk-cli Stack-B call sites to `time::OffsetDateTime::now_utc()` (16 sites, 9 files). REQ-K6-001.
  - refactor(domain): migrate sddk-domain Stack-B call sites to `time::OffsetDateTime::now_utc()` (3 sites). REQ-K6-002.
  - refactor(domain): delete `sddk_domain::format::now_rfc3339_utc` wrapper and its test. Preserve `format_rfc3339_utc(epoch_secs)` + 5 pinned-value tests. REQ-K6-003.

## [1.33.0] - 2026-08-20

### Refactor
  - refactor(cli): consolidate `uat_common::time::now_rfc3339` → `sddk_domain::format::now_rfc3339_utc` (13 call sites in 9 files). Delete 43 LOC Hinnant orphan. REQ-K5-001.

### Features
  - feat(domain): extend `assert_variant_count_eq!` macro to 7 more enums (Phase=10, CycleStatus=10, RiskLevel=4, RuleSeverity=3, StalenessState=5, PackRisk=4, ReleaseChannel=4). Negative-tested: literal 10→11 breaks build. REQ-K5-002.
  - fix(domain): macro diagnostic now embeds `stringify!($enum)` + concat-based panic msg; PM-3 clippy fixes (`for_kv_map`, `expect_fun_call`, `collapsible_if`, `const_is_empty`); 12 runtime shape tests (`variant_counts.rs`). REQ-K5-004.

### Housekeeping
  - docs(agents): trim AGENTS.md 229→≤100 LOC; extract §2.6 Distribución → `docs/RELEASING.md`; extract §3 Layout → `docs/ARCHITECTURE-MODEL.md`. REQ-K5-003.

## [1.32.0] - 2026-08-20

### Refactor
  - refactor(domain): `now_rfc3339_utc()` wrapper en `sddk_domain::format` (delegates to `format_rfc3339_utc(epoch_secs)`). Los 3 call sites (`projections.rs:206`, `projections.rs:395`, `graph.rs:251`) ahora usan el wrapper. Cierra S-001 (orphan Hinnant cleanup).
  - refactor(storage): `proj_store_conn_mut()` marcado como test-only surface via `#[doc(hidden)]` + rustdoc explicativo. El escape hatch `&mut rusqlite::Connection` permanece público solo porque los integration tests no ven `#[cfg(test)]` (compile la lib sin `cfg(test)`). Cierra S-002.

### Features
  - feat(domain): macro `assert_variant_count_eq!` (`crates/sddk-domain/src/macros.rs`) — compile-time guard contra variant drift. Combinación de counter literal + exhaustive `match` sin wildcard. Estable en rustc ≥ 1.75 (usa solo `stringify!`, const fn aritmética, `assert!`). Aplicado a los 5 enums trimmed del cycle 3: `CompileError` (8), `WorkflowError` (3), `AttemptError` (1), `NodeRunError` (1), `WorkflowRunError` (2). Negatively-tested: edición literal 8→9 rompe el build con E0080. Cierra S-003.

### Housekeeping
  - chore(repo): `*.proptest-regressions` añadido a `.gitignore` (proptest deterministic-replay cache; safe to delete, regenerable).
  - chore(repo): staging residuals del release resync del cycle 3 (`sddk-linux-x86_64-gnu*`, `software-development-decision-kernel.tar.gz*`) cleared del CWD antes de este commit. `docs/old/*` (712K) intacto pendiente decisión humana cycle 5. Cierra S-004 (parcial).

## [1.31.0] - 2026-08-20

### Features
  - feat(domain): `format_rfc3339_utc` extraído a `sddk-domain::format` (Hinnant `civil_from_days` + `z += 719_468` shift). 5 tests pinned-value. Cierra W-DV-1 (state_updated_at fix) + W-DV-7 (cross-crate Hinnant duplication).
  - feat(arch): `architecture-rules.yaml` schema_version 1.1.0 → 1.2.0 + WV-0027 phase-string waiver para `compiler.rs`/`validator.rs` (10 reglas, 2 waivers). Cierra WV-0027.

### Refactors
  - refactor: `Storage::current_iso8601` + `state_updated_at` delega a `sddk_domain::format::format_rfc3339_utc`. `sddk-cli::uat::now_rfc3339` ya no tiene bloque Hinnant propio.
  - chore(domain): error variant audit — trim 15 unused en 5 enums (`CompileError`, `WorkflowError`, `AttemptError`, `NodeRunError`, `WorkflowRunError`). Cierra U6 (REQ-K3-001).

### Tests
  - test(domain): `compiler_determinism` proptest 1000 iters (hash determinístico + formato `sha256:<64-hex>`). REQ-K3-002 #1.
  - test(domain): `validator_closure` proptest 500 iters (closure property `validate(compile(m))`). REQ-K3-002 #2.
  - test(storage): `graph_store_roundtrip` proptest 200 iters (in-memory `SqliteGraphStore` roundtrip). REQ-K3-002 #3.
  - test(domain): `capsule_validate` proptest 7 invariants (Pointer always valid, sha256 format/length, size bound, digest integrity). REQ-K3-002 #4.
  - test(domain): `budgets_proptests` 5 invariants algebraicos (zero identity, underflow, hard limits, fits-within componente-wise, monotonicity consume). REQ-K3-002 #5.

### Fixes
  - fix(domain): bump `ARCHITECTURE_RULES_SCHEMA_VERSION` constant 1.1.0 → 1.2.0 (closes WU-3b schema gap — runtime must accept the YAML it produces).

## [1.30.0] - 2026-08-19

### Features
  - feat(domain): `WorkflowCompiler` (8-stage deterministic pipeline, no LLM) translates `WorkflowManifest` legacy → `WorkflowIR`. Phase → capability mapping for all 10 `Phase` variants. Closure: `validate(compile(m))` is either `Ok` or a single gate error.
  - feat(domain): `WorkflowValidator` (7 gates: G1 schema, G2 operators, G3 cycle-free, G4 guards, G5 budgets, G6 expansion permissions, G7 context capsules). Short-circuit on first failure. `validate_with_template()` for full G6 allowlist check.
  - feat(storage): 6 `GraphStore` methods implemented in `SqliteGraphStore` against `ir_digests_v1`, `execution_graph_revisions_v1`, `attempts_v1`: `record_ir_digest`, `record_graph_revision`, `load_node_attempts`, `attempt_count`, `load_revision`, `latest_revision`. Closes W-DV-1 (HIGH).
  - feat(domain): `ExpansionPermission::is_allowed` split into `is_known_permission()` + `is_allowed_by(allowlist)`. Old `is_allowed` marked `#[deprecated(since = "1.30.0")]` for removal in cycle 3. Closes W-DV-3 (MED).
  - feat(domain): `Budgets::consume(&sub) -> Result<Budgets, BudgetError>` semantics drives validator G5. Closes W-DV-4 (LOW bonus).
  - feat(domain): `ContextCapsuleRef::validate()` for inline summaries (sha256 format + size bound + digest integrity). Closes W-DV-5 (LOW bonus).

### Fixes
  - fix(domain): elimina 6 defaults `unimplemented!()` del trait GraphStore — LSP closure verificado por sddk-verify correction cycle
  - feat(testkit): 3 fixtures A-min/A-lite/A-full + constantes A_MIN/LITE/FULL_COMPILED_HASH pinned — cierre spec §2.1 scenario 2

### Documentation
  - docs(adr): ADR-0043 — Compiler determinista sin LLM (closure property + golden hashes)
  - docs(adr): ADR-0044 — Validator con 7 gates en short-circuit (orden + rationale)
  - docs(adr): ADR-0045 — GraphStore port con 6 métodos IR-revision (LSP closure)

## [1.29.0] - 2026-08-19

### Features
  - feat(domain): Workflow IR types — `WorkflowTemplate`, `WorkflowIR`, `Operator` (12 variants), `Budgets`, `ExpansionPermission` — landed in `crates/sddk-domain/src/workflow_ir.rs` with deterministic `compute_content_hash()` using `sha256:<64-hex>` format. All collections use `BTreeMap`/`BTreeSet` exclusively.
  - feat(domain): WorkflowRun types — `WorkflowRun`, `NodeRun`, `Attempt`, `AttemptOutcome`, `WorkflowRunState`, `NodeRunState` — state machine with sticky terminal states. `Attempt::complete()` transitions in_flight → terminal.
  - feat(domain): `ExecutionGraphRevision` added to `crates/sddk-domain/src/graph.rs` with parent-chain digest (`sha256(parent_digest || events || nodes || edges)`). Schema version `u32 = 1`.
  - feat(storage): Migration 011 adds 5 new tables: `workflow_runs_v1`, `node_runs_v1`, `attempts_v1`, `execution_graph_revisions_v1`, `ir_digests_v1`. Append-only triggers on `attempts_v1`.
  - feat(kernel): `GraphStore` trait gains 7 default-implemented methods: `record_ir_digest`, `record_graph_revision`, `load_node_attempts`, `attempt_count`, `save_revision`, `load_revision`, `latest_revision`.
  - feat(arch): ARCH008 heuristic evaluator with 4-pattern `RegexSet` (`\bPhase::`, `\bCyclePath::`, variant-qualified SDD names, `match\s+phase\s*\{`). Scope: `workflow_ir.rs`, `workflow_run.rs`, `sddk-engine/lib.rs`. ARCH013–015 stubbed as `NotApplicable`.
  - feat(arch): `architecture-rules.yaml` schema bumped `1.0.0 → 1.1.0`. WV-0026 waiver covers `workflow.rs` and `event_bus.rs:96-117` until v1.31.0.

### Tests
  - test(domain): Property tests for `WorkflowIR` — hash determinism, BTreeMap insertion-order independence, JSON roundtrip, `sha256:<64-hex>` format, 12-operator nesting to depth 10.
  - test(domain): State machine tests for `WorkflowRun` — `pending → running → completed`, `cancelled`, `pause/resume` budget preservation, terminal idempotency.
  - test(domain): `ExecutionGraphRevision` tests — chain divergence, parent digest embedding, `revision 0` root validation, JSON roundtrip.
  - test(domain): `architecture-rules.yaml` parse tests — 1.1.0 structure, ARCH008 scope globs, WV-0026 waiver coverage (legacy files only, not new IR modules).
  - test(domain): 4 IR event golden fixtures added to `event_envelope_golden.rs` — `workflow.ir.compiled`, `workflow.run.started`, `workflow.run.cancelled`, `workflow.graph.revision.accepted`.
  - test(testkit): `ir_fixtures.rs` module with `sample_template()`, `sample_ir()`, `sample_workflow_run()` golden fixtures.

### Documentation
  - docs(adr): ADR-0040 — BTreeMap mandate for IR collections (deterministic hashing requirement)
  - docs(adr): ADR-0041 — `SCHEMA_VERSION: u32` constant per IR type (monotonic integer vs semver)
  - docs(adr): ADR-0042 — ARCH008 SDD-agnostic kernel runtime + WV-0026 legacy compat seam waiver

## [1.28.1] - 2026-08-19

### Fixes
  - fix(editor-adapters): `sddk dev link` now refreshes stale agent paths in `opencode.json` / `zcode.json` when the framework bundle root changes (e.g. after a fresh install or upgrade). Existing entries with paths matching the new root are skipped byte-untouched; entries with paths matching a previous sddk install (`/sddk-framework/agents/` or `/sddk/framework/`) have only the `prompt` field refreshed — user customizations (`description`, `model`, `hidden`, `mode`) are preserved. User-customized paths that look unrelated to sddk (`/my/personal/prompts/...`) are left untouched. Telemetry: `AdapterReport.updated_stale` + `LinkReport.agents_updated_stale`, with a per-editor warning `"refreshed N stale agent paths (framework root changed)"`.

## [1.28.0] - 2026-08-19

### Features
  - feat(orchestrator): Pre-flight Gate 0 — rebuild state from CLI (sddk cycle lock status + cycle status + vault validate) before each phase delegation. Replaces in-memory-only state_token with pull-based reconstruction; survives compaction and session restarts.
  - feat(skill): sddk-cycle-resume — pull-based state_token envelope with cycle_id, phase, branch, head_sha, fencing_token, vault drift count, head drift flag. Hard rules for lease desync, head drift, and "cycle closed by another orchestrator" cases.
  - feat(mcw): Phase 0 Step 0.2 now queries `sddk cycle lock status` as the authoritative gate (vault `_active.md` becomes a secondary informational view).

### Fixes
  - fix(install): bootstrap.sh now symlinks the `workflows/` tree and the `prompts/sddk/workflows/*.yaml` registry. Previously the orchestrator fell back to mcw-prose for path-specific sequences because the YAMLs were never linked into the editor.

### Documentation
  - docs(repo): CONTRIBUTING.md — contribution guide (commit conventions, review process, CI policy, layout, release procedure).
  - docs(repo): README.md — install procedure now documented as `sddk dev install --prefix ~/.local --source .` + `sddk dev link --editor all` (was `bootstrap.sh --all` only).

### Other
  - chore(fmt): rustfmt --all aligns with the repo's rustfmt.toml (cosmetic only).
  - chore(manifest): regenerated MANIFEST.sha256 with the 92nd skill entry.

### Distribution
  - chore(distribution): SDDK now ships as pre-compiled binaries on GitHub Releases. The `scripts/install.sh` one-liner (rustup/mise model) detects platform, downloads the binary + SHA256, verifies integrity, and links the framework bundle into the chosen editor. Linux x86_64-musl and Linux aarch64-musl are the first supported targets (both static, run on any distro). Closes the gap between docs (which claimed "asdf-vm model") and reality (which was "git clone + cargo build").

## [1.27.0] - 2026-08-19

### Features
  - chore: SDDK 2.0 roadmap complete (Phases 1-4, all MUST done, all SHOULD discarded with rationale)

## [1.26.0] - 2026-08-18

### Features
  - feat(test): golden dataset 10 cases + ratchet/channel e2e (phase9)
  - feat(cli): release channel promote + signed gate receipts + rules ratchet (phase9)
  - feat(domain): release channels + HMAC gate signing (phase9)

## [1.25.0] - 2026-08-18

### Features
  - feat(cli): sddk explore render views + explorer template (phase8)
  - feat(domain): view descriptors + view models for moldable explorer (phase8)

### Other
  - test(cli): explore e2e views graph/timeline/verification + embedded template (phase8)

## [1.24.0] - 2026-08-18

### Features
  - feat(cli): fork replay with kernel ledger fallback event source (phase7)
  - feat(cli): fork create/set/run/diff/promote commands + e2e (phase7)
  - feat(storage): fork store + response cache tables (migration 9) (phase7)
  - feat(domain): fork model + replay engine + structural diff + promote check (phase7)

## [1.23.0] - 2026-08-18

### Features
  - feat(cli): stale list/impact/gate + graph why-stale commands (phase6)
  - feat(domain): context-read tracing recorder bounded + graph skip (phase6)
  - feat(domain): universal staleness derivation over graph + UAT mapping (phase6)

### Other
  - test(cli): stale/impact/why-stale/gate e2e + uat stale surface intact (phase6)

## [1.22.0] - 2026-08-18

### Features
  - feat(cli): sddk graph query/why/rebuild commands (phase5)
  - feat(domain): pattern query BFS + proposal-only behavior runtime (verifies/depends_on) (phase5)
  - feat(storage): graph store port + sqlite adapter + rebuild from ledger (phase5)
  - feat(domain): graph projection nodes/edges/provenance + bounded views (phase5)

## [1.21.0] - 2026-08-18

### Features
  - feat(pack): uat pack boundary, conformance fixtures, evidence aliases canonical (phase4)
  - feat(cli): pack list/inspect/install/verify/enable/disable commands (phase4)
  - feat(engine): pack registry lifecycle discover/verify/enable/disable/install (phase4)
  - feat(domain): pack manifest v2 requires/integrates/conflicts/provides (phase4)

## [1.20.0] - 2026-08-18

### Features
  - feat(approval): CLI commands for human approval list|grant|deny (PR3)
  - feat(gateway): add ApprovalExpired, ApprovalAlreadyResolved, ApprovalReasonRequired variants
  - feat(domain): add ApprovalPending status, ApprovalReceipt types, and ApprovalProjection
  - feat(dev): gum TUI para agent-models.yaml
  - feat(dev): uninstall/doctor cubren claude y codex
  - feat(dev): dev link registra 4 editores sin fallback hardcoded
  - feat(dev): Codex adapter writes native TOML agents
  - feat(dev): Claude Code adapter writes native .md agents
  - feat(dev): ZCode adapter mirrors opencode JSON registration
  - feat(dev): sddk dev models list/set/validate
  - feat(dev): ship agent-models.yaml under assets
  - feat(dev): agent-models.yaml schema + tier/override resolution

### Fixes
  - fix(dev): clippy --all-targets CI gate (M1-M3)

### Other
  - fmt: apply formatting to human-approval-events PRs
  - docs(adr): ADR-0017..0020 modelos + adapters + TUI
  - test(dev): align codex body assertion with frontmatter-newline semantics
  - refactor(dev): EditorAdapter trait + OpenCode JSON adapter

## [1.19.0] - 2026-08-18

### Features
  - feat(engine): event_bus module with emit_phase_event dual envelope appender (SDDK2-204 MS-01+MS-02)
  - feat(cli+engine): emit workflow.phase events on cycle transition success (SDDK2-204 MS-03 wire-up)
  - feat(engine): PhaseEventInput struct with actor metadata for workflow.phase events
  - feat(cli): RuntimeContext.paths field added for ledger path access in event emission

### Tests
  - test(engine): phase events integration — PE-01 dual-emit, PE-02 idempotency, PE-03 rebuild, PE-04 ledger coexistence

### Other
  - chore(rules): refresh WV-0015 granted_until_sha to f0db2bd (post-SDDK2-204)
  - fix(engine): event_bus uses EventStore trait (not concrete SqliteEventStore) to satisfy ARCH001

## [1.18.0] - 2026-08-17

### Features
  - feat(domain): Projection trait + Checkpoint + CycleStateProjection (SDDK2-203)
  - feat(storage): MIGRATION_6 projection_checkpoints_v1 + SqliteProjectionStore (SDDK2-203)
  - feat(storage): rebuild() algorithm with chain-verify + tamper detection (SDDK2-203)
  - feat(cli): dev projection rebuild command (SDDK2-203 MS-05)

### Other
  - chore(domain+storage): rustdoc strict-pedantic polish for SDDK2-203 modules
  - chore(fmt): rustfmt formatting for SDDK2-203 modules (verification cleanup)
  - chore(rules): refresh WV-0015 granted_until_sha to f054680 (post-SDDK2-203)

## [1.17.0] - 2026-08-17

### Features
  - feat(domain): EventStore trait + EventAppended for ledger-first events (SDDK2-202)
  - feat(storage): MIGRATION_5 events_v1 table with append-only triggers
  - feat(storage): SqliteEventStore struct with XDG-open constructors
  - feat(storage): SqliteEventStore append + reads (SDDK2-202 MS-04 essential)
  - feat(storage): MS-05 event_store integration tests and compute_content_hash fix

### Other
  - chore(rules): refresh WV-0015 granted_until_sha to dbc1dbb (post-v1.16.0)
  - fix(domain): compute_content_hash() no longer hashes itself or sequence/recorded_at

## [1.16.0] - 2026-08-17

### Features
  - feat(domain): EventEnvelopeV1 wire format with sub-types (EntityRef, ActorRef, ActorKind, EntityRefVersion, EventTypeError)
  - feat(domain): canonical JSON serialization + sha256 content_hash for EventEnvelopeV1
  - feat(domain): validate_event_type with regex namespacing
  - feat(domain): stub EventEnvelopeV1 module skeleton

### Other
  - chore(rules): refresh WV-0015 granted_until_sha after hygiene cycle
  - fix(cli): replace clone().as_slice() with slice::from_ref in release_cmd.rs
  - chore(cli): silence pre-existing dead_code warnings in dev/ and telemetry helpers
  - chore(tests): allow unused_variables and needless_* in sddk-cli tests
  - chore(gateway): allow dead_code in git_push_credential test fixtures
  - chore: commit manifest.rs allow and fmt fixes for hygiene baseline
  - test(domain): proptest for content_hash determinism under insertion order
  - test(domain): golden vector tests against regenerated uat-acceptance.jsonl
  - test(domain): regenerate uat-acceptance.jsonl with real SHA-256 content_hash values

## [1.15.0] - 2026-08-17

### Features
  - feat(domain+storage): introduce ControlPlane port + SqliteControlPlane adapter
  - feat(cli): compose() composition root + telemetry through ControlPlane port (SDDK2-103)

### Other
  - refactor(cli): re-route sddk_storage types to sddk_domain
  - feat(cli): route telemetry through ControlPlane trait, remove rusqlite dep
  - refactor(cli): route Storage through type alias, eliminate cycle.rs direct import
  - chore(fmt): apply rustfmt after CLI port migration
  - test(engine): ARCH003 WAIVED under WV-0015 (ADR-0015)

## [1.14.0] - 2026-08-16

### Features
  - feat(engine): Ledger port + Engine<L> genérico — Phase 1 M1 exit
  - feat(deep-research): integrate 22 skills + orchestrator agent + b-research workflow
  - feat(domain): extract Ledger port trait from Storage

### Other
  - refactor(deep-research): consolidate 22 skills into master+sub hierarchical pattern

## [1.13.1] - 2026-08-16

### Other
  - refactor(domain): mueve los value types de sddk-storage a sddk-domain (Phase 1 Sub-ciclo A)

## [1.13.0] - 2026-08-16

### Features
  - feat(arch): phase-1 evaluators — live capture + real rules + CLI

## [1.12.1] - 2026-08-16

### Other
  - docs(domain): documenta la API pública del módulo rules (60 items)

## [1.12.0] - 2026-08-16

### Features
  - feat(storage): GateOutcomeStatus::Waived — waiver explícito de gates

## [1.11.0] - 2026-08-16

### Features
  - feat(gateway): resolver genérico CapabilityPolicy::env_allowlist (v2)

## [1.10.3] - 2026-08-16

### Other
  - refactor(testkit): fixtures Git compartidos — API git en TestRepository

## [1.10.2] - 2026-08-16

### Other
  - refactor(dev): copy_tree unifica tree-copy y hace atómico el install de bundles

## [1.10.1] - 2026-08-16

### Other
  - refactor(testkit): ChildGuard RAII compartido; elimina kills manuales duplicados

## [1.10.0] - 2026-08-16

### Features
  - feat(dev): doctor detecta incoherencia binario/bundle (INC-DEBT-005)

## [1.9.22] - 2026-08-16

### Fixes
  - fix(storage): valida longitud de gate 1..=128 en ambos paths de inserción

## [1.9.21] - 2026-08-16

### Fixes
  - fix(release): release-bump antepone CHANGELOG y autocorrige drift de manifest.toml

## [1.9.20] - 2026-08-16

### Other
  - refactor(dev): consolida helpers duplicados y añade smoke tests behaviorales

## [1.9.19] - 2026-08-16

Refactor: `dev_cmd.rs` (3022 líneas) se divide en 13 submódulos bajo `crates/sddk-cli/src/dev/`.
Mejora la legibilidad, reduce el coste de code review y allana el camino para extraer
capacidades reutilizables. Sin breaking changes para el CLI.

### Changed
  - refactor(cli): `dev_cmd.rs` se elimina y se reemplaza por `crates/sddk-cli/src/dev/`
    con 13 submódulos: `mod.rs`, `paths.rs`, `common.rs`, `manifest.rs`, `registry.rs`,
    `doctor.rs`, `framework_check.rs`, `install.rs`, `uninstall.rs`, `update.rs`,
    `link.rs`, `use_cmd.rs`, `check.rs`, `verify.rs`. El módulo reduce la cohesión
    por archivo y deja cada subcomando con su propia superficie.
    (`crates/sddk-cli/src/dev/`, `crates/sddk-cli/src/dev_cmd.rs`)
  - refactor(cli): visibilidad apretada a la allow-list de la spec — los símbolos
    compartidos entre submódulos usan `pub(crate)` o `pub(super)` en lugar de `pub`.
    (`crates/sddk-cli/src/dev/`)
  - refactor(cli): `dev/framework_check.rs` extrae `framework_agent_names`,
    `register_opencode_agents`, `AgentFrontmatter`, `parse_frontmatter`,
    `PRIMARY_AGENTS`, `LinkReport`, `link_report_text` y `sync_assets` desde
    `link.rs` para reducir el tamaño de los submódulos.
    (`crates/sddk-cli/src/dev/framework_check.rs`, `crates/sddk-cli/src/dev/link.rs`)

### Tests
  - test(cli): 25 tests unitarios se migran de `dev_cmd.rs` a `dev/tests/` (un archivo
    por subcomando: `manifest_tests.rs`, `reconciliation_tests.rs`,
    `skill_registry_tests.rs`) usando `#[path]` para preservar el módulo actual.
    (`crates/sddk-cli/src/dev/tests/`)
  - test(cli): 4 smoke tests nuevos en `crates/sddk-cli/tests/cli.rs` cubren el
    cableado de `dev install`, `dev doctor`, `dev verify` y `dev list`.

### Fixed
  - fix(gateway): suprime el lint `expect_fun_call` (clippy 1.91) en
    `classify_auth_failure` test. `expect(&format!(...))` se sustituye por
    `unwrap_or_else(|| panic!(...))` para evitar el `format!` cuando el `Ok`
    es el caso esperado. Hallazgo del gate local del release.
    (`crates/sddk-gateway/src/git.rs:795-798`)

## [1.9.18] - 2026-08-15

Corrige el race condition en la asignación de `seq` para `GateReceipt`:
`allocate_gate_receipt_seq` + `insert_gate_receipt` eran dos llamadas separadas;
entre ellas otro thread podía allocatear el mismo seq, causando UNIQUE violation
o receipt_ids duplicados. INC-DEBT-007 (ponytail death).

### Fixed race
  - fix(storage): `insert_gate_receipt_next_seq` colapsa allocate + format + insert
    en una sola transacción `IMMEDIATE` SQLite. El lock de escritura serializa
    las asignaciones concurrentes; `seq` y `receipt_id` se producen juntos.
    (`crates/sddk-storage/src/lib.rs:946-1005`)

### Removed
  - remove(storage): `Storage::allocate_gate_receipt_seq` eliminado (INC-DEBT-007).
    El método era el ponytail del race; no tiene más usuarios tras la migración.
    (`crates/sddk-storage/src/lib.rs`)

### Changed
  - refactor(engine): `Engine::evaluate_gate` ahora llama una sola vez a
    `insert_gate_receipt_next_seq` en lugar de allocate + insert separados.
    El formatter `gate-{gate}-{plan_hash[7..23]}-{seq}` se mueve al storage.
    (`crates/sddk-engine/src/lib.rs:882-896`)
  - refactor(storage): `debug_assert!` en `build_gate_receipt_id` se sustituye
    por guarda real que retorna `StorageError::PlanHashTooShort { actual, required }`
    si `plan_hash.len() < 23`. Se extrae `pub fn Storage::build_gate_receipt_id`
    (ahora retorna `Result<String>`) y `pub const RID_FORMAT_REGEX` a nivel de
    módulo — el regex deja de duplicarse entre `cycle_authority.rs` y
    `sqlite_storage.rs`. (`crates/sddk-storage/src/lib.rs:140-167`, `956-973`)

### Tests
  - test(storage): reescritura de `storage_insert_gate_receipt_concurrent_allocations_observe_distinct_seq`
    (100 iter × 2 threads, BD compartida, `Arc<Barrier>`, seq exactos 1..=201)
  - test(storage): nuevo `storage_insert_gate_receipt_next_seq_golden_rid_format`
    verifica formato byte-idéntico del `receipt_id`
  - test(storage): nuevo `storage_insert_gate_receipt_next_seq_rejects_short_plan_hash`
    verifica la guarda `PlanHashTooShort` (sustituye el `debug_assert!`)
  - test(engine): `engine_evaluate_gate_increments_seq_on_reevaluation` añade regex
    lock `^gate-.{1,128}-[0-9a-f]{16}-[0-9]+$` sobre el receipt_id

## [1.9.17] - 2026-08-15

Cierra el leak de puerto en `uat_stale_tests::stale_detects_geometry_change` (INC-DEBT-006):
el test pinchaba el puerto 49152 y lanzaba `python3 -m http.server` sin guard, de modo que un
`panic` previo dejaba un proceso huérfano; la siguiente corrida del workspace chocaba con
`EADDRINUSE` y devolvía `ERR_EMPTY_RESPONSE` en el cliente. Ahora el puerto es efímero
(`TcpListener::bind("127.0.0.1:0")`), el proceso servidor está envuelto en un `ServerGuard`
RAII cuyo `Drop` envía `SIGKILL` al hijo y espera `wait()`, y el cliente hace `readiness-poll`
con deadline en lugar de asumir respuesta inmediata. Skip limpio en hosts sin `python3`.

### Fixes
  - fix(uat): `ServerGuard` RAII (`spawn → kill on Drop → wait`) garantiza que un panic
    o un fallo de readiness mata el hijo en lugar de dejarlo escuchando.
    (`crates/sddk-cli/src/uat.rs:4696-4761`)
  - fix(uat): puerto efímero (`TcpListener::bind("127.0.0.1:0")`) elimina la condición de
    carrera entre runs y permite runs paralelos del test. (`crates/sddk-cli/src/uat.rs:4771-4798`)
  - fix(uat): readiness-poll con `deadline = Instant::now() + timeout` y backoff corto
    reemplaza el `read_to_end` ciego que devolvía `ERR_EMPTY_RESPONSE` cuando el servidor
    aún no estaba listo. (`crates/sddk-cli/src/uat.rs:4821-4852`)
  - fix(uat): probe de `python3` en `PATH` antes de spawnear — si falta, el test se salta
    con `Ok(())` en lugar de propagar `ENOENT`. (`crates/sddk-cli/src/uat.rs:4751-4766`)
  - style(uat): alinea formato del guard con rustfmt y silencia `dead_code` de
    `ServerGuard::take` (helper movido al module-level para uso futuro).

### Tests
  - test(uat): `stale_detects_geometry_change` re-pasa en runs consecutivas del workspace
    y en runs paralelos (el puerto ya no es fijo).
  - El test sigue marcado `#[ignore]` (skip explícito de la suite); la verificación de no-leak
    se realiza re-ejecutando el workspace tras una corrida forzada de panic, ver
    `verification-report.md` §2.7.

## [1.9.16] - 2026-08-15

Corrige la ergonomía de `cycle start` y `git push` para el path A-min en repos
trunk-linear: INC-DEBT-003 cambia el default de `--branch` para A-min a `main`;
INC-DEBT-004 añade `GIT_TERMINAL_PROMPT` al allowlist del runner y clasifica
errores de autenticación de `git push` con hint accionable.

### Fixes
  - fix(cli): `cycle start --path a-min` (sin `--branch`) registra
    `manifest.branch = "main"` en lugar de `feat/<name>`. A-lite, A-full,
    B-direct mantienen el default `feat/<name>`. `--branch` explícito siempre
    gana. (INC-DEBT-003, `crates/sddk-cli/src/cycle.rs:471-473`)
  - fix(gateway): `GIT_TERMINAL_PROMPT` se añade a `LOCAL_GIT_ENV_KEYS` para
    que los helper de credentials fallen rápido en lugar de bloquear en un TTY
    inexistente. (INC-DEBT-004, `crates/sddk-gateway/src/git.rs:11-22`)
  - fix(gateway): `git push` que falla con error de autenticación devuelve
    `GitError::AuthFailed { stderr, hint }` con hint de cuatro líneas:
    `gh auth login`, `gh auth setup-git`, o `git config credential.helper store`.
    `apply_local_release` propaga el hint sin modificar el manifest.
    Clasificador de marcadores: `could not read Username`, `terminal prompts
    disabled`, `403 Forbidden`, `Bad credentials`, `failed to authenticate`,
    `fatal: Authentication failed`. (INC-DEBT-004)

### Tests
  - test(cli): `cli_cycle_start_without_branch_for_a_min_uses_main_default`
  - test(cli): `cli_start_with_explicit_branch_for_a_min_persists_value`
  - test(cli): `cli_cycle_start_without_branch_for_a_full_uses_feat_default`
  - test(cli): `cli_release_apply_rejects_a_min_when_manifest_branch_is_feat_x`
  - test(cli): `cli_capability_apply_git_push_forwards_git_terminal_prompt`
  - test(gateway): `local_git_env_keys_includes_git_terminal_prompt`
  - test(gateway): `local_git_env_keys_excludes_gh_token`
  - test(gateway): `git_push_auth_failure_classifies_stderr`
  - test(gateway): `runner_run_forwards_git_terminal_prompt`
  - test(cli/gateway): end-to-end `release apply --route local` hint-on-auth-failure
    scenario is covered indirectly by `git_push_auth_failure_classifies_stderr`
    (unit) — the orchestrator-accepted E2E test was omitted because the
    `file://` remote used in sandbox runs does not exercise credential prompts.
    See `verification-report.md` § Behavioral Compliance Matrix B1.REQ-5.

## [1.9.15] - 2026-08-15

Corrige la evaluación de gate receipts: INC-DEBT-001 evita colisión UNIQUE en re-evaluación
agregando columna `seq` por grupo (gate, plan_hash); INC-DEBT-002 exige `--outcome` explícito.

### Fixes
  - fix(storage): secuencia receipts de gate y colisión UNIQUE — MIGRATION_3 añade columna
    `seq INTEGER NOT NULL DEFAULT 1` y índice único parcial `(gate, plan_hash, seq)`.
    `Storage::allocate_gate_receipt_seq` usa `TransactionBehavior::Immediate` para serializar
    asignaciones concurrentes.
  - fix(storage): `GateReceiptInput` y `GateReceipt` incluyen campo `seq`.
  - fix(engine): `Engine::evaluate_gate` deriva receipt_id como
    `gate-{gate}-{plan_hash[7..23]}-{seq}` y persiste `seq` en la fila.
  - fix(cli): `--outcome <passed|failed>` es ahora argumento requerido en
    `sddk cycle evaluate-gate`. Recetas que omiten la bandera deben actualizarse.
    El silent `Failed` default es eliminado. (Breaking change.)

### Breaking Changes
  - `sddk cycle evaluate-gate --outcome <passed|failed>` es ahora obligatorio.
    Recetas existentes que omiten `--outcome` producirán error de parsing.

## [1.9.13] - 2026-08-14

Corrige la integridad del bundle de release: el manifest se genera desde rutas
tracked publicables, falla cerrado ante errores Git y rutas no UTF-8, conserva
rutas UTF-8 especiales y se verifica en staging antes de actualizar el runtime.

### Fixes
  - fix(manifest): enumera `git ls-files` limitado a superficies publicables y hashea bytes actuales
  - fix(manifest): drena salida Git concurrentemente y rechaza rutas tracked publicables no UTF-8
  - fix(manifest): serializa rutas UTF-8 especiales con escape reversible
  - fix(dev): verifica bundles descargados en staging antes de tocar el destino
  - fix(release): empaqueta el manifest canónico comprometido y elimina cuatro rutas phantom

## [1.9.12]

Cierra el ciclo SDDK2-009 (phase.build.complete). Cinco work units resueltas: U1+U2 bundle seam (dev install --source + skill-registry writer), U3 knowledge pipeline prefight (-with-knowledge --approve con quarantine rule), U4 --outcome passed en todos los evaluate-gate, U5 bump 1.9.12 + BACKLOG + CHANGELOG.

### Features
  - feat(dev): add --source flag to dev install — copies MANIFEST and verifies SHA256
  - feat(dev): add --write-registry to dev link — writes skill-registry.md to XDG project dir
  - feat(dev): write_skill_registry() — scans skills/*/SKILL.md, skips _shared, writes sorted markdown table
  - feat(agent): init knowledge preflight with --with-knowledge --approve pipeline
  - feat(cli): add --outcome passed to all evaluate-gate calls (8 SKILL.md files)

### Fixes
  - fix(cli): --outcome passed added to evaluate-gate in all phase SKILLs
  - fix(backlog): SDDK2-009 inserted after SDDK2-008.DEBT

## [1.9.11]

Cierra el ciclo SDDK2-008 (phase0-knowledge-ingestion). El pipeline `scan → plan → import → verify` con CAS, provenance, authority y quarantine está gobernado por la knowledge vault en `~/.sddk-knowledge/`. Tres negative tests aseguran que `--approve` en candidatos Quarantine (R10) o con razón "relation conflicts" (R5 surface) son rechazados por `is_approvable_change()`. Distribución corregida: 7 crates en `version.workspace = true` alineados a 1.9.11.

### Features
  - feat(knowledge): scan → plan kp-<hex16> → import → verify governed pipeline
  - feat(knowledge): TOCTOU cerrada por re-hash en import
  - feat(knowledge): Authority::Trusted exige disposition=Import o --approve + is_approvable_change
  - feat(knowledge): receipt kr-<hex16> determinista para not_applicable
  - feat(knowledge): CliFixture + git_commit_all scaffolding para integración tests

### Fixes
  - fix(dist): 7 crates con version.workspace=true alineados a 1.9.11

### Other
  - test(knowledge): approve_quarantine_candidate_fails — R10 negative test
  - test(knowledge): approve_relation_conflict_candidate_fails — R5 surface negative test
  - test(knowledge): relation_key_is_deterministic_for_path_invariants — case normalization invariant
  - docs(BACKLOG): SDDK2-008 y SDDK2-008.DEBT insertados entre SDDK2-007 y SDDK2-101

## [1.9.10] - 2026-08-14

Cierra el release del ciclo SDDK2-006 (`sddk-2-0-phase0-doc-governance`). Tras el bump inicial a `v1.9.9` y el commit `docs(handoff): refresh with final HEAD dbf93c7` (`cbe26db`) que reescribió el handoff con el SHA final, el tag `v1.9.9` quedó apuntando al commit previo (`dbf93c7`) sin cubrir el refresh de handoff. Se corta `v1.9.10` como tag anotado en un nuevo `chore(release)` para preservar la linear-history (AGENTS.md §2.2) y satisfacer el contrato `sddk-release` que exige tag-peels-to-HEAD. Sin cambios de código de producción; el diff acumulado del ciclo sigue siendo puramente documental (SDDK2-006 fue zero-intrusion por diseño).

### Other
  - chore(release): bump to v1.9.10 (sddk-2-0-phase0-doc-governance) — corte de tag post-handoff-refresh; repara tag/HEAD gap documentado como W2 en el vault

## [1.9.9] - 2026-08-13

Split AGENTS.md into stable/history/handoff surfaces (SDDK2-006 doc-governance).

### Other
  - feat(docs): split AGENTS.md — stable ≤150 LOC + history archived + handoff; renumber BACKLOG SDDK2-004→006 / SDDK2-005→007; reconcile vault ID collision

## [1.9.1] - 2026-08-11

Cosmetic fixes and documentation improvements from post-release cleanup.

### Other
  - chore(docs): bootstrap.sh — rename `SHARED_DIR` → `SDDK_FRAMEWORK_ROOT` for clarity; the variable always pointed to the CWD but the name was misleading
  - docs(sdk): add "resolved state" section to SPEC.md documenting the 2026-08-08 elimination of `~/.sddk-shared/` and current verified state

## [1.9.0] - 2026-08-11

Guided Runner UX (F13, M-002): a human-governed UAT flow with immutable sign-off, stale advisories, blind checks, evidence gates, checkpoints, diagnostics, and designer/runner/reviewer modes. Minor bump for the new RF-024..028 capabilities, 13 domain types, and plan schema v4.

### Features
  - feat(uat): F13 Guided Runner UX — immutable SHA-256 sign-off, stale advisory, blind checks with evidence gate, checkpoints with AI diagnostics, and designer/runner/reviewer modes
  - feat(uat): RELEASE ACCEPTANCE wizard with immutable acceptance records and release gate integration for RF-024..028
  - feat(domain): 13 UAT domain types for runner modes, blind checks, completion policies, checkpoints, diagnostics, acceptance, and staleness
  - feat(uat): plan schema v4 with backwards-compatible parsing of schema v3

## [1.8.1] - 2026-08-11

Endurece el CI local (act + podman): el lint de `dev-doc-check` ya enforza SDK009/SDK010 para los docs/inventory regenerados, así que los steps redundantes `generate docs/inventory --check` en `ci.yml` se eliminan (bajo `act` con bind mount el check directo daba falsos "stale"). Patch bump por fix + chore (sin features nuevas); el CI local queda verde con un solo gate lint.

### Fixes
  - fix(ci): eliminar steps redundantes de `generate docs/inventory --check` — el lint `dev-doc-check` ya valida SDK009/SDK010 (sha256-pinned entries, INVENTORY sync) como gate único de los docs/inventory regenerados; bajo `act` con bind mount el check directo daba falso stale y hacía fallar el workflow aunque el contenido estuviera sincronizado

### Other
  - chore(style): `cargo fmt --all` en workspace — uniforma el estilo de los 7 crates; 72 diffs (20 del ciclo surface-brevity + 52 pre-existentes de v1.7.0); CI local (act) verde

## [1.8.0] - 2026-08-11

Cierra la deuda INC-001 (surface-brevity-standard) y formaliza el estándar de concisión de superficies (ADR-016). El orquestrador pasa de 1366 líneas a un shell de 288 que delega MCW/políticas/tablas a `prompts/sddk/`; el doctor detecta superficies que exceden el umbral y subdirectorios vacíos. Minor bump por dos features (`feat(dev)` ×2) más un refactor estructural.

### Features
  - feat(dev): `sddk dev doctor` surface.briefness — detecta agentes/skills/prompts que exceden el umbral (300/150/200 líneas); `--strict` promueve la violación a exit 1; por defecto es advisory en el report
  - feat(dev): `sddk dev doctor` surface.empty_dirs — detecta subdirectorios vacíos o phantom en agents/skills/prompts; se mantiene advisory bajo `--strict` (no auto-elimina); elimina la skill fantasma `skills/logseq-vault/`

### Refactors
  - refactor(agents): `agents/orchestrator.md` shell ≤300 — extrae arsenal, dynamic-workflow, escalation-policy, status-query, entropy-policy y document-catalog a `prompts/sddk/`; routing A–D, gates y comandos preservados; tabla MCW step index retirada del shell
  - docs(adr): ADR-016 surface-brevity — agentes ≤300 / skills ≤150 / prompts ≤200 líneas; estructura Pocock (frontmatter + workflow + examples); sin excepciones nominales; `sddk dev doctor` lo enforza como advisory, `--strict` lo promueve

### Other
  - chore(agents): prune `skills/logseq-vault/` (skill fantasma, directorio vacío preexistente; el doctor lo detectaba pero no lo eliminaba)
  - chore(agents): `skills/_shared/` se mantiene como referencia técnica no-namespace (no es skill ejecutable; queda fuera del scope doctor)

## [1.6.1] - 2026-08-10

Endurece la release local CI/CD-independent: el workflow SDDK no depende de ningún sistema CI/CD (CI/CD queda como distribución opcional posterior al tag), con reconciliación idempotente de receipts, precondiciones de trunk/HEAD/cycle y autorización efectiva de `git.inspect`. Patch bump por refactor + fix (sin features nuevas).

### Fixes
  - fix(release): endurecer release local CI/CD-independent — recibos `git.push`/`git.tag` `Started` reconciliados contra el efecto remoto por SHA (los pre-efecto se reintentan, los post-efecto cierran sin duplicar), ciclo ligado a trunk/HEAD (exige trunk limpio y `HEAD` ancestro del commit del manifest), `--cycle` propagado por CLI/agente/skill/prompt, `git.inspect` añadido a la autorización efectiva, orden release → archive coherente y prohibición de comandos ejecutables PR/CI/CD del proveedor

### Other
  - refactor(release): desacoplar workflow SDDK de CI/CD — ruta de release local `validate → push main → verificar SHA remoto → tag anotado` idempotente; Forge integración opcional, nunca gate ni autoridad; precondiciones locales exigen trunk limpio y `HEAD` ancestro del commit del manifest

## [1.6.0] - 2026-08-10

Consolida la integridad UAT fail-closed (P0) y el vault persistente por identidad estable (P1), cierra el loop dashboard → control plane (wizard → ingest), normaliza las superficies a `sddk-*` con cero intrusión (ADR-0011) y elimina el segundo checkout `~/.sddk-shared/` a favor del modelo asdf-vm (CWD + bundle XDG). Minor bump por las dos features (`feat(uat)` + `feat(persistence)`).

### Features
  - feat(uat): integridad UAT fail-closed con gate de release (P0) — el gate `release-uat-approved` exige sesión humana con verdict y verifica build fingerprint (commit/branch/tag/dirty) antes de permitir el tag; `sddk uat gate release --tag X` emite `BLOCKED`/`ALLOWED` con recovery plan cuando hay mismatch
  - feat(persistence): vault por identidad estable con CLI knowledge (P1) — `sddk vault <id>` resuelve el vault XDG del proyecto por identidad (no por path), `sddk knowledge` añade listado/búsqueda/export del vault (markdown + JSON)
  - feat(uat): schema v2 — plan con `context.{user_story, preconditions, workspace, timing, help, failure_protocol, postconditions, test_data}`, session con `metadata.{tester, env_fingerprint, build, duration_ms}`, evidence tipada (`file | screenshot | command_output | assertion | metric | note`), risk + automation + provenance, manifest XDG-resident con sha256-pinned entries + `sddk uat verify-integrity` (exit 0=ok / 0=partial / 1=fail)
  - feat(uat): history aggregator — `sddk uat history --release X --plan P --sessions S1 [--sessions S2 ...]` con per-scenario `runs_total/passing/failing/blocked`, `success_rate`, `flakiness_score`, `first/last_run` (con commit + tester_id), `defect_ids[]`, `avg/p95_duration_ms`, `trend`
  - feat(uat): wizard v2 (browser) — pre-flight checklist, sticky context bar (window/est-ceiling/risk/help), typed steps (shell/api → `<pre>`, ui/file/manual → prose), typed evidence capture por `evidence.kinds[]`, failure protocol flow con checklist + auto-filled defect template + clipboard copy + `linked_defect`, teardown checklist, persistent tester id `T-XXXX`
  - feat(uat): wired dashboard → control plane — `sddk uat open` levanta HTTP server en `127.0.0.1:0` (OS-assigned), wizard POSTea `/ingest`, server cierra con Ctrl+C vía `AtomicBool` shutdown flag. Mismo origen (GET / sirve el wizard HTML) → sin CORS
  - feat(uat): suggester + apply — `sddk uat scenario-context --plan FILE [--apply]` reglas deterministas (timing desde `est_minutes`, preconditions desde `step.kind`, risk desde `priority`, evidence default Note, automation Manual, provenance desde plan metadata); `user_story` queda placeholder para humano/LLM

### Fixes
  - fix(uat): wizard script order — `storage.js` debe cargar antes de `plan.js`/`wizard.js` (window.storage undefined rompía init)
  - fix(uat): collapse nested if-let en `apply_suggestion` user_story branch (clippy collapsible_if)
  - fix(uat): `uat history` acepta `--sessions X Y` (positional, `num_args = 1..`) además de `--sessions X --sessions Y`
  - fix(docs): replace all `.sddk-shared/` paths con CWD + XDG bundle runtime — 12 referencias en 8 archivos (AGENTS.md, docs/, scripts/, knowledge vault)

### Other
  - refactor(namespace): normalizar superficies a `sddk-*` y cero intrusión (ADR-0011) — `orchestrator`/`sddk-*`/`prompts/sddk/` activos; aliases `sdd-*`/`sdd-kernel-*`/`gentle-orchestrator` eliminados; cero ficheros framework plantados en repos de proyectos
  - docs(agents): AGENTS.md — directorio layout (asdf-vm inspired) + regresiones detectadas + recovery procedures + pre-commit checklist + 3 roles (repo de desarrollo / bundle runtime / workspace de uso) + resolution order
  - docs(agents): add session handoff section (current state + next steps) — qué está implementado, qué queda pendiente, cómo reabrir la sesión
  - docs(generated): regenerar inventory/workflow y alinear SPEC/BACKLOG/ADR con cero intrusión — alineado con RS-2026-08 / CP-2026-08

## [1.5.3] - 2026-08-07

Cierra U5 del milestone UAT-2026-08: el gate `release-uat-approved` deja de ser inerte — ahora se evalúa contra la config del proyecto (XDG) por tipo de release.

### Features
  - feat(uat): `sddk uat config show|set` — config per-proyecto XDG-resident (`~/.local/share/sddk/projects/<id>/uat.toml`): política `release_gate` por tipo (major/minor/patch → required/skip/advisory), `human` (developer/architect availability), `activation` (umbrales min_features/min_diff_lines/critical_domains). Default: major+minor=required, patch=skip.
  - feat(uat): `sddk uat gate release --tag X [--previous-tag Y|--release-type major|minor|patch]` — evalúa `release-uat-approved` para el release type derivado (semver diff). Emite `BLOCKED` con plan de recovery cuando `required`, `ALLOWED` cuando `skip`/`advisory`. JSON para orquestadores.
  - feat(uat): `UatConfig` + `ReleaseGateAction` (required/skip/advisory) + `ReleaseType` (major/minor/patch) en `sddk-domain`. Funciones puras: `evaluate_release_gate()`, `release_type_from_diff()`.

### Fixes
  - fix(uat): gate `release-uat-approved` ya no es inerte — antes declarado sin requires en transiciones; ahora evaluado dinámicamente por el orchestrator antes de tagear.

## [1.5.2] - 2026-08-07

Consolida el milestone UAT-2026-08 (U1-U7) y las correcciones post-1.5.0: cierra el loop humano end-to-end (wizard canónico → ingest → failures → agente estudia).

### Features
  - feat(uat): `sddk uat plan/validate/dashboard/ingest/report/status` — data-driven YAML canónico (ADR-0012)
  - feat(uat): `sddk uat open` — render dashboard + abrir en navegador del sistema sin servidor (file://); SO-aware (xdg-open/open/cmd-start); `--browser` override
  - feat(uat): `sddk uat failures` — lista FAIL/BLOCKED con contexto completo (feature, priority, assignee, rationale, comment, evidence); JSON para que el agente estudie cada fallo
  - feat(uat): dashboard kit en bundle (`assets/uat-dashboard/`) — kit/templates/views (guided/matrix/traceability); templates HTML inlinean JS+CSS (100% autocontenido, ADR-0010)
  - feat(uat): workflow fase `uat` + status `UAT_WAITING` + gates `uat-activated/uat-verdict/release-uat-approved` (ADR-0012)
  - feat(uat): control plane `uat_results` (verdict, coverage, defects por tag_version) + panel "UAT readiness" en dashboard de telemetría
  - feat(uat): 4 agentes (`uat-planner/guide/runner/reporter`) + 4 skills (`uat-dashboard/traceability/guided-mode/evidence`)

### Fixes
  - fix(uat): views HTML inlinean storage.js/components.js (Chrome bloqueaba scripts file:// vía CORS — el HTML ahora es 100% autocontenido y abre vía file://)
  - fix(uat): wizard canónico — `Finalizar y exportar reporte` genera JSON con la forma exacta de `UatSession` (schema_version, executor, executed_by, started_at, finished_at, results con evidence por hash); compatible directo con `sddk uat ingest`
  - fix(uat): guard de integridad en `uat ingest` — `executor: human` exige `executed_by` + `finished_at` + (evidencia o non-PASS); rechaza sesiones humanas fabricadas
  - fix(agents): `uat-planner` craft rule 9 — quoting YAML-safe (textos con `:` rompen el plan; hallazgo del dogfooding)
  - fix(skills): contradicciones ADR-0011 v3.5 — `adopt apply` ya no planta `workflow/workflow.yaml`; política Local-Only v3.3→v3.5 (docs al knowledge vault)
  - fix(tests): workspace completo verde 202+ tests (AdoptionStoragePaths new fields en test domain + unused binary)

## [1.4.0] - 2026-08-07

### Features
  - feat(uat): milestone UAT-2026-08 U1-U7 — dashboard kit en bundle (assets/uat-dashboard), dominio uat.rs, CLI uat plan/validate/dashboard/ingest/report/status, workflow fase uat + status UAT_WAITING + gates uat-activated/uat-verdict/release-uat-approved, control plane uat_results + panel "UAT readiness" en dashboard telemetría, agentes uat-planner/guide/runner/reporter + 4 skills (ADR-0012/0013, RF-019/020, RNF-010)
  - feat(uat): U8 dogfooding parcial — uat-plan v1.5.0 (6 features, 13 escenarios), dashboard guiado generado y validado (determinismo, cero URLs externas); la sesión humana queda PENDIENTE de validación real (la sesión inicial fue fabricada por el agente y eliminada del control plane)
  - fix(agents): uat-planner craft rule 9 — quoting YAML-safe (colon-space rompe el plan; hallazgo del dogfooding)
  - fix(skills): contradicciones ADR-0011 — adopt no planta workflow.yaml (C1/C2), política Local-Only v3.3→v3.5 (C3/C4, docs al knowledge vault)
  - fix(tests): workspace completo verde — AdoptionStoragePaths new fields en test domain + unused binary (202 tests PASS)
  - feat(telemetry): G5 research packet cross-proyecto — analytics research --all-projects desde control plane + resumen por proyecto (CP-2026-08)
  - feat(rs): RS-6 resolución de versión asdf — sddk version con .sddk-versions → current → path: (ADR-0011)
  - feat(rs): RS-5 bundle runtime multi-versión — dev use (asdf-style) + dev link/update resuelven framework activo (ADR-0011)
  - feat(rs): RS-4 generate docs/inventory → XDG por defecto con --in-repo explícito (ADR-0011)
  - feat(rs): RS-3 cycle artifacts en XDG — cycle artifacts-dir + prompts/skills a {cycle-artifacts-dir} (ADR-0011)
  - feat(rs): RS-1 multiplataforma dirs + RS-2 adopt/lint no intrusivos — cero ficheros framework en repos de proyectos (ADR-0011)
  - feat(telemetry): control plane local — telemetry ingest/aggregate/status/dashboard + metrics record upsert (CP-2026-08 G1-G4)
  - feat(distribution): ALL Linux builds standalone (musl static) — aarch64 included (#92)
  - feat(validation): E2E suite — install variants, render, multi-language validation (#91)

### Fixes
  - fix(rs): framework_agent_names fallback a agentes del bundle sin permissions.yaml (RS-7 migración)
  - fix(ci): update release PR branch when behind before auto-merge (#83)
  - fix(ci): tag-release reads version from origin/main, not dirty worktree (#86)

### Other
  - docs(control-plane): CP-2026-08 IMPLEMENTADO — README control plane, milestone cerrado, backlog 49/49 (ADR-0009/0010)
  - docs(roadmap): RS-2026-08 IMPLEMENTADO — milestone cerrado, backlog E12 completa (ADR-0011)
  - docs(roadmap): milestones CP-2026-08 (ADRs 0009/0010) + RS-2026-08 (ADR-0011, modelo asdf, multiplataforma) — specs, PRD, backlog
  - docs(control-plane): CP-2026-08 milestone — ADRs 0009/0010, spec, PRD RF-016/017, roadmap, backlog E11
  - docs(validation): N3 editor checklist PASS — E2E-2026-08 fully closed (8/8)
  - docs(roadmap): E2E-2026-08 milestone implemented — 7/7 suites PASS (#91)
  - test(cli): environment-robust doctor test; docs: local-first CI via act (#90)
  - docs(validation): E2E validation plan — install, deploy, multi-language, render (#89)

## [1.3.0] - 2026-08-06

### Features
  - feat(cli): completion install — installs shell completions (#84)

### Fixes
  - fix(ci): gh pr list --head does not glob; filter with startswith (#81)

## [1.2.0] - 2026-08-06

### Features
  - feat(ci): release robot — cron poller that removes all bot friction (#79)

### Fixes
  - fix(ci): dispatch Release workflow explicitly from tag-release (#77)

## [1.1.0] - 2026-08-06

### Features
  - feat(ci): fully automatic release pipeline (#71)
  - feat(distribution): hardened installer, completions, signed assets, brew tap (#66)
  - feat(install): interactive installer with framework release bundle (#64)

### Fixes
  - fix(ci): trigger via Auto-merge workflow_run + anti-loop (#74)
  - fix(ci): extract pending tag from explicit new-tag line (#73)
  - fix(ci): reindent release PR body block (invalid YAML) (#72)
  - fix(install): cosign identity regexp + dev update creates missing root (#70)
  - fix(release): pin cosign v3.8.1 — v4.1 sign-blob breaks on output-signature/certificate (#69)
  - fix(release): build darwin-x86_64 cross from arm64 runner (macos-13 retired) (#68)
  - fix(agents): normalize frontmatter models to provider-qualified names (#63)

### Other
  - docs(validation): v1.0.0 published (#62)

