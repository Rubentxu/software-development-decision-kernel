# Estado contrastado y límites de las fuentes

## Alcance

Se revisaron en GitHub a `da37510` los seis documentos de la cadena y se cotejaron archivos de `sddk-domain`, `sddk-engine`, `sddk-gateway`, `sddk-cli` y la entrada normativa. Los hallazgos de uso del bundle instalado 1.169.88, búsquedas de consumidores y selftest corresponden **a las auditorías suministradas**; no se ha recreado su entorno local ni se ha ejecutado la batería Rust aquí. Cualquier afirmación de ausencia de invocador se limita a las rutas/bundle auditados.

## Cadena de documentos, orden temporal y disposición

- 15:03 `docs/proposals/2026-09-19-jcode-anti-corruption-adapter-PROPOSAL.md`, `b442635`: JCode necesita frontera de capability/receipt segura; **propuesta no implementada**. SEC-1 ya dispone de redacción: no crear sistema nuevo de secretos.
- 15:51 `docs/proposals/2026-09-19-a6-cognicode-cc-s0-protocol-spike-PROPOSAL.md`, `aa6855e`: plantea CC-S0; a las 18:18 ya existe código de puerto, fake y tests. No reabrir CC-S0 como si faltase el trait.
- 18:18 `docs/proposals/2026-09-19-kernel-adaptativo-productores-encaje.md`, `df60a8a`: exige productor real→normalización→consumidor; IR declarado ≠ operador ejecutable; bases y digests provisionales.
- 18:34 `docs/proposals/2026-09-19-input-persistencia-proyeccion-encaje.md`, `6251af7`: puentes de observación→persistencia con contradicciones y CLI/proveedor→`SoftwareObservation`.
- 18:43/18:55 `docs/proposals/2026-09-19-inputs-origen-y-adopcion-auditoria.md`, `c717aab`: clasifica productores vivos, efímeros y código sin conexión productiva; amplía auditoría a skills, agentes y 30 comandos top-level; 12 sin mención en bundle auditado, NO necesariamente sin consumidores externos.
- 19:12 `docs/proposals/2026-09-19-inputs-auto-vs-delegado.md`, `da37510`: propone captura por ciclo/evento y CLI especializadas delegadas al agente. Este paquete mantiene la división, corrige automatización universal, seguridad, idempotencia y alcance.

## Matriz: observado en código vs conexión productiva

| Necesidad | Evidencia técnica a base | Estado honesto y límite |
|---|---|---|
| Planning/agenda | `planning/mod.rs`: `WorkItemV1`, `DependencyEdgeV1`, `DecisionRecordV1`; `planning/projections.rs`: `project_next`, `project_blocked`; `plan.rs::run_roadmap` | Proyección real sobre snapshot de spine; **no** scheduler general ni admisión Authority. Más de un Active y refs ausentes son escenarios explícitos. |
| Captura Git | `architecture_cmd.rs::git_changed_paths`, `changed_basis`, `path_overlaps` | Productor invocado bajo demanda; relación entre paths modificados y locators declarados **no** es impacto transitivo de código. |
| Observación/Verify | `observation/` contiene `SoftwareObservation`, basis, soporte/contradicción; `verify_kernel_cmd.rs` crea `ObservationSet::new()` | Sustrato real, pero esa ruta de CLI verifica sin observaciones importadas; ausencia no implica PASS. |
| A6 puerto estático | `code_intelligence_port.rs`, Fake/Null, ADR-0137 aceptada y test CC-S0 | Seam de spike, no proveedor CogniCode real ni AC10 en producción. `DigestSha256::of` implementa FNV-1a de 64 bits, no SHA-256: **no usar para identidad durable**. |
| Runner externo | `gateway/runner.rs` (`RunSpec`, `RunOutcome`, timeout, stdout/stderr limitados); `gateway/test_runner/` seis familias | Código reusable. No inferir ingestión de reportes por testcase ni una ruta CLI/Verification completa; límite de salida puede causar evidencia incompleta. |
| Knowledge scan | `sddk-cli/src/knowledge_ingest.rs` scan/import/verify con registry propio | Código expuesto; 0 invocaciones scan/import en prompts/skills auditados. No confundir registry documental con KnowledgeBasis del engine. |
| Contexto/handoff | `context_capsule.rs::ContextCompiler`, `CapsuleInputs`; `cold_start.rs`; `agent_contribution_envelope.rs` | Código y contratos parciales; no se ha demostrado handoff tras reinicio entre agentes reales con evidence carry-forward. `arch-spec-007` propone tipos que no deben declararse implementados solo por aparecer en documento. |
| Secretary L0/L1/L2 | `secretary_l0.rs`, `secretary_l1.rs`, `secretary_l2_replan.rs` | Algoritmos y tests de módulo; auditoría no encuentra integración productiva Planning+Knowledge en los prompts/CLI observados. |
| Insights | `intelligence_loop::compose_intelligence_loop`, `intelligence_advisory::derive_advisory_context` | Composición de inputs inyectados; **no** se convierten en analizadores productores. |
| Workflow IR/runtime | `workflow_ir.rs` declara Task, Sequence, Parallel, Map, Choice, Join, Race, Loop, Gate, Wait, SubWorkflow, Compensate | `engine/operator.rs::build_operator` construye Task/Sequence/Parallel/Choice/Map; otras variantes citadas retornan `NotImplementedInCycle16` en esa ruta. No equiparar IR con comportamiento disponible. |
| Chronos | IPB, documentación A7 | No se constató adaptador productivo en rutas auditadas; requiere escenario, instrumentación y comparabilidad. |

## Autoridad documental y contradicciones de estado

`docs/architecture/README.md` y `docs/architecture/a5/A5-CURRENT-ROADMAP.md` son normativos. El roadmap consultado indica A5-C certificado en v1.169.88, A6/A7 pendientes, A8 bloqueado, J2..J6 paralelos, R11 evaluación futura. CC-S0/ADR-0137 son posteriores al texto histórico inicial que decía que el trait no existía: reconciliar la línea A6 en el próximo sync, **sin inferir que STATIC_ENHANCED está entregado**. SPEC-042 Secretary Runtime es histórica/proposed respecto a la autoridad actual: no revivir agenda Vault canónica. A5-C, SEC-1 y sus receipts no se recertifican.

## Referencias técnicas base

- `docs/architecture/README.md`; `docs/architecture/a5/A5-CURRENT-ROADMAP.md`.
- `docs/architecture/adrs/ADR-0137-CODE-INTELLIGENCE-PORT-SEAM.md`; `docs/architecture/specs/arch-spec-021-intelligence-provider-boundary.md`.
- `crates/sddk-domain/src/planning/mod.rs`, `planning/projections.rs`, `workflow_ir.rs`, `workflow_run.rs`.
- `crates/sddk-engine/src/operator.rs`, `code_intelligence_port.rs`, `observation/`, `context_capsule.rs`, `cold_start.rs`, `agent_contribution_envelope.rs`, `intelligence_loop/mod.rs`.
- `crates/sddk-cli/src/verify_kernel_cmd.rs`, `architecture_cmd.rs`, `knowledge_ingest.rs`.
- `crates/sddk-gateway/src/runner.rs`, `test_runner/`.
