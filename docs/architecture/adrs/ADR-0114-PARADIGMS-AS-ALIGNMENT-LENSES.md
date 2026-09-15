---
id: ADR-0114-PARADIGMS-AS-ALIGNMENT-LENSES
status: accepted
supersedes_history: false
proposed_at: 2026-09-14
accepted_at: 2026-09-15
accepted_by_cycle: p-63676b11dc0ef88f/a3-4-paradigm-lens-profile
implementation_evidence:
  - "crates/sddk-engine/src/paradigm_profile/mod.rs (ParadigmProfileOverlay public surface; state-class doc; re-exports)"
  - "crates/sddk-engine/src/paradigm_profile/types.rs (ParadigmProfileKind 11-closed, ParadigmLensKind 11-closed, LensStatus 7-closed, EvidenceBasis 5-closed, ParadigmAnchorKind 3-closed; ProjectIntentRef; BoundedContextRef; SoftwareUnitRef; LensAssessment data shape; LensAssessmentId)"
  - "crates/sddk-engine/src/paradigm_profile/overlay.rs (add_anchor/add_project/add_bounded_context/add_software_unit/add_profile/add_lens/add_assessment; find_profiles_for_anchor; find_anchors_for_paradigm; find_assessments_for_anchor; Query ADT 3-closed; QueryResult 3-closed)"
  - "crates/sddk-engine/src/paradigm_profile/rebuild.rs (sorted rebuild for determinism; REQ-AC3-013..015)"
  - "crates/sddk-engine/src/paradigm_profile/tests.rs (acceptance + 4 anti-encroachment pins + 6 bonus pins)"
  - "docs/architecture/specs/arch-spec-A3-S4-paradigm-lens-profile.md (cycle-bounded spec, 25 REQs)"
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
