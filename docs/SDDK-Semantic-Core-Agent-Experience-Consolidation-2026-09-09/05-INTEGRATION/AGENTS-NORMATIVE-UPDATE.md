# Proposed AGENTS.md normative rules

Merge these into the repository's normative agent/architecture instructions after adopting this package.

```markdown
### Semantic ownership

- Do not introduce a new persistent model/store/graph/event log/evidence hierarchy/revision system without proving existing canonical abstractions cannot represent the responsibility.
- Every storage-backed model MUST declare: state class (Fact/Object/Projection/Ephemeral), owner, authority, lifecycle, rebuild source, retention and extension boundary.
- One semantic concept has one canonical authority. Compatibility mirrors/projections MUST be named/documented as such.
- Context, summaries, search indexes, vectors, graphs and Vault pages are never canonical authority unless an explicit ADR changes this architecture.

### Agent experience contracts

- Prompt text is a rendering, not an architectural source of truth. Workflow, authority, CLI syntax and durable knowledge MUST live in typed contracts.
- Agents MUST receive a versioned/contextual command contract for routine CLI use; do not teach commands through duplicated handwritten cheat sheets.
- Human help, machine CLI schemas, examples and agent command surfaces MUST derive from one Command Registry.
- Every stable command example MUST be executable/tested or explicitly marked non-executable.
- Skills describe procedure and output expectations; Skills MUST NOT grant Capabilities or bypass Authority.
- AgentProfile semantics MUST be provider-independent unless an explicit provider-specific ADR/namespace is justified.
- Provider adapters MUST NOT implement workflow transitions, storage semantics, authority decisions or direct Decision Memory mutation.
- EffectiveInstructions MUST be compiled from typed sources with explicit conflict semantics; normative conflicts cannot silently resolve by text order.
- Governed agent executions MUST record profile/task/context/instruction/skill/command/tool/policy/config provenance refs/hashes. Do not persist hidden chain-of-thought.

### Extension discipline

- Prefer Pack extension points and namespaced schemas over adding pack-specific variants to core enums.
- Prefer adding a Target/Task to a new top-level CLI command.
- All governed side effects route through Authority Engine and produce verifiable receipts/evidence.

### Consolidation rule

A new core abstraction is accepted only if its PR names at least one existing abstraction/path it replaces, consolidates or makes unnecessary. Pure additive architecture during consolidation requires explicit ADR approval.
```
