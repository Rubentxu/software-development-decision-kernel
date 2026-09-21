# Executive proposal — one coherent semantic kernel and one coherent agent interface

## Diagnosis

SDDK has accumulated useful capability faster than its semantic boundaries and agent-facing contracts have converged. Two forms of entropy now reinforce each other.

**Domain entropy:** multiple representations can appear equally authoritative: Cycle vs Run state, Spine vs Planning Ledger, GraphProjection vs ActiveGraph, multiple evidence models, several revision mechanisms, multiple admission/gate components and overlapping “frontier/next” concepts.

**Agent-interface entropy:** prompts, skills, agent profiles, command examples and procedural instructions can embed internal architectural knowledge and CLI syntax directly. When the kernel evolves, these assets can silently become stale even when the Rust code still compiles.

The remedy is **semantic consolidation plus contract-driven agent experience before further feature expansion**.

## Non-negotiable invariants

1. **One canonical event log.** Mirrors may exist only during migration and are never co-authoritative.
2. **One authority per semantic concept.** Every durable concept declares exactly one source of truth.
3. **One universal Evidence model.** Pack/domain specializations are typed relations and metadata.
4. **One revision substrate.** Plan revisions, Decision Memory and compatible experimental history reuse content-addressed primitives.
5. **One Semantic Graph substrate.** Other graphs are typed views, local adapter indexes or compatibility projections.
6. **One admission path for governed side effects.** Permission/risk/gates/approval feed one Authority Engine.
7. **Context is never authority.** Context capsules, summaries, indexes, graph views and Vault pages are derived inputs.
8. **Prompt text is never architecture.** Effective prompts are compiled representations of typed role/task/skill/context/policy/command contracts.
9. **Skill is not Capability.** A Skill teaches/structures how to perform work; a Capability is an executable effect surface governed by Authority.
10. **CLI documentation has one source.** Human help, completion, machine contracts and agent cheat sheets derive from the same Command Registry.
11. **Examples are executable contracts.** Published command examples must parse and pass fixture/UAT validation.
12. **Knowledge of a command is not permission.** AgentCommandSurface and CapabilityGrant/AdmissionDecision remain orthogonal.

## Four state classes

Every important model MUST be classified into exactly one state class:

| Class | Meaning | Rebuildable | Examples |
|---|---|---:|---|
| Fact | append-only canonical occurrence | no | event, decision acceptance, approval |
| Object | immutable content-addressed material | no | evidence body, revision object, agent profile version |
| Projection | derived query/index state | yes | run view, semantic graph, command registry export |
| Ephemeral | task/session compilation | yes | ContextCapsule, EffectiveInstructions, AgentCommandSurface |

## Canonical SDLC chain

```text
Goal / Intent
   ↓
WorkItem
   ↓
WorkflowDefinition
   ↓ compile
ExecutablePlan / WorkflowIR
   ↓ instantiate
Run
   ↓
ExecutionOutcome + Contribution
   ↓
Decision / Evidence / Decision Memory
```

`Cycle` is a change/delivery container and summary boundary, not another runtime state machine.

## Canonical agent chain

```text
Target + TaskContract
AgentProfile
Project Instructions
Selected Skills
Policy Snapshot
ContextCapsule
Command Registry
        │
        ▼
InstructionCompiler + CommandSurfaceResolver
        │
        ├── EffectiveInstructions
        └── AgentCommandSurface
                │
                ▼
          ExecutionRequest
                │
                ▼
              Agent
                │
        Outcome + Contribution
```

The provider adapter (Codex, Claude, OpenCode, local model, etc.) is replaceable and must not own SDDK semantics.

## Vault boundary

Vault is retained with a narrow responsibility:

> Human-readable curated knowledge, wiki/history, runbooks, ADR narrative and optional retrieval source.

Vault may improve human understanding and context compilation. Its FTS/wiki graph is an internal projection. Vault never becomes Decision Memory, canonical Event Log or SemanticGraph authority.

## Product surface

Daily porcelain remains deliberately small:

```text
sddk status
sddk plan
sddk run <target>
sddk next
sddk why <entity>
sddk diff
sddk memory ...
sddk context ...
sddk doctor
```

Agent support is generated around that surface:

```text
sddk help agent
sddk help agent --target verify --format json
sddk context explain --instructions --commands
```

The agent normally receives these contracts from its adapter; it does not need to invoke help during normal execution.

## What this unlocks

- adding Decision Memory without creating a fourth history mechanism;
- causal `why` grounded in facts/evidence rather than ad-hoc graph interpretation;
- provider-independent agents whose behavior contracts survive CLI refactors;
- skills composable across agents without granting hidden side-effect authority;
- machine-tested cheat sheets that never drift from CLI syntax;
- reproducible execution provenance without storing chain of thought;
- extension Packs that contribute tasks, schemas, skills and context without growing core enums;
- recovery after long pauses from canonical project state rather than chat transcript replay.
