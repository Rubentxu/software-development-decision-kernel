# ADR-013 — Compile effective agent instructions from typed sources

**Status:** Proposed  
**Decision:** agent instructions are compiled deterministically from typed/versioned sources; prompt text is a rendering, not an architectural source of truth.

## Context

SDDK currently risks embedding workflow, CLI and architectural behavior in AGENTS files, prompts, skills and provider-specific templates. Such text can drift silently when domain contracts change.

## Decision

Introduce:

```text
InstructionSource
InstructionSet
InstructionCompiler
EffectiveInstructions
InstructionDiagnostic
```

Supported source classes are KernelInvariant, PolicyConstraint, TaskInstruction, ProjectInstruction, AgentRoleInstruction, SkillInstruction and ContextualHint.

Composition rules are structural, not “last text wins”. Conflicting normative directives fail closed or produce an explicit diagnostic according to schema. Rendering order is deterministic and hashes are recorded in execution provenance.

## Consequences

- prompts become generated/provider-specific views;
- changing architecture requires changing typed contracts/tests, not hunting hidden prompt text;
- effective instructions can be explained and reproduced without storing chain of thought;
- legacy monolithic prompts require migration/splitting.
