---
id: ADR-0102-UNIFIED-AUTHORITY-ENGINE
package_local_id: ADR-009
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-009-UNIFIED-AUTHORITY-ENGINE.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# ADR-0102 — UNIFIED-AUTHORITY-ENGINE

> **Mirror of package ADR `ADR-009`.** Repository-native numbering is `ADR-0102-UNIFIED-AUTHORITY-ENGINE` (range ADR-0094..0110). Per SUPERSESSION.md, the package's original ADR ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `ADR-009` |
| Repository native | `ADR-0102-UNIFIED-AUTHORITY-ENGINE` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-009-UNIFIED-AUTHORITY-ENGINE.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# ADR-009 — Unified authority/admission engine

**Status:** Proposed

## Decision

All governed side effects pass through one logical Authority Engine:

```text
ActionProposal + Actor + Facts + PolicySnapshot
        ↓
AdmissionDecision
  = Allow | Deny(reason) | RequireApproval(requirement)
```

Permission checks, risk policy, gates, role contracts and approvals are inputs/rules/facts. They do not independently execute effects.

Capabilities execute only after Allow and emit receipts/evidence/postconditions.

## Consequences

A single audit path answers “why was this action allowed?”. Packs add policy rules through bounded extension points.
