---
id: arch-spec-045-software-alignment-domain
title: Software Alignment domain
status: contract-ready
milestone: A4
implemented_by: pending (A4-3)
depends_on: arch-spec-042-evidence-observation-provenance
reconciles: historical SPEC-019-SOFTWARE-ALIGNMENT-DOMAIN
---

# arch-spec-045 — Software Alignment domain

> **Contract-ready, NOT implemented.** Reconciled from the historical context-first
> package (see `arch-spec-047` for the ID-collision record). The historical content
> is **not copied verbatim**; it is re-expressed against post-A3 reality.

## Kept from the historical model

Assessment statuses (7, closed):

```text
ALIGNED | TENSION | MISALIGNED | ACCEPTED | REVIEW_DUE | UNKNOWN | NOT_APPLICABLE
```

Finding kinds:

```text
ContractViolation | AlignmentTension | ImprovementOpportunity
```

with `ArchitecturalIntentSnapshot` as an **input**, not a finding.

## Rules kept verbatim in spirit

- **`ContractViolation` requires an explicit contract or invariant.** Heuristic
  disagreement is never automatically a violation. A smell is not a violation.
- Alignment is **advisory**. A3 closeout made that executable: alignment feeds
  `AdvisoryContext` (closed `AdvisoryKind::AlignmentTension`), which provably
  cannot change `effective_instruction_set_hash`. Only a context with real
  authority can originate a `MUST`.

## Pipeline (A4-3)

```text
ArchitecturalIntentSnapshot + Observed Software Reality + KnowledgeBasis + Evidence
  → Alignment Lens
  → AlignmentAssessment
```

## Constraints

- `fact != assessment != recommendation != authority`. Four classes, never merged.
- No universal quality score.
- Alignment never denies anything; it computes a delta and presents attention.
  Governance alone converts explicit policy into a ratchet or gate.
- Consumption of `arch-spec-042` observations is the only evidence channel.
