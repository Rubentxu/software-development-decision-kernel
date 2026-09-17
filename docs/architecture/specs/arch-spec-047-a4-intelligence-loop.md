---
id: arch-spec-047-a4-intelligence-loop
title: A4 intelligence loop and repository spec-ID reconciliation
status: contract-ready
milestone: A4
implemented_by: partial — A4-5a composition shipped (v1.169.64); AdvisoryContext + WHY pending A4-5b; acceptance pending A4-5C
depends_on: 042, 043, 044, 045, 046
---

# arch-spec-047 — A4 intelligence loop

> **Partially implemented (A4-5a composition shipped v1.169.64).**
> **AdvisoryContext + WHY pending A4-5b; acceptance pending A4-5C.**

## The loop

```text
Sources + Decision Memory
  → Knowledge / KMT + SemanticGraph
  → Observations + Evidence          (arch-spec-042)
  → Verify                           (arch-spec-043)
  → DebVerify                        (arch-spec-044)
  → Alignment                        (arch-spec-045/046)
  → AdvisoryContext                  (delivered in A3 closeout)
```

Governance enters **only at the end**:

```text
VerificationReceipt + AlignmentAssessment
  → Governance evaluates explicit policy
```

**Never** `MISALIGNED → DENY`. Alignment computes the delta and presents
attention; only Governance can turn explicit policy into a ratchet.

**Never** `Alignment → Authority`, `Alignment → Capability`,
`Alignment → InstructionSource`. Alignment produces findings and
assessments; it must never become a ratchet on its own. Authority is
granted by Governance (after the loop), Capability is granted by the
runtime, and InstructionSource is a separate provenance direction.

## The ordering is the point

> SDDK does not evaluate first and look for evidence afterwards. SDDK observes,
> keeps provenance, formulates/evaluates claims, and only then interprets
> Alignment.

This order is what prevents Software Alignment from becoming another "intelligent
analyzer" whose opinions cannot be justified: every conclusion must be traceable
back to the relations and evidence that produced it.

## Spec-ID reconciliation (A4-0 record)

The historical context-first package
(`docs/SDDK-Context-First-Semantic-Core-Agent-Experience-Software-Alignment-2026-09-10/`)
defines Alignment/Verify/DebVerify under ids **SPEC-019 … SPEC-034**.

Those numbers are **already owned** in the normative repository
(`arch-spec-019` production-readiness convergence through `arch-spec-035` paradigm
lens system). Reusing them would collide. A4 therefore consolidates at **042+**,
which was verified free (highest assigned: 041).

Historical ids that would have collided: `SPEC-019-SOFTWARE-ALIGNMENT-DOMAIN`,
`SPEC-024-ALIGNMENT-LENSES`, `SPEC-025-ALIGNMENT-WORKBOOKS`,
`SPEC-027-VERIFY-DELTA-INTELLIGENCE`,
`SPEC-028-DEBVERIFY-GLOBAL-RECONCILIATION`,
`SPEC-029-LLM-ALIGNMENT-EVALUATOR`, `SPEC-034-STATIC-RUNTIME-ALIGNMENT`.

## A4-5a composition seam (v1.169.64, 2026-09-17)

> Shipped by cycle `p-63676b11dc0ef88f/a4-5a-intelligence-loop-composition`.

The four authority outputs (Knowledge, Observation, Verify, DebVerify,
AlignmentLens, reduce_alignment) are now bound under one content-addressed
`IntelligenceLoopReceiptId` via the `intelligence_loop` module. Two new
public types: `IntelligenceLoopResult` (EPHEMERAL) and
`IntelligenceLoopReceipt` (PROJECTION). Composition receives — never
calls — the authority outputs. NO AdvisoryContext, NO WHY, NO
Governance, NO authority derivation. MISALIGNED ≠ DENY is structurally
pinned by the type system. See `crates/sddk-engine/src/intelligence_loop/`
and ADR-0126.

## Explicitly out of A4

CogniCode, Chronos, JCode reactive loop, LLM alignment evaluator,
workbooks/control tower, counterfactual planning, proof-carrying changes — except
for the seams that are genuinely unavoidable.
