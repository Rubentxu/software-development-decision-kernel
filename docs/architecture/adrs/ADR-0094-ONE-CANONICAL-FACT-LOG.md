---
id: ADR-0094-ONE-CANONICAL-FACT-LOG
package_local_id: ADR-001
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-001-ONE-CANONICAL-FACT-LOG.md
status: accepted
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
accepted_at: 2026-09-12
accepted_by_cycle: "p-63676b11dc0ef88f/adr-promotion-batch-2"

implementation_evidence:
  - "crates/sddk-engine/src/canonical_event_log.rs:222 — pub trait CanonicalEventLog + InMemoryCanonicalEventLog (line 232)"
  - "docs/architecture/specs/arch-spec-001-canonical-authority.md"


superseded_by: []
related_adrs:
  - "ADR-0001-ADR-PROMOTION-PROCESS"
  - "ADR-0096-SDLC-LIFECYCLE-SEMANTICS"
stale_after: 2027-09-12

---

# ADR-0094 — ONE-CANONICAL-FACT-LOG

> **Mirror of package ADR `ADR-001`.** Repository-native numbering is `ADR-0094-ONE-CANONICAL-FACT-LOG` (range ADR-0094..0110). Per SUPERSESSION.md, the package's original ADR ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `ADR-001` |
| Repository native | `ADR-0094-ONE-CANONICAL-FACT-LOG` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-001-ONE-CANONICAL-FACT-LOG.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# ADR-001 — One canonical fact log

**Status:** Proposed

## Context

SDDK currently contains more than one event/ledger representation and even consistency checks between them. Long-term dual authority creates drift and forces every projection to choose a source.

## Decision

SDDK SHALL expose one logical `CanonicalEventLog` port as the sole ordered authority for domain facts. Physical migration mirrors are compatibility mechanisms only.

`EventStore`, legacy ledger interfaces and concrete SQLite tables may coexist temporarily, but one is designated canonical in each migration phase and writes MUST flow through one application service.

## Consequences

- projections consume one logical stream contract;
- cross-ledger consistency becomes a migration check, not permanent architecture;
- replay/rebuild semantics become simpler;
- append schemas require explicit compatibility/version policy.

## Rejected

Permanent dual-write with reconciliation: too much operational ambiguity for a local-first deterministic kernel.
