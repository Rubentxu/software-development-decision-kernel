# Responsibility map and semantic ownership

## Canonical ownership table

| Concept | Canonical owner | State class | Derived views | Must not own |
|---|---|---|---|---|
| Goal | Planning domain | Fact/Object | goal status | runtime node state |
| WorkItem | Planning domain | Facts | backlog/progress | execution attempt state |
| Cycle | Delivery domain | Facts | cycle summary | UAT/approval/retry mechanics |
| WorkflowDefinition | Workflow domain | Object | docs | live state |
| ExecutablePlan / IR | Workflow domain | Object/Revision | DAG view | user intent |
| Run | Runtime | Facts | RunStateView | planning backlog |
| ExecutionFrontier | Runtime projection | Projection | legal actions | strategic recommendations |
| ContinuationOptions | Decision/advisory | Ephemeral/Projection | ResumeView | legal transition authority |
| Decision | Decision domain | Fact + immutable rationale refs | memory/graph | execution state |
| Evidence | Evidence domain | Object + fact refs | assurance views | separate per-pack models |
| Decision Memory | Memory domain | Revisioned Objects + ref facts | log/tree/diff | raw chat transcript authority |
| Semantic Graph | Projection domain | Projection | why/impact/views | canonical facts |
| Vault | Knowledge adapter | external Markdown source + local Projection | FTS/wiki | decision authority |
| ContextCapsule | Context compiler | Ephemeral | agent input | long-term memory |
| AgentProfile | Agent Experience | Object/versioned definition | provider rendering | runtime permission |
| SkillDefinition | Agent Experience / Pack SDK | Object/versioned definition | selected skill set | side-effect capability |
| ProjectInstructions | project configuration/knowledge | Object/source text | compiled instructions | Decision Memory |
| EffectiveInstructions | Instruction Compiler | Ephemeral | provider prompt/messages | canonical policy |
| CommandSpec Registry | CLI contract owner | source definitions + Projection/export | help/completion/json | capability permission |
| AgentCommandSurface | Agent Experience | Ephemeral | cheat sheet/json | Authority decision |
| ExecutionRequest | Application/agent protocol | Ephemeral + receipt refs | provider call | durable truth by itself |
| ExecutionOutcome | Runtime/agent protocol | Fact/Object refs | run views | epistemic recommendation |
| Contribution | Agent protocol | Object/Fact refs | synthesis/graph | side-effect success |
| SynthesisReceipt | Decision/handoff | Object + fact | audit/why | source contribution mutation |
| AdmissionDecision | Authority | Fact | explanation | command discovery |
| Metrics | Observability | Projection | metric views | workflow authority |
| Analytics | Query/presentation | Projection | reports | telemetry authority |
| Telemetry | Observability input | Facts where required | metrics | business/domain authority |

## Naming rules

- `*Fact`, event/domain noun: canonical occurrence.
- `*Object`, `*Definition`, `*Profile`: immutable/versioned material.
- `*Projection`, `*View`, `*Index`, `*RegistryExport`: rebuildable.
- `*Capsule`, `*Candidate`, `*Option`, `*Surface`: ephemeral/advisory unless explicitly specified.
- `*Receipt`: immutable evidence of a governed operation or synthesis.
- `*Ref`: typed reference, never embedded mutable state.

Avoid unqualified names such as `State`, `Graph`, `Result`, `Context` or `Decision` across bounded contexts.

## Frontier terminology

- **ExecutionFrontier** — legal, gate-satisfied runtime transitions derived from canonical run facts + workflow definition.
- **ContinuationOptions** — advisory candidates. Never legal-action authority.
- **ResumeView** — presentation composing Memory head, blockers, ExecutionFrontier and ContinuationOptions.

## Agent output terminology

```text
AgentExecution
 ├─ ExecutionOutcome   # what operationally happened
 └─ Contribution?      # what was learned/proposed
```

## Instruction terminology

```text
InstructionSource
 ├─ KernelInvariant
 ├─ PolicyConstraint
 ├─ TaskInstruction
 ├─ ProjectInstruction
 ├─ AgentRoleInstruction
 └─ SkillInstruction
        │
        ▼
InstructionCompiler
        │
        ▼
EffectiveInstructions
```

Free-form prompt text is a rendered representation of these contracts and has no independent semantic authority.

## Skill vs Capability

**Skill** = procedural method/instruction/expected output for accomplishing a class of task.  
**Capability** = executable effect surface such as filesystem write, command execution, Git mutation or network call.

A Skill may require zero or more Capabilities. It never grants them.

## Amendment — Software Alignment responsibilities

| Concept | Canonical responsibility |
|---|---|
| ArchitecturalIntent | declared/decided expectations assembled from Decision Memory, invariants, contracts and selected lenses |
| AlignmentAssessment | advisory comparison of observed evidence vs intent/lens |
| AlignmentTension | potentially useful discrepancy without claiming an absolute defect |
| ContractViolation | mismatch against an explicit MUST/forbidden contract |
| ImprovementOpportunity | possible direction with tradeoffs, never required action |
| KnowledgeAssertion | temporal typed statement with provenance; lives in knowledge substrate, not Alignment authority |
| AlignmentWorkbook | projection/read model by focus/lens |
| Verify | delta-oriented knowledge synchronization |
| DebVerify | project-wide knowledge reconciliation and debt assessment |
