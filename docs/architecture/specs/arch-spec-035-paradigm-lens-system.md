---
id: arch-spec-035-paradigm-lens-system
status: proposed
proposed_at: 2026-09-14
source: docs/history/legacy-packages/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/
supersedes_history: false
---

# arch-spec-035 — Paradigm Lens System

## Intent

SDDK SHALL reason about programming paradigms relative to scoped project intent, not global ideology.

## Requirements

- AC-035-001: `ParadigmProfile` may vary by project/context/software unit.
- AC-035-002: initial lenses include OO, Functional/Pure Functional, ADT and Typed DSL.
- AC-035-003: additional event-driven/reactive/data-oriented/custom lenses are extensible without shared-kernel ontology explosion.
- AC-035-004: absent intent/evidence yields UNKNOWN or NOT_APPLICABLE rather than MISALIGNED.
- AC-035-005: Alignment status never modifies EffectiveInstructions, capabilities or AuthorityEngine decisions directly.
- AC-035-006: multiple paradigms may be simultaneously valid in one system.
- AC-035-007: assessments cite concrete evidence and lens version/basis.

## Acceptance

AC-UAT-011..015.
