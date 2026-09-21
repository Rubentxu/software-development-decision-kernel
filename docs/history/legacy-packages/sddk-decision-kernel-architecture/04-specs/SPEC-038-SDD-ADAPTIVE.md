# SPEC-038 — SDD Adaptive

**Status:** Experimental

## Goal
Provide a compact SDD workflow that preserves quality/traceability while reducing mandatory agents, handoffs and phase boundaries.

## Core invariant
A change is complete only when **Intent ⇄ Implementation ⇄ Evidence** are coherent.

## Stages

### PREFLIGHT — deterministic
Identity/adoption, Git/worktree state, relevant policy/project context and baseline signals.

### SHAPE
Produce/accept a `ChangeContract`.

Required minimum:
```yaml
intent:
  problem: ...
  desired_outcome: ...
scope:
  in: []
  out: []
requirements:
  - id: R1
    behavior: ...
    acceptance: ...
constraints: {}
decisions: []
risks: []
verification:
  obligations: []
work_units: []
```

SHAPE dynamically invokes explore/research/architecture/security/test-design only when context/risk requires them.

### BUILD
Convert WorkUnits to isolated implementation nodes/worktrees. Dependencies determine parallelism. New WorkUnits may be discovered during build.

### CONVERGE
Run cheap deterministic checks first, then adaptive specialist verification. Produce gaps linked to requirements/constraints/evidence. Gaps become remediation WorkUnits and return to BUILD.

Max convergence rounds are policy/budget bounded.

### INTEGRATE
Deterministic/governed release, knowledge projection, receipts, SBOM/provenance and metrics. Agent assistance is optional for summaries/docs.

## Document compatibility
Human documents remain available as projections:
- `proposal.md` ← approach/risks;
- `spec.md` ← requirements/acceptance;
- `design.md` ← decisions/components/interfaces;
- `tasks.md` ← WorkGraph/WorkUnits;
- `verify-report.md` ← Convergence evidence.

## Risk profiles
Not fixed paths. Runtime derives a verification/shaping plan from risk/uncertainty signals. Presets may be exposed for UX but are compiler hints.

## Baseline policy
Do not replace A-full until Workflow Laboratory demonstrates non-inferior quality on agreed golden/real tasks.

## Acceptance criteria
- no required SDD invariant is lost when optional phase documents are absent;
- low-risk change can complete with substantially fewer cognitive handoffs;
- high-risk change can expand to specialists/UAT/adversarial review;
- all adaptive decisions are explainable from events/policy.
