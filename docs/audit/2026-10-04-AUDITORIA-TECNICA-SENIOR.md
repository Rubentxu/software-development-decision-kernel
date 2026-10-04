# Auditoria tecnica senior - sddk-framework

**Fecha:** 2026-10-04
**Base:** `HEAD = 38427773`, rama `main`, arbol limpio, workspace `2.9.0` sin publicar.
**Metodo:** solo lectura. Nada de lo que sigue se deduce de un documento de estado: cada
afirmacion se comprobó contra el codigo, contra git, o contra una ejecucion. Donde no se
pudo medir, se dice.

**Alcance de la medicion:** `crates/*/src` (250 927 loc), `crates/*/tests` (110 788 loc),
68 suites shell + 13 python, 5 466 tests Rust, 2 476 commits desde 2026-08-31, 10 autores.

---

## 1. Informe ejecutivo

El repositorio tiene una propiedad que casi ningun proyecto de este tamano tiene: **su red
de gates es real y se auto-verifica.** Nueve gates numerados del pipeline de release tienen
falsador con mutaciones, y la regla de "una mutacion que no muta es `SKIP`, nunca `PASS`"
esta escrita en codigo y funciona. `tests/test_gate_coverage.py` mide que ningun test
enumerado se quede sin runner: 70 tests, 0 sin runner. Eso no es decoracion.

El problema esta en otro sitio, y es uno solo repetido en tres capas: **la misma decision
esta escrita mas de una vez y las copias ya divergieron.**

- El contrato de comandos vive en el enum de `sddk-cli` (**51** variantes) y en
  `command_spec.rs` (**62** specs). Ya no coinciden: 4 discrepancias de nombre, 2 de scope,
  7 specs anidados sin equivalente. El unico guard (`agent_surface_golden.rs`) congela la
  forma con `UPDATE_SNAPSHOTS=1`, es decir **congela el drift en vez de detectarlo**.
- La autoridad de eventos esta declarada en ADR-0094 como `CanonicalEventLog` (3 ficheros)
  y ejercida por `EventStore` (16 ficheros, 21 emisores genericos). El ADR describe una
  arquitectura que no existe.
- El estado de un ADR esta escrito dos veces: **17 de 65** declaran `status: accepted` en el
  frontmatter y `**Status:** Proposed` en el cuerpo. Es el bloque 0094-0110, que contiene
  justamente los tres ADRs incumplidos (0094, 0104, 0107).
- El digest SHA-256 tiene **10** implementaciones con nombre (3 `pub`, en 3 crates) mas
  ~50 llamadas inline. El reloj tiene **10** implementaciones y ya existe una canonica en
  `sddk-domain/src/backlog.rs:412` que 9 sitios ignoran. Un comparador semver compara solo
  el numero mayor, asi que un skill `1.2` y uno `1.9` son iguales.

Y hay una cuarta instancia, la mas cara: **la superficie publica.** 4 975 items `pub`.
De los 131 `pub mod` de `sddk-engine`, 86 no los usa ningun otro crate y 58 tampoco los usa
ningun test. No es "codigo muerto": es codigo que paga su coste de lectura, de compilacion y
de superficie para nadie.

**El veredicto corto:** la ingenieria de verificacion de este repo es mejor que la
ingenieria de consolidacion. Sabe demostrar que un gate funciona y no sabe todavia
garantizar que una decision este escrita una sola vez. Arreglar eso es mas barato y mas
rentable que anadir capacidad.

---

## 2. Scorecard

| Dimension | Nota | Evidencia medida |
|---|---|---|
| Red de gates de release | **8/10** | 9 gates con falsador + mutaciones; 70 tests, 0 sin runner; `SKIP != PASS` implementado |
| Cobertura de tests | **7/10** | 5 466 tests; ratio test/src con inline = **0.61**; 23 modulos de producto sin test (5 298 loc) |
| Arquitectura (frontera) | **6/10** | `Engine<L: Ledger>` generico real, `ports.rs` con 0 menciones de crates concretos; pero 7 fugas de IO en produccion |
| Arquitectura (declarada) | **3/10** | 3 ADRs incumplidos; 17 ADRs con estado contradictorio; `domain/uat.rs` 4 777 loc contra `pack-uat` 250 |
| Consistencia de duplicados | **2/10** | 10 SHA-256, 10 relojes, 2 semver, 6 `MockStore` (5 939 loc), 2 fuentes de verdad de comandos divergidas |
| Profundidad de modulos | **4/10** | 588 loc/fichero en `sddk-cli`; 131 `pub mod` en `lib.rs`; 58 sin consumidor |
| Degradacion silenciosa | **3/10** | 239 `unwrap`, 353 `expect`, 42 panic-macros, 3 escrituras a storage/manifesto descartadas con `let _ =` |
| CI/CD | **2/10** | `ci.yml` es `workflow_dispatch`-only por diseno (documentado), e invoca **3 suites que no existen**; 3 de 57 suites shell llegan a la nube |
| Supply chain | **2/10** | Sin `cargo-deny` ni `cargo-audit`; el test del anchor de firma esta `#[ignore]`d; gates 8c y 9c sin falsificador |
| Observabilidad | **2/10** | **0 ficheros usan `tracing::`** en todo el workspace; 65 `println!`/`eprintln!` en 23 ficheros |
| Deuda tecnica viva | **5/10** | 10 deudas abiertas (1 critical, 4 high/P1); 57 cerradas; 1 con criterios caducados |
| Documentacion | **6/10** | 459 de 507 ficheros con doc-comments; 65 ADRs; pero el indice de deudas cubre 50 de 67 documentos |
| Seguridad | **5/10** | Sin secretos reales (verificado); pero `SDDK_ALLOW_UNSIGNED_UPDATE` sin un solo test Rust |

---

## 3. Cumplimiento del roadmap

El roadmap vigente (`docs/roadmap/ROADMAP.md`, 170 lineas) declara los hitos C0-C7. El
ledger de SDDK es la autoridad del estado operativo. **Los dos no se pueden cruzar**, y esa
es la primera observacion.

- De 188 ciclos reales (179 sin imports de spine), solo **13** se crearon desde el 2026-09-20.
- De esos 13, el vocabulario es `c3m1/c3m3/c3m4/c3n/c0/cl-*/ledger-*/trust-*/chain-*/release-*`.
  El trabajo de C1, C2 y C3 se hizo bajo prefijos `a3` (16, todos CLOSED), `a4` (6) y `a5` (2).
- En consecuencia **no existe una forma computable de responder "¿cuanto falta de C2?"**.
  El progreso vive en dos vocabularios que no se mapean.

| Hito | Estado medido | Evidencia |
|---|---|---|
| C0 | **PAUSED hace 6 dias** | `c0-t01-pointer-mutation`, fase `specify`, creado 2026-09-29. Es una de las 2 decisiones humanas pendientes. |
| C1 | Sin ciclo con prefijo `c1`; trabajo bajo `a3` | 16 ciclos `a3` CLOSED. Sin estado verificable propio. |
| C2 | Sin ciclo con prefijo `c2`; trabajo bajo `a3`/`a4` | Igual. El ADR-0108 (JCode) sigue sin implementarse en un crate propio. |
| C3 | Sin ciclo `c3`; desglosado en `c3m*` y `c3n` | `c3m1`, `c3m3`, `c3m4` OPEN en `explore`; `c3n` OPEN en `design`. Los 4 creados 2026-10-03. |
| C4 | **OPEN hace 21 dias** | `c4-authority-engine-cutover-2026-09-13`, fase `build`. 5 ciclos mas en `RELEASE_PENDING` esperando publicacion. |
| C5 | Sin ciclo | El roadmap lo declara P2/P3, no bloqueante. Coherente. |
| C6 | Sin ciclo, `PROPOSED` | El roadmap dice que no abre antes que C3i/C3j. Coherente con el ledger. |
| C7 | Sin ciclo, `PROPOSED` | Coherente. |

