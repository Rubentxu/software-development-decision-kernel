# Consolidated glossary

**ActionProposal** — requested governed side effect before admission.  
**AdmissionDecision** — Allow, Deny or RequireApproval from AuthorityEngine.  
**AgentCommandSurface** — contextual ephemeral subset of CommandRegistry rendered for one agent task; not permission.  
**AgentExecutionReceipt** — provenance record tying an execution to profile/task/context/instruction/skill/command/tool/policy/config versions/hashes.  
**AgentProfile** — provider-independent role/responsibility/output contract.  
**Artifact/Object** — immutable content-addressed material.  
**CanonicalEventLog** — sole ordered source of canonical domain facts.  
**Capability** — executable effect surface admitted by AuthorityEngine.  
**CommandRegistry** — typed single source for CLI command contracts and generated help/agent surfaces.  
**ContextCapsule** — bounded ephemeral context compiled for one target/attempt.  
**Contribution** — epistemic/advisory output from an agent.  
**ContinuationOption** — advisory next-step candidate; not a legal transition.  
**Cycle** — bounded change/delivery container.  
**Decision Memory** — typed revisioned durable decision/rationale knowledge.  
**EffectiveInstructions** — deterministic compiled instructions for one task/execution.  
**Evidence** — immutable verifiable material supporting observations/decisions/actions.  
**ExecutionFrontier** — projected legal runtime transitions whose requirements are satisfied.  
**ExecutionOutcome** — operational result of one execution attempt.  
**Fact** — canonical append-only occurrence.  
**Goal** — durable intent/outcome sought.  
**InstructionCompiler** — deterministic composer of typed instruction sources.  
**InstructionSource** — versioned source of procedural/constraint directives.  
**KnowledgeSource** — material available for retrieval (Vault, code, docs, connectors).  
**ObjectId/OID** — digest identity of canonical immutable bytes.  
**Pack** — namespaced extension implementing public SDDK contracts.  
**Projection** — rebuildable derived read/query model.  
**ProviderAdapter** — provider/model-specific renderer/invoker that owns no SDDK semantics.  
**Run** — one execution instance of an ExecutablePlan.  
**SemanticGraph** — rebuildable typed relationship projection.  
**SkillDefinition** — versioned procedural method contract; never a capability grant.  
**SynthesisReceipt** — auditable disposition of multiple agent contributions.  
**Target** — CLI/workflow aggregate resolving to a Task DAG; distinct from domain Goal.  
**Task** — declarative executable unit within a Target graph.  
**Vault** — curated human-readable wiki/history KnowledgeSource, non-authoritative.  
**WorkItem** — operational planning unit.

## Software Alignment additions

- **Software Alignment:** advisory bounded context comparing observed software with intent/decisions/lenses.
- **Universal Concern:** paradigm-neutral property worth examining.
- **Alignment Lens:** optional paradigm/architecture-specific interpretation.
- **Architectural Intent:** compiled expectation view from decisions/contracts/config.
- **Alignment Tension:** discrepancy worth attention, not necessarily defect.
- **ContractViolation:** mismatch against explicit MUST/forbidden contract.
- **Knowledge Merkle Tree:** hierarchical fingerprint/freshness substrate.
- **Knowledge Dependency Overlay:** SemanticGraph relations used for cross-unit invalidation.
- **Verify:** recent delta knowledge synchronization.
- **DebVerify:** whole-project knowledge/debt reconciliation.
- **Enhanced Mode:** availability profile for optional intelligence providers.
