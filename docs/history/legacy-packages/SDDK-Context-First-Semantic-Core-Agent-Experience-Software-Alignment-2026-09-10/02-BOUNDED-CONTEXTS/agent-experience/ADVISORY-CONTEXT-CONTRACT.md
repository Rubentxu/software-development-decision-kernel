# Advisory Context Contract

## Prompt assembly separation

```text
ExecutionRequest
  TaskContract
  ContextCapsule
    facts
    decisions
    evidence
    advisory_context[]   <-- Alignment only here
  EffectiveInstructions
    normative/project/policy/skill procedure
  AgentCommandSurface
  ReturnContract
```

## Normative invariant

`AlignmentAdvisory` MUST NOT implement/convert to `InstructionSource`.

A compiler fitness test debe impedir dependencias `alignment -> instruction_compiler` y serializaciones automáticas a policy instructions.

## Provenance

`AgentExecutionReceipt` registra por separado:

- `context_capsule_hash` (incluye advisory context);
- `effective_instruction_set_hash` (no incluye advice);
- `alignment_snapshot_refs[]` opcional.