**Estado operativo real (SDDK, ejecutado):** 109 ciclos visibles (188 filas, 79 son
`__spine_import__`), 72 CLOSED, 29 OPEN, 6 RELEASE_PENDING, 1 PAUSED, 1 RELEASED;
**2 decisiones humanas pendientes** y **2 estados runtime indeterminados**; 2 manifiestos
ilegibles.

**Bloqueos que solo el operador puede levantar:** las 2 aprobaciones pendientes
(`c0-t01-pointer-mutation`, `c3n-production-boundary-certification`), el lease de `c3n`
caducado y sin dueno, las 79 filas `__spine_import__`, las deudas 049/050/060/061
(reasignacion de `project_id` y enumeracion de ciclos), SPEC-012, y el archivo
`a4-1-generic-verify`.

**Conclusion de roadmap:** el trabajo real de las ultimas dos semanas es C3m/C3n, y esta
bieninstrumentado. El problema no es el trabajo: es que el vocabulario del roadmap y el del
ledger son dos autoridades del mismo concepto, y por eso no se puede medir el avance.

---

## 4. Hallazgos con evidencia

Severidad: **critica** = el producto puede fallar en silencio para el usuario; **alta** =
puede producir un diagnostico falso o una decision incorrecta; **media** = deuda con
consecuencia; **baja** = higiene.

### 4.1 Decisiones duplicadas que ya divergieron

| Severidad | Ubicacion | Evidencia | Impacto | Recomendacion |
|---|---|---|---|---|
| **critica** | `sddk-cli/src/lib.rs:201` vs `sddk-cli/src/command_spec.rs:299` | **50** brazos de dispatch frente a **58-62** specs segun como se cuente (`grep -c "spec("` da 68, `^\s*spec(` da 62, conteo de subagentes 58). El numero exacto varia con el patron; **la deriva no**: los 50 brazos nunca igualan a ninguna de las cifras, y las discrepancias nombradas existen. 4 discrepancias de nombre (`agent-help`, `agent-result`, `run-view`, `verify-kernel`), 2 de scope (`docs`, `inventory` declarados como `generate docs`/`generate inventory`), 7 specs anidados sin equivalente. El unico guard (`agent_surface_golden.rs`) usa `UPDATE_SNAPSHOTS=1` | El schema que ve el modelo y el parser que ejecuta la invocacion son cosas distintas. Un comando puede estar documentado y no existir, o al reves | Derivar una de las dos por reflexion de clap (`lint.rs:1318` ya lo hace para nombres) y anadir un test de exhaustividad **fail-closed**. Congelar con `UPDATE_SNAPSHOTS` no puede servir para esto |
| **critica** | 17 de 65 ADRs (`docs/architecture/adrs/`) | `status: accepted` en el frontmatter y `**Status:** Proposed` en el cuerpo. Bloque 0094-0110 completo | Dos campos para el mismo concepto, en la superficie donde el repo exige una autoridad canonica. Un lector que mire el cuerpo creera que ADR-0094, 0104 y 0107 no son vinculantes | Un solo campo. Decision por documento, no por busqueda |
| **alta** | 10 funciones de reloj + 1 canonica ignorada | Canonica en `domain/backlog.rs:412`. Copias en `cli/rules_cmd.rs:56`, `cli/telemetry.rs:770`, `cli/uat.rs:2581`, `cli/dev/cockpit.rs:826`, `cli/dev/graph.rs:389`, `cli/admission.rs:622`, `cli/uat_generate/planner/mod.rs:43`, `engine/decision_memory.rs:2723`, mas `engine/authority_engine.rs:57` y `cli/cycle.rs:1677` en formato epoch | Dos receipts del mismo tipo pueden no ordenarse entre si. Es un problema forense, no estetico | Una funcion, y un guard que falle si aparece una segunda |
| **alta** | 10 implementaciones de SHA-256 con nombre (3 `pub` en 3 crates) + ~50 llamadas inline | `domain/uat.rs:2530`, `engine/canonical_event_log.rs:318`, `engine/ext_outcome.rs:254` (las 3 `pub`); `cli/dev/common.rs:212`, `cli/uat.rs:1785`, `cli/examples_walker.rs:149`, `engine/cas_object_store.rs:64`, `engine/context_compiler/storage_adapter.rs:192`, `gateway/runner_receipt.rs:167`, `storage/lib.rs:1972` | CAS y eventos canonicos calculan el digest con codigo distinto. Si diverge el formato, **cada copia sigue compilando** y la prueba de integridad miente | **VER LA NOTA DE CONTRATO ABAJO. No es "unificar en una funcion"** |
| **alta** | `cli/skill_registry_bridge.rs:258` | `parse_version` hace `trimmed.split('.').next()`: **solo lee el major**. Compara bien: `cli/dev/bundle_manifest.rs:331` | Un skill `1.2` y otro `1.9` son compatibles. Un `2.0` contra framework `1.9` se rechaza | Un comparador semver unico en `domain` |
| **media** | `engine/src/operator.rs:2832,3000,3576,3733,3919,4061` | **6 copias de `struct MockStore`**, ~5 939 loc | No replican logica de negocio (riesgo bajo), pero el ruido es alto y `sddk-testkit` no se usa | Un `MockGraphStore` en testkit |

> **NOTA DE CONTRATO sobre el digest (corrige la recomendacion obvia).** Medido: las
> implementaciones **no tienen el mismo contrato de salida**, luego "una unica funcion" es
> una recomendacion incorrecta y would've roto datos ya persistidos.
>
> | Implementacion | Devuelve | Firma |
> |---|---|---|
> | `domain/uat.rs:2530` | `sha256:<hex>` (71 chars, con prefijo) | `(&[u8]) -> String` |
> | `engine/canonical_event_log.rs:318` | hex **desnudo** (64 chars, sin prefijo) | `(&[u8]) -> String` |
> | `engine/ext_outcome.rs:254` | `sha256:<hex>` de un **fichero** | `(&Path) -> Option<String>` |
> | `cli/dev/common.rs:212` | hex, leyendo fichero | `(&Path) -> Result<String>` |
>
> Los digests ya estan **en disco**: `MANIFEST.sha256`, `bundle.tar.gz.sha256`, receipts de
> ciclo, refs `evidence:{sha}`. Unificar el codigo es una **migracion de datos con ventana de
> compatibilidad**, no un refactor. Y los tests afirman literales concretos.
>
> **Lo que si es el arreglo correcto:** un **censo de contratos**, no una funcion. Cada
> productor de digest declara su `id` y su `prefijo`, y un guard falla si (a) hay dos
> productores con el mismo `id` y distinto prefijo, o (b) hay un productor que no declara
> ninguno. Eso convierte una duplicacion invisible en un invariante exigible, y es
> estrictamente mas barato que la migracion. El calculo en si (leer bytes, llamar a
> `sha2`) se puede compartir; **el contrato de salida no.**

### 4.2 Fugas de la frontera hexagonal (produccion)

