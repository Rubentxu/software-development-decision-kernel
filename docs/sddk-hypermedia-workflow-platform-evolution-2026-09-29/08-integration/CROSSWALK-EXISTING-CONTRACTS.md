# Crosswalk con contratos existentes

## Objetivo

Marcar explícitamente qué propuesta **extiende** algo ya existente y dónde sería peligroso duplicar una autoridad.

| Existente | Estado observado | Nueva propuesta | Regla |
|---|---|---|---|
| `workflow_ir.rs` | rico operator vocabulary + budgets/expansion | WorkflowDefinition compiler | reutilizar como IR; no `WorkflowIR2` |
| `workflow_runtime.rs` | runtime real | single workflow authority | ampliar/cutover, no scheduler nuevo |
| `workflow.rs` / manifest | lifecycle usado por cycle | legacy adapter | compatibility durante strangler |
| `prompts/sddk/dynamic-workflow.md` | LLM compone YAML/pasea phases | generated WorkflowDefinition | dejar de ser execution authority tras C6 |
| `SkillDefinition` | task_kind/context/caps/evidence | StepAugmentor input | no SkillDefinitionV2 paralela |
| `operator_contract.rs` | schemas de input/output | StepDefinition contracts | referenciar/extender donde encaje |
| `typed_child_output.rs` | typed lineage | StepRun outputs | persist adapter, no output model duplicado |
| `agent_contribution_envelope.rs` | context lease/delegation/contribution substrate | Contribution envelope | consolidar como base |
| `context_compiler` | adapters/provenance/staleness | context bootstrap | wiring durable/productivo |
| `context_bridge` | bootstrap/delta monotonic | durable ContextDelta | persist, no sustituir semantics |
| `agentic_session_binding` | session != run, binding modes | SessionBindingStore | durable adapter |
| `context_read.rs` | bounded context reads | provenance de expansions | persist/consume |
| `evidence_ref.rs` | universal evidence | provider/agent evidence | usar, no provider evidence store |
| planning WorkItems | work/deps/decisions | tasks→planning | auto-persist por reconciler |
| semantic graph | canonical projection | relations from contributions | project, no second graph |
| `arch-spec-021` | intelligence boundary | capability/provider resolver | completar product wiring |
| `arch-spec-024` | session binding intent | C3j | operationalize durable store |
| `arch-spec-026` | context delta intent | C3j/C6e | operationalize + hypermedia |
| CapabilityGateway | governed effects | action invocation | reuse authority/effects boundary |
| AgentProfile/CommandSpec | side effects/authority | affordance admission | map/reuse, no independent role model |

## Contracts que probablemente se superseden parcialmente

### `sddk-cycle-resume/SKILL.md`

Se mantiene como adapter/usage guidance, pero deja de ser autoridad de los argv de bootstrap cuando exista `context bootstrap` application service.

### Phase prompt CLI recipes

Se deprecian por responsabilidad, no por archivo completo. El prompt conserva reasoning/domain instructions; persistence/lifecycle recipe migra a runtime/reconciler.

### `dynamic-workflow.md`

Puede conservarse como **workflow proposal skill** hasta que el authoring UX viva en commands/tools, pero la salida debe ser WorkflowDefinition validada y compilada.

## Contratos que no deben cambiar de autoridad

- host transcript ownership;
- Knowledge vs Alignment vs Verification vs Authority boundaries;
- Base mode sin providers;
- provider as evidence source, not truth authority;
- semantic graph as projection from canonical facts;
- CAS/event log integrity;
- explicit human authority for governed destructive decisions.
