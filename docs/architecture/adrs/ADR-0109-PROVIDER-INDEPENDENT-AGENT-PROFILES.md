---
id: ADR-0109-PROVIDER-INDEPENDENT-AGENT-PROFILES
package_local_id: ADR-016
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-016-PROVIDER-INDEPENDENT-AGENT-PROFILES.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# ADR-0109 — PROVIDER-INDEPENDENT-AGENT-PROFILES

> **Mirror of package ADR `ADR-016`.** Repository-native numbering is `ADR-0109-PROVIDER-INDEPENDENT-AGENT-PROFILES` (range ADR-0094..0110). Per SUPERSESSION.md, the package's original ADR ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `ADR-016` |
| Repository native | `ADR-0109-PROVIDER-INDEPENDENT-AGENT-PROFILES` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-016-PROVIDER-INDEPENDENT-AGENT-PROFILES.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# ADR-016 — Agent profiles are provider-independent roles

**Status:** Proposed

## Decision

Define stable `AgentProfile` objects around role semantics: responsibilities, admissible task kinds, context requirements, authority ceiling, preferred skills, output contract and quality constraints.

Provider/model concerns live in `ProviderAdapter` + `ModelDescriptor` configuration.

An agent profile is not a giant provider prompt.

## Consequences

The same reviewer/researcher/implementer role can run through Codex, Claude, OpenCode or local models. Provider-specific prompt/message formatting is replaceable and cannot redefine SDDK semantics.