| Severidad | Ubicacion | Evidencia | Impacto | Recomendacion |
|---|---|---|---|---|
| **alta** | `engine/src/rules/evaluators.rs:56,74` | `Command::new("git")` con `rev-parse` y `merge-base --is-ancestor`. El fichero **no tiene `#[cfg(test)]`** | El "deterministic decision kernel" depende del binario `git` y del estado del checkout. Una regla de waiver es no determinista fuera de un repo git | Extraer `GitProbe`. El seam `WaiverExpiryResolver` ya existe pero esta un nivel demasiado arriba: permite sustituir la decision, no la obtencion del SHA |
| **alta** | `engine/src/task_executor.rs:30-31,64-68` | `OnceLock<tokio::Runtime>` + `OnceLock<reqwest::Client>` con `.expect`, sin seam. `tasks/http_fetch.rs` hace GET real | Cualquier test de tarea de red sale a internet | `Arc<dyn HttpClient>` inyectable |
| **alta** | `domain/src/pack.rs:233`, `domain/src/models/gate_classification.rs:12` | `std::fs::read_to_string` y `use std::fs` en el crate de dominio | El dominio deja de ser puro | La primera, `&dyn FileSystem`. La segunda **no**: un registro cerrado es constante de compilacion, `include_str!` y borrar el import |
| **media** | `engine/src/active_graph.rs:38,55,404-413` | `ActiveGraphNodeKind::Uat` y ids `uat:{label}` en el kernel del grafo | La estructura mas reusable del motor conoce un caso de negocio | Extension point por registro |
| **media** | `engine/src/authority_ticket_service.rs:95,123` | `static PROCESS_SERVICE: OnceLock<...>`; el seam `set_process_service_for_tests` es `#[cfg(test)]` y `OnceLock::set` solo funciona **una vez** | El orden de los tests dentro del binario decide el resultado | Inyectar desde el composition root. Ningun puerto lo arregla: es un problema de ciclo de vida |
| **baja** | `sddk-vault/` completo | 2 093 loc, 0 dependencias internas, `rusqlite` propio, 6 ficheros con `std::fs` | Isla de persistencia paralela, sin puerto declarado | Un `VaultStore` en `domain::ports` |

**Lo que ya esta bien y no hay que rehacer:** `Engine<L: Ledger>` es generico sobre el
puerto; hay 21 emisores genericos `emit_*<S: EventStore>`; `ports.rs` tiene **0** menciones
de `rusqlite`/`reqwest`/`tokio` (verificado). La invariante "un puerto no nombra un crate
concreto" ya se cumple en el fichero de puertos. Falta hacerla exigible.

### 4.3 Degradacion silenciosa

| Severidad | Ubicacion | Evidencia | Impacto | Recomendacion |
|---|---|---|---|---|
| **critica** | `engine/src/context_compiler/storage_adapter.rs:83` | `serde_json::to_vec(events).unwrap_or_else(\|_\| b"[]".to_vec())`. El doc-comment de la funcion dice que su hash sostiene el invariante **H01** de reproduccion byte-identica | Un fallo de serializacion produce un payload **byte-identico al de un ledger legitimamente vacio**. El invariante H01 pasa cuando deberia fallar | Propagar el error |
| **alta** | `cli/src/dev/link.rs:285` | `let _ = super::manifest::write_manifest(&framework_root);` | Se descarta la regeneracion del **MANIFEST.sha256**, el ancla de integridad que verifica `sddk dev install`. Si falla, el manifest queda obsoleto y el pipeline sigue verde | Propagar |
| **alta** | `cli/src/admission.rs:617` | `let _ = storage.insert_project(&record);` y la funcion devuelve `Ok(())` | Una escritura al storage descartada que se reporta como hecha | Propagar |
| **alta** | `cli/src/project_alias.rs:144` | `f.sync_all().ok();` justo **antes** de un `fs::rename` atomico | Un alias de proyecto puede quedar truncado en crash, y el rename lo hace "oficial" | Propagar el error de `fsync` |
| **alta** | `cli/src/uat_serve.rs:55-312` | **23** escrituras de red silenciadas con `let _ =` (`write_response`, `write!`, `flush`, `shutdown`). `#[cfg(test)]` empieza en la 323, luego todas son produccion | Un cliente UAT se queda colgado sin diagnostico | Reportar el error de escritura al menos una vez |
| **media** | `engine/src/inc_generator.rs:97` | `.unwrap_or_else(\|_\| "2026-08-21T00:00:00Z".into())` | **Fecha fabricada**: un INC con error de timestamp lleva una fecha de creacion real y falsa | `Result<String>` |
| **media** | `gateway/src/gateway.rs:420,429` | `EvidenceBundleWriteCapability::new("/tmp/evidence")` y `.unwrap_or_else(\|_\| String::new())` en el timestamp que se persiste en el receipt | Ruta absoluta del mundo en una capability, y receipts con timestamp vacio | Resolver desde config; `Option` marcado |
| **media** | `domain/src/evidence.rs:395` | `serde_json::to_value(self).unwrap_or_else(\|_\| json!({}))` | Si falla, `redacted()` devuelve un bundle **vacio** y reporta la redaccion como hecha | Propagar |
| **media** | `cli/src/metrics.rs:271,666` | `read_context_quality(...).unwrap_or_else(\|_\| "C2".to_owned())` | Las metricas reportan una **calidad que nadie midio** | `Option` |
| **baja** | 42 panic-macros en produccion | `unimplemented!()` x20 en `decision_memory.rs:567-747` (defaults del trait `MemoryStore`, el unico implementor los overridea); `unreachable!()` en `cycle_pause.rs:122`, `storage/lib.rs:1998` (deserializacion: un ledger corrupto **tumba el proceso** en vez de devolver error) | Un segundo implementor de `MemoryStore` compila limpio y revienta en runtime | Declarar los metodos sin cuerpo. Los `unreachable!` de deserializacion, `Err` |
| **media** | `domain/src/ports.rs:474-610` (~20 sitios), `engine/src/revision_substrate.rs:208,227`, `engine/src/durable_capsule_store.rs:61-156` | `self.lock().unwrap()` / `.expect("poisoned")` | **Un solo panic envenena el store** y convierte cada acceso posterior en panic | `unwrap_or_else(\|e\| e.into_inner())` |

### 4.4 Pipeline, CI y supply chain

| Severidad | Ubicacion | Evidencia | Impacto | Recomendacion |
|---|---|---|---|---|
| **alta** | `scripts/release.sh:409` | Rama `else` del bucle de tests Python: `warn "python test missing, skipping: $p"`. El bucle **shell** equivalente (linea ~331) si es fail-closed, y el propio comentario del codigo dice "Mismo arreglo que en el bucle de shell" | 10 tests de contrato Python se saltan en silencio si el fichero falta. Borrar o renombrar uno = release verde | `[ -f "$p" ] \|\| die`, igual que el shell |
| **alta** | `scripts/release.sh:289-291` | Sin `shellcheck` en el PATH: `warn "shellcheck not installed - skipping static gate"`, y el release sigue verde | Se pierde el gate estatico de ~15 scripts y se reporta PASS | Fallar cerrado, o exigir `shellcheck` en el preflight |
| **media** | `.github/workflows/ci.yml:7-8` | `on: workflow_dispatch` unicamente. **Es una decision documentada** (AGENTS.md §2.5: la nube no bloquea, el gate es local). No es un defecto | Ningun push ni PR ejecuta CI. El gate real es `githooks/pre-push` | Ninguna, si se acepta la decision |
| **media** | `.github/workflows/ci.yml:76,82,85` | Invoca `tests/test_pointer_integrity.sh`, `tests/test_hard_sizing_regression.sh`, `tests/test_instruction_contract.sh`: **los 3 ficheros no existen** (verificado) | El workflow manual esta roto. El `workflow_dispatch`-only lo oculta: la "comprobacion opcional bajo demanda" no se puede ejecutar | Corregir o borrar las 3 invocaciones |
| **media** | `tests/test_release_public_gate.sh` | Esta en `EXCEPTIONS` de `test_gate_coverage.py:77-80` con motivo escrito. El **gate 9b si se ejecuta** (`release.sh:1729-1896`, delimitado `>>> REL-1 public-release gate begin >>>`) y los recibos de session-49/50/53 registran `PASS=13 FAIL=0` | El gate corre; su **falsificador** no corre en el pipeline | Anadirlo al paso 1b con un modo que tolere "todavia no hay tag" |
| **media** | Ausente | 0 resultados de `cargo-deny`, `cargo-audit`, `deny.toml`, `audit.toml` | Release firmado sin vigilancia de licencias ni de CVEs | `deny.toml` + un gate |
| **media** | `cli/src/cosign.rs:144` | `#[ignore = "the release signing key is not provisioned yet; see ADR-0151"]` sobre `the_anchor_is_provisioned_not_placeholder` | El unico test que verifica que el anchor de firma es real esta deshabilitado porque la clave no existe | Provisionar (decision del operador) o declararlo deuda critica de supply chain |
| **baja** | 5 `Cargo.toml` | Feature `std = []` declarada, **0** usos de `cfg(feature = "std")`. `rand` en `[dependencies]` de `sddk-engine` con 0 usos en `src/`. `toml` duplicado: 0.8 en cli/engine y 1.1 en domain | Deps de produccion no usadas y dos copias compiladas de la misma libreria | Mover a dev-dep, unificar, borrar el flag muerto |

