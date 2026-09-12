---
id: ADR-0100-UNIVERSAL-EVIDENCE
package_local_id: ADR-007
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-007-UNIVERSAL-EVIDENCE.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
deferred_until: "EvidenceRef Verifies + ObservedFor relations built and pinned by tests"
deferred_reason: "Partial implementation: `EvidenceRef` + `Supports` + `Gates` relations exist in `crates/sddk-engine/src/semantic_kind.rs::CoreRelationKind`; `Verifies` and `ObservedFor` relations are absent. Promotion blocked on construction, not audit. Per ADR-0001 §3.2 criterion 1."
superseded_by: []
related_adrs:
  - "ADR-0001-ADR-PROMOTION-PROCESS"
  - "ADR-0096-SDLC-LIFECYCLE-SEMANTICS"
stale_after: 2027-09-12
---

# ADR-0100 — UNIVERSAL-EVIDENCE

> **Mirror of package ADR `ADR-007`.** Repository-native numbering is `ADR-0100-UNIVERSAL-EVIDENCE` (range ADR-0094..0110). Per SUPERSESSION.md, the package's original ADR ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

> **Status:** proposed (2026-09-12 review). Promotion deferred per `deferred_until` frontmatter. The ADR's `evidence_kind_v1` lint counterpart in `deprecated_patterns.toml` stays `default: allow` until this ADR is accepted.

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
