---
id: ADR-0098-ONE-SEMANTIC-GRAPH
package_local_id: ADR-005
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-005-ONE-SEMANTIC-GRAPH.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# ADR-0098 — ONE-SEMANTIC-GRAPH

> **Mirror of package ADR `ADR-005`.** Repository-native numbering is `ADR-0098-ONE-SEMANTIC-GRAPH` (range ADR-0094..0110). Per SUPERSESSION.md, the package's original ADR ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `ADR-005` |
| Repository native | `ADR-0098-ONE-SEMANTIC-GRAPH` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-005-ONE-SEMANTIC-GRAPH.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# ADR-005 — One semantic graph projection, many typed views

**Status:** Proposed

## Decision

`GraphProjection` evolves into the canonical rebuildable **SemanticGraphProjection** over canonical facts and typed object references.

`ActiveGraphProjection` becomes a compatibility/typed view, not a second graph authority. Vault wiki graph remains an internal adapter/index. Pack-specific graph concepts use namespaced kinds/relations.

## Core relations

Relations distinguish semantics explicitly: `depends_on`, `caused_by`, `supported_by`, `contradicted_by`, `justified_by`, `selected_over`, `supersedes`, `produced_by`, `gates`, `references`.

## Consequences

Why/impact queries gain consistent semantics; graph storage can later change without changing authority.
