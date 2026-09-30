# Roadmap único de continuación — SDDK production ready

**Edición:** 2026-09-21 · **Baseline inspeccionado:** `main@2ffff3127e7179b5f3c3104c471c8ad2c7d920ff`, workspace `1.169.127`; última release pública comprobada durante la auditoría: `v1.169.122`. **Estos valores son una fotografía, no un estado en tiempo real.** Consultar [CURRENT.md](CURRENT.md) y Git antes de actuar. Este plan NO declara que esa release sea actual ni que los gates estén verdes hoy.

## 0. Intención, autoridad y continuidad temporal

**Objetivo:** conservar el núcleo semántico ya entregado, cerrar desviaciones verificadas, certificar perfiles operativos reales y solo entonces abrir ampliaciones host/proveedor/arquitectura con valor demostrado. Mantener Knowledge (qué sabemos) → Alignment (interpretación/advisory) → Verification (qué se comprobó) → Governance/Authority (qué está permitido) como fronteras con una sola autoridad por concepto. Local-first, sin segunda fact log, segundo grafo canónico, segundo scheduler ni doble aprobación.

**No partir de cero.** Baseline certificado: M0–M9/C0–C7 históricos; A0–A5 Base certificado en `v1.169.88` **condicionado** a aceptación de G11 no verificado en su momento; revisión R14 posterior a nivel de código, no auditoría adversarial. Hitos AC0–AC14, J0–J6 y AIW-S0–S8 tienen cierres **de alcance concreto** en los recibos de 19–21/09. Las verticales reales CogniCode/Chronos NO equivalen a perfil enhanced completo; JCode semántico NO equivale a adapter productivo + AG2. Ver [certificación Base](../architecture/a5/A5-C-BASE-PRODUCTION-READY-CERTIFICATION.md), [estado AIW](../proposals/2026-09-19-adaptive-inputs-workflows/STATE-OF-AIW.md) y [último estudio de gaps](../research/2026-09-21-roadmap-gaps-deep-research.md).

**Estados admisibles de un WorkItem:** `PROPOSED | READY | IN_PROGRESS | BLOCKED | IMPLEMENTED | VERIFIED | CERTIFIED | DEFERRED | SUPERSEDED`. `IMPLEMENTED` exige commit y scope; `VERIFIED` requiere prueba OBSERVED vinculada a ese SHA; `CERTIFIED` requiere todos los gates del perfil en [CERTIFICATIONS.md](CERTIFICATIONS.md). Solo mover a `DEFERRED` con razón, dependencia y disparador; no contarlo como PASS. No porcentajes inventados: avance = requisitos verificados / requisitos aplicables congelados por hito; `NOT_RUN`, `BLOCKED` y `N/A` se contabilizan por separado.

## 1. Dependency DAG: continuidad sin abrir trabajo antes de tiempo

```text
C0 Reconciliar HEAD/release y congelar baseline + contratos
  └── C1 Endurecer contratos/identidades/boundaries (H01,H02,H05,H06)
        ├── C2 Certificar integración real: CogniCode, Chronos, JCode
        └── C3 Seguridad, Authority y Storage adversarial/recovery
               └── C4 Certificación del perfil de producto y publicación
                     └── C5 Opcionales evidencia-driven (X08,J7,J8,J9,R11)
```

C2 y C3 pueden realizarse en paralelo **solo después** de C1 y con el mismo contrato de seguridad; C4 exige el conjunto de gates aplicables de ambos. Se puede publicar Base sin exigir enhanced/Agentic; no etiquetar Full/GA por coexistencia de módulos o cuentas de tests.

## 2. Hitos, entregables y salida falsable

### C0 — Reconciliación y baseline reproducible (P0, primero)

**Entradas:** SHA, Cargo.toml, tag/release remota, A5-C, mini-roadmap 14/09, recibos A6/A7/A8/J/AIW, AGENTS, incidencias abiertas. Confirmar si existe una release posterior antes de copiar la fotografía de 21/09.

**Acciones:** inventariar `IMPLEMENTED/VERIFIED/CERTIFIED/DEFERRED` por feature; enlazar commit, test, evidencia real/fake, aceptación y límites; detectar drifts; identificar versión real del binario/bundle/config/schema; fijar los contratos de política/capacidades relevantes; preparar o reconciliar mecanismo de release `chore(release): bump version` sin publicar de manera automática.

