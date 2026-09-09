# ADR-017 — Record agent execution provenance hashes, not chain of thought

**Status:** Proposed

## Decision

Every governed/important agent execution records sufficient provenance to answer what inputs and contracts shaped the execution, without persisting private reasoning traces.

Minimum receipt references/hashes:

```text
agent_profile_version
model_descriptor
provider_adapter_version
task_contract_hash
context_capsule_hash
instruction_set_hash
skill_set_hash
command_surface_hash
tool_contract_hash
policy_snapshot_hash
resolved_config_hash
output_refs
```

Provider nondeterminism is acknowledged. This contract supports auditability and controlled replay, not a false guarantee of bit-for-bit LLM determinism.
