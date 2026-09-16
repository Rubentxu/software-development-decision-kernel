---
id: arch-spec-043-generic-verify
title: Generic Verify — delta-scoped verification kernel
status: implemented
milestone: A4
implemented_by: A4-1 + A4-2M (crates/sddk-engine/src/verify_kernel, crates/sddk-cli/src/verify_kernel_cmd; AC4 convergence via `compute_conformance_delta_core` + `ArchitectureVerificationDomain::evaluate_with_context`)
depends_on: arch-spec-042-evidence-observation-provenance
---

# arch-spec-043 — Generic Verify

> **Implemented in A4-1** as the `verify_kernel` engine module and the
> `verify-kernel` CLI command. See `crates/sddk-engine/src/verify_kernel/` and
> `crates/sddk-cli/src/verify_kernel_cmd.rs` for the working implementation.

## Purpose

Verify answers: **what changed, what can be affected, and which
knowledge/evaluations must be refreshed.** It is not a test runner and not a
per-domain engine.

## Shape

```text
ChangeBasis → AffectedSubjects → VerificationClaims → ProbePlan
            → Observations → Evidence → VerificationResult
```

## Constraints

- **Polymorphic by claim, not by domain.** One kernel; an "architecture verify
  engine", a "security verify engine" and a "performance verify engine" must not
  each invent their own receipts, evidence and statuses.
- **Reuse `arch-spec-042` subject.** A probe consumes `SoftwareObservation`s; it
  never mints a second evidence representation.
- Closed result vocabulary, no `bool` + `Option<String>`:
  `Verified | Contradicted | Unknown(gap) | Stale(basis) | NotApplicable`.
- `evaluate(claim, evidence_set) -> VerificationResult` is pure and deterministic
  once the evidence set is fixed.
- Missing evidence is `Unknown`/`NOT_EVALUATED`, never a pass and never a
  fabricated confidence number.
- **AC4 is a specialization, not a rewrite.** `Architecture Verify` becomes
  `Generic Verify → ArchitectureVerificationDomain`; `architecture receipt` keeps
  its current behaviour.
- No universal quality score.

## Open design questions (for A4-1)

- closed ADT core with registry/port extension, versus a fully generic trait;
- how `ProbePlan` minimality is proven;
- where the delta scope ends and DebVerify begins (`arch-spec-044`).
