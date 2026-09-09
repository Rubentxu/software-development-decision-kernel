---
id: ADR-0101-AGENT-OUTCOME-CONTRIBUTION-SYNTHESIS
package_local_id: ADR-008
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-008-AGENT-OUTCOME-CONTRIBUTION-SYNTHESIS.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# ADR-0101 — AGENT-OUTCOME-CONTRIBUTION-SYNTHESIS

> **Mirror of package ADR `ADR-008`.** Repository-native numbering is `ADR-0101-AGENT-OUTCOME-CONTRIBUTION-SYNTHESIS` (range ADR-0094..0110). Per SUPERSESSION.md, the package's original ADR ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `ADR-008` |
| Repository native | `ADR-0101-AGENT-OUTCOME-CONTRIBUTION-SYNTHESIS` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-008-AGENT-OUTCOME-CONTRIBUTION-SYNTHESIS.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# ADR-008 — Separate execution outcome, contribution and synthesis

**Status:** Proposed

## Decision

Agent interactions use three distinct contracts:

- `ExecutionOutcome`: operational facts about execution.
- `Contribution`: epistemic/advisory content learned or proposed.
- `SynthesisReceipt`: how multiple contributions were combined, omitted, rejected or carried forward.

`AgentResult` is deprecated as a mixed legacy contract.

Synthesis MUST preserve material dissent, risk and evidence with explicit dispositions.

## Benefit

Execution can succeed while a recommendation is low-confidence; a contribution can be useful even if an execution partially fails. The model no longer conflates them.
