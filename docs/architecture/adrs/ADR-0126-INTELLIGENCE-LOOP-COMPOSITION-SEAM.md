---
id: ADR-0126-INTELLIGENCE-LOOP-COMPOSITION-SEAM
status: accepted
supersedes_history: false
proposed_at: 2026-09-17
accepted_at: 2026-09-17
accepted_by_cycle: p-63676b11dc0ef88f/a4-5a-intelligence-loop-composition
implementation_evidence:
  - "crates/sddk-engine/src/intelligence_loop/mod.rs (composition seam; two public types)"
  - "crates/sddk-engine/tests/a4_5a_intelligence_loop_composition.rs (16 pins)"
  - "docs/architecture/specs/arch-spec-047-a4-intelligence-loop.md"
  - ".sddk/cycles/p-63676b11dc0ef88f-a4-5a-intelligence-loop-composition/spec.md"
superseded_by: []
---

# ADR-0126 — Intelligence Loop Composition Seam

> Status: **accepted** (A4-5a shipped v1.169.64; acceptance closed by A4-5C)
> Cycle: `p-63676b11dc0ef88f/a4-5a-intelligence-loop-composition`
> Spec:  `arch-spec-047` part-1
> Companion ADRs: ADR-0123 (Verify/DebVerify are distinct),
>                 ADR-0124 (Alignment is advisory),
>                 ADR-0125 (Generic Alignment Lens Kernel + Registry).

## Context

After A4-3/A4-4/A4-4MR, the SDDK engine has four independent authorities
producing independent outputs:

- `KnowledgeBasis` (semantic anchor; content-addressed)
- `ObservationSet` (semantic anchor; content-addressed)
- `VerifyKernel::evaluate(claim, observations, domain) -> VerificationResult`
  (paired with `VerificationClaim`)
- `DebVerifyKernel::reconcile(scope, baseline, evidence, strategies) ->
  ReconciliationSummary` (closed enum, 6 variants)
- `AlignmentLensKernel::evaluate(registry, input) -> KernelOutcome { Ok(
  LensEvaluation { contributions, gaps }) | Refused }` — preserves gaps
  per A4-4MR.
- `reduce_alignment(intent, basis, observations, contracts, decisions,
  time) -> AlignmentAssessment` — with `AlignmentAssessmentId` derived
  by `derive_id()`.

Each authority is **pure**, **independent**, and **content-addressed**.
There is, however, no single composition surface that ties them
together under one content-addressed identity.

## Decision

Introduce **one composition module** that:

1. Receives the four authority outputs verbatim. Does NOT call any
   authority itself. UAT exercises the real chain end-to-end and feeds
   the outputs in.
2. Produces two new public types:
   - `IntelligenceLoopResult` (EPHEMERAL) — side-by-side bundle, no
     derivation, no verdict.
   - `IntelligenceLoopReceipt` (PROJECTION) — content-addressed,
     order-independent, rebuildable.
3. Derives `IntelligenceLoopReceiptId` from:
   - `KnowledgeBasis::basis_hash()`.
   - `ObservationSet::canonical_digest()`.
   - Sorted pair-digests of `(VerificationClaim, VerificationResult)`.
   - `BaselineHash` from DebVerify.
   - Semantic digest of `ReconciliationSummary` (variant tag + inner
     canonical tags; NOT `serde` JSON, NOT `Display` text).
   - Sorted `LensContributionId`s.
   - Sorted `(NotEvaluatedReason, UniversalConcern)` gap tags.
   - `AlignmentAssessmentId`.
4. `evaluation_time` is carried in the receipt but is **not** part of
   identity (so two compositions at different times over the same
   semantic inputs produce the same id).

## Non-goals (hard)

- **NO** advisory (A4-5b).
- **NO** WHY/WHY-NOT (A4-5b).
- **NO** governance evaluation.
- **NO** `AuthorityDecision`, `Capability`, `InstructionSource`
  consumption. `MISALIGNED ≠ DENY` is a hard invariant.
- **NO** overall status / health / score / verdict / confidence /
  risk_score / pass / fail.
- **NO** mutation of any authority's types.
- **NO** call to the legacy `paradigm_lens::evaluate_lens()`
  compatibility facade. Production `AlignmentLensKernel` path only.

## Consequences

- Two new public types. No new error taxonomy.
- One new module: `crates/sddk-engine/src/intelligence_loop/`.
- One new public function: `compose_intelligence_loop`.
- One new public helper: `derive_receipt_id`.
- 16-pin corpus in `crates/sddk-engine/tests/a4_5a_intelligence_loop_composition.rs`.
- `arch-spec-047` `implemented_by` updated to `partial — A4-5a
  composition shipped; AdvisoryContext + WHY pending A4-5b; acceptance
  pending A4-5C`.

## Acceptance gates

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `cargo test --workspace` all green.
- 16-pin corpus all green (P1–P16).
- Real UAT through `VerifyKernel::evaluate`, `DebVerifyKernel`
  construction, production `AlignmentLensKernel`, and
  `reduce_alignment`.
- PublicReleaseGate PASS for the v1.169.64 release.

## See also

- `docs/architecture/specs/arch-spec-047-a4-intelligence-loop.md`
- `.sddk/cycles/p-63676b11dc0ef88f-a4-5a-intelligence-loop-composition/spec.md`
- `crates/sddk-engine/src/intelligence_loop/mod.rs`
- `crates/sddk-engine/tests/a4_5a_intelligence_loop_composition.rs`
- ADR-0123 (Verify/DebVerify are distinct)
- ADR-0124 (Alignment is advisory)
- ADR-0125 (Generic Alignment Lens Kernel + Registry)
