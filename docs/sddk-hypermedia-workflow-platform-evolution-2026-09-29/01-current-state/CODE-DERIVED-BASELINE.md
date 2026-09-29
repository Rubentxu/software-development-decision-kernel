# Baseline derivado del código

## Alcance

Este documento distingue **sustrato existente** de **wiring de producto existente**. No usa un roadmap o receipt para afirmar que una feature está implementada.

Baseline de inspección: rama `main` observada el 2026-09-29; el repositorio se mueve rápidamente, por lo que los SHA concretos deben revalidarse al aplicar el paquete.

## Sustrato existente que debe reutilizarse

| Área | Código observado | Uso en esta propuesta |
|---|---|---|
| Workflow IR | `crates/sddk-domain/src/workflow_ir.rs` | Base de la definición compilada; no crear un segundo IR. |
| Workflow runtime | `crates/sddk-engine/src/workflow_runtime.rs` | Controller/reconcile loop futuro. |
| Operators | `Task`, `Sequence`, `Parallel`, `Map`, `Join`, `Race`, `Choice`, `Loop`, `Gate`, `Wait`, `SubWorkflow`, `Compensate` | Vocabulario de composición; completar semánticas pendientes de forma incremental. |
| Expansion | `ExpansionPermission::{Map,Discover,Replan}` | Discovery/replan runtime gobernado. |
| Planning | `sddk plan workitem`, deps, evidence, decisions | Persistencia semántica de trabajo. |
| Universal evidence | `evidence_ref.rs`, `EvidenceBundle` | Evidence substrate canónico. |
| Semantic relations | `semantic_kind`, `semantic_graph`, evidence relation mapping | Relaciones ObservedFor/Verifies/References/Justifies y evolución. |
| Context compiler | `context_compiler` + storage adapters | Composición de contexto; ampliar, no reemplazar. |
| Context delta | `context_bridge.rs` | Base de actualización incremental. |
| Session binding | `agentic_session_binding.rs` | Identidad host session ≠ Run; necesita store durable. |
| Child outputs | `typed_child_output.rs` | Outputs tipados/lineage para StepRun. |
| Agent contribution | `agent_contribution_envelope.rs` | Base para contributions y leases de contexto. |
| Context read tracing | `sddk-domain/src/context_read.rs` | Provenance de qué contexto consumió cada ejecución. |
| CogniCode port | `code_intelligence_port*`, static provider adapter | Provider de capabilities estáticas. |
| Chronos port | `runtime_evidence_port*`, runtime provider adapter | Provider de capabilities runtime. |
| Provider boundary | `arch-spec-021` + adapters | Anti-corruption boundary; conservar. |
| Capability gateway | `sddk-gateway` | Side effects gobernados y receipts. |
| Event log/replay | storage/event log/rebuild/replay proof | Base determinista para reconciliación. |
| Skill model | `SkillDefinition`, `AppliesWhen`, capability requirements, evidence contract | Selección automática en StepAugmentor. |

## Gaps de wiring confirmados

### G1 — Dos generaciones de workflow no convergen

El ciclo de producto sigue usando el manifest/lifecycle tradicional mientras WorkflowIR/WorkflowRuntime expresa una topología mucho más rica. `prompts/sddk/dynamic-workflow.md` añade una tercera ruta donde un LLM compone YAML y el orchestrator recorre `phases[]`.

**Riesgo:** tres autoridades parciales sobre control-flow.

### G2 — Resume desfasado respecto a cycle inference

`crates/sddk-cli/src/cycle.rs` puede inferir un ciclo cuando hay un único active lease y falla tipadamente ante 0/N candidatos. `skills/sddk-cycle-resume/SKILL.md` aún afirma que no hay descubrimiento global y bloquea sin cycle ID confiable.

**Efecto:** sesiones nuevas pueden no rehidratar un ciclo que el runtime sí sabe resolver.

### G3 — Adoption idempotente en engine, UX todavía repetitiva

El engine distingue estados de adopción/convergencia, pero la experiencia agentica mantiene adoption como ritual explícito en bootstrap. Falta un `ensure/bootstrap` de alto nivel que no pregunte por acciones reparables automáticamente.

### G4 — Context compiler potente, bootstrap pobre

El sustrato permite planning/run/memory/graph/vault, provenance y staleness. El bootstrap agentico real recupera un conjunto bastante más pequeño de estado y obliga a releer ficheros o recomponer decisiones.

### G5 — Stores críticos todavía in-memory/default-null

Existen seams de store para capsules/session/output, pero parte del wiring productivo usa stores in-memory o persistence nula. La semántica durable está mejor definida que su incorporación al camino feliz.

### G6 — CogniCode/Chronos son integraciones, no todavía capacidades workflow-native

Los adapters existen. Los prompts mencionan providers de forma desigual: `design` incluye ambos; otras fases no. Su selección depende demasiado del launch plan/prompt y no de un resolver de capabilities asociado al StepDefinition.

### G7 — Planning/evidence/decision CLI infrautilizado por los workflows

Los comandos existen y persisten a storage/CAS, pero muchos workflows producen principalmente Markdown. El workflow debería producir primero entidades tipadas y renderizar Markdown como proyección humana.

### G8 — Handoff aún narrativo/procedural

Existen `ContextDelta`, `ContextLease`, typed outputs y semantic refs, pero el siguiente agente no recibe una representación uniforme del estado con affordances legales. Continúa necesitando conocer demasiado protocolo de antemano.

## Consecuencia arquitectónica

No se recomienda añadir otro scheduler, otro graph store, otra base de memoria, otro formato de evidence o un nuevo engine de workflow. La evolución debe ser de **convergencia y wiring**.

## Hipótesis a falsar durante implementación

1. `WorkflowIR` actual puede representar los workflows software por defecto sin pérdida material tras completar operadores necesarios.
2. `SkillDefinition::select_for(task_kind, context_keys)` puede alimentar augmentation sin convertirse en authority de permisos.
3. ContextCompiler puede evolucionar a un ensamblador de referencias tipadas sin necesidad de duplicar semantic graph.
4. Hypermedia affordances pueden derivarse de state + policy + authority sin persistirse como hechos canónicos.
5. Un workflow de libro puede ejecutarse con el mismo core, demostrando que `software.*` pertenece a un pack.