**Salida:** `CURRENT.md` / `STATE.yaml` consistentes, matriz [UAT](UAT-MATRIX.md) congelada y recibo de baseline con referencias y riesgos aceptados. **No** pasar a C1 con repo dirty o SHA ambiguo. No modificar el histórico para forzar coherencia.

### C1 — Falsación de contratos y hardening acotado (P1)

**H01:** en `structured_work::shape_matches`, descriptor no soportado no puede dar `true`; versión/esquema explícito, campos requeridos y adicionales según contrato. **H02:** `request_id` duplicado no sustituye silenciosamente una petición no equivalente; errores de schema nunca interpolan valores sensibles. **H05:** test-seam `set_process_service_for_tests` con semántica real y aislamiento de producción. **H06:** defensa gateway de argumentos/bytes, límites agregados y redacción completa de las salidas.

**Método:** caracterización RED→GREEN por cambio mínimo, UAT T01–T07, revisión de compatibilidad/migración, verificar regresión acotada en `apply` y global en `verify`. No convertir el validador en nuevo motor de schemas sin justificación ni duplicar autorización en host.

**Salida:** código + tests negativos + recibo por slice con escenarios reales/fake distinguidos; resultados válidos/inválidos estables; cero exposición de canarios en corpus definido. Riesgos de cambios breaking documentados.

### C2 — Integraciones reales y declaraciones por perfil (P1, tras C1)

**C2a CogniCode:** preservar S1 real observado (v0.97.1) como evidencia histórica, verificar versión actual, ciclo de vida/capability negotiation, completitud/frescura del grafo, falsos negativos, contradicciones, restart y degradación Base sin proveedor. No elevar `find_usages` sobre grafo lightweight a evidencia de cobertura global. Ejecutar EXT con binario real y documentar limitaciones.

**C2b Chronos:** preservar S5 real histórico (v0.1.0); revisar adaptador, escenarios multi-programa/probes, timeout/cancelación, capture vacía, restart, recursos, compatibilidad, ausencia de exit-status inferido y no bypass de Authority. Ejecutar EXT real.

**C2c JCode:** distinguir J2–J6 semánticos de adaptación real. Verificar si el adapter público vive en otro repo antes de implementar uno duplicado; demostrar boundary `jcode-sdk` / SDDK SDK (o ADR de equivalencia), session≠run, transcript host-owned, event coalescing, context delta, structured return, redacción en gateway, negociación de capacidades y ejecución companion + orchestrated. No declarar JCODE_CORE_GA sin AG0–AG2 y recibo específico.

**Salida:** recibos verificables e independientes por proveedor/host, entorno, hashes, versiones, casos UAT T08–T18; `NOT_EVALUATED` si falta binario o consumidor real; ausencia de proveedor nunca es PASS.

### C3 — Resiliencia, seguridad y fiabilidad (P1, paralelo a C2 tras C1)

**Authority:** delimitar seq de proceso vs event log, digest/política y fencing; test de interleavings, políticas concurrentes, reinicios y no side effects al denegar. Ninguna ampliación multi-proceso antes de probar su contrato. **Storage:** inventariar 8 sitios IMMEDIATE diferidos, reproducir contención y priorizar únicamente rutas activas; idempotencia, CAS, fallo parcial y crash/reopen. **Seguridad:** pruebas adversariales con canarios en args/env/stdout/stderr/error/receipt/CAS, trust-boundary para host y provider, secret-screen y dependencias/supply chain. **Rendimiento:** presupuesto medible de p95 y recursos en tres escenarios fijados (Base, static, runtime); baseline antes de optimizar.

**Salida:** UAT T19–T27 y recibos con incidentes restantes clasificados; pruebas de contención/recuperación observadas, sin promesas de seguridad universal.

### C3i — Coherencia de bootstrap/adoption/cycle recovery (P0/P1, follow-up de C3, fuente: paquete hypermedia)

