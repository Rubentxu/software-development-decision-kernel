---
id: ADR-0110-AGENT-EXECUTION-PROVENANCE
package_local_id: ADR-017
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-017-AGENT-EXECUTION-PROVENANCE.md
status: accepted
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
accepted_at: 2026-09-12
accepted_by_cycle: "p-63676b11dc0ef88f/adr-promotion-batch-2"

implementation_evidence:
  - "crates/sddk-cli/src/execution_receipt.rs:156 — AgentExecutionReceipt, :333 — AgentExecutionReceiptBuilder"
  - "M7.6 spec pins required provenance fields"


superseded_by: []
related_adrs:
  - "ADR-0001-ADR-PROMOTION-PROCESS"
  - "ADR-0096-SDLC-LIFECYCLE-SEMANTICS"
stale_after: 2027-09-12

---

# ADR-0110 — AGENT-EXECUTION-PROVENANCE

> **Mirror of package ADR `ADR-017`.** Repository-native numbering is `ADR-0110-AGENT-EXECUTION-PROVENANCE` (range ADR-0094..0110). Per SUPERSESSION.md, the package's original ADR ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `ADR-017` |
| Repository native | `ADR-0110-AGENT-EXECUTION-PROVENANCE` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-017-AGENT-EXECUTION-PROVENANCE.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

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