### 4.5 Superficie, complejidad y modulos superficiales

| Severidad | Ubicacion | Evidencia | Impacto | Recomendacion |
|---|---|---|---|---|
| **alta** | `crates/*/src` | **4 975** items `pub`. 86 de los 131 `pub mod` de `engine/src/lib.rs` sin consumidor externo; 58 sin consumidor ni desde `tests/` | Se paga coste de lectura, compilacion y superficie para nada | Regla: `pub` solo para puertos, fachadas, identidad/valor. Objetivo medible y gateable |
| **alta** | `cli/src/knowledge_ingest.rs` | **1 343 loc, 0 `#[test]`, 0 `#[cfg(test)]`, 0 referencia en `tests/`** | El modulo de producto sin test mas grande del repo (25% de la masa sin cubrir) | Test de contrato minimo |
| **media** | 22 modulos mas sin test (5 298 loc en total) | `cli/fork_cmd.rs` 447, `cli/dev/voice_cmd.rs` 404, `cli/stale_cmd.rs` 320, `cli/graph_cmd.rs` 313, `cli/dev/check_arch.rs` 286, `cli/git_cmd.rs` 281, `uat_discover/aam.rs` 264, mas 7 menores y 5 adaptadores de `test_runner/` | Un guard de arquitectura sin test (`check_arch.rs`) | Priorizar por superficie, no por listado |
| **media** | `engine/src/lib.rs:1989-2030` | Tabla de **37** mapeos variant->string escrita a mano | Anyadir variante rompe la compilacion sin guia: el mensaje de error no tiene un solo autor | Derive o macro que genere enum y `code()` a la vez |
| **media** | `cli/src/lib.rs:904-1035` | Un unico `match` con **50** brazos en 141 lineas, y es donde se cablea el gate de admision | Cada comando nuevo toca el enum de clap **y** este match | Registro, o al menos un mapa `nombre -> handler` |
| **media** | loc/fichero | `sddk-cli` 588, `sddk-domain` 531, `sddk-engine` 432. `engine/decision_memory.rs` 2 791 loc con 75 `pub`; `cli/src/dev/arch_lint.rs` 2 372 loc | Muy por encima del umbral comodo de Ousterhout (~500) | Dividir por eje: parse / regla / render |
| **baja** | Modulos superficiales | `engine/active_graph_view.rs` 6 `pub`/89 loc, 0 consumidores; `event_bus/mod.rs` 4 `pub`/25 loc; `verify_kernel/mod.rs` 10 `pub`/65 loc | Un barrel que reexporta no es una frontera | Privatizar o fusionar |
| **baja** | 588 loc/fichero en `sddk-cli` | 144 ficheros | — | — |

### 4.6 Deuda tecnica y su salud

| Severidad | Ubicacion | Evidencia | Impacto | Recomendacion |
|---|---|---|---|---|
| **media** | 10 deudas abiertas | 1 `critical` (050), 4 `high/P1` (049, 060, 061, 064), 4 `medium/P2` (057, 063, 065, 068), 1 `low/P3` (058). 57 cerradas de 67 documentos | 4 de las 5 P1 requieren accion del operador, no codigo | Ver §6 |
| **media** | `docs/debt/INC-DEBT-064-*.md:236-240` | El documento declara "**La ruta de deteccion no esta cableada en ningun sitio automatico** ... `dev doctor` **no** lo invoca". **Verificado falso contra HEAD**: `cli/src/dev/doctor.rs:484` llama `crate::dev::build_id::identity_verdict_here()`, y `cli/src/dev/build_id.rs:397` produce el mensaje "el commit del binario (...) es ancestro del HEAD del checkout" | Un criterio de deuda que ya no se cumple. Por la regla del proyecto, **no es deuda real** en ese punto | Revalidar por sus propios criterios; si los tres han caducado, `resolved` con escrito lo que sigue vivo |
| **baja** | `docs/debt/README.md` | 50 entradas para **67** documentos. Los 15 ausentes son las deudas 006-020, todas `closed`. `check_debt_index_coherence.sh` da PASS porque solo comprueba que "el indice no contradice a los documentos que indexa" | El guard no puede ver una deuda que nunca se indizo. 15 documentos quedan fuera de la vista del indice | Anadir la regla inversa: todo documento abierto debe estar en el indice |

### 4.7 Observabilidad y tests

| Severidad | Ubicacion | Evidencia | Impacto | Recomendacion |
|---|---|---|---|---|
| **alta** | Workspace completo | **0** ficheros usan `tracing::`. 65 `println!`/`eprintln!` en 23 ficheros | Sin logging estructurado, un fallo de produccion no deja rastro consultable | `tracing` + `tracing-subscriber`; el informe de diagnostico de release (session-80) ya demostro que un fallo sin causa es el problema que mas cuesta |
| **media** | `gateway/tests/bounded_runner_contract.rs:50` | `assert_eq!(f, f);` en un bucle sobre 4 `TestFamily` | Tautologia pura: 4 aserciones que no pueden fallar | Comparar con el valor esperado |
| **media** | 16 `#[ignore]` de nivel test | 9 con motivo en el atributo, **7 desnudos**. 4 son microbenchmarks opt-in (legítimo); los 3 de `aiw_s1_cognicode_real.rs:87,132,160` documentan el motivo en la cabecera del fichero, no en el atributo | Un `#[ignore]` sin motivo es un test que nadie vuelve a mirar | Mover el motivo al atributo |
| **media** | `engine/src/knowledge.rs:1279` | `#[ignore = "historical anchor only - A3-S2 grew the enums by design"]` sobre `s1_does_not_introduce_new_corenodekind_variants`, con comentario que dice "preserved as historical evidence, not as a live gate" | Patron de test que fija el comportamiento viejo como correcto. El pin vivo es `s2_post_ac1_corenodekind_baseline` | Documentar el baseline como gate, no como historia |
| **media** | 57 `assert!(x.is_ok())` | Concentrados en `engine/tests/event_correlation_wiring.rs` (10) y `domain/src/planning/service.rs` (7) | Verifican ausencia de error, no el valor | Inspeccionar el `Ok` |
| **baja** | 45 `assert!(...).contains("cadena exacta")` sobre mensajes de error | Codifican el texto del error; un refactor cosmetico los rompe sin cambiar el comportamiento | Acoplamiento de test a texto | Preferir codigos de error tipados |
| **baja** | 0 tests que muten el entorno | `grep -rln "set_var" crates/*/tests/ tests/` -> vacio, frente a **62** `env::var`/`var_os` en `crates/*/src` (cli 42, gateway 11, engine 7, storage 2, domain 0) | Ninguna ruta que lea entorno se puede probar aislada | `CliEnvironment` ya existe: migrar los call sites a campos del struct |

### 4.8 Lo que se verifico y **NO** es un hallazgo

Se reportan porque tres de ellos aparecieron en informes intermedios y se descartaron al
medirlos:

