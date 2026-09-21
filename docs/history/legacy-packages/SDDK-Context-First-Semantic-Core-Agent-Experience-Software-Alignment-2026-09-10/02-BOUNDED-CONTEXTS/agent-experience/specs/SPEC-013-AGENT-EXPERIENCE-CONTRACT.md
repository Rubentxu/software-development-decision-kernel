# SPEC-013 — Agent Experience Contract

## Objective

Make SDDK usable by coding/review/research agents without hidden architectural knowledge, repeated CLI discovery or provider-specific coupling.

## Required contracts

```text
AgentProfile
SkillDefinition
TaskContract
ContextCapsule
InstructionSource
EffectiveInstructions
CommandSpec
AgentCommandSurface
ExecutionRequest
ExecutionOutcome
Contribution
AgentExecutionReceipt
```

## Execution assembly

For each agent task:

1. resolve Target/Task;
2. resolve AgentProfile;
3. compile bounded ContextCapsule;
4. select applicable Skills;
5. resolve Policy constraints and configured project instructions;
6. compile EffectiveInstructions;
7. derive contextual AgentCommandSurface from CommandRegistry;
8. create ExecutionRequest with typed refs/hashes;
9. render through ProviderAdapter;
10. validate ExecutionOutcome/Contribution against return contracts;
11. emit execution provenance receipt/events.

## Invariants

- provider prompt content never changes authority;
- a command can be known but unavailable/denied at runtime;
- agents use stable refs and JSON schemas where automation is expected;
- normal execution does not depend on exploratory `--help` calls;
- prompt/skill/command changes are versioned and observable;
- no chain-of-thought persistence is required.

## 2026-09-10 amendment — advisory alignment context

Agent context MUST label alignment content by epistemic weight: FACT/OBSERVATION/ASSESSMENT/SUGGESTION/DECISION. Advisory assessments MUST NOT be rendered as imperative instructions.