**Fuente:** paquete `docs/sddk-hypermedia-workflow-platform-evolution-2026-09-29/` (delta propuesto, adoptado como follow-up descubierto; no reabre receipts históricos de C3a-h). **Dependencia:** tras C3. **Estado:** **VERIFIED (session-48) — implementación completa desde session-36/38; las dos últimas UAT NOT_RUN se cerraron con evidencia real.** **Superficie:** `skills/sddk-cycle-resume/SKILL.md`, `prompts/sddk/mcw.md`, `skills/_shared/cli-usage-contract.md`, `tests/test_workflow_contract.py`, application service de adoption en `sddk-cli`.

**Problema falsable (RESUELTO — verificado en código 2026-09-29, cerrado en session-36; el enunciado de abajo se conserva como historia):** `crates/sddk-cli/src/cycle.rs` (`resolve_cycle_context_with_cwd`) infiere el ciclo activo con degradación tipada (0 → `NoActiveCycle`, >1 → `AmbiguousCycle` con candidates), mientras `skills/sddk-cycle-resume/SKILL.md` y `prompts/sddk/mcw.md` aún enseñan que "no hay descubrimiento global" y bloquean con `runtime-active-cycle-discovery-unavailable`. Las sesiones nuevas no rehidratan un ciclo que el runtime sí sabe resolver. Pines vigentes en `tests/test_workflow_contract.py` (línea 1086) congelan la afirmación obsoleta.

**Objetivos:** (1) alinear la superficie de orquestación con la realidad del resolver (descubrimiento 0/1/N tipado); (2) bootstrap/adoption repetido en proyecto convergido = no-op semántico, sin ritual de re-adopción por sesión; (3) 0/1/N ciclos producen estados tipados y recoveries correctas; (4) el agente no vuelve a pedir adopción en cada sesión; (5) una única identidad project/workspace/cycle coherente del bootstrap.

**Exit gate:** 20 reinicios del mismo proyecto convergido no vuelven a pedir adopción; ciclo único inferido sin ID explícito; múltiples ciclos → ambigüedad tipada, nunca adivinar; sin remote → misma identidad persistida; callers legacy siguen funcionando.

**UAT:** CTX-UAT-001..005, MIG-UAT-001 (en [UAT-MATRIX.md](UAT-MATRIX.md)). **Cierre session-48:** CTX-UAT-001 PASS (x20, recibo byte-estable), **CTX-UAT-002 y CTX-UAT-003 PASS** (`tests/uat_ctx_004_cycle_inference.sh`, con falsador RED→GREEN observado: neutralizar el guard de ambigüedad hace caer el script), CTX-UAT-004 re-verificada (4/4) tras arreglar su premisa obsoleta de exit code. Siguen abiertas: CTX-UAT-005 (skill contra runtime 0/1/N en un mismo binario) y MIG-UAT-001 (migración skill vieja vs nueva), ambas automatizables con el mismo patrón si se priorizan. **No-objetivos:** no cambiar la semántica del resolver de `cycle.rs`; no tocar C4; no abrir C6.

### C3j — Contexto durable, session binding y handoff hipermedia mínimo (P1, depende de C3i)

**Fuente:** mismo paquete. **Dependencia:** C3i. **Superficie:** CapsuleStore y SessionBindingStore persistentes detrás de los seams existentes (`context_bridge.rs`, `agentic_session_binding.rs`), application service `context bootstrap`, progressive refs + `context expand`, ContextDelta durable/monotónico entre procesos, primera representación hipermedia de Project/Run/Step.

**Objetivos:** (1) CapsuleStore persistente; (2) SessionBindingStore persistente; (3) `context bootstrap` service; (4) progressive refs + expand mínimo; (5) ContextDelta durable entre reinicios de proceso; (6) primera representación hipermedia de estado, sin cutover de control-flow.

**Exit gate:** dos procesos secuenciales reconstruyen el mismo basis/capsule desde storage; el segundo recibe solo delta si hay cambio; session ≠ run preservado; el transcript no se importa.

**UAT:** CTX-UAT-006..015, HYP-UAT-001..004. **No-objetivos:** no WorkflowDefinition custom, no migración completa de prompts, no augmentors de terceros.

### C3k — Integridad de evidencia UAT/identidad/gates CLI (P1, fuente: defectos 2.2.33 reportados desde agent-secretless)

