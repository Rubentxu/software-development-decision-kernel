# Bounded Context Map

## Contextos y ownership

| Contexto | Tipo | Posee | Consume | No posee |
|---|---|---|---|---|
| Shared Kernel | kernel | IDs, Evidence primitives, canonical event envelope, CAS/revision primitives | — | lógica de negocio de contextos |
| Planning | core/supporting | Goal, WorkItem, desired planning manifest, dependencies de planificación | Decision, Evidence | Run state, approvals |
| Execution | core | WorkflowDefinition, ExecutablePlan/IR, Run, ExecutionFrontier, Task execution semantics | Planning, Governance | Decision Memory, Alignment |
| Decision | core | Decision Memory, Revision/Ref semantics de decisiones, TradeoffRecord, assumptions, dissent refs | Evidence, Knowledge refs | métricas, workbooks |
| Knowledge | supporting | KnowledgeAssertion lifecycle, KnowledgeBasis, KMT, freshness, knowledge cards, semantic relations/projections | Evidence, Decision refs, provider evidence | consejos de diseño, governance |
| Software Alignment | supporting | AlignmentAssessment, AlignmentTension, ImprovementOpportunity, LensDefinition, ArchitecturalIntentSnapshot, AlignmentWorkbook definitions | Knowledge, Decision, provider observations | policies, gates, code mutation |
| Verification | application/core workflow | VerifyReceipt, DebVerifyBaselineReceipt, analysis scope/deepening orchestration, evidence-gap semantics | Knowledge, Alignment, Providers, Governance | Alignment heuristics, provider internals |
| Governance | core | AuthorityEngine, policy, admission, approval, waiver authorization, gates | Evidence, explicit ContractViolation | alignment heuristics |
| Agent Experience | supporting | AgentProfile, Skill, InstructionCompiler, Context assembly, CommandSurface, AgentExecutionReceipt | all read models | business authority |
| Extension Platform | supporting | Pack SPI, IntelligenceProvider SPI, registry/capability negotiation, adapters | context contracts | provider-specific domain models |

## Relaciones DDD

```text
Planning  ---> Execution
Decision  ---> Knowledge
Knowledge ---> Software Alignment
Decision  ---> Software Alignment
Software Alignment --advisory--> Verification
Knowledge --------facts/views--> Verification
Extension Platform ---> Verification
Verification ----evidence----> Governance
Governance --------admission-> Execution/Capabilities
Knowledge + Alignment --context--> Agent Experience
```

## Anti-corruption boundaries

CogniCode y Chronos quedan **fuera** del bounded context map interno. Se conectan mediante adapters de Extension Platform:

```text
CogniCode RPC -> CogniCodeAdapter -> CodeIntelligencePort
Chronos RPC   -> ChronosAdapter   -> RuntimeIntelligencePort
```

Sus protobuf DTOs nunca atraviesan hacia Domain.
