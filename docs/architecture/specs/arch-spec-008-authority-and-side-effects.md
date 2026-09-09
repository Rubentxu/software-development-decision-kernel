---
id: arch-spec-008-authority-and-side-effects
package_local_id: SPEC-008
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-008-AUTHORITY-AND-SIDE-EFFECTS.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# arch-spec-008 — AUTHORITY-AND-SIDE-EFFECTS

> **Mirror of package SPEC `SPEC-008`.** Repository-native identifier is `arch-spec-008-authority-and-side-effects` (range arch-spec-001..018). Per SUPERSESSION.md, the packages original SPEC ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `SPEC-008` |
| Repository native | `arch-spec-008-authority-and-side-effects` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-008-AUTHORITY-AND-SIDE-EFFECTS.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# SPEC-008 — Authority, policy and governed side effects

## Flow

```text
ActionProposal
→ Policy evaluation
→ AdmissionDecision
→ optional HumanApproval
→ Capability execution
→ postcondition verification
→ Evidence + Receipt
```

## Requirements

- AU-001 default deny for undeclared governed capabilities;
- AU-002 exactly one AdmissionDecision per execution attempt;
- AU-003 denied/awaiting-approval proposals perform no side effect;
- AU-004 irreversible actions require declared authority policy;
- AU-005 receipt references policy snapshot, actor, capability input digest, result and evidence;
- AU-006 pack rules can influence policy but cannot bypass the engine;
- AU-007 `sddk why action:<id>` can explain admission.
