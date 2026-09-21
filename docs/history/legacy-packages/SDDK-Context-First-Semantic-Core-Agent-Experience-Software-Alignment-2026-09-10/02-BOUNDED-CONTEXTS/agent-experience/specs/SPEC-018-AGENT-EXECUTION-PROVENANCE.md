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
