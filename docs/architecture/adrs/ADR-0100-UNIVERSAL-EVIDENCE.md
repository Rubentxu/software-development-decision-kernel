---
id: ADR-0100-UNIVERSAL-EVIDENCE
package_local_id: ADR-007
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-007-UNIVERSAL-EVIDENCE.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# ADR-0100 — UNIVERSAL-EVIDENCE

> **Mirror of package ADR `ADR-007`.** Repository-native numbering is `ADR-0100-UNIVERSAL-EVIDENCE` (range ADR-0094..0110). Per SUPERSESSION.md, the package's original ADR ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `ADR-007` |
| Repository native | `ADR-0100-UNIVERSAL-EVIDENCE` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-007-UNIVERSAL-EVIDENCE.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# ADR-007 — Universal evidence model

**Status:** Proposed

## Decision

`EvidenceBundle`/`EvidenceArtifact` are the single core evidence model. Planning/UAT/assurance specializations attach typed relationships and metadata rather than defining parallel evidence taxonomies.

Planning-specific evidence attachment types are migrated to `EvidenceRef` plus relation (`supports`, `verifies`, `gates`, `observed_for`).

## Consequences

Redaction, content-addressing, provenance and verification are implemented once. Packs extend evidence metadata through namespaced schemas.
