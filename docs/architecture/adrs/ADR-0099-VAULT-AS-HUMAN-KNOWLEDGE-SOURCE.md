---
id: ADR-0099-VAULT-AS-HUMAN-KNOWLEDGE-SOURCE
package_local_id: ADR-006
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-006-VAULT-AS-HUMAN-KNOWLEDGE-SOURCE.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# ADR-0099 — VAULT-AS-HUMAN-KNOWLEDGE-SOURCE

> **Mirror of package ADR `ADR-006`.** Repository-native numbering is `ADR-0099-VAULT-AS-HUMAN-KNOWLEDGE-SOURCE` (range ADR-0094..0110). Per SUPERSESSION.md, the package's original ADR ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `ADR-006` |
| Repository native | `ADR-0099-VAULT-AS-HUMAN-KNOWLEDGE-SOURCE` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-006-VAULT-AS-HUMAN-KNOWLEDGE-SOURCE.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# ADR-006 — Vault is curated human knowledge, not authority

**Status:** Proposed

## Decision

Keep `sddk-vault` as a human-readable knowledge source: Markdown + frontmatter + wikilinks + FTS/HTML browsing.

It may feed Context Compiler and Semantic Graph through adapters. Its local graph/search index is rebuildable and non-authoritative.

Decision Memory never stores “truth” merely because text appears in the Vault; durable decisions require canonical decision facts/memory objects with provenance.

## Rationale

The Vault is valuable precisely because it optimizes explanation and human maintenance, not deterministic authority.
