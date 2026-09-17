---
id: arch-spec-046-alignment-intent-and-lenses
title: Alignment intent, universal concerns and lenses
status: implemented
milestone: A4
implemented_by: part-1 (model surface: closed vocabularies, typed intent, pure reducer) shipped as v1.169.52 / A4-4a with applicability semantics correction v1.169.54 / A4-4aR; part-2 (kernel+registry) shipped as v1.169.57 / A4-4b with subject-general evidence v1.169.58 / A4-4bR; paradigm_lens migration (A4-4M) shipped as v1.169.59; receipt/UAT closure (A4-4C) shipped as v1.169.60. Loop integration (A4-5) remains open — see arch-spec-047.
depends_on: arch-spec-045-software-alignment-domain
reconciles: historical SPEC-024-ALIGNMENT-LENSES
---

# arch-spec-046 — Alignment intent, universal concerns and lenses

> **Contract-ready, NOT implemented.**

## UniversalConcern vs AlignmentLens

The distinction the historical model got right and this spec preserves:

```text
Concern:  Coupling
Lens:     architecture.hexagonal
          oo.solid-grasp
          functional.effects-and-adt
```

A **concern** is a universal question the domain asks of any software. A **lens**
is an interpretation framework selected by project/unit intent. This is what stops
a paradigm from becoming religion: the same datum

```text
A depends on B
```

is interpreted differently under a different project intent, bounded context,
paradigm profile or explicit decision.

## Universal concerns (validated in A4-4a as the closed 10-member vocabulary)

cohesion · coupling · boundary integrity · state safety · effect visibility ·
dependency direction · semantic ownership · temporal coupling · testability ·
freshness.

## Constraints

- Lenses are selected by **declared intent**, never applied universally.
- The poly-paradigm policy stands: OO, Functional/Pure, ADTs, Typed DSL,
  reactive/event-driven and custom profiles are lenses, not policy.
- `arch-spec-035`'s paradigm profiles are an input, not a competing registry.
- No lens may produce a `ContractViolation` without an explicit contract.