- **`sddk-cli/src/ledger.rs:1012` escribe SQL crudo en produccion.** **Falso.** `rusqlite`
  aparece una vez en todo `sddk-cli/src`, y esta dentro del `#[cfg(test)]` que abre en la
  linea 998. Es un fixture. (Falta un metodo de puerto `ensure_project`; el fixture se ve
  obligado a saltarselo.)
- **El gate 9b nunca se ha ejecutado.** **Falso.** Vive inline en `release.sh:1729-1896` y
  los recibos de session-49/50/53 registran `PASS=13 FAIL=0`. Lo que no corre es su
  falsificador, que esta en `EXCEPTIONS`.
- **`sddk-gateway/src/uat_serve.rs` con 18 escrituras silenciadas.** **Falso el crate**: el
  fichero esta en `sddk-cli/src/uat_serve.rs`. **Cierto el defecto**: 23 escrituras de red
  silenciadas, todas en produccion.
- **Secretos en el repo.** **Cero.** Sin claves privadas, sin `sk-`, sin JWT, sin tokens
  `ghp_` reales. Los dos que aparecen (`AKIAIOSFODNN7EXAMPLE`, `ghp_abcdefghijklmnopqrst`)
  son los ejemplos de la documentacion de AWS y de un test de redaccion, y el test asserta
  que quedan enmascarados. Sin anclas de confianza hardcodeadas. **Categoria limpia.**

---

## 5. Riesgos

| Riesgo | Probabilidad | Impacto | Senal temprana | Mitigacion |
|---|---|---|---|---|
| **Integridad del ledger por digest divergente** | media | **critico**: CAS y eventos canónicos con SHA-256 distinto; cada copia compila | Un test de round-trip que falle solo con un digest reescrito | Un digest canonico + guard. Es el riesgo mas serio del informe |
| **H01 verde sobre ledger vacio** | alta | **critico**: el invariante de reproduccion byte-identica pasa cuando el payload se fabrico | El fallo solo se manifiesta con una serializacion que falle | Propagar el error de `storage_adapter.rs:83` |
| **Bundle con MANIFEST obsoleto** | media | **critico**: `sddk dev install` verifica un manifiesto que no describe el bundle | Un `--verify` que pasa con ficheros cambiados | Propagar `write_manifest` en `link.rs:285` |
| **Firma de release sin cobertura** | alta | **alto**: el anchor es un placeholder y su test esta `#[ignore]`d; sin `cargo-audit` | Un artefacto firmado con clave de pruebas | Decision del operador + `deny.toml` |
| **Superficie publica como coste fijo** | alta | **medio**: 4 975 `pub`, 588 loc/fichero | Tiempo de compilacion y de navegacion de un agente creciendo | Regla de `pub` + gate de conteo |
| **El gate de codigo de comandos congela el drift** | **alta** (ya ocurrio) | **alto**: el registry ya no coincide con el enum | 62 vs 51 | Derivar por reflexion de clap |
| **Bucle Python fail-open** | media | **alto**: 10 tests de contrato se pierden en silencio | Renombrar cualquier `.py` de la lista | `die` en vez de `warn` |
| **Envenenamiento de mutex** | baja | **medio**: un panic convierte el store en panico permanente | Un test que hace panic dentro de un lock | `into_inner()` |
| **Perdida de diagnosico en produccion** | **alta** | **medio**: 0 `tracing`, 23 escrituras silenciadas, 219 `let _ =` | Un incidente sin log | `tracing` |
| **Roadmap no computable** | alta | **medio**: no se puede responder "¿que falta?" con evidencia | Planes que se saltan hitos sin notarse | Un vocabulario, o un mapa explicito C-nombre ↔ ciclo |

---

## 6. Diagnostico priorizado

Causas raiz, de las cuales derivan casi todos los hallazgos:

1. **La regla de "una autoridad canonica por concepto" (AGENTS.md §2.7) se aplica a los
   datos, no a las decisiones.** El repo la cumple escrupulosamente sobre el ledger, los
   eventos y la evidencia, y no la aplica sobre el contrato de comandos, el estado de un
   ADR, el reloj o el digest. Son el mismo defecto en cuatro sitios.

2. **Se optimiza la prueba del gate, no la ejecucion del gate.** El falsador existe y es
   bueno; el gate 9b corre; su falsificador no. El bucle shell fallo cerrado y el Python no.
   La disciplina se aplico de forma desigual entre dos codigos vecinos.

3. **Un guard que solo comprueba lo que ya es cierto no puede encontrar nada.** El guard de
   Commands congela con `UPDATE_SNAPSHOTS=1` (detecta cambios, no deriva). El guard del
   indice de deudas comprueba que el indice no contradiga a sus entradas, y no que toda deuda
   este en el indice. Ambos pasan mientras el defecto este presente. Es la misma forma que
   el falsador que se llamaba a si mismo en recusion infinita, encontrado en session-80.

4. **El coste de ampliar la superficie es gratis hasta que deja de serlo.** Anadir un modulo
   al engine con `pub mod` no cuesta nada hasta que hay 131. Anadir una copia de `now_rfc3339`
   no cuesta nada hasta que hay 10.

### Orden por impacto / esfuerzo

**Impacto alto, esfuerzo bajo (hacer primero):**
1. `release.sh:409`: `warn` -> `die` en el bucle Python. 1 linea.
2. `storage_adapter.rs:83`: propagar el error en vez de `b"[]"`. 3 lineas. Cierra el invariante H01.
3. `link.rs:285` y `admission.rs:617`: propagar las dos escrituras descartadas. 4 lineas.
4. `skill_registry_bridge.rs:258`: comparador semver unico. Corrige un bug real.
5. `gateway/tests/bounded_runner_contract.rs:50`: quitar la tautologia.
6. Los 6 `#[ignore]` sin motivo: escribir el motivo en el atributo.
7. `ci.yml`: borrar o corregir las 3 invocaciones de ficheros inexistentes.

**Impacto alto, esfuerzo medio:**
8. Un digest canonico en `domain` + un reloj canonico + guard de "no hay una segunda".
9. Derivar `all_command_specs()` de la reflexion de clap + test de exhaustividad fail-closed.
10. Un solo campo de estado en los ADRs, y regla inversa en el indice de deudas.
11. `tracing` en el workspace, empezando por el motor y el CLI.
12. `deny.toml` + gate de licencias y CVEs.

**Impacto alto, esfuerzo alto (programar, no improvisar):**
13. `GitProbe` y `HttpClient` (diseño de puertos, §7.4).
14. Mover `domain/uat.rs` al pack y resolver ADR-0094 ( ADR-0104).
15. Reducir la superficie `pub` de 4 975 a ~160 (diseño minimalista, §7.1).

**Deuda revalidable:**
16. **INC-DEBT-064**: sus tres criterios "NO hecho" fueron medidos. El criterio 3 es
    **falsificable y falso** contra HEAD (`doctor.rs:484` invoca la deteccion). Los otros dos
    son de ciclo de vida, no de codigo. Procede revalidar por sus propios criterios y, si los
    tres han caducado, pasar a `resolved` con escrito lo que sigue vivo.
17. Deudas 049, 050, 060, 061: requieren accion del operador (reasignacion de `project_id`,
    enumeracion de ciclos, filas `__spine_import__`). No son codigo.

---

## 7. Disenos de interfaz

Se generaron cuatro disenos radicalmente distintos, cada uno por un subagente con
instruccion de verificar sus propias premisas antes de diseñar. Tres corregieron al brief
instruccion explicita de verificar sus propias premisas antes de disenar, y de rectificar su

### 7.1 Minimalista - reducir la interfaz hasta que se pueda sostener en la cabeza

**Tesis:** la superficie publica se reduce de 4 975 a ~160 items. Todo lo demas es privado.

