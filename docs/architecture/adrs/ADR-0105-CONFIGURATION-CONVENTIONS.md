---
id: ADR-0105-CONFIGURATION-CONVENTIONS
package_local_id: ADR-012
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-012-CONFIGURATION-CONVENTIONS.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# ADR-0105 — CONFIGURATION-CONVENTIONS

> **Mirror of package ADR `ADR-012`.** Repository-native numbering is `ADR-0105-CONFIGURATION-CONVENTIONS` (range ADR-0094..0110). Per SUPERSESSION.md, the package's original ADR ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `ADR-012` |
| Repository native | `ADR-0105-CONFIGURATION-CONVENTIONS` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-012-CONFIGURATION-CONVENTIONS.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# ADR-012 — Convention-first hierarchical configuration

**Status:** Proposed

## Decision

Configuration precedence is deterministic:

```text
built-in defaults
< user/global
< project
< scoped module/pack
< local ignored override
< environment
< CLI
```

Every effective setting is explainable with source provenance (`sddk config explain <key>`).

A lock file may pin schema/pack/workflow/policy contract versions, never secrets.

## Principle

Prefer useful defaults and discovered conventions; require configuration only when ambiguity or risk is material.
