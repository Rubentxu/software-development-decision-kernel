---
id: ADR-0106-TYPED-INSTRUCTION-COMPILATION
package_local_id: ADR-013
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-013-TYPED-INSTRUCTION-COMPILATION.md
status: accepted
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
accepted_at: 2026-09-12
accepted_by_cycle: "p-63676b11dc0ef88f/adr-promotion-batch-2"

implementation_evidence:
  - "crates/sddk-cli/src/instruction_compiler.rs:301 — EffectiveInstructions, :348 — InstructionCompiler"
  - "AX-S2 conflict algebra pinned by tests"


superseded_by: []
related_adrs:
  - "ADR-0001-ADR-PROMOTION-PROCESS"
  - "ADR-0096-SDLC-LIFECYCLE-SEMANTICS"
stale_after: 2027-09-12

---

# ADR-0106 — TYPED-INSTRUCTION-COMPILATION

> **Mirror of package ADR `ADR-013`.** Repository-native numbering is `ADR-0106-TYPED-INSTRUCTION-COMPILATION` (range ADR-0094..0110). Per SUPERSESSION.md, the package's original ADR ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `ADR-013` |
| Repository native | `ADR-0106-TYPED-INSTRUCTION-COMPILATION` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-ADRS/ADR-013-TYPED-INSTRUCTION-COMPILATION.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# ADR-013 — Compile effective agent instructions from typed sources

**Status:** Proposed  
**Decision:** agent instructions are compiled deterministically from typed/versioned sources; prompt text is a rendering, not an architectural source of truth.

## Context

SDDK currently risks embedding workflow, CLI and architectural behavior in AGENTS files, prompts, skills and provider-specific templates. Such text can drift silently when domain contracts change.

## Decision

Introduce:

```text
InstructionSource
InstructionSet
InstructionCompiler
EffectiveInstructions
InstructionDiagnostic
```

Supported source classes are KernelInvariant, PolicyConstraint, TaskInstruction, ProjectInstruction, AgentRoleInstruction, SkillInstruction and ContextualHint.

Composition rules are structural, not “last text wins”. Conflicting normative directives fail closed or produce an explicit diagnostic according to schema. Rendering order is deterministic and hashes are recorded in execution provenance.

## Consequences

- prompts become generated/provider-specific views;
- changing architecture requires changing typed contracts/tests, not hunting hidden prompt text;
- effective instructions can be explained and reproduced without storing chain of thought;
- legacy monolithic prompts require migration/splitting.
