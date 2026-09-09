---
id: arch-spec-014-instruction-compiler
package_local_id: SPEC-014
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-014-INSTRUCTION-COMPILER.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# arch-spec-014 — INSTRUCTION-COMPILER

> **Mirror of package SPEC `SPEC-014`.** Repository-native identifier is `arch-spec-014-instruction-compiler` (range arch-spec-001..018). Per SUPERSESSION.md, the packages original SPEC ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `SPEC-014` |
| Repository native | `arch-spec-014-instruction-compiler` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-014-INSTRUCTION-COMPILER.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# SPEC-014 — Instruction Compiler

## Inputs

```text
KernelInvariantSet
PolicyInstructionSet
TaskContract
ProjectInstructionSet
AgentProfile
SelectedSkillSet
ContextualHints
```

## Output

`EffectiveInstructions` contains ordered sections, source refs, diagnostics, compatibility version and content hash.

## Conflict semantics

Instruction composition is typed:

- invariant/policy constraints cannot be overridden by lower classes;
- task requirements cannot be silently weakened by project/skill text;
- duplicate equivalent directives deduplicate by semantic key;
- conflicting normative directives produce `InstructionConflict` and fail closed unless an explicit resolver policy exists;
- advisory hints may coexist and are labelled advisory.

There is no generic “later file wins” rule for normative instructions.

## Explainability

`context explain --instructions` (or equivalent application query) reports source, reason selected, semantic key, strength, version/hash and excluded/conflicting fragments.

## Provider rendering

ProviderAdapter renders `EffectiveInstructions` into provider-native system/developer/user/tool structures. Rendering may change formatting but not semantic keys/strength.
