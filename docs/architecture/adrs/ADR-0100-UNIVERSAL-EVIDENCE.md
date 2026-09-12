---
id: ADR-0100-UNIVERSAL-EVIDENCE
package_local_id: ADR-007
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-007-UNIVERSAL-EVIDENCE.md
status: accepted
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
accepted_at: 2026-09-12
accepted_by_cycle: p-63676b11dc0ef88f/evidence-relations-core
implementation_evidence:
  - "crates/sddk-engine/src/semantic_kind.rs:83 — `pub enum CoreRelationKind` includes `Verifies` (Evidence → Decision/Assumption/Contribution) and `ObservedFor` (Evidence → Risk/Goal/Run) per ADR-0100"
  - "crates/sddk-engine/src/semantic_kind.rs:99 — `CoreRelationKind::ALL` array extended to 14 entries (was 12)"
  - "crates/sddk-engine/src/semantic_kind.rs:339 — pin test `parse_recognises_evidence_relations` asserts both relations round-trip through `RelationKind::parse` as Core variants, not as NamespacedKind extension"
  - "crates/sddk-engine/src/semantic_kind.rs:358 — pin test `core_relation_kinds_have_14_entries_after_evidence_relations` enforces the count so future additions are explicit"
  - "crates/sddk-engine/src/semantic_kind.rs:371 — pin test `evidence_relations_have_canonical_tags` enforces `Verifies → \"verifies\"` and `ObservedFor → \"observed_for\"` snake_case tags"
superseded_by: []
related_adrs:
  - "ADR-0001-ADR-PROMOTION-PROCESS"
  - "ADR-0096-SDLC-LIFECYCLE-SEMANTICS"
  - "ADR-0095-FOUR-STATE-CLASSES"
stale_after: 2027-09-12
---

# ADR-0100 — UNIVERSAL-EVIDENCE

> **Mirror of package ADR `ADR-007`.** Repository-native numbering is `ADR-0100-UNIVERSAL-EVIDENCE` (range ADR-0094..0110). Per SUPERSESSION.md, the package's original ADR ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Promotion (2026-09-12 review)

Promoted from `proposed` to `accepted` by cycle `p-63676b11dc0ef88f/evidence-relations-core` (v1.168.35). The construction gap that blocked the previous `deferred_until` ("EvidenceRef Verifies + ObservedFor relations built and pinned by tests") is now closed:

- `CoreRelationKind::Verifies` and `CoreRelationKind::ObservedFor` added as closed-set variants in `crates/sddk-engine/src/semantic_kind.rs`
- `CoreRelationKind::ALL` extended from 12 to 14
- `domain_tag` returns lowercase snake_case (`verifies`, `observed_for`)
- `RelationKind::parse("verifies")` and `parse("observed_for")` route through the closed-set parser (not the NamespacedKind extension), pinning them as first-class core relations

This unlocks `evidence_kind_v1` lint promotion: the canonical EvidenceRef + `Verifies`/`ObservedFor` substitution is now implementable. Cycle 7 (`synthesis-dissent-runner-extension`) is the remaining gate for that lint; the Evidence relation construction is the first half of the unblock.

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
