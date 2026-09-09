# SPEC-017 — AgentProfile and ProviderAdapter

## AgentProfile

Provider-neutral object fields:

```text
id/version
role/responsibilities
admissible_task_kinds
context_requirements
authority_ceiling
preferred/required skills
output_contract
quality constraints
budget defaults
```

The authority ceiling is a maximum request envelope, never an admission grant.

## ProviderAdapter

Responsibilities:

- map EffectiveInstructions to provider-native messages/instructions;
- expose provider/model/tool descriptors;
- invoke model/tool API through configured gateway;
- normalize provider output into ExecutionOutcome/Contribution candidates;
- report token/usage/provider errors.

Must not:

- decide SDDK workflow transitions;
- mutate Decision Memory directly;
- interpret Vault as authority;
- bypass Capability/Authority;
- inject undocumented commands.

## Compatibility

An AgentProfile must be testable against more than one fake/provider adapter. Provider-specific profiles are permitted only when a capability truly cannot be expressed portably and must be explicitly namespaced.
