---
id: ADR-0103-TARGET-TASK-PORCELAIN
package_local_id: ADR-010
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-010-TARGET-TASK-PORCELAIN.md
status: accepted
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
accepted_at: 2026-09-12
accepted_by_cycle: "p-63676b11dc0ef88f/adr-promotion-batch-2"

implementation_evidence:
  - "crates/sddk-engine/src/target_task/mod.rs:142 — pub struct Task, :173 — pub struct Target"
  - "crates/sddk-engine/src/target_task/executor.rs — DagExecutor"


superseded_by: []
related_adrs:
  - "ADR-0001-ADR-PROMOTION-PROCESS"
  - "ADR-0096-SDLC-LIFECYCLE-SEMANTICS"
stale_after: 2027-09-12

---

# ADR-0103 — TARGET-TASK-PORCELAIN

> **Mirror of package ADR `ADR-010`.** Repository-native numbering is `ADR-0103-TARGET-TASK-PORCELAIN` (range ADR-0094..0110). Per SUPERSESSION.md, the package's original ADR ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `ADR-010` |
| Repository native | `ADR-0103-TARGET-TASK-PORCELAIN` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-010-TARGET-TASK-PORCELAIN.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# ADR-010 — Target/Task graph and porcelain CLI

**Status:** Proposed

## Decision

Keep domain `Goal` as user/project intent. Use **Target** for Maven/Gradle-like CLI workflow aggregation to avoid overloading Goal.

A Target resolves to a deterministic Task DAG. Tasks declare dependencies, inputs, outputs, side-effect class, authority requirement, evidence contract, cacheability and retry semantics.

Default user CLI is porcelain; low-level existing commands remain plumbing/compatibility during migration.

## Cache rule

Deterministic local tasks may be up-to-date/cached by declared inputs. LLM outputs are never treated as deterministic build-cache truth; replayed outputs are candidate/evidence requiring the same governance rules.
