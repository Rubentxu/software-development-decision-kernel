# SPEC-014 — Instruction Compiler

## Inputs

```text
KernelInvariantSet
PolicyInstructionSet
TaskContract
ProjectInstructionSet
AgentProfile
SelectedSkillSet
ContextualHints
```

## Output

`EffectiveInstructions` contains ordered sections, source refs, diagnostics, compatibility version and content hash.

## Conflict semantics

Instruction composition is typed:

- invariant/policy constraints cannot be overridden by lower classes;
- task requirements cannot be silently weakened by project/skill text;
- duplicate equivalent directives deduplicate by semantic key;
- conflicting normative directives produce `InstructionConflict` and fail closed unless an explicit resolver policy exists;
- advisory hints may coexist and are labelled advisory.

There is no generic “later file wins” rule for normative instructions.

## Explainability

`context explain --instructions` (or equivalent application query) reports source, reason selected, semantic key, strength, version/hash and excluded/conflicting fragments.

## Provider rendering

ProviderAdapter renders `EffectiveInstructions` into provider-native system/developer/user/tool structures. Rendering may change formatting but not semantic keys/strength.
