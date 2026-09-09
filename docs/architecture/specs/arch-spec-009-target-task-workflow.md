---
id: arch-spec-009-target-task-workflow
package_local_id: SPEC-009
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-009-TARGET-TASK-WORKFLOW.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# arch-spec-009 — TARGET-TASK-WORKFLOW

> **Mirror of package SPEC `SPEC-009`.** Repository-native identifier is `arch-spec-009-target-task-workflow` (range arch-spec-001..018). Per SUPERSESSION.md, the packages original SPEC ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `SPEC-009` |
| Repository native | `arch-spec-009-target-task-workflow` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-009-TARGET-TASK-WORKFLOW.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# SPEC-009 — Target/Task workflow model

## Goal

Reduce CLI bureaucracy by grouping lower-level capabilities into declarative, explainable execution graphs inspired by Maven lifecycle phases and Gradle task graphs.

## Target

A named user intent such as `change`, `verify`, `ship`, `recover`, `audit`.

## Task contract

Each Task declares:

```text
id
inputs
outputs
depends_on
side_effect_class
authority_requirement
determinism
cacheability
retry_policy
evidence_contract
memory_effects
```

## UX

```text
sddk run verify --dry-run

context.resolve        UP-TO-DATE
plan.validate          UP-TO-DATE
build                  EXECUTE
test                   EXECUTE
assurance              WAITING
evidence.collect       WAITING
```

## Rules

- target names are not domain `Goal`s;
- target resolution is deterministic under effective config/pack versions;
- LLM execution output is never silently reused as deterministic cache truth;
- side-effect tasks route through Authority Engine.
