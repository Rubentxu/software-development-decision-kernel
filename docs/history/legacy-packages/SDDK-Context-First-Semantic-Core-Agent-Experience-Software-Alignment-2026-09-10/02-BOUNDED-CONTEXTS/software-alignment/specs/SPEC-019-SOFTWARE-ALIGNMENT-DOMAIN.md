# SPEC-019 — Software Alignment Domain

## Mission

Help humans/agents understand whether observed software remains coherent with architectural intent, accepted decisions and selected paradigms.

## Core ADTs

```text
AlignmentAssessment
  subject
  concern
  lens?
  status
  evidence_refs[]
  intent_refs[]
  tradeoff_refs[]
  confidence_components
  basis

AlignmentStatus =
  ALIGNED
  | TENSION
  | MISALIGNED
  | ACCEPTED
  | REVIEW_DUE
  | UNKNOWN
  | NOT_APPLICABLE

AlignmentFinding =
  Observation
  | AlignmentTension
  | Misalignment
  | ImprovementOpportunity
  | Contradiction
  | ContractViolation
```

`ContractViolation` requires an explicit contract/invariant. Heuristic disagreement is never automatically a violation.

## Non-responsibilities

Alignment does not execute refactors, approve releases, grant capabilities, own raw metrics or create imperative agent instructions.