**Fuente:** report externo `agent-secretless/docs/receipts/sddk-2.2.33-defects.md` + confirmación en código de este repo (`docs/research/2026-09-30-sddk-cli-defects-evolution-plan.md`, 11 hallazgos confirmados con file:line). **Estado:** **COMPLETO (session-46b, release v2.3.0)** — recibo en `tests/cycle-artifacts/p-63676b11dc0ef88f/c3k-release-v2.3.0/RECEIPT.md`. **Dependencia:** tras C3 (mismo dominio de integridad).

**Problema falsable (verificado en código 2026-09-30):** (D1) `run_uat_signoff` (uat.rs:1687) firma aceptaciones sin parsear el plan, con snapshot de evidencia literal `"sha256:{}"` y `--actor` libre: un agente puede fabricar una aceptación humana indistinguible sobre cero escenarios. (D2) la identidad del proyecto se deriva de la URL del remote sin normalizar el case del path (`identity.rs:310`) y `ensure_project_row` (admission.rs:593) materializa ledgers nuevos en silencio: un cambio de case del remote escribe con éxito en el ledger equivocado. (S3.1/S3.2) `register_evaluator` (engine lib.rs:1165) no tiene call sites en el CLI mientras `prompts/sddk/phases/release.md:109` instruye evaluar `release-receipt`/`no-pending-effects` con `--evaluator sddk.cli`: seguir el prompt cuesta `ENGINE_UNREGISTERED_EVALUATOR` garantizado. (D3/D7) backlog sin guard de linaje en discard ni check de drift en render. (D4/D5/D6/S3.3/S3.4) proyecciones UAT sin anclaje de root, sin validación de `--from`, esqueleto sin declarar, sin schema de session/report, y `upsert_uat_result` (control_plane.rs:255) pisa multi-sesión con `session_count` mal contado.

**Objetivos (W1..W7 en el doc de research):** (1) sign-off exige plan con escenarios, evidencia real y marca `--agent-authored` para actores agente; (2) identidad: normalizar case del path, `sddk project pin`, warning fail-loud ante ledger vacío nuevo; (3) discard con `--superseded-by` verificado; (4) gates del workflow de release con evaluador registrado (o prompts corregidos, según decisión); (5) status/plan UAT anclados a root y validados; (6) `render --check` para CI; (7) ingest multi-sesión acumulativa con schemas de session/report.

**Exit gate:** los comandos de la sección 6 del report de agent-secretless reproducidos contra el framework no materializan aceptaciones falsas, ledgers divergentes ni errores de evaluador inexistente; tests RED→GREEN negativos por WI; regressiones con las URLs exactas del report.

**UAT:** UAT-SIGN-001..004, UAT-IDEN-001..004, UAT-BLG-001..003, UAT-GATE-001..002, UAT-UAT-001..004 (alta en UAT-MATRIX.md al pasar a READY). **No-objetivos:** no re-diseñar el modelo de identidad ni el motor de gates; poblar y cerrar mecanismos existentes.

### C4 — Release y certificación de producto (P0 para cada declaración)

Evaluar [CERTIFICATIONS.md](CERTIFICATIONS.md), ejecutar perfil completo local sin `--skip-tests`, construir y verificar binario/bundle/manifest/SBOM/hashes, clean-machine UAT, migración/replay, publicar mediante `bash scripts/release.sh` **solo con autorización del operador**, verificar assets públicos y registrar SHA/tag/env/resultado. Un fallo de gate bloquea **esa certificación**; Base puede permanecer certificada aunque Enhanced no lo esté si no se ha roto Base. Certificación historical != current HEAD.

**Salida:** `CERTIFICATION-RECEIPT` reproducible para cada perfil válido, cero bloqueadores sin disposición, riesgos aceptados explícitos con caducidad/revisit trigger y versión pública exactamente correlacionada con recibos. UAT T28–T33.

### C5 — Evolución condicionada a pruebas de valor (P2/P3, no bloquea C4-Base)

**X08:** Jev benchmark solo con corpus etiquetado, baseline y métrica definidos. **J7:** MCP pull solo si un consumidor demuestra necesidad que push/SDK no resuelve. **J8:** operaciones host avanzadas tras negociación de capabilities y UAT propios. **J9:** segundo host real antes de declarar `AGENTIC_API_STABLE/1.0`. **R11:** split de crates solo con métricas sostenidas de dependencia, frecuencia de cambios y compilación; conservar puertos/ownership, no hacer un split cosmético. Future async Parallel es **nueva feature**, no reapertura de R12 ya cerrado.

