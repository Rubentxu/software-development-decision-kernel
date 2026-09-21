# Kernel adaptativo: encaje basado en productores y consumidores

**Estado:** propuesta de aterrizaje, no ADR aceptada ni nuevo roadmap.
**Fecha:** 2026-09-19.
**Fuente inspeccionada:** `8dd56bfe499feae8a25b762baf6138b06464838f`.
**Runtime consultado:** SDDK 1.169.88, workspace adoptado, modo `on`.
**Alcance:** revisión estructural y consultas read-only. No implementación, release ni recertificación de A5.

## 1. Dictamen

El enfoque encaja, pero no se puede presentar como una conexión menor entre capacidades ya operativas. Hay tres estados distintos: productores reales, contratos/algoritmos reutilizables y conexiones todavía ausentes. La prioridad no es construir una oficina ni una agenda más sofisticada. Es cerrar una cadena observable desde un productor hasta una decisión útil para un consumidor existente.

**Recomendación:** conservar A6 como línea de implementación. Su siguiente entrega debe demostrar una observación estática real, normalizada y consumida por Verification. La agenda contextual se alimentará de esa cadena y del Planning Ledger. No anteponer un programa E0–E4 de consolidación general a A6.

A5-C permanece cerrado. SEC-1 y SEC-WORKSPACE-FLAKE mantienen sus gates separados. Esta revisión no valida ni modifica la eliminación de tests pendiente en el árbol de trabajo.

## 2. Qué significa «hay productor»

Para admitir una funcionalidad, registrar en su contrato de trabajo existente:

1. Pregunta concreta que necesita resolver un consumidor.
2. Fuente y productor ejecutable: operación, símbolo, entrada real y condiciones de observación.
3. Salida tipada existente y naturaleza: observación, declaración, inferencia o referencia.
4. Base: identidad de proyecto/workspace, revisión y dirty state cuando corresponda, configuración, versión del productor, scope, entorno relevante y completitud.
5. Autoridad y clase de estado: Fact, Object, Projection o Ephemeral. No persistir una copia canónica del resultado de una proyección.
6. Consumidor y cambio de comportamiento medible. Si ningún consumidor cambia su decisión o contexto, el insight no justifica una nueva entrega.
7. UAT positivo, falsificación, invalidación y respuesta ante productor ausente/parcial.

Esto es una plantilla de admisión, **no** un agregado `KnowledgeProducer`, un registry nuevo ni un enum universal de insights. Los nombres de futuros slices que aparecen abajo tampoco son IDs registrados en Planning.

Un resultado determinista no es necesariamente una observación completa ni correcta. «No encontrado en el scope analizado» no equivale a «no existe en el sistema». `Partial`, error, contradicción y ausencia deben sobrevivir hasta el consumidor.

## 3. Matriz de encaje real

Las referencias son relativas a la raíz del repositorio. STRUCTURAL significa código inspeccionado, no ejecución de un UAT.

