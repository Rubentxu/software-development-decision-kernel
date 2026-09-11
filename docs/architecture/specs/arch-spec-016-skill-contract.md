---
id: arch-spec-016-skill-contract
package_local_id: SPEC-016
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-016-SKILL-CONTRACT.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# arch-spec-016 — SKILL-CONTRACT

> **Mirror of package SPEC `SPEC-016`.** Repository-native identifier is `arch-spec-016-skill-contract` (range arch-spec-001..018). Per SUPERSESSION.md, the packages original SPEC ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `SPEC-016` |
| Repository native | `arch-spec-016-skill-contract` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-016-SKILL-CONTRACT.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# SPEC-016 — Skill Contract

## Definition

A Skill is a versioned procedural extension that teaches how to execute or reason about a class of task.

```yaml
id: core.architecture-review
version: 1
applies_when:
  task_kinds: [review.architecture]
inputs:
  - ContextCapsule
outputs:
  contract: ContributionV2
capabilities:
  required: [filesystem.read]
  optional: [command.run]
evidence_contract:
  minimum: source_refs
instruction_fragments:
  - review-checklist
```

## Required properties

- namespaced id and schema version;
- applicability predicate;
- typed input/output contract;
- required/optional capabilities (declaration only);
- evidence expectation;
- instruction fragment refs;
- compatibility range with Agent Experience contract;
- deterministic selection metadata where possible.

### Namespaced id: dotted-name convention (M7.9 clarification)

A Skill id MUST be namespaced (contain at least one `.`). The
canonical form is `<author-or-org>.<area>.<verb>`, e.g.
`core.workflow-orchestration`. SKILL.md frontmatter `name` values
that already contain a `.` are preserved verbatim as the
`SkillDefinition::id` token; the `SkillScope` (`framework`,
`user`, `project`) is recorded as metadata used by
`load_registry` for precedence dedupe but does NOT participate
in the id. SKILL.md frontmatter `name` values without a `.` are
wrapped with the scope prefix at parse time (the historical
default). A name whose first segment equals a scope label
(`framework.foo`, `user.foo`, `project.foo`) is rejected by
the parser to prevent double-wrapping.

## Packs

Packs may register Skills through Pack SDK. They cannot use Skill loading as a backdoor to register privileged executable code outside normal Task/Capability contracts.

## Anti-patterns

A Skill MUST NOT be:

- a hidden agent identity;
- a 1,000-line monolithic prompt containing product architecture;
- a permission grant;
- a private store;
- a second workflow engine.
