---
id: arch-spec-001-canonical-authority
package_local_id: SPEC-001
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-001-CANONICAL-AUTHORITY.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# arch-spec-001 — CANONICAL-AUTHORITY

> **Mirror of package SPEC `SPEC-001`.** Repository-native identifier is `arch-spec-001-canonical-authority` (range arch-spec-001..018). Per SUPERSESSION.md, the packages original SPEC ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `SPEC-001` |
| Repository native | `arch-spec-001-canonical-authority` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-001-CANONICAL-AUTHORITY.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# SPEC-001 — Canonical authority and persistence contracts

## Requirements

- CA-001: exactly one logical CanonicalEventLog is authoritative for ordered facts.
- CA-002: immutable payloads larger than bounded event metadata use CAS references.
- CA-003: every projection declares source stream(s), projector version and rebuild behavior.
- CA-004: every store/model declares Fact/Object/Projection/Ephemeral class.
- CA-005: no projection may be the sole evidence for an irreversible side effect.
- CA-006: event/object canonicalization is deterministic and schema-versioned.
- CA-007: compatibility mirrors are explicitly marked and have removal criteria.

## UAT

1. delete SemanticGraph/metrics/run read models; rebuild from canonical facts and verify semantic equality;
2. inject divergence into a compatibility mirror; canonical reads remain unaffected and doctor reports divergence;
3. architecture lint rejects a second component declaring authority for the same concept.