| Necesidad | Productor y salida existentes | Consumidor / conexión observada | Límite y decisión |
|---|---|---|---|
| Próximo trabajo y bloqueos | `crates/sddk-domain/src/planning/projections.rs`: `project_next`, `project_blocked`, sobre `RoadmapSnapshot` | `crates/sddk-cli/src/plan.rs::run_roadmap` obtiene `storage.snapshot_roadmap()` y renderiza la proyección | Reutilizar. Es selección de spine, no scheduler general ni autorización de efectos. |
| Cambios del repositorio | `crates/sddk-cli/src/architecture_cmd.rs`: `git_changed_paths`, `changed_basis`, `path_overlaps` | Consultas de arquitectura cruzan paths y contratos declarados | Productor real de cambios y solapamientos. No demuestra call graph, impacto transitivo ni consecuencias de negocio. |
| Contradicciones entre contratos arquitectónicos declarados | `architecture_cmd.rs::build_context`, `run_debverify_audit`, `FindingBasis` | `findings`, `receipt`, `why` comparten preparación de contexto | Reutilizar findings existentes. No convertir consistencia de declaraciones en prueba de que el código las cumple. |
| Observaciones del software | `crates/sddk-engine/src/observation/`: `SoftwareObservation`, `ObservationBasis`, `ObservationSet`, resolución y proyección | `build_context` carga las observaciones incluidas en la declaración | Es sustrato y entrada declarada, no analizador automático. `observation/mod.rs` dice expresamente que no ejecuta probes. |
| Verificación de una claim | `crates/sddk-engine/src/verify_kernel/` consume claims y observaciones | `crates/sddk-cli/src/verify_kernel_cmd.rs::run_verify` construye actualmente `ObservationSet::new()` vacío | Aquí hay un consumidor concreto para A6. Sin evidencia, conservar Unknown, no fabricar un PASS. La observación se limita a esta ruta, no a todas las verificaciones del producto. |
| Impacto estático CogniCode | `code_intelligence_port.rs`, `code_intelligence_port_fake.rs`: puerto, Fake y Null | `crates/sddk-engine/tests/a6_cognicode_protocol_spike.rs` caracteriza el protocolo | CC-S0 demuestra el seam, no un productor estático real. La observación del spike es texto libre. Falta adaptación al sustrato canónico y consumo real. |
| Comportamiento runtime / comparación Chronos | Namespace/origin previsto y roadmap A7/R8 | No se identificó un adapter Chronos operativo en las rutas inspeccionadas | No usar Chronos para justificar una entrega actual. Mantener A7 con productor real, entorno, escenario y comparabilidad como gate. |
| Resultado de ejecutar una herramienta de test | `crates/sddk-gateway/src/test_runner/mod.rs::dispatch` construye spec y llama `runner::run`; `runner.rs::RunOutcome` conserva estado, salida y timeout | Hay adapters por familia y contratos en `crates/sddk-gateway/tests/bounded_runner_contract.rs` | Tener runners no demuestra normalización por testcase, asociación test→claim ni ingestión en Knowledge. Esas conexiones deben demostrarse. |
| Topología de tests | `crates/sddk-domain/src/test_adapters.rs::ProfileAdapterV1` y `compose_polyglot_topology` | Producen topología/capabilities desde un snapshot inyectado | No confundir este productor de topología con un parser de resultados JUnit ni con selección libre de scope. |
| Contexto y recuperación | `crates/sddk-engine/src/context_capsule.rs::ContextCompiler`, `CapsuleInputs`, `cold_start.rs::RecoveryCapsuleInputs` | `cold_start` compila desde `RunStateView` y cápsula previa; `agent_host.rs` integra recuperación | Reutilizar ruta de cápsulas. No hay demostración aquí de agenda+Knowledge+outcome persistido→nuevo encargo tras reinicio. |
| Handoff y síntesis | `agent_contribution_envelope.rs::AgentContributionEnvelope`, `orchestration_synthesis.rs::DefaultSynthesisBuilder` y validadores | Subsistemas concretos de contribución/síntesis | `ExecutionRequest`/`ExecutionOutcome` son nombres del contrato documental. No se localizaron definiciones Rust con esos nombres; `execution_receipt.rs` incluso advierte `MissingExecutionOutcome`. No declarar un protocolo end-to-end ya conectado. |
| Secretary reactivo | `secretary_l0.rs`, `secretary_l1.rs`, `secretary_l2_replan.rs`: reglas, propuestas acotadas, replan | Algoritmos y tests de módulo inspeccionados | No se identificó wiring productivo de L0/L1 con Planning/Knowledge en la búsqueda de consumidores. Son piezas reutilizables, no una agenda operativa demostrada. |
| Insights compuestos | `intelligence_loop/mod.rs::compose_intelligence_loop`, `intelligence_advisory/mod.rs::derive_advisory_context` | Componen resultados de Knowledge, Verify, DebVerify, lenses y Alignment | Reutilizar. La composición recibe resultados, no los produce ni ejecuta sus evaluadores. No necesita otro «motor de insights». |

## 4. Correcciones necesarias a las premisas

### 4.1 Planning no equivale a «puede ejecutarse»

`project_next` reanuda un único Active o selecciona por `spine_order`, dependencias terminales y promoción. Más de un Active produce error. `Done`, `Superseded` y `Cancelled` cuentan como terminales. `project_status` clasifica varios estados como ejecutables sin aplicar Authority.

