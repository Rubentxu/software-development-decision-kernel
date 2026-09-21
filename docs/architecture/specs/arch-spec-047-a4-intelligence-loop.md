---
id: arch-spec-047-a4-intelligence-loop
title: A4 intelligence loop and repository spec-ID reconciliation
status: implemented
milestone: A4
implemented_by: A4-5a (v1.169.64); A4-S15R (v1.169.65); A4-5b (v1.169.66); A4-5C acceptance (2026-09-17)
depends_on: 042, 043, 044, 045, 046
---

# arch-spec-047 — A4 intelligence loop

> **Implemented.** A4-5a composition (v1.169.64), A4-S15R provenance
> (v1.169.65), A4-5b AdvisoryContext + WHY (v1.169.66), and A4-5C
> acceptance (2026-09-17) have all shipped.
>
> Acceptance receipt:
> `docs/architecture/receipts/A4-5C-arch-spec-047-acceptance-receipt.md`
> — every normative clause PASS, no MUST `NOT_PROVEN`.

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
(`docs/history/legacy-packages/SDDK-Context-First-Semantic-Core-Agent-Experience-Software-Alignment-2026-09-10/`)
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

## A4-5b AdvisoryContext + WHY integration (v1.169.66, 2026-09-17)

> Shipped by cycle `p-63676b11dc0ef88f/a4-5b-advisory-context-why-integration`.

The A4-5a composition output now feeds the **existing A3
`AdvisoryContext`** (reused verbatim; no parallel advisory model) and a
typed, provenance-backed WHY. New module:
`crates/sddk-engine/src/intelligence_advisory/`.

- `derive_advisory_context(&IntelligenceLoopResult) -> AdvisoryContext` —
  deterministic, canonical; one item per verification pair, the
  reconciliation, each lens contribution, each lens coverage gap, and the
  alignment posture. Nothing from the loop is lost.
- `explain_advisory(result, receipt_id, graph, subject) -> AdvisoryWhy` —
  typed trace: `basis` (clock-stable) + typed `legs` + `unresolved` +
  `why_not`.
- `AdvisoryKind` gained `VerificationOutcome`, `DebtReconciliation`,
  `LensCoverageGap`; `AlignmentTension` (reserved at A3 closeout) is now
  produced.
- The AC2 overlay gained a read-only `contract_provenance(contract_id)`
  accessor exposing A4-S15R's two axes (`SpecifiedBy` spec payloads,
  `VerifiedBy` typed evidence) without conflating them.

Invariants pinned (20-test corpus
`crates/sddk-engine/tests/a4_5b_advisory_context_why.rs`):

- `SpecifiedBy` does not imply `VerifiedBy`; an empty evidence axis is
  **reported** (as `unresolved`), never negated.
- `no edge != evidence of negation`.
- WHY explains an existing conclusion; it never creates a stronger one.
- `MISALIGNED != DENY` (and no authority/capability/instruction coupling).
- No hidden orchestration: the module never calls `VerifyKernel`,
  `DebVerifyKernel`, `AlignmentLensKernel`, `reduce_alignment`, the legacy
  `paradigm_lens` evaluator, or any provider.
- No global verdict (`status`/`score`/`verdict`/`health`/`confidence`/…).

See `crates/sddk-engine/src/intelligence_advisory/` and ADR-0128.

## A4-5C acceptance (2026-09-17)

> Cycle `p-63676b11dc0ef88f-a4-5c-arch-spec-047-acceptance`.
> Budget: ACCEPTANCE / RECEIPT / UAT ONLY (zero production semantics).

Cross-cutting acceptance that every normative clause of this spec holds
simultaneously, including negative cases. 34-test corpus
`crates/sddk-engine/tests/a4_5c_arch_spec_047_acceptance.rs`; durable
Acceptance Receipt at
`docs/architecture/receipts/A4-5C-arch-spec-047-acceptance-receipt.md`.

Result: **every normative clause PASS; no MUST `NOT_PROVEN`.** No
production change was required (no STOP triggered). The spec is promoted
to `status: implemented`.

## Explicitly out of A4

CogniCode, Chronos, JCode reactive loop, LLM alignment evaluator,
workbooks/control tower, counterfactual planning, proof-carrying changes — except
for the seams that are genuinely unavoidable.
