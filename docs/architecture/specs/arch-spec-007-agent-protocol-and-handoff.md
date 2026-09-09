---
id: arch-spec-007-agent-protocol-and-handoff
package_local_id: SPEC-007
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-007-AGENT-PROTOCOL-AND-HANDOFF.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# arch-spec-007 — AGENT-PROTOCOL-AND-HANDOFF

> **Mirror of package SPEC `SPEC-007`.** Repository-native identifier is `arch-spec-007-agent-protocol-and-handoff` (range arch-spec-001..018). Per SUPERSESSION.md, the packages original SPEC ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `SPEC-007` |
| Repository native | `arch-spec-007-agent-protocol-and-handoff` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-007-AGENT-PROTOCOL-AND-HANDOFF.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# SPEC-007 — Agent execution, delegation and synthesis protocol

This spec defines **what crosses agent/handoff boundaries**. Agent assembly/instructions/skills/CLI knowledge are specified by SPEC-013..018.

## Core contracts

```text
ExecutionRequest
ExecutionOutcome
Contribution
DelegationRequest
SynthesisReceipt
ActionProposal
```

`ExecutionRequest` references, rather than embeds as untracked text, the AgentProfile, ContextCapsule hash, EffectiveInstructions hash, selected Skill set, AgentCommandSurface hash, TaskContract and policy/config snapshots.

## Contribution

Contains findings, alternatives, claims, assumptions, uncertainty, risks, questions, Evidence refs, Artifact refs, recommendation and confidence. It does not claim side-effect success.

## ExecutionOutcome

Contains factual operational result: completed/failed/interrupted status, errors, Artifacts, Evidence, usage and side-effect Receipts.

## Synthesis disposition invariant

Every material contribution item gets exactly one disposition:

```text
CONSUMED
REJECTED(reason)
DEFERRED(reason)
SUPERSEDED(by)
OMITTED(justification)
```

Material High/Critical risk and required Evidence MUST NOT be silently omitted.

## Dissent

Dissent is first-class with stable id, kind/claim ref, source, materiality, Evidence refs, status, resolution and revisit trigger.

## Delegation

Delegation creates a new typed ExecutionRequest with an explicit context delta, profile/task contract and authority ceiling. It does not forward an opaque transcript as the durable contract.

## Metrics

- handoff information-loss rate;
- dissent retention rate;
- stale-context handoff rate;
- evidence carry-forward completeness;
- instruction/command contract mismatch rate;
- provider normalization failure rate.

Mandatory risk/evidence information-loss target is zero.