Por tanto, una agenda inicial debe decir **candidato según planificación**, no «ejecución admitida». La satisfacción semántica de una dependencia, la evidencia de aceptación y los permisos son comprobaciones distintas. No ampliar ahora esta consulta hasta convertirla en scheduler paralelo.

Además, ante una referencia de dependencia inexistente, `project_next` no la considera satisfecha, mientras `project_blocked` la omite de los bloqueantes conocidos. Es una asimetría STRUCTURAL a falsificar si se utiliza una agenda combinada, no un fallo productivo demostrado en esta revisión.

**OBSERVED:** el binario instalado devolvió `GA-002` como próximo trabajo, y bloqueos con IDs como `DW-RUNTIME-005`, `DEC-PLANE-001` y `RX-SECRETARY-001`. La consulta funciona, pero no demuestra que ese snapshot represente la línea actual A6. Antes de usarlo para actuar hay que reconciliar identidad de importación, revisión y vínculos con el roadmap vigente. No se mutó el ledger.

### 4.2 Hay más de un «ContextCompiler»

`context_compiler.rs` contiene un ensamblador de bytes con adapters y staleness por log head. `context_capsule.rs` contiene el compilador estructurado usado por `cold_start` y reexportado por el engine. Conectar «el Context Compiler» sin nombrar esta ruta sería ambiguo.

La conexión propuesta debe dirigirse a `CapsuleInputs`/`ContextCapsule` y sus consumidores efectivos. No crear un tercer compilador. Tampoco retirar uno por similitud de nombre sin revisar compatibilidad y consumidores.

### 4.3 IR no equivale a runtime ejecutable

`crates/sddk-engine/src/operator.rs::build_operator` construye Task, Sequence, Parallel, Choice y Map. Rechaza Join, Race, Loop, Gate, Wait, SubWorkflow y Compensate mediante `NotImplementedInCycle16`.

Choice selecciona ramas mediante guards, no demuestra automáticamente la semántica de un Selector que intenta una alternativa tras el fallo de otra. Retry existente tampoco demuestra idempotencia o compensación. Un ciclo dinámico debe nombrar la semántica exacta que necesita y sus efectos, no implementar todos los operadores por simetría.

### 4.4 KnowledgeBasis no produce conocimiento

`knowledge.rs` clasifica KnowledgeAssertion como Object y KnowledgeBasis como Projection, con hashes e invalidación. `observation` conserva soporte, refutación y contradicción. Ninguno obtiene automáticamente código, procesos o resultados de tests.

La línea de ingestión documental en `crates/sddk-cli/src/knowledge_ingest.rs` usa sus propios planes/entradas/registry. No se ha demostrado su conversión general al KnowledgeBasis del engine. No tomar «knowledge import existe» como prueba de esa conexión.

### 4.5 Los digests del spike no sirven aún como identidad durable segura

`code_intelligence_port.rs::DigestSha256::of` calcula FNV-1a de 64 bits, aunque el tipo se llame SHA-256. Es explícitamente no criptográfico y propio del spike. Antes de basar reutilización durable o evidencia en ese valor, alinear algoritmo/nombre/contrato con el sustrato de identidades vigente. No extender esa identidad provisional a una caché global.

### 4.6 Vault no es una autoridad alternativa por reconciliar desde cero

SPEC-042 Secretary Runtime pertenece al paquete histórico y sigue proposed. ADR-0099 tiene frontmatter accepted desde 2026-09-12, aunque conserva el cuerpo original con «Proposed». La autoridad actual se resuelve mediante `docs/architecture/README.md`, no dando el mismo peso a todos los textos.

La agenda Markdown puede ser una vista humana. No se admite escritura bidireccional ni estado operativo canónico en Vault. La consolidación requerida es una referencia explícita al contrato vigente al importar trabajo histórico, no reabrir A5 ni recertificar M0–M9.

## 5. Separación que debe conservar cada insight

| Contexto | Responsabilidad | Lo que no puede asumir |
|---|---|---|
| Knowledge | Observación/declaración/inferencia con origen, base, vigencia y límites | Que un hallazgo es una prohibición o una prueba de aceptación |
| Software Alignment | Tensión/tradeoff respecto de una lens y sus supuestos | Que misalignment implica deny o gate rojo |
| Verification | Claim, scope, profundidad, evidencia faltante, resultado y receipt | Que la herramienta ejecutada cubre todo el requisito |
| Governance | Permisos, obligaciones, gates vinculantes y waivers | Que una clasificación probabilística concede autoridad |

