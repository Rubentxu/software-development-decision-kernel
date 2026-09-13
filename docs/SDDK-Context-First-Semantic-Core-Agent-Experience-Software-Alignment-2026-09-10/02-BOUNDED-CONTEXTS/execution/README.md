# Execution bounded context

## Owns

WorkflowDefinition, WorkflowIR/ExecutablePlan, Run, ExecutionFrontier, Task/Target execution semantics.

## Explicitly does not own

No posee decisiones de producto ni conocimiento arquitectónico.

## Source layout

Domain types: `crates/sddk-domain/src/execution/` (except Shared Kernel -> `shared/`).

Application/use cases: `crates/sddk-engine/src/execution/` where applicable.

Concrete persistence remains in `sddk-storage` adapters/projections.
