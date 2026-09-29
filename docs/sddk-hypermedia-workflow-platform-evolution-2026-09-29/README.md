# SDDK Hypermedia Workflow Platform Evolution

**Fecha:** 2026-09-29  
**Tipo:** paquete de evolución integrable, no sustitución silenciosa del roadmap vigente  
**Repositorio objetivo:** `Rubentxu/software-development-decision-kernel`

## Propósito

Este paquete refina una evolución de SDDK desde un kernel con muchas capacidades ya implementadas pero todavía parcialmente desconectadas hacia una **plataforma agent-first de workflows dinámicos, reutilizables y navegables por estado**.

La propuesta une cinco ideas:

1. **Handoff/contexto hipermedia** inspirado en HATEOAS: cada recurso expone estado, relaciones y acciones legales en ese instante.
2. **WorkflowDefinition → WorkflowIR → WorkflowRuntime** como única cadena de autoridad de ejecución.
3. **StepDefinition reutilizable** como unidad composable independiente del workflow que la consume.
4. **Augmented Steps**: SDDK enriquece automáticamente cada step con contexto, skills, capabilities, providers, evidence, persistence, authority y observabilidad.
5. **Core genérico + packs de dominio**: el software development es el primer pack, no una restricción estructural. Un segundo dominio —creación de libros— se usa como falsador de generalidad.

## Regla de integración

Este paquete **NO declara hitos actuales como cerrados ni modifica receipts históricos**. Propone un delta sobre `docs/roadmap/ROADMAP.md`:

- **C3i** — bootstrap/adoption/cycle recovery coherente e idempotente.
- **C3j** — contexto y session binding durables + handoff hipermedia mínimo.
- **C4** — se mantiene sin cambio semántico.
- **C5** — se mantiene como carril opcional existente.
- **C6** — convergencia de plataforma de workflows/steps/augmentation/hypermedia.
- **C7** — generalización por packs y segundo dominio real.

C3i/C3j son hardening/recovery. C6/C7 son evolución de producto posterior a la baseline certificable, no excusa para reabrir trabajo ya cerrado.

## Principios no negociables

- Una sola fact log canónica.
- Un solo grafo semántico canónico.
- Un solo runtime de workflows.
- Los prompts no son control-flow ni autoridad de persistencia.
- Los workflows piden capacidades; no acoplan providers concretos salvo override explícito.
- CogniCode/Chronos son fuentes de observación, nunca autoridad de verdad o aprobación.
- El usuario que crea un workflow no debe aprender los comandos internos de SDDK para obtener persistencia, evidence, decisiones o handoff correcto.
- Los steps deben ser reutilizables mediante contratos tipados de entrada/salida.
- La evolución es strangler: aditiva, observable y reversible hasta cada cutover.
- Ningún porcentaje sin denominador y ninguna certificación por lectura de código.

## Índice

### Contexto y arquitectura
- `00-executive/EXECUTIVE-SYNTHESIS.md`
- `01-current-state/CODE-DERIVED-BASELINE.md`
- `02-architecture/TARGET-ARCHITECTURE.md`
- `02-architecture/BOUNDED-CONTEXTS-AND-DEPENDENCY-RULES.md`
- `02-architecture/EMERGENT-ARCHITECTURE-STRATEGY.md`

### Especificaciones
- `03-specs/SPEC-001-HYPERMEDIA-RESOURCE-AFFORDANCE-PROTOCOL.md`
- `03-specs/SPEC-002-WORKFLOW-DEFINITION-COMPILATION.md`
- `03-specs/SPEC-003-STEP-DEFINITION-AND-STEP-RUN.md`
- `03-specs/SPEC-004-AUGMENTATION-CAPABILITY-PROVIDER-RESOLUTION.md`
- `03-specs/SPEC-005-DURABLE-CONTEXT-SESSION-HANDOFF.md`
- `03-specs/SPEC-006-OBSERVATION-EVIDENCE-KNOWLEDGE-INGESTION.md`
- `03-specs/SPEC-007-CONTRIBUTION-RECONCILIATION-AND-AUTOPERSIST.md`
- `03-specs/SPEC-008-PACKS-USER-WORKFLOWS-AND-REUSE.md`
- `03-specs/SPEC-009-CLI-MCP-AND-PROTOCOL-SURFACE.md`
- `03-specs/SPEC-010-SECURITY-DETERMINISM-COMPATIBILITY.md`

### ADRs
- `04-adrs/ADR-001-HYPERMEDIA-HANDOFF-INSTEAD-OF-PROMPT-HANDOFF.md`
- `04-adrs/ADR-002-ONE-WORKFLOW-AUTHORITY-IR-RUNTIME.md`
- `04-adrs/ADR-003-REUSABLE-STEP-DEFINITION.md`
- `04-adrs/ADR-004-AUGMENTATION-AS-ADMISSION-AND-MUTATION.md`
- `04-adrs/ADR-005-CAPABILITIES-NOT-PROVIDER-NAMES.md`
- `04-adrs/ADR-006-DURABLE-CONTEXT-BASIS-AND-DELTAS.md`
- `04-adrs/ADR-007-DOMAIN-NEUTRAL-CORE-AND-PACKS.md`
- `04-adrs/ADR-008-STRANGLER-MIGRATION-NO-BIG-BANG.md`

### Roadmap, UAT e integración
- `05-roadmap/ROADMAP-OVERLAY.md`
- `05-roadmap/MILESTONES-AND-DEPENDENCIES.md`
- `05-roadmap/MIGRATION-PLAN.md`
- `05-roadmap/RISK-REGISTER.md`
- `06-uat/UAT-MATRIX.md`
- `06-uat/FAILURE-MODEL-AND-ADVERSARIAL.md`
- `06-uat/TEST-STRATEGY.md`
- `07-examples/EXAMPLE-SOFTWARE-WORKFLOW.md`
- `07-examples/EXAMPLE-BOOK-WORKFLOW.md`
- `07-examples/EXAMPLE-HYPERMEDIA-REPRESENTATIONS.md`
- `08-integration/REPO-INTEGRATION-MAP.md`
- `08-integration/IMPLEMENTATION-BACKLOG.md`
- `08-integration/OPEN-QUESTIONS-AND-SPIKES.md`

## Cómo incorporar el paquete

1. Copiar este directorio inicialmente bajo una ubicación de propuesta, por ejemplo `docs/proposals/2026-09-29-hypermedia-workflow-platform/`.
2. No sustituir `docs/roadmap/ROADMAP.md` completo. Aplicar únicamente el delta explícito de `05-roadmap/ROADMAP-OVERLAY.md` tras revisión.
3. Abrir primero **C3i**, no C6.
4. Cada milestone implementado debe obtener SCOPE-CONTRACT, tests quirúrgicos, UAT observable y receipt propio.
5. Cuando C6 demuestre equivalencia runtime, los prompts procedurales se reducen gradualmente; no antes.

