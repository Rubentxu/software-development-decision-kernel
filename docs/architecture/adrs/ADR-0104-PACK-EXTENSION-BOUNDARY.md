---
id: ADR-0104-PACK-EXTENSION-BOUNDARY
package_local_id: ADR-011
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-011-PACK-EXTENSION-BOUNDARY.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# ADR-0104 — PACK-EXTENSION-BOUNDARY

> **Mirror of package ADR `ADR-011`.** Repository-native numbering is `ADR-0104-PACK-EXTENSION-BOUNDARY` (range ADR-0094..0110). Per SUPERSESSION.md, the package's original ADR ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `ADR-011` |
| Repository native | `ADR-0104-PACK-EXTENSION-BOUNDARY` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-011-PACK-EXTENSION-BOUNDARY.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# ADR-011 — Packs extend the kernel through namespaced contracts

**Status:** Proposed

## Decision

Core knows no UAT/Incident/Security-specific domain semantics. Packs register versioned contributions through SDK extension points:

- workflow/target/task providers;
- schema providers;
- projection contributors;
- context contributors;
- evidence validators;
- policy rules;
- query/explanation contributors.

Extension kinds use qualified names such as `uat.scenario` or `incident.root_cause`.

Packs cannot write concrete storage tables or mutate canonical projections directly.