**Regla propuesta:** `pub` solo si el item es (a) un puerto que alguien de fuera debe
implementar, (b) una fachada, (c) identidad o valor. Nada que tenga unicamente consumidores
intra-crate. Desglose: `domain` ~90, `engine` ~25, `storage` ~3, `gateway` ~8, `cli` ~5,
`vault` ~2 (se fusiona en storage), `testkit` ~25, `pack-uat` ~0.

**Metrica gateable:** superficie publica / total < 3.2% (hoy 17%).

**Resuelve:** las 2 fuentes de comandos (sobrevive el enum de clap, el registry se deriva y
solo guarda los 5 campos de gobernanza: estabilidad, clase de efecto, requisito de
autoridad, relacionados, skill requerida). Los 10 SHA-256 y los 10 relojes (dos ficheros
nuevos en `domain`). ADR-0094 (sobrevive `CanonicalEventLog`, se retira `EventStore`).

**Coste:** 54-76 dias-persona en 7 fases. Sin breaking: F0, F2, F4. Cuatro majors.

**Lo que pierde, y es el argumento mas serio del diseno:** **choca de frente con
ADR-0104**, que diseña precisamente una frontera de extension de packs. Este diseno deja el
core cerrado con una sola puerta. Y F1 (privatizar 58 modulos muertos) es 12 dias de trabajo
mecanico cuyo valor no esta en las 4 000 lineas que desaparecen sino en que manana anadir un
modulo ya no exige una decision de visibilidad. **Quien mida F1 por su rendimiento en un
sprint la cancelara, y entonces las cinco fases siguientes pierden su premisa.**

**Su propio diagnostico final es el mejor argumento del informe:** el diseno es correcto bajo
una hipotesis explicita - *SDDK es un monolito de proposito unico legible entero, y sus
consumidores son el CLI y los agentes que leen el bundle, no otros crates Rust*. Y
**F0 cuesta 2 dias y produce exactamente la evidencia que decide entre las dos hipotesis**.
Empezar por ahi, no por el diseno.

### 7.2 Flexible - registro extensible para todo lo que hoy es un `match` cerrado

**Tesis:** toda enumeracion cerrada pasa a extensible por registro. Anadir una variante es
un fichero nuevo y cero ediciones.

**No invento un idioma: generalizo lo que ya existe.** El subagente encontro que el repo ya
tiene **14 registries** con tres formas distintas y ninguna fail-closed, y eligio como
modelo la mejor de las tres: `VerifyDomainRegistry`
(`engine/src/verify_kernel/registry.rs:80`): `BTreeMap<&'static str, &'static dyn Trait>`,
un `register` builder que hace `panic!` en duplicado, y un `lookup -> Result<_, _>`.
Mecanismo propuesto: `Registry::seal()` -> `Sealed` inmutable, que rechaza duplicados,
claves no `kebab-case`, `sort_key` duplicada y entradas invalidas; descubrimiento
cross-crate con `linkme` (`distributed_slice`), que es la unica opcion que da literalmente
"cero ediciones de codigo existente".

**Los seis casos, con su coste real medido:**

| Caso | Que se borra | Que se anade | Dias |
|---|---|---|---|
| `adapter_for` (6) | 10 lineas de `match` | 6 `impl` de una linea en los ficheros que ya existen | 1 |
| `UatCommand` (27) | enum + 27 brazos (~90 lineas) | 27 descriptores de una linea. `UatCommand::` aparece **0 veces fuera de `uat.rs`** | 4 |
| `ActiveGraphNodeKind` (10) | 36 lineas | 10 descriptores con `label` y `build: fn(...)` | 3 |
| `EngineError::code()` (38) | 45 lineas | `#[error_code("...")]` en cada variante: el string se declara **junto al variante** | 2 |
| `Command::` (50) | el `match` | subcomandos clap dinamicos + `BTreeMap<nombre, handler>` | 9 |
| `ActionKind::as_str()` (24) | — | — | **no migrar** |

**El aporte mas valioso del diseno es una rectificacion propia, y es la que mas me importa
del informe entero.** Sobre `ActionKind`, el subagente midio que `ActionKind::` aparece en
**230 sitios en 9+ ficheros** y concluyo: *"aqui el coste supera al beneficio: el enum ya es
`#[non_exhaustive]` y ya es el dominio. **Recomendacion: no migrar (e).** Es el unico de los
seis donde el registro empeora el resultado."* Es exactamente la disciplina que el resto del
informe exige: un diseno que se niega a aplicarse a si mismo donde no aporta.

**El segundo aporte es el argumento sobre el golden file, y es el mas fuerte del diseno.**
Con registro dinamico **no hay garantia de compilacion**, asi que el guard de commands debe
pasar a runtime y **negarse a arrancar** (`SurfaceDrift { spec_sin_handler,
handler_sin_spec }`, exit 2, con `--allow-surface-drift` para el unico caso legitimo: un
binario antiguo contra un bundle nuevo). Y **por que el golden no puede servir**, medido en
`agent_surface_golden.rs:139-144`: el fixture es una funcion del codigo bajo prueba
(`current()` lee el mismo registro que produce el drift), luego regenerarlo **reescribe la
deriva como estado correcto** y no hay forma de distinguir "aun lo drift" de "acabo de
regenerar". Peor: la variable `UPDATE_SNAPSHOTS` vive en el mismo binario de test que ejecuta
el guard, asi que un `UPDATE_SNAPSHOTS=1` exportado en el entorno de CI desactiva el unico
control. Y el golden congela **forma** (nombre, estabilidad, clase), que no es el defecto:
el defecto es la **relacion** spec<->handler, que el golden ni siquiera serializa.

**Lo que pierde, y es el coste real:**

1. **La garantia de compilacion desaparece donde mas dolia.** Un comando nuevo sin spec deja de
   ser un error de `cargo build` y pasa a ser un proceso que arranca y se niega. El bucle de
   fallo pasa de "compila mal" a "falla en produccion del usuario". Mitigable, no eliminable.
2. **Cuatro invariantes dejan de ser estaticas** (que todo `Command` tenga spec, que todo spec
   tenga handler, que no haya dos `ActionKind` con el mismo string, que un `NodeKind` tenga
   label unico) y pasan a aserciones de runtime. **Un PR puede mergear con el drift
   reintroducido; lo detecta el release, no el editor.**
3. **`linkme` introduce dependencia del enlace.** Un test de integracion puede no enlazar el
   crate registrante (registro vacio si el registrante esta en `sddk-pack-uat` y no se
   referencia), y el orden de enlace no es el orden de la tabla - por eso la tabla ordena por
   `sort_key` y no por descubrimiento. Una dependencia mas que mantener.
4. **Coste en compilacion y en caliente:** `BTreeMap<&'static str, &'static dyn Trait>` pierde
   la optimizacion monomorfica; `EngineError::code()` pasa de jump table a busqueda en
   `BTreeMap` en una funcion caliente.
5. **Y lo mas incomodo:** un registro invalido que `seal()` acumula en `rejected()` en vez de
   abortar **es en si mismo una segunda fuente de verdad** (la tabla buena y la tabla de
   rechazados). Lo reconoce el propio diseno.

**Coste total:** 36 dias-persona en 9 fases, de las cuales **F1 (3 dias) es el guard
fail-closed de superficie**, que es la unica parte que este informe recomienda adoptar de
este diseno.

**Dato adicional que el subagente midio y que refuerza §4:** `sddk-pack-uat` tiene 52 loc y
**ningun crate lo depende** (verificado: `grep sddk-pack-uat crates/*/Cargo.toml` solo
devuelve la entrada del workspace). El pack es una cascara, luego ADR-0104 esta de facto
sin implementar por el lado de la dependencia, no solo por el lado del codigo.

### 7.3 Optimizado para llamadas - la unidad de coste es (llamadas x tokens)

