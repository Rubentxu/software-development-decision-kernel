---
id: arch-spec-017-agent-profiles-and-provider-adapters
package_local_id: SPEC-017
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-017-AGENT-PROFILES-AND-PROVIDER-ADAPTERS.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# arch-spec-017 — AGENT-PROFILES-AND-PROVIDER-ADAPTERS

> **Mirror of package SPEC `SPEC-017`.** Repository-native identifier is `arch-spec-017-agent-profiles-and-provider-adapters` (range arch-spec-001..018). Per SUPERSESSION.md, the packages original SPEC ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `SPEC-017` |
| Repository native | `arch-spec-017-agent-profiles-and-provider-adapters` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-017-AGENT-PROFILES-AND-PROVIDER-ADAPTERS.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

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
