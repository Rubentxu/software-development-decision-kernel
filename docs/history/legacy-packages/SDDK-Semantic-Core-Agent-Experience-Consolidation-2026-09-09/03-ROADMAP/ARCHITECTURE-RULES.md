# Architecture rules / ratchets

Suggested machine-enforced rules after migration baselines are established.

| Rule | Requirement |
|---|---|
| ARCH-SC-001 | every storage-backed model declares a state class |
| ARCH-SC-002 | only one component declares canonical authority per semantic concept |
| ARCH-SC-003 | domain/application crates do not depend on concrete storage adapters |
| ARCH-SC-004 | packs cannot depend on CLI or concrete storage |
| ARCH-SC-005 | pack-specific schema additions use namespaces; no core enum change without ADR |
| ARCH-SC-006 | projections implement version + rebuild contract |
| ARCH-SC-007 | governed side-effect code path references AuthorityEngine admission |
| ARCH-SC-008 | evidence references use universal Evidence model |
| ARCH-SC-009 | context/search/vector modules cannot implement canonical write ports |
| ARCH-SC-010 | no new `*GraphStore`, `*MemoryStore`, `*EventStore` authority without architecture review |
| ARCH-SC-011 | user-facing deprecations have adapter/parity fixture/removal milestone |
| ARCH-SC-012 | top-level CLI additions require UX justification; prefer Target or pack namespace |

## Ownership registry

Add a small machine-readable registry (format implementation choice) from which docs/tests can derive:

```text
concept
owner
state_class
authority
store_port
projection_source
deprecates
extension_namespace
```

This registry is more valuable than relying on comments to preserve architecture over time.


## Agent Experience fitness rules

**ARCH-A01 — No handwritten authoritative agent command syntax**  
Agent-visible CLI syntax/examples must resolve to CommandRegistry entries.

**ARCH-A02 — Skill cannot grant Capability**  
Skill loading/selection APIs may declare requirements but cannot acquire effect permissions.

**ARCH-A03 — Provider adapters are semantic leaves**  
Provider adapters cannot depend on concrete storage or mutate domain state outside application ports.

**ARCH-A04 — Prompt text cannot own workflow state**  
Workflow transitions referenced by prompts must map to Target/Task/Runtime contracts.

**ARCH-A05 — Instruction conflicts are explicit**  
Typed normative instruction keys cannot silently resolve by source order.

**ARCH-A06 — Execution provenance completeness**  
Governed agent execution receipts must include required contract hashes/refs.

**ARCH-A07 — Examples compile/test**  
All stable agent-facing examples are executable fixtures or explicitly marked illustrative/non-executable.

**ARCH-A08 — No deprecated semantic names in active assets**  
After each migration deadline, active prompts/skills/profiles cannot reference deprecated Cycle/Graph/Evidence/CLI semantics.
