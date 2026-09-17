# ADR-0128 — Advisory Context + WHY Integration (absence ≠ negation)

> Status: **proposed** (this cycle promotes it to **accepted** after release)
> Cycle: `p-63676b11dc0ef88f/a4-5b-advisory-context-why-integration`
> Spec:  `arch-spec-047` part-2
> Companion ADRs: ADR-0126 (Intelligence Loop composition seam),
>                 ADR-0127 (VerifiedBy targets EvidenceRef),
>                 ADR-0121 (Architecture WHY traversal),
>                 ADR-0124 (Alignment is advisory).

## Context

After A4-5a, the four authority outputs are composed into one
`IntelligenceLoopResult` / `IntelligenceLoopReceipt`. The A3 closeout
already delivered an `AdvisoryContext` payload
(`sddk-domain/src/workflow_run.rs:710`) that is structurally incapable of
becoming authority. What was missing:

1. a deterministic projection from the loop output into that existing
   `AdvisoryContext`, without losing any authority output; and
2. a typed, provenance-backed answer to "why is this advisory item here?".

A3-S15 had already left an `architecture_why` module that explains
**contract/finding** provenance over the AC2 overlay. It is not a fit for
advisory items, which come from Verify / DebVerify / AlignmentLens /
Alignment outputs, not from contracts and findings.

A4-S15R (ADR-0127) had just given the graph two clean, separate
provenance axes (`SpecifiedBy → spec`, `VerifiedBy → evidence`). The
single most important rule this ADR must protect is **`absence !=
negation`**: a missing edge is information about the substrate, never a
negative claim about the world.

## Decision

### 1. Reuse the A3 `AdvisoryContext`; do not create a parallel model

`derive_advisory_context(&IntelligenceLoopResult) -> AdvisoryContext`
returns the A3 type itself. No `AdvisoryContext2`, no
`IntelligenceAdvisoryContext`. `AdvisoryKind` is extended with the
minimum vocabulary the loop needs:

- `VerificationOutcome` — one per `(VerificationClaim, VerificationResult)`.
- `DebtReconciliation` — one per `ReconciliationSummary`.
- `LensCoverageGap` — one per `NotEvaluated`.
- `AlignmentTension` — reserved at A3 closeout; now produced.

`ParadigmObservation` continues to cover lens contributions.

### 2. AdvisoryContext stays advisory

The derivation and the WHY are structurally incapable of granting or
removing a permission, changing a capability, producing an
`AuthorityDecision`, compiling instructions, or mutating Governance.
`AlignmentState::Misaligned` yields advisory content and an explanation —
NEVER `DENY`. Source-pinned.

### 3. A typed WHY for advisory items

`crates/sddk-engine/src/intelligence_advisory/` adds a small typed
surface (EPHEMERAL — no persistence, no identity):

- `AdvisorySubjectRef` — a typed selector into the loop result.
- `subject_key(&AdvisorySubjectRef) -> String` — the **single** render
  point; never parsed back.
- `AdvisoryWhy { subject, basis, legs, unresolved, why_not }`.
- `AdvisoryWhyBasis` — clock-stable (`receipt_id` + anchors; **no**
  `evaluation_time`).
- `AdvisoryWhyLeg` — the exact typed loop values (`Verification`,
  `Reconciliation`, `LensContribution`, `LensGap`, `Alignment`,
  `ContractProvenance`).
- `AdvisoryWhyNot` — only typed negative reasons
  (`VerificationNotVerified`, `ConcernNotEvaluated`, `AlignmentUnknown`).
- `AdvisoryWhyUnresolved` — the same shape as
  `architecture_why::WhyUnresolved`.

### 4. Two WHY surfaces, one philosophy

`architecture_why` explains contract/finding provenance over the AC2
overlay. `intelligence_advisory` explains advisory items over the
intelligence loop. These are **different subject domains**, not a
duplicate subsystem created for convenience: the advisory WHY reuses the
underlying typed reasons (`EvidenceGap`, `NotEvaluatedReason`,
`ReconciliationSummary`, `AlignmentState`) rather than re-encoding them,
and shares the `legs`/`unresolved`/`why_not` shape. Both obey the same
rule: explain an existing conclusion; never strengthen it; report absence.

### 5. `absence != negation` (the load-bearing rule)

- An empty `VerifiedBy` axis means "no evidence is linked on that axis".
  It is reported via `unresolved`, never turned into
  `Unknown`/`Contradicted`/`NotApplicable`/`Verified`.
- WHY-NOT fires only on a **typed** negative reason. A missing graph edge
  is not a negative reason.
- `SpecifiedBy` does not imply `VerifiedBy`.
- Epistemic distinctions (`NotApplicable` / `Unknown` / `NotEvaluated` /
  `EvidenceGap` / `Contradicted` / `Stale`) are never collapsed into one
  another by absence of information.

### 6. Provenance is read-only and typed

The AC2 overlay gains one read-only accessor,
`contract_provenance(contract_id) -> ContractProvenance`, exposing the
two A4-S15R axes as `specified_by` (spec payloads, as stored) and
`verified_by` (typed `EvidenceRef`, reconstructed losslessly from the
props A4-S15R stashed). Graph traversal is owned by the overlay; the
advisory module consumes typed values. No new relation is added. A closed
tag codec, `EvidenceKind::from_domain_tag`, is added (fail-closed:
unknown tag → `None`, never a default).

### 7. No orchestration

The seam receives the loop result; it never calls `VerifyKernel`,
`DebVerifyKernel`, `AlignmentLensKernel`, `reduce_alignment`, the legacy
`paradigm_lens` evaluator, or a provider. Source-pinned in the corpus
(comments stripped, code only).

### 8. No global verdict

No `status` / `health` / `score` / `quality_score` / `confidence` /
`risk_score` / `verdict` / `pass` / `fail` / `ready` field on any new
surface. Pinned by a debug-repr check.

## Consequences

Positive:

- The loop's conclusions are now deliverable as the A3 advisory payload,
  and every advisory item is explainable by typed provenance.
- `absence != negation` is preserved end-to-end from A4-S15R into the
  advisory surface.
- The two WHY surfaces stay independent but philosophically identical.

Neutral:

- `AdvisoryKind::ALL` grew from 3 to 6. No existing pin referenced the
  count.
- `ArchitectureGraphOverlay` gained a read-only accessor and `EvidenceKind`
  gained a codec.

Negative:

- None within the budget. A4-5C owns formal acceptance.

## Compliance evidence

- `crates/sddk-engine/src/intelligence_advisory/mod.rs` — the seam.
- `crates/sddk-domain/src/workflow_run.rs` — `AdvisoryKind` extension.
- `crates/sddk-engine/src/architecture_graph/overlay.rs` —
  `contract_provenance` + `evidence_from_node`.
- `crates/sddk-engine/src/evidence_ref.rs` — `EvidenceKind::from_domain_tag`.
- `crates/sddk-engine/src/architecture_why/types.rs` — corrected stale
  "no evidence node kind" claim.
- `crates/sddk-engine/tests/a4_5b_advisory_context_why.rs` — 20 pins
  (P1–P18 + 2 real end-to-end UAT).

## Cycle closure

A4-5C remains the acceptance cycle and is NOT auto-opened. `FU-A3-S15-1`
(`evidence → observes → software_relation`) is a different relation and
stays open.
