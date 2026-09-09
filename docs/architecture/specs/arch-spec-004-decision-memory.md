---
id: arch-spec-004-decision-memory
package_local_id: SPEC-004
package_source: docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-004-DECISION-MEMORY.md
status: proposed
supersedes_history: false
adopted_at: 2026-09-09
adoption_cycle: p-63676b11dc0ef88f/architecture-adoption-m0-supersession
---

# arch-spec-004 — DECISION-MEMORY

> **Mirror of package SPEC `SPEC-004`.** Repository-native identifier is `arch-spec-004-decision-memory` (range arch-spec-001..018). Per SUPERSESSION.md, the packages original SPEC ID is preserved in `package_local_id` and the canonical content is copied verbatim into this file.

## Crosswalk

| Surface | Identifier |
|---|---|
| Package local | `SPEC-004` |
| Repository native | `arch-spec-004-decision-memory` |
| Source | `docs/SDDK-Semantic-Core-Agent-Experience-Consolidation-2026-09-09/02-SPECS/SPEC-004-DECISION-MEMORY.md` |
| Adoption cycle | `p-63676b11dc0ef88f/architecture-adoption-m0-supersession` |

---

# SPEC-004 — Decision Memory

## Purpose

Recover durable decision context across sessions, agents and long-running SDLC work without storing chain-of-thought or treating raw chat as authority.

## Memory object kinds

Core kinds: Claim, Decision, Alternative, Assumption, Risk, EvidenceRef, Question, Dissent, ContributionRef, SynthesisRef, OutcomeRef, RevisitTrigger, ContextDelta.

## Memory classes

- Decision/rationale memory: durable and authoritative through decision facts/provenance.
- Episodic memory: what happened, backed by run/event references.
- Semantic/project memory: validated facts/conventions.
- Procedural memory: project/team working conventions and policies.
- Negative memory: rejected alternatives, invalidated assumptions, dissent and failure lessons.
- Code topology memory: rebuildable index, never durable authority.

Working memory is NOT a durable memory class; it is a ContextCapsule.

## CLI

```text
sddk memory status
sddk memory log
sddk memory tree
sddk memory show <ref|oid>
sddk memory diff <a>..<b>
sddk memory merge-base <a> <b>
sddk memory reflog
sddk memory audit
```

`diff` is semantic: decisions, assumptions, dissent, evidence, risks and frontier changes.

## Promotion

Raw agent text enters durable memory only through a typed extraction/promotion step with source provenance and evidence/authority classification.
