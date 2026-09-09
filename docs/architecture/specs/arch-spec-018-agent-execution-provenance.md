---
id: arch-spec-018-agent-execution-provenance
package_local_id: SPEC-018
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-018-AGENT-EXECUTION-PROVENANCE.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# arch-spec-018 — AGENT-EXECUTION-PROVENANCE

> **Mirror of package SPEC `SPEC-018`.** Repository-native identifier is `arch-spec-018-agent-execution-provenance` (range arch-spec-001..018). Per SUPERSESSION.md, the packages original SPEC ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `SPEC-018` |
| Repository native | `arch-spec-018-agent-execution-provenance` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-018-AGENT-EXECUTION-PROVENANCE.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# SPEC-018 — Agent execution provenance and reproducibility envelope

## Purpose

Answer: “what did this agent receive, what contract governed it, which model/adapter executed it, and what did it produce?”

## Receipt

```text
AgentExecutionReceiptV1
  execution_id
  run_id/task_id
  agent_profile_ref
  model_descriptor
  provider_adapter_ref
  context_capsule_hash
  effective_instructions_hash
  selected_skill_refs/hash
  command_surface_hash
  tool_contract_hash
  policy_snapshot_hash
  resolved_config_hash
  started/finished metadata
  execution_outcome_ref
  contribution_ref?
  provider_usage
  replay_class
```

## Replay classes

- `deterministic` — no nondeterministic provider/tool I/O;
- `recorded_io` — replay possible using captured external responses;
- `nondeterministic` — provenance comparable, output not guaranteed identical.

## Privacy

Do not persist hidden chain-of-thought. Persist explicit Contributions, decisions, evidence, source refs and public execution diagnostics only.
