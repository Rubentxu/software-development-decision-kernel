---
id: ADR-0095-FOUR-STATE-CLASSES
package_local_id: ADR-002
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-002-FOUR-STATE-CLASSES.md
status: accepted
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
accepted_at: 2026-09-12
accepted_by_cycle: "p-63676b11dc0ef88f/adr-promotion-batch-2"

implementation_evidence:
  - "crates/sddk-engine/src/state_class_lint.rs:16 — pub enum StateClass { Fact, Object, Projection, Ephemeral }"
  - "crates/sddk-domain/src/lib.rs — StateClass types"


superseded_by: []
related_adrs:
  - "ADR-0001-ADR-PROMOTION-PROCESS"
  - "ADR-0096-SDLC-LIFECYCLE-SEMANTICS"
stale_after: 2027-09-12

---

# ADR-0095 — FOUR-STATE-CLASSES

> **Mirror of package ADR `ADR-002`.** Repository-native numbering is `ADR-0095-FOUR-STATE-CLASSES` (range ADR-0094..0110). Per SUPERSESSION.md, the package's original ADR ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `ADR-002` |
| Repository native | `ADR-0095-FOUR-STATE-CLASSES` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-002-FOUR-STATE-CLASSES.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# ADR-002 — Facts, Objects, Projections and Ephemeral state

**Status:** Proposed

## Decision

Every SDDK data model/store is classified as exactly one of: Fact, Object, Projection, Ephemeral.

No feature may introduce a persistent store without documenting its class and authority.

## Rationale

This creates a simple invariant stronger than naming conventions and prevents read models, context caches or search indexes from becoming accidental sources of truth.

## Enforcement

Architecture lint scans a machine-readable ownership registry and fails unknown/duplicate authorities after the migration ratchet is enabled.