La agenda presenta estas salidas juntas, conservando sus tipos y referencias. No crea un score global, no convierte cada finding en un WorkItem y no decide política. `intelligence_loop` ya explicita `MISALIGNED ≠ DENY`.

Authority interviene antes de obtener evidencia cuando observar tiene efectos/permisos y vuelve a validar antes de ejecutar. No se deja toda la gobernanza para el último peldaño de una cadena cognitiva. Una preselección no es un ticket irrevocable.

## 6. Slices propuestos, no nuevo roadmap

Autoridad de secuencia: `docs/architecture/README.md` → `docs/architecture/a5/A5-CURRENT-ROADMAP.md`. A6/R7 es estático, A7/R8 runtime, A8 combina ambos. R9 contiene atención/watchlists; R11 evalúa separaciones por evidencia. Las etiquetas siguientes son descripciones de trabajo candidatas, no nuevos hitos E0–E4 ni autorización para abrir ciclos.

### Primer resultado útil con productores disponibles hoy

**Insight acotado:** «estos contratos declarados están asociados a paths modificados y estas claims no tienen evidencia suficiente». No llamarlo impacto semántico ni incumplimiento del código.

- **Productor de cambio:** `architecture_cmd.rs::git_changed_paths` y `changed_basis` sobre base explícita y estado del workspace.
- **Productor de asociación:** `path_overlaps` aplicado a locators de contratos obtenidos por `load_declaration`. Es una derivación por paths, no un call graph.
- **Productor de postura de evidencia:** `observation::resolve_subject` / `resolve_relation` sobre las observaciones de `build_context`, y Verify para la claim correspondiente. Sin observaciones se conserva la insuficiencia, no se inventa un hallazgo.
- **Consumidor:** las superficies existentes de arquitectura/WHY y una cápsula de contexto pueden presentar contrato, cambio, base y evidencia faltante sin un LLM que lea el diff. Conectar la cápsula es trabajo pendiente, no una ruta afirmada como implementada.
- **Persistencia:** conservar refs a cambios, contratos y evidencia existentes. El cruce es Projection, no un nuevo store ni un WorkItem automático.
- **Falsificación:** cambio dentro del locator incluye el contrato; fuera lo excluye; renombre/borrado se evalúan según el diff real; locator sin resolución queda Unknown; una declaración de relación no cuenta como observación estática independiente. Revisión o dirty state distintos invalidan la reutilización.

Este es el caso Base admisible para demostrar el consumidor antes del proveedor Enhanced. No necesita un gran ciclo E0. Si las salidas existentes ya responden toda la pregunta, reutilizarlas sin añadir una feature. A6 añade evidencia estructural real a la misma pregunta cuando el proveedor esté disponible; no se admite implementar un dashboard de impacto mientras solo haya Fake/Null.

### A. A6: una observación real que cambia una verificación

**Pregunta:** ¿qué evidencia estructural existe sobre una relación concreta entre dos unidades dentro del scope analizado?

**Cadena:** revisión y diff reales → proveedor CogniCode negociado por `CodeIntelligencePort` → observación tipada en `observation::SoftwareObservation` → Evidence/Knowledge con base y refs del proveedor → `VerifyKernel` → resultado/receipt → salida existente.

**Primera unidad entregable:** escoger una relación que el proveedor real anuncie y pueda medir en este repo. No asumir call graph, impacto o cobertura negativa porque aparecen en una spec. Si el proveedor disponible no produce esa relación, elegir otra demostrable o declarar el bloqueo de capacidad. Fake solo cubre el contrato, no el UAT de producto.

**Scope:** adapter de frontera, normalización de una familia de observación, identidad coherente, consumidor Verify existente. Sin AST del proveedor dentro del dominio, sin nuevo enum genérico de insights, sin agenda persistida.

**UAT obligatorio:** dos revisiones con un cambio estructural controlado producen evidencia distinta y el resultado esperado de la claim. Repetición sobre la misma base semántica conserva identidad. Evidencia de otra revisión no se reutiliza como actual. Null/no-provider deja Base operativo y la claim sin evidencia como Unknown. Timeout, incompatibilidad, cancelación y parcial no se convierten en éxito. Un conjunto vacío no prueba ausencia global.

