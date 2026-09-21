# ADR-044 — Improve SDDK through governed experiments, not autonomous self-modification

**Status:** Proposed

## Context

ADR-035, SPEC-024, SPEC-033 and SPEC-040 already establish evaluation feedback, fork/replay/diff and Workflow Laboratory.

Recent agent research demonstrates useful patterns such as trace-based reflection, skill curation, workflow search, candidate lineages, holdout evaluation and multi-objective selection.

Adding a generic `autoresearch` or self-evolution subsystem would duplicate existing SDDK concepts and weaken authority boundaries.

## Decision

Extend the existing evaluation/laboratory architecture into **Governed Continuous Improvement (GCI)**.

GCI may improve versioned decision-support artifacts:

```text
skills
prompts
agent/provider manifests
routing policies
context strategies
workflow templates/strategies
verifier policies
```

GCI does NOT automatically modify active kernel code, project source, production/deployment state or hidden evaluation sets.

## Canonical lifecycle

```text
Experience
→ ImprovementProposal
→ Candidate
→ EvaluationContract
→ ForkedExperiment
→ Comparison
→ PromotionRecommendation
→ Policy/Approval
→ Shadow/BoundedRollout
→ Promote or Revert
```

## Authority

- LLMs diagnose and propose.
- deterministic tools evaluate measurable evidence where possible.
- Workflow Laboratory owns controlled comparison.
- normal governance owns promotion.
- Event Ledger records lifecycle.

## Candidate diversity

Multiple candidates may coexist. Lineage/population strategies are optional providers, not kernel primitives.

## No new generic research product

Scientific hypothesis/theory management is outside current SDDK core roadmap. External packs may implement it later using generic primitives.

## Consequence

SDDK can improve its decision harness without becoming a self-authorizing system.