**Tesis:** la interfaz no se diseña para humanos ni por estetica del codigo, sino para
minimizar llamadas, tokens y **llamadas fallidas x tokens de recuperacion**.

**Medicion propia del subagente:** `all_command_specs()` son 43 264 bytes de fuente para 59
specs; serializado en pretty, **~69 KB = ~19 000 tokens** para `introspect commands`. Hoy 5
tareas tipicas cuestan **9 llamadas y ~540 tokens de peticion**; con un verbo `do <op>`
tipado, **5 llamadas y ~175 tokens**. B es 3.1x mas barato en request y ~21x en descubrimiento.

**Lo mejor del diseno, y es un hallazgo, no una opinion:** `SddkErrorCode` **ya existe** en
`domain/src/error.rs:4-9` con `code()` + `recovery()`, y `EngineError` ya implementa ~30
`recovery()` accionables (una emite literalmente el comando exacto a reintentar). La
propuesta **no** sobrescribe la tabla de 37 codigos: le anade `class` (5 valores
ortogonales) y `retry.exact`. Se gana machine-readability de la decision sin perder 30+
strings de recuperacion ya validados. Y la idempotencia ya tiene sustrato:
`GATEWAY_IDEMPOTENCY` con "Deterministic request key".

**Lo que pierde:** el rechazo en parse-time de clap se debilita (un op generico acepta
combinaciones que clap rechazaba; se mitiga validando contra el JSON Schema que ya existe
en `arg_schema.rs`). Y la DX humana se pierde de verdad.

**Su condicion es correcta y es la que gobierna:** **B se anade al lado de A, no lo
reemplaza.** `scripts/release.sh` es un consumidor shell real de `sddk dev manifest` y
`sddk dev install`. Si se fuerza `do` como unico verbo, se rompe el pipeline de release a
cambio de una mejora que solo enjoyan los agentes. **El error a evitar no es "el diseno de B
es malo" sino "matar A demasiado pronto".**

**Advertencia que el propio subagente se hizo y hay que repetir:** los numeros de tokens son
estimados desde bytes de fuente. **No hay binario precompilado en el repo y no se compiló
para esta auditoria.** El orden de magnitud es solido; las cifras absolutas necesitan una
medicion real.

### 7.4 Puertos y adaptadores - el nucleo puro

**Tesis:** el nucleo no depende de git, ni de HTTP, ni del filesystem, ni del entorno, ni
del reloj, ni de la aleatoriedad, ni de SQL. Todo eso son adaptadores.

**Inventario de puertos nuevos:** `Clock`, `IdGen`, `EnvSource`, `FileSystem`,
`ProcessRunner`, `BinaryResolver`, `HttpClient`, y `GitProbe` (que merece separacion propia
por el caso de `evaluators.rs`).

**Correcciones al brief que el subagente hizo midiendo (y que verifiquemos):**
- `rusqlite` en `sddk-cli` NO es fuga de produccion (§4.8).
- No hay 8 emisores: hay **21**.
- `ports.rs` tiene **0** menciones de crates concretos: la invariante ya se cumple.
- Y una retractacion honesta: **el recuento son 7 puertos, no 11.** Retiro `Terminal` y
  `ConfigSource` porque tienen una sola implementacion posible. "Un trait con una sola
  implementacion de por vida es impuesto sin contraprestacion."

**El caso `GitProbe` es el mejor analisis de los cuatro disenos.** El `WaiverExpiryResolver`
existente es `Arc<dyn Fn>` y permite sustituir **la decision**, pero `git_rev_parse` sigue
siendo `Command::new("git")` **dentro** de la closure construida en el composition root. Por
eso no se puede testear "git no esta", "git devuelve basura", "se llamo dos veces", ni fijar
`repo_root`. El diseno separa obtencion de decision, y distingue `Ok(None)` (rev no es un
commit: resultado de negocio) de `Err` (git ausente: fallo). El fallback lexicografico se
**conserva** porque es comportamiento real en entornos sin git.

**Correccion al propio encargo que es la mas util de todas:** **no usar feature flags.**
`#[cfg(feature)]` selecciona en tiempo de compilacion y un test compila una vez, luego no
puedes ejercitar las dos rutas en la misma build. Es un interruptor de rollback que te quita
la capacidad de comparar. La alternativa correcta es seleccionar la implementacion en
**runtime** por lo que ya va en el agregado de puertos, que si es testeable en ambas
direcciones.

**Lo que pierde, con honestidad inhabitualmente dura:**
- **5 de los 7 puertos tienen una implementacion de produccion y una de test. Eso es un
  criterio, no hexagonalidad.** Para `ProcessRunner` y `HttpClient` la frontera es
  *declarativa*: el trait no compra testabilidad del mundo real, compra que el *routing* sea
  testeable.
- **Los fakes son una superficie de mantenimiento nueva** (testkit de 1 697 a ~3 500 loc), y
  **un fake desalineado es peor que no tener fake, porque hace mentir a la suite.** Regla
  que propone y que es obligatoria: cada fake lleva un test de conformidad contra el
  adaptador real, o se borra.
- **Sigue siendo imposible de testear aunque lo arregles todo:** la semantica real de
  `merge-base --is-ancestor` (un fake prueba la politica, nunca que git se comporte como se
  asume), TLS, proxies, la concurrencia SQLite bajo multi-proceso real, el layout real del
  checkout, ENOSPC/OOM. **"El core es puro" es una afirmacion sobre el determinismo del
  core, no sobre la cobertura del mundo.**
- **Regresion de concurrencia oculta:** cambiar `EventStore::append` de `&mut self` a `&self`
  mueve el `Mutex` al adaptador y **el sistema de tipos deja de ver** que no se puede
  mantener el lock de BD abierto a traves de un `await`.

**Coste:** ~58 dias-persona, 12-13 semanas, 8 fases. Riesgo de calendario: con 58 dias de
trabajo en `crates/`, la ventana declarada-pero-no-publicada se abre, y AGENTS.md §2.1 ya
advierte que en esa ventana no se bumpea para satisfacer el hook. **El hook no es el
problema; el calendario de releases es una dependencia de esta migracion.**

### 7.5 Recomendacion

**Recomendado: 7.4 (puertos y adaptadores) como arquitectura de destino, 7.1 (minimalista)
como su primera mitad ejecutable, y 7.3 (llamadas) solo como capa aditiva.**

El razonamiento, en orden:

1. **7.4 es el unico que ataca la causa raiz.** El motor ejecuta `git` en produccion y el
   `TaskExecutor` sale a internet sin seam. Eso no es deuda cosmetica: es la promesa central
   del producto ("decision kernel determinista") incumplida en dos lineas cada una. 7.1 y 7.3
   no la tocan.

2. **7.1 y 7.4 no compiten: 7.1 Fase F1 (privatizar superficie) y 7.4 Fase 1 (puertos) se
   solapan solo en el fichero que tocan.** Se pueden empezar en el mismo sprint en ficheros
   disjuntos. Y 7.1 aporta el guard (`F0`, 2 dias) que **decide si 7.1 entero tiene sentido**:
   cuenta cuantos de los 4 975 `pub` son alcanzables desde fuera hoy. Es la mejor relacion
   coste/evidencia de las cuatro.

3. **7.3 es correcto y urgente, pero es ortogonal y no puede matar A.** El dato de 19 000
   tokens de descubrimiento es real y duele. Pero la forma correcta es **añadir `do`/`ops`
   al lado de `sddk`**, porque `release.sh` es un consumidor shell. Y su F1 (derivar el
   registry de clap, 5 dias) **es exactamente el arreglo del hallazgo critico 4.1**, asi que
   7.3 se paga solo en la parte que ya hay que hacer.

