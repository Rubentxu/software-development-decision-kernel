---
id: arch-spec-010-pack-sdk
package_local_id: SPEC-010
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-010-PACK-SDK.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# arch-spec-010 — PACK-SDK

> **Mirror of package SPEC `SPEC-010`.** Repository-native identifier is `arch-spec-010-pack-sdk` (range arch-spec-001..018). Per SUPERSESSION.md, the packages original SPEC ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `SPEC-010` |
| Repository native | `arch-spec-010-pack-sdk` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-010-PACK-SDK.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# SPEC-010 — Pack SDK and extension boundaries

## Extension points

A pack may provide versioned contributions for:

- schemas/node/relation namespaces;
- workflow definitions;
- Targets/Tasks;
- evidence validators;
- projection contributors;
- context contributors;
- policy rules;
- explanation/query contributors;
- CLI subcommands under its namespace only when a Target is insufficient.

## Isolation requirements

- no concrete SQLite dependency from pack domain;
- no direct writes to SemanticGraph storage;
- no additions to core closed enums solely for one pack;
- explicit pack compatibility range and schema version;
- disable/unload degrades gracefully when optional.

## Conformance suite

Every pack passes: manifest validation, deterministic registration, namespace collision tests, projection rebuild tests, disabled-pack behavior and Authority Engine bypass test.


## Agent Experience extension points

Packs MAY contribute namespaced:

```text
SkillDefinition
AgentProfile supplement (rare; prefer core roles + skills)
Task templates
ContextContributor
InstructionSource fragments
Command examples for commands/tasks the pack owns
Output/Evidence schemas
```

Packs MUST NOT grant Capabilities, override KernelInvariant/PolicyConstraint instructions, inject unregistered CLI syntax or mutate Decision Memory/projection storage directly.
