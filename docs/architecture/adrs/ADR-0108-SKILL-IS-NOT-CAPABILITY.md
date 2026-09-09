---
id: ADR-0108-SKILL-IS-NOT-CAPABILITY
package_local_id: ADR-015
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-015-SKILL-IS-NOT-CAPABILITY.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# ADR-0108 — SKILL-IS-NOT-CAPABILITY

> **Mirror of package ADR `ADR-015`.** Repository-native numbering is `ADR-0108-SKILL-IS-NOT-CAPABILITY` (range ADR-0094..0110). Per SUPERSESSION.md, the package's original ADR ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `ADR-015` |
| Repository native | `ADR-0108-SKILL-IS-NOT-CAPABILITY` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-015-SKILL-IS-NOT-CAPABILITY.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# ADR-015 — Skill and Capability are orthogonal contracts

**Status:** Proposed

## Decision

A `SkillDefinition` describes procedural knowledge and an expected task/output contract. A `Capability` describes an executable effect surface governed by Authority.

```text
Skill: core.architecture-review
  may require: filesystem.read, command.run

Capability: filesystem.write
  admission: AuthorityEngine
```

A Skill may declare required/recommended capabilities but cannot grant, acquire or bypass them.

## Consequences

- reusable skills remain safe across provider adapters;
- enabling a skill never silently increases authority;
- Packs can contribute skills without gaining privileged access;
- tests can independently validate skill selection and capability admission.