**Salida:** cada idea es `DEFERRED` hasta que el disparador y el SCOPE existan; no convertirlas en backlog ejecutable por inercia.

### C6 — Convergencia de plataforma de workflows (post-C4, siblings con C5, fuente: paquete hypermedia)

**Fuente:** mismo paquete. **Estado:** PROPOSED. **No abre** hasta que C3i/C3j dejen continuidad de sesión no frágil y exista cutover post-C4. Detalle completo en `docs/sddk-hypermedia-workflow-platform-evolution-2026-09-29/05-roadmap/ROADMAP-OVERLAY.md` (C6a contracts → C6b compiler+equivalence shadow → C6c StepAugmentor+CapabilityResolver+ProviderRegistry → C6d ContributionReconciler+vertical software → C6e hypermedia surface+prompt slimming → C6f user workflows). Reutiliza sin duplicar: `WorkflowIR` (`crates/sddk-domain/src/workflow_ir.rs`), `WorkflowRuntime` (`crates/sddk-engine/src/workflow_runtime.rs`), ContextCompiler/Capsule/Delta, `SkillDefinition`/registry, CapabilityGateway, puertos CogniCode/Chronos como **capabilities resueltas, no providers hardcodeados**, evidence/EvidenceRef, semantic graph, typed child outputs, session binding. Reglas duras: una fact log, un grafo, un runtime; `Step != Skill != Capability != Provider`; prompts sin control-flow ni autoridad de persistencia; strangler, no big bang.

### C7 — Generalización por packs y segundo dominio (experimental, tras C6)

**Fuente:** mismo paquete. **Estado:** PROPOSED, no abrir antes de C6. Falsar fuga de conceptos software en el core: boundary del software pack (C7a), pack de autoría/book con providers fakes deterministas primero (C7b, p.ej. `create-book`), certificación experimental de portabilidad al segundo dominio con el mismo runtime/capsule/protocolo/evidence y cero forks de core (C7c). Si no se cumple, no declarar el core genérico; registrar qué abstracción falló.

## 3. Contrato de cambio para cualquier slice

1. `SCOPE-CONTRACT`: baseline SHA, objetivo falsable, invariantes y no-objetivos, riesgo, superficie de código, tests, dependencias, stop conditions y presupuesto.
2. Test de caracterización pre-fix cuando proceda; `apply` ejecuta solo test scope derivado del SUT; en `verify` ejecutar perfil completo.
3. `UAT-EVIDENCE`: entrada real/fake, comando, SHA, entorno, resultados, prueba negativa, output digests y limitaciones. `NOT_RUN` nunca se convierte en PASS por lectura de código.
4. `RECEIPT`: commit, pruebas, decisiones, riesgos, semántica de compatibilidad y release/tag cuando exista; subir documentación en el mismo cambio de concernencia.
5. Actualizar CURRENT + STATE + SESSION-JOURNAL **después** de comprobar el resultado. No alterar en silencio la baseline para acomodar resultados.

## 4. Indicadores operativos

Registrar, sin universal score: requisitos aplicables vs verificados por perfil, pruebas flakey/ignored, tiempo de ciclo verify, bloqueos por proveedor, defects escaped, duration p50/p95, tiempos de recuperación, cambios cruzados de crates y número de riesgos aceptados pendientes. Cualquier porcentaje informa denominador, SHA, suite y alcance. La ausencia de medida se presenta como `NOT_MEASURED`.

## 5. Fuentes históricas conservadas

[Arquitectura actual](../architecture/README.md) · [14/09 mini-roadmap histórico](../SDDK-Production-Readiness-Alignment-2026-09-14/02-MINI-ROADMAP.md) · [A5 certificación](../architecture/a5/A5-C-BASE-PRODUCTION-READY-CERTIFICATION.md) · [AIW state 21/09](../proposals/2026-09-19-adaptive-inputs-workflows/STATE-OF-AIW.md) · [Roadmap gaps 21/09](../research/2026-09-21-roadmap-gaps-deep-research.md) · [Archivo](../history/README.md).
