---
id: ADR-0114-PARADIGMS-AS-ALIGNMENT-LENSES
status: proposed
supersedes_history: false
proposed_at: 2026-09-14
accepted_at: null
accepted_by_cycle: null
implementation_evidence: []
superseded_by: []
related_adrs:
  - "ADR-0106-TYPED-INSTRUCTION-COMPILATION"
stale_after: 2027-03-14
---

# ADR-0114 — Programming Paradigms Are Alignment Lenses, Not Global Policy

## Context

SDDK must reason about object-oriented design, pure/functional programming, ADTs, typed DSLs, event-driven/reactive design and other styles. A universal rule that one paradigm is superior would be incorrect and would turn Alignment into opinionated governance.

## Decision

Represent paradigm intent through scoped `ParadigmProfile` values attached to project/context/unit intent. Software Alignment evaluates evidence through selected lenses.

Initial lenses include:

- Object-Oriented;
- Functional / Pure Functional;
- ADT and illegal-state modeling;
- Typed DSL architecture;
- optional event-driven/reactive/data-oriented lenses.

A lens may report ALIGNED/TENSION/MISALIGNED/ACCEPTED/REVIEW_DUE/UNKNOWN/NOT_APPLICABLE. It may suggest changes but SHALL NOT grant capabilities, modify EffectiveInstructions or directly gate effects.

No OO/FP/DSL rule applies where the corresponding intent/lens is not selected or inferable with accepted evidence.

## Consequences

SDDK can support poly-paradigm systems while still detecting meaningful drift such as hidden mutation in a declared pure core or stringly state in a declared ADT-heavy domain.

## Replaces

This rejects global style scoring and paradigm dogma as architecture policy.
