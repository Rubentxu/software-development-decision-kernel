---
id: arch-spec-013-agent-experience-contract
package_local_id: SPEC-013
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-013-AGENT-EXPERIENCE-CONTRACT.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# arch-spec-013 — AGENT-EXPERIENCE-CONTRACT

> **Mirror of package SPEC `SPEC-013`.** Repository-native identifier is `arch-spec-013-agent-experience-contract` (range arch-spec-001..018). Per SUPERSESSION.md, the packages original SPEC ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `SPEC-013` |
| Repository native | `arch-spec-013-agent-experience-contract` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-013-AGENT-EXPERIENCE-CONTRACT.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# SPEC-013 — Agent Experience Contract

## Objective

Make SDDK usable by coding/review/research agents without hidden architectural knowledge, repeated CLI discovery or provider-specific coupling.

## Required contracts

```text
AgentProfile
SkillDefinition
TaskContract
ContextCapsule
InstructionSource
EffectiveInstructions
CommandSpec
AgentCommandSurface
ExecutionRequest
ExecutionOutcome
Contribution
AgentExecutionReceipt
```

## Execution assembly

For each agent task:

1. resolve Target/Task;
2. resolve AgentProfile;
3. compile bounded ContextCapsule;
4. select applicable Skills;
5. resolve Policy constraints and configured project instructions;
6. compile EffectiveInstructions;
7. derive contextual AgentCommandSurface from CommandRegistry;
8. create ExecutionRequest with typed refs/hashes;
9. render through ProviderAdapter;
10. validate ExecutionOutcome/Contribution against return contracts;
11. emit execution provenance receipt/events.

## Invariants

- provider prompt content never changes authority;
- a command can be known but unavailable/denied at runtime;
- agents use stable refs and JSON schemas where automation is expected;
- normal execution does not depend on exploratory `--help` calls;
- prompt/skill/command changes are versioned and observable;
- no chain-of-thought persistence is required.