**Dependencia real:** disponibilidad de proveedor ejecutable y contrato de transporte. La documentación CC-S0 reparte de forma no uniforme adapter/ingestión entre CC-S1 y CC-S2/3. Resolverlo en un solo scope antes de asignar el próximo ID; no prometer que CC-S1 completo ya está definido.

### B. Agenda mínima: explicar lo que el ledger sabe, no lo que imaginamos

**Pregunta:** ¿qué candidato recomienda Planning y qué dependencias registradas impiden avanzar?

**Cadena:** snapshot de Planning identificado/reconciliado → `project_next` + `project_blocked` → vista de atención read-only → humano/Orchestrator. La compilación no registra tareas ni ejecuta efectos.

**UAT:** A bloquea B; mientras A está activo la vista conserva esa causa; tras su cierre válido desaparece el bloqueo; dos reconstrucciones de la misma base semántica producen los mismos campos de decisión. Dependencia ausente, snapshot no reconciliado, múltiples Active y contradicciones se muestran explícitamente, nunca como «todo listo». JSON y texto expresan las mismas razones.

**Recorte:** no preparar una tercera cola de «preparable» hasta tener permisos de lectura, inputs mínimos y presupuesto comprobables. No recalcular incrementalmente todavía: empezar con reconstrucción pura. Incrementalidad solo cuando haya dependencias de invalidación completas y coste medido que la justifique.

**Encaje:** consulta/contexto sobre capacidades Base; eventual consumidor R9. No es prerrequisito para retrasar A6 ni nueva autoridad de planificación.

### C. Handoff: una cápsula reconstruida sin conversación

**Pregunta:** ¿qué necesita el siguiente intento para continuar y qué evidencia obligatoria sigue faltando?

**Cadena:** estado de run + envelope validado + síntesis + refs de artefactos/evidencia → adapter de `CapsuleInputs` → `context_capsule::ContextCompiler` → contrato efectivo de delegación → siguiente intento.

**UAT:** cerrar el proceso, reabrir almacenamiento real y reconstruir la cápsula sin transcript; evidencia requerida recuperable y verificable; refs ausentes/stale impiden declarar handoff completo; nuevo intento conserva relación con el mismo WorkItem y revisión de plan. No repetir efectos completados. La memoria in-process no satisface este escenario.

**Recorte:** probar primero handoff entre dos tareas estáticas. No exigir dos LLM para demostrar compilación y recuperación. Adaptar los contratos ejecutables de envelope/delegación y contrastar arch-spec-007, no crear WorkHandoff/WorkSession por su nombre.

**Encaje:** conexión con host/contexto y línea agéntica vigente, pendiente de reconciliar el WorkItem concreto. No se da por incluida ni cerrada dentro de A6.

### D. Resultado estructurado de tests: entrega separada si el consumidor lo requiere

**Pregunta:** ¿qué test falló, en qué ejecución y revisión, y qué claim queda sin evidencia?

**Cadena:** runner admitido y scope planificado → reporte estructurado de una familia realmente instalada → normalización por testcase/attempt → evidencia referenciada → Verification. Reutilizar gateway; no usar `tail` o una frase del agente como productor de éxito.

**UAT:** PASS, fallo de assertion, fallo de infraestructura, timeout y reporte truncado/incompleto permanecen distintos. Proceso fallido con salida que contiene «passed» no produce PASS. Un test pasado sin relación registrada con el requisito no lo verifica. Scope ajeno al cambio no rellena cobertura faltante.

**Recorte:** no construir seis parsers a la vez ni prometer que los adapters actuales ya ingieren JUnit. Solo abrir este slice cuando se identifique la claim consumidora y el formato real del runner.

### E. Workflow dinámico: solo detrás de un disparador verificable

Un caso admisible sería: una observación nueva identifica un contrato afectado y sin evidencia; Verification devuelve el gap; una regla acotada propone añadir una comprobación ya definida. Orchestrator selecciona y Authority admite. Una nueva PlanRevision compila sin perder los nodos completados.

