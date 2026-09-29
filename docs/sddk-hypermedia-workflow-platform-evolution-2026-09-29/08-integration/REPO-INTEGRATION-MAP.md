# Mapa de integración con el repositorio actual

## Principio

Los nombres siguientes son **targets sugeridos**, no mandato de crear nuevos crates. Primero integrar dentro de boundaries existentes y medir.

## C3i targets

### `crates/sddk-cli/src/cycle.rs`

Reutilizar `resolve_cycle_context` como autoridad de cycle inference. No duplicar walker/resolver en skill/prompt.

### Adoption

Localizar el composition path actual de `adopt status/apply` y extraer un application service semántico tipo `ensure_adoption` sólo si evita duplicación. El engine `adoption.rs` sigue siendo la lógica de convergencia.

### `skills/sddk-cycle-resume/SKILL.md`

Eliminar la afirmación obsoleta de que no hay active-cycle discovery. Skill describe porcelain/bootstrap contract, no recipe duplicado si el nuevo command lo encapsula.

### Tests

- `sddk-cli` inference tests;
- workflow contract tests;
- adoption identity regression;
- repeated-session harness.

## C3j targets

### `crates/sddk-engine/src/context_compiler/`

Mantener ContextCompiler. Añadir adapters reales sólo donde haya source authority clara.

### `crates/sddk-engine/src/context_bridge.rs`

Preservar monotonic delta semantics. Separar core delta ADT de in-memory bridge state si hace falta para persistent adapter.

### `crates/sddk-engine/src/agentic_session_binding.rs`

Mantener identity/binding ADTs. Extraer `SessionBindingStore` port si no existe de forma utilizable productivamente.

### `crates/sddk-storage`

Posibles tablas/projections:

```text
context_capsules
context_bindings
context_deltas
context_reads (si no existe durable consumer)
```

No crear tablas si event/projection existing model ya cubre la necesidad. Hacer spike de storage shape antes.

### `crates/sddk-cli/src/context.rs` (posible nuevo módulo)

Porcelain `context bootstrap/expand`. Si command registry naming aconseja otro lugar, mantener application service separado del CLI parser.

## C6a targets

### `sddk-domain`

Nuevos contracts estrictamente puros:

```text
resource_ref.rs        (si no hay primitive equivalente reutilizable)
affordance.rs
step_definition.rs
workflow_definition.rs
capability_descriptor.rs
observation.rs         (si la actual observation model cubre, extender en lugar de duplicar)
```

Antes de crear cada módulo, buscar autoridad existente (`operator_contract`, `workflow_ir`, evidence models, semantic nodes). Preferir extensión/alias a duplicación.

### `sddk-cli/src/skill_definition.rs`

Reutilizar AppliesWhen/task_kind/context_keys/capability requirements. Evitar un segundo SkillRegistry.

## C6b targets

### `crates/sddk-domain/src/compiler.rs`
### `crates/sddk-domain/src/execution_graph_compiler.rs`
### `crates/sddk-domain/src/workflow_ir.rs`
### `crates/sddk-engine/src/workflow_runtime.rs`

Objetivo: una cadena de compilación. No crear `workflow_runtime_v2.rs` salvo spike/feature branch transitorio sin coexistencia productiva.

### Legacy workflow

`workflow/workflow.yaml` y `workflow.rs` se consumen mediante compatibility adapter durante shadow/equivalence. No borrar al inicio.

## C6c targets

### Nuevo engine service posible

```text
step_augmentation.rs
capability_resolver.rs
provider_registry.rs
```

Los nombres pueden variar; responsabilidades no.

### Adapters existentes

- `code_intelligence_port.rs`
- `code_intelligence_port_mcp.rs`
- `runtime_evidence_port.rs`
- `runtime_evidence_port_mcp.rs`
- `verify_kernel/adapter_*_provider.rs`

Envolver, no reescribir por estética.

## C6d targets

### Contribution side

Reutilizar:

- `agent_contribution_envelope.rs`;
- `typed_child_output.rs`;
- `evidence_ref.rs`;
- planning APIs;
- semantic relation mapping;
- artifact/CAS APIs.

Posible `contribution_reconciler.rs` en engine/application layer.

### Prompt migration

Targets iniciales:

- `prompts/sddk/phases/explore.md`
- `prompts/sddk/phases/design.md`
- `prompts/sddk/phases/tasks.md`
- `prompts/sddk/phases/apply.md`
- `prompts/sddk/phases/verify.md`

Cada prompt pierde sólo recipes ya asumidas por runtime/reconciler.

## C6e targets

### Agent experience

Posibles módulos:

```text
resource_projection.rs
affordance_resolver.rs
problem_detail.rs
```

La proyección lee state/policy/capability snapshot; no escribe facts.

### CLI/MCP

CLI composition root + generic MCP tools. No crear una MCP tool por StepDefinition.

## C6f/C7 targets

### Registry

Extender precedencia framework/user/project ya utilizada por SkillRegistry a Step/Workflow definitions con rules compartidas, no tres implementaciones divergentes.

### Packs

Primero directorios/data registries; crate split sólo con trigger R11.

Propuesta conceptual:

```text
packs/software/
packs/authoring/
```

sin obligar a reorganizar todo el repo en el primer slice.

## Architecture ratchets sugeridos

1. `no_provider_types_in_domain`.
2. `no_pack_direct_storage_writes`.
3. `workflow_runtime_is_single_executor`.
4. `step_definition_no_provider_name_without_override_field`.
5. `hypermedia_projection_read_only`.
6. `default_workflow_definition_compiles`.
7. `prompt_no_duplicate_lifecycle_recipe` por slice migrado.