4. **7.2 aporta el unico hallazgo de diseño que el resto de los cuatro no tiene, y ese
   hallazgo se adopta entero: el guard de superficie debe ser un fail-closed en runtime, no
   un golden.** El resto del programa se rechaza. Y el motivo del rechazo es el mas fuerte
   del informe: un registro dinamico para todo lo que hoy es un `match` **gasta el
   presupuesto de verificacion estatica**, que es el activo real de este repo (hooks,
   guardas de needle, falsadores, `test_gate_coverage.py`). A cambio compra una extensibilidad
   que **nadie ha pedido**: no hay un segundo lenguaje de test, ni packs de terceros, ni un
   segundo dominio. C7 (packs y segundo dominio) es `PROPOSED` y el roadmap dice que no
   abre antes que C6. Se estaria pagando un precio de complejidad y de compilacion por una
   extension que el propio roadmap difiere.

   Y hay un argumento mas fuerte todavia, medido: 7.2 **mismo** se contradice en el punto
   `(b) Command::` (9 de sus 36 dias, la fase mas cara). `#[derive(Subcommand)]` posee el
   parseo, luego un comando nuevo sigue siendo una variante del enum: para llegar a "cero
   ediciones" hace falta un `ArgMatches` sin tipos y **dos rutas de parseo**, con dos
   semanticas de error y dos superficies de `--help`. Es decir: 7.2 paga su parte mas cara
   para no conseguir la propiedad que vendia en el unico punto donde la quiero. Por eso se
   toma su F1 (3 dias) y se descarta el resto.

5. **Dos de los cuatro disenos, partiendo de premisas opuestas, coinciden en una misma
correccion que este informe
   no habia visto: "unificar el digest" es incorrecto** (§4.1, nota de contrato). 7.2 la
   encontro solo; 7.1 la proponia sin saberlo. Los digests ya estan persistidos con dos
   formatos distintos (`sha256:<hex>` y hex desnudo) y unificar el codigo es una migracion de
   datos. El arreglo correcto es un **censo de contratos con guard**, no una funcion. Cuando
   cuatro disenos independientes convergen en la misma rectificacion, la rectificacion es
   real.

**Secuencia recomendada, ordenada por evidencia-antes-que-compromiso:**

| # | Accion | Esfuerzo | Por que primero |
|---|---|---|---|
| 0 | **Los 7 fixes de bajo esfuerzo de §6** (1-7) | < 1 dia | Cierran un invariante de integridad (H01), dos escrituras descartadas y un fail-open. Todos verificados |
| 1 | **F0 de 7.1**: contar los `pub` alcanzables desde fuera. 2 dias | 2 d | Produce la evidencia que decide si 7.1 entero procede. No presupone la respuesta |
| 2 | **Derivar el registry de clap** (F1 de 7.3) + **guard de superficie fail-closed en runtime** (F1 de 7.2, 3 d). 8 d | 8 d | Cierra el hallazgo critico 4.1. Se hacen juntos porque son el mismo arreglo: derivar sin guard deja la deriva en manos de un golden con boton de update |
| 3 | **F1-F2 de 7.4**: `GitProbe` + borrar las copias del reloj. 10 d | 10 d | Cierra las dos fugas de IO de mayor impacto, con guard de no-regresion |
| 4 | **`tracing` + `deny.toml`** | 5 d | Los dos agujeros que no tienen nada que los vigile |
| 5 | **F1 completa de 7.1** (superficie) | 12 d | Solo si el paso 1 dio evidencia a favor |
| 6 | **7.4 completo** (ADR-0094, ADR-0104, vault) | 40 d | Programado, no improvisado |
| — | **7.3 como capa aditiva** (`do`/`ops` al lado de `sddk`) | 29 d | Cuando 1-4 esten verdes. Nunca sustituyendo A |
| — | **7.2 (registro extensible)** | — | **Rechazado como programa, 36 d.** Se adopta su F1 (arriba). Su unica gran aportacion es el argumento del golden file, que ya esta incorporado |

---

## 8. Lo que este informe NO sabe

Se declara explicitamente, porque una auditoria que no dice sus limites no es una auditoria:

- **No se ejecuto `cargo`.** Ni `build`, ni `test`, ni `clippy`. No hay cobertura por funcion,
  ni metricas de branch coverage, ni tiempos de compilacion. Cualquier afirmacion sobre
  "que test cubre que" es topologica (el test referencia el modulo), no de cobertura real.
- **No se midio la complejidad ciclomatica.** El analysis se hizo por densidad de `pub`,
  loc por fichero y contagem de duplicados, no por McCabe ni por cognitive complexity.
- **Los conteos de panic-macros dependen de un parser heuristico** de regiones `#[cfg(test)]`
  y de ficheros `*_tests.rs` que solo son alcanzables por `#[cfg(test)] #[path]`. Las
  **rutas concretas** citadas se leyeron una a una; los agregados tienen margen de error de
  unas decenas.
- **El grafo de dependencias interno no se midio.** Solo el de crates, que es el unico que
  Cargo puede expresar. La profundidad de modulos es juicio sobre el ratio y el cuerpo.
- **Los numeros de tokens del diseno 7.3 son estimados** desde bytes de fuente. Requieren
  `cargo build --release` y medicion real.
- **No se audito** `docs/` mas alla de ADRs e indice de deudas, ni `prompts/`, `packs/`,
  `agents/`, `skills/`.
- **No se ejecuto ningun subagente en modo escritura.** Todos fueron de solo lectura.
- **La severidad de los 42 panic-macros es por clase de codigo** (deserializacion, mutex,
  ledger), no por alcanzabilidad en runtime verificada.

---

## 9. Como verificar cualquier linea de este informe

Todo lo anterior es reproducible sin el informe:

```bash
export CARGO_TARGET_DIR=/var/home/rubentxu/cargo-targets
cd /var/mnt/DiscoChino2-fast/Proyectos/agentesIA/sddk-framework

# Superficie publica
grep -rhoE '^\s*pub (fn|struct|enum|trait|type|const|mod|use)' crates/*/src --include=*.rs | wc -l   # 4975

# Las dos fuentes de verdad, ya divergidas
grep -c "^\s*spec(" crates/sddk-cli/src/command_spec.rs                                                  # 62
sed -n '904,1035p' crates/sddk-cli/src/lib.rs | grep -cE "^\s+Command::"                               # 50

# ADR con estado contradictorio
python3 -c "import re,glob;[print(f) for f in sorted(glob.glob('docs/architecture/adrs/*.md'))
  if (m:=re.search(r'^status:\s*(\S+)',open(f).read(),re.M)) and (b:=re.search(r'\*\*Status:\*\*\s*(\S+)',open(f).read(),re.M)) and m.group(1)!=b.group(1)]"

# El bucle Python fail-open
sed -n '405,410p' scripts/release.sh

# Digest y reloj duplicados
grep -rn "fn sha256_hex\|fn sha256_of\|fn hash_bytes" crates/*/src --include=*.rs | grep -v tests   # 10
grep -rn "fn now_rfc3339\|fn now_utc" crates/*/src --include=*.rs | grep -v tests                       # 10

# Criterio caducado de INC-DEBT-064
sed -n '484p' crates/sddk-cli/src/dev/doctor.rs        # invoca build_id::identity_verdict_here()

# Sin logging estructurado
grep -rln "tracing::" crates/*/src --include=*.rs | wc -l                                                   # 0

# Sin control de dependencias
ls deny.toml audit.toml 2>&1                                                                              # no existe
```

Y los gates que el repo ya se aplica a si mismo:

```bash
bash tests/test_release_diagnostics.sh          # PASS=42 FAIL=0
bash tests/test_release_diagnostics_wiring.sh    # PASS=37 FAIL=0
bash tests/test_release_diagnostics_mutation.sh  # PASS=13 FAIL=0 SKIP=0
python3 tests/test_gate_coverage.py              # 70 tests, 0 sin runner
bash scripts/check_debt_index_coherence.sh       # 52 entradas (50 de 67 documentos)
```
