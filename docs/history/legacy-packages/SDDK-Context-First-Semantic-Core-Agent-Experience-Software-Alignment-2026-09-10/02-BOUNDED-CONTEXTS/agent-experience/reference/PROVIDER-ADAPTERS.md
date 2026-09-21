# Provider adapters

## Examples

Potential adapters include Codex/OpenAI, Claude, OpenCode orchestration and local model runtimes. The architecture must not require any one provider.

## Port

Conceptually:

```text
ProviderAdapter.execute(
  ExecutionRequest,
  EffectiveInstructions,
  ContextCapsule,
  AgentCommandSurface,
  CapabilityToolBindings
) -> ProviderExecutionResult
```

Normalization then validates output into ExecutionOutcome/Contribution.

## Provider-specific optimizations

Adapters may use native tool schemas, prompt caching, system-message roles or structured outputs. Those optimizations cannot alter semantic authority, command contracts or Skill/Task meaning.
