# Target architecture

## Logical architecture

```mermaid
flowchart TB
  UI[CLI / API / Human UI]
  ADP[Agent Provider Adapters]
  APP[Application Use Cases]
  TGT[Target / Task Registry]
  CTX[Context Compiler]
  INS[Instruction Compiler]
  CMD[Command Registry]
  ACS[Agent Command Surface]
  AG[Agent Execution]
  DK[Decision Kernel]
  WR[Workflow Runtime]
  AUTH[Authority Engine]
  LOG[Canonical Event Log]
  CAS[Content Addressed Object Store]
  MEM[Decision Memory]
  SG[Semantic Graph Projection]
  VLT[Vault Knowledge Source]
  PACK[Pack SDK]

  UI --> APP
  ADP --> AG
  APP --> TGT
  TGT --> WR
  TGT --> CTX
  TGT --> INS
  CMD --> ACS
  CTX --> AG
  INS --> AG
  ACS --> AG
  APP --> DK
  APP --> AUTH
  AG --> APP
  DK --> LOG
  DK --> CAS
  WR --> LOG
  AUTH --> LOG
  CAS --> MEM
  LOG --> SG
  MEM --> SG
  VLT --> CTX
  MEM --> CTX
  SG --> CTX
  PACK --> TGT
  PACK --> CTX
  PACK --> INS
```

## Dependency direction

Production dependency direction MUST be inward:

```text
adapters / cli / gateway / storage / provider adapters
                         ↓
                    application
                         ↓
                       domain
```

Packs may depend on public kernel/pack SDK contracts, never CLI internals, concrete SQLite adapters or provider-specific SDKs.

## Kernel owns

- stable identity and typed references;
- canonical fact/event schemas;
- universal Evidence and provenance primitives;
- Decision and Authority primitives;
- Revision primitives;
- workflow plan/run contracts;
- Target/Task contracts;
- Pack extension contracts;
- invariant validation.

## Agent Experience layer owns

- `AgentProfile` provider-independent role contracts;
- `SkillDefinition` procedural contracts;
- typed instruction sources and deterministic `InstructionCompiler`;
- `CommandRegistry` contract export;
- contextual `AgentCommandSurface`/cheat sheet derivation;
- provider adapter translation;
- execution provenance hashes for context/instructions/commands/skills/tools/model descriptor.

It does **not** own business decisions, capability authorization, Decision Memory or runtime state.

## Storage topology

```text
CanonicalEventLog          immutable CAS
       │                       │
       ├────► projections       ├────► artifacts/evidence bodies
       ├────► run state         └────► revisions/memory objects
       ├────► semantic graph
       └────► observability facts where canonical
```

Only facts and immutable objects are durable authorities. A projection store may be deleted and rebuilt.

## Knowledge topology

```text
Code / docs / Vault / project rules / Memory / Evidence
                        │
                        ▼
                  Context Compiler
                        │
                 ContextCapsule
```

Project instructions and Agent/Skill instructions are **procedural inputs**, not Decision Memory. The Context Compiler selects what the agent should know about the problem; the Instruction Compiler defines how the agent should operate.

## Agent execution topology

```text
Target/Task ───────────────┐
AgentProfile ──────────────┤
Project rules ─────────────┤
SkillDefinitions ──────────┤
Policy constraints ────────┤──► InstructionCompiler ─► EffectiveInstructions
ContextCapsule ────────────┤
CommandRegistry ─► Resolver┘                         └► AgentCommandSurface
                                                       │
                                                       ▼
                                                ExecutionRequest
                                                       │
                                                       ▼
                                                 ProviderAdapter
                                                       │
                                                       ▼
                                                     Model
                                                       │
                                          Outcome + Contribution
```

## Critical separation: command visibility vs authority

`AgentCommandSurface` answers **“what commands are relevant and how are they called?”**.

`AuthorityEngine` answers **“may this effect occur now, for this actor, with this evidence and policy?”**.

No provider adapter or prompt fragment can bypass the second question.