**Productor del disparador:** la observación/claim de A, no un texto genérico «convendría investigar». **Productor de la rama:** regla versionada que compone operaciones existentes. Si faltan esas dos piezas, no hay caso para expansión.

**UAT:** evento duplicado, base stale, reinicio durante expansión, presupuesto agotado, permiso denegado y nodo ya completado. La rama no duplica efectos ni reescribe resultados previos. La prueba identifica el enlace WorkItem→PlanRevision→nodo→attempt y la semántica de operador efectivamente construida.

**Diferido:** no implementar Behaviour Trees, Wait/Loop/Race ni motor reactivo completo por anticipado.

## 7. Disposición de la propuesta original

| Propuesta | Disposición |
|---|---|
| No crear WorkGoal/WorkAssignment/WorkHandoff/WorkSession anticipadamente | Aceptar como restricción de esta propuesta. Reutilizar Goal/WorkItem/plan/run y contratos efectivos. |
| E0 consolidar todo antes de implementar | Recortar a la cadena del próximo slice. Esta revisión aporta el mapa, no abre auditoría general. |
| E1 agenda + conocimiento | Separar consulta de planificación disponible de producción estática aún pendiente. No ocultar lo segundo con una vista bonita. |
| E2 handoff completo | Reutilizar cápsulas/envelopes; exigir UAT durable antes de afirmar integración completa. |
| E3 workflows dinámicos | Diferir hasta disponer del productor y regla de expansión de un caso real. |
| E4 Jev y routing probabilístico | Fuera de ruta crítica. Sin corpus, ambigüedad residual medida y baseline determinista, no hay caso para adapter. |
| Caché incremental de conocimiento | Reutilizar identidades/CAS/revisiones, pero no declarar caché válida con hash de código únicamente. Empezar por una familia de resultados reproducibles. |
| División en varios CLI | No crear binarios. Extraer solo lógica concreta que deba compartir más de un consumidor. `run_roadmap` ya delega proyecciones, mientras `architecture_cmd::build_context` y `knowledge_ingest` sí muestran composición/lógica candidatas a separar. |

No se evaluaron documentación, precios ni rendimiento actual de TypeSafe en esta revisión. No se seleccionó proveedor. La utilidad de Jev sigue siendo una hipótesis, no una conclusión del código.

## 8. Métricas ligadas al productor

- Agenda: igualdad de campos semánticos al reconstruir la misma base, causas con refs resueltas, consultas generativas evitadas en ese caso. No exigir igualdad del timestamp de presentación.
- Observación estática: porcentaje de claims del scope con evidencia válida, Unknown/Partial explícitos, coste de producir/reutilizar y falsos resultados medidos con cambios controlados.
- Handoff: referencias obligatorias recuperables tras reinicio, omissions declaradas y número de reconstrucciones dependientes del transcript.
- Expansión: efectos duplicados, resultados completados preservados y decisiones rechazadas por base/autoridad/presupuesto.
- Complejidad: autoridades, stores y conceptos durables añadidos/retirados. Una reorganización de archivos no prueba simplificación.
- Tokens/latencia/coste: misma tarea, inputs, política y alcance de calidad antes/después. No inventar ahorros sin instrumentación de uso real.

## 9. Evidencia y límites de esta revisión

**OBSERVED:** modo on/adopción complete, runtime 1.169.88; consultas `sddk plan roadmap next` y `blocked`; selftest del resolver: 14 ok, 0 fail. El runtime instalado no incluye el CC-S0 local.

**STRUCTURAL:** fuentes y rutas citadas, búsquedas de consumidores/implementaciones y tests existentes. No se ejecutó la batería Rust ni se certificó ningún UAT propuesto. Ausencia de conexión significa «no localizada en las rutas inspeccionadas», salvo errores explícitos del código como los operadores rechazados.

**DOCUMENTED:** roadmap/ADRs/specs y límites CC-S0. Sus estados no se deducen de la existencia de archivos o tests.

**DERIVED:** priorización y slices de este documento. Pendientes de admisión como trabajo, no hechos sobre capacidades entregadas.

No se alteró el roadmap, las specs históricas, el ledger, los cambios pendientes del flake ni el gate de release. Próxima intervención recomendada: acordar el scope A6 de una relación observable real y cerrar productor→normalización→Verify, antes de ampliar agenda, agentes o modelos.
