# Agent Experience bounded context

## Owns

AgentProfile, SkillDefinition, InstructionCompiler, Context assembly, CommandRegistry/Surface y AgentExecutionReceipt.

## Explicitly does not own

No posee decisiones, Alignment semantics ni authorization.

## Source layout

Domain types: `crates/sddk-domain/src/agent_experience/` (except Shared Kernel -> `shared/`).

Application/use cases: `crates/sddk-engine/src/agent_experience/` where applicable.

Concrete persistence remains in `sddk-storage` adapters/projections.
