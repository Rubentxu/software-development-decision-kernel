# Prompt assembly and effective instructions

## Anti-pattern

```text
agent-security.md = role + workflow + CLI docs + policy + context + examples + output format
```

This creates connascence of meaning across every architectural change.

## Target

```text
AgentProfile
TaskContract
ProjectInstructions
PolicyConstraints
SelectedSkills
ContextCapsule
AgentCommandSurface
       │
       ▼
ExecutionRequest
       │
ProviderAdapter
       │
provider-native prompt/messages/tools
```

`PromptAssembly` may be the internal renderer structure, but its semantic inputs remain individually versioned and explainable.

## No last-write-wins prose

Normative conflicts are resolved structurally by the InstructionCompiler or rejected. Provider rendering cannot silently reorder precedence to change meaning.
