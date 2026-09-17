---
id: ADR-0125-GENERIC-ALIGNMENT-LENS-KERNEL-REGISTRY
status: accepted
supersedes_history: false
proposed_at: 2026-09-16
accepted_at: 2026-09-16
accepted_by_cycle: p-63676b11dc0ef88f/a4-4b-alignment-lens-kernel
implementation_evidence:
  - "crates/sddk-engine/src/alignment_lens/mod.rs (public surface + anti-encroachment comments)"
  - "crates/sddk-engine/src/alignment_lens/types.rs (LensId, LensVersion, InsufficientGap, LensDescriptor, LensInput)"
  - "crates/sddk-engine/src/alignment_lens/id.rs (LensContributionId content-addressed)"
  - "crates/sddk-engine/src/alignment_lens/lens.rs (AlignmentLens trait, object-safe, no defaults)"
  - "crates/sddk-engine/src/alignment_lens/registry.rs (AlignmentLensRegistry + secondary by_concern index)"
  - "crates/sddk-engine/src/alignment_lens/error.rs (LensError + KernelError)"
  - "crates/sddk-engine/src/alignment_lens/kernel.rs (AlignmentLensKernel total function; KernelOutcome)"
  - "crates/sddk-engine/src/alignment_lens/contribution.rs (LensContribution — no score/confidence)"
  - "crates/sddk-engine/src/alignment_lens/not_evaluated.rs (NotEvaluated typed gap)"
  - "crates/sddk-engine/tests/a4_4b_alignment_lens_kernel.rs (24 pin tests; integration)"
  - "crates/sddk-engine/tests/alignment_lens_fixture.rs (two heterogeneous reference lenses under tests/ root, NOT exported)"
  - "this ADR is the contract of record for the lens kernel (no separate arch-spec-048 was created; the dangling reference was removed at A5-1)"
  - ".sddk/cycles/p-63676b11dc0ef88f-a4-4b-alignment-lens-kernel/spec.md (scope contract, ABSTRACTION/KERNEL ONLY)"
superseded_by: []
related_adrs:
  - "ADR-0118-PARADIGM-LENS-EVALUATION"
  - "ADR-0122-EVIDENCE-OBSERVES-SOFTWARE"
  - "ADR-0124-ALIGNMENT-IS-ADVISORY"
stale_after: 2027-03-16
---

# ADR-0125 — Generic AlignmentLens kernel + registry

## Context

`paradigm_lens/` was introduced by **A3-S8 (AC7)** — the alignment
paradigm probes (OO/FP/ADT/DSL) — as the historical per-paradigm lens
implementation. It works, but its surface is paradigm-specific and its
contribution identity is not content-addressed.

ADR-0124 made alignment advisory rather than normative. ADR-0122 makes
evidence observe software rather than adjudicate it. The next layer must
be a **generic, paradigm-agnostic** kernel that any lens — present-day
paradigm lens (AC7), future built-in lens, pack-supplied lens,
provider-backed observation lens — can plug into, with
**content-addressed contribution identity** so that the same lens, the
same concern, the same observation set, and the same evidence
resolution always yield the same id (regardless
of wall clock, message text, registration order, or producer labels).

The historical `software_alignment::reduce_alignment` reducer
(A4-3 closed) computed a 7-state `AlignmentState` with three finding
kinds. That reducer is **unchanged** by this ADR; the new kernel
sits beside it, not inside it.

## Decision

### 1. One new top-level context module: `alignment_lens/`

`crates/sddk-engine/src/alignment_lens/` is a fresh context module with
its own public surface. It does **not** depend on `paradigm_lens/`, on
`software_alignment::reduce_alignment`, on the CLI, on the provider SDK,
on the authority engine, on the instruction compiler, or on any
production concrete lens implementation.

### 2. Vocabulary is strictly typed; no parallel terms

Five epistemic states are kept distinct at the type level:

- `NotApplicable` — the concern does not apply to the unit under
  evaluation. Set by `Intent::applicable_concerns()` reducer, not by the
  kernel.
- `NotEvaluated` — concern applies, but no lens registered (or all
  registered lenses refused). Carries `NotEvaluatedReason`.
- `Unknown` / `Ungrounded` — observation-layer states; not produced by
  the lens kernel.
- `InsufficientEvidence` — concern applies, lens ran, evidence was
  insufficient; `EvidenceResolution::Insufficient` carries
  `InsufficientGap` (a typed enum), never a free-form string.
- `EvidenceResolution::{Supported, Contradicted, Conflicted}` — lens
  output for a single relation.

`Missing lens != NotApplicable`. `Insufficient evidence != NotApplicable`.
`Conflicted evidence is preserved as Conflicted` (both supporting and
contradicting observations carried).

### 3. Lens surface: `AlignmentLens` trait (object-safe)

We chose a trait over an ADT + dispatch because:

- Future built-in lenses (kernel-supplied), pack lenses, and
  provider-backed observation lenses must all be registerable.
- The trait returns `LensEvaluationOutcome::{Contribution, Refused}`
  with **no default methods**, so the type system prevents accidental
  identity drift via `Default`-style fallbacks.

The trait surface does **not** carry:

- `score: f64` — no numeric score.
- `confidence: f64` — no confidence.
- `alignment_state: AlignmentState` — no re-introduction of A4-3 vocabulary.
- `capability: Capability` — no authority crossing.
- `authority_decision: AuthorityDecision` — same.
- `instruction_source: InstructionSource` — same.

These omissions are **type-locked**: the trait's return type is
`LensEvaluationOutcome`, and `LensContribution` has no field for any of
them. Anti-encroachment pin tests grep the module for forbidden
identifiers.

### 4. Registry: `AlignmentLensRegistry`

- `BTreeMap<LensId, Arc<dyn AlignmentLens>>` for primary lookup.
- `BTreeMap<UniversalConcern, BTreeSet<LensId>>` as secondary index.
- `register` returns `Result<&Self, LensError>`. Outcomes:
  - First registration: success.
  - Same `LensId` + same `LensVersion` + same `supported_concerns`:
    idempotent success (no kernel churn).
  - Same `LensId` + same `LensVersion` + different
    `supported_concerns`: `InconsistentSupportedConcerns` typed refusal.
  - Same `LensId` + different `LensVersion`: `InconsistentLensVersion`
    typed refusal.
  - Different `LensId` under concern already taken: success (multiple
    lenses per concern are allowed).

This is **deliberately different** from A4-2's `ChallengeStrategySet::register`,
which silently replaced by id. The kernel's contract is "refuse
ambiguity, never replace".

### 5. Identity: `LensContributionId` content-addressed

`LensContributionId` is a 32-byte sha256 over:

```
sha256(
  domain              ||        # `&'static str` lens_id.as_str()
  lens_id             ||        # duplicate; pinned for clarity
  lens_version        ||        # major.minor
  concern             ||        # UniversalConcern discriminant
  observation_set_tag ||        # canonical tag of the input ObservationSet
  evidence_resolution ||        # EvidenceResolution::canonical_tag()
  sorted_ref_keys             # sorted-by-ordering-key evidence_refs[]
)
```

Explicit exclusions:

- **Wall clock** — `Instant::now()`, `SystemTime`, `Utc::now()`,
  `chrono::Utc` all banned.
- **Message text** — `&str` / `String` payload fields banned.
- **Registration order** — `Vec` insertion order banned.
- **Producer labels** — `producer: &str`, `actor: &str` banned.
- **Severity / score / confidence** — banned.
- **Rendered text** — banned.

`Insufficient { gap: String }` is the **only** free-form field. To
prevent it leaking into identity, the id computation uses
`InsufficientGap` (a typed enum) **only**, never the raw string.
This is pinned by `id::tests::insufficient_gap_string_does_not_reach_identity`.

### 6. Kernel: `AlignmentLensKernel::evaluate`

A **total function** `(&registry, &input) -> KernelOutcome`.

- Empty registry → `NotEvaluated { reason: NoRegisteredLens }`.
- Applicable concern with lens but ALL lenses refused →
  `NotEvaluated { reason: LensExistsButRefused }`.
- At least one lens contributes → `Ok(LensEvaluation { contributions })`.
- Determinism probe: duplicate `LensContributionId` in the output →
  `KernelError::DeterminismProbeFailed`.

The kernel **does not** call `software_alignment::reduce_alignment`.
Pin `pin_a2_does_not_call_reduce_alignment_textual_probe` greps the
module's source files for that identifier outside doc-comments.

### 7. `NotApplicable` input is refused at the boundary

`LensInput::try_new(concern, unit, observations)` refuses
`NotApplicable` concerns at construction time. The kernel never sees a
`NotApplicable` input; that state is the upstream reducer's responsibility.

## Consequences

Positive:

- A future cycle (A4-4M) can migrate `paradigm_lens/` to register under
  the new kernel without changing `reduce_alignment`.
- Provider-backed observations can register a lens for a new concern
  without forking the registry.
- `LensContributionId` is a stable, deterministic receipt that
  reconciliation and acceptance engines can compare.
- Anti-encroachment is **structurally pinned**: no field of the right
  shape exists for score / confidence / AlignmentState / Capability /
  AuthorityDecision / InstructionSource.

Negative:

- `paradigm_lens/` and `alignment_lens/` now coexist as parallel surfaces
  until A4-4M migrates. This is the strangler pattern: the old reducer
  remains the **runtime** authority for AC7 until the migration closes,
  with the kernel providing a side-by-side reception path.

## Alternatives considered

- **ADT + dispatch** for `AlignmentLens`: rejected. We need to accept
  pack-supplied lenses at runtime; a closed ADT would force a fork on
  every new lens.
- **Replace-on-duplicate** in registry: rejected. Replace semantics hide
  contract drift; the kernel's job is to refuse ambiguity, not smooth
  it over.
- **Identity includes wall clock** (e.g. for ordering): rejected.
  Identity must be a content receipt. Ordering is a function of sorted
  keys + registry membership, both deterministic.
- **Replace `EvidenceResolution::Insufficient { gap: String }`** with a
  free-form note: rejected. The kernel locks the gap behind
  `InsufficientGap` so the identity can hash it without leaking
  free-form text.

## Migration notes

- AC7 (A3-S8) historically produces `LensAssessment` carrying a
  `LensStatus` (Aligned / Misaligned / Tension / Unknown). `AlignmentState`
  is A4-3's vocabulary, not AC7's. A4-4M migrates `paradigm_lens/` so
  that the historical `LensAssessment + LensStatus` output is replaced
  by `LensContribution[]` carrying the same four-variant epistemic
  shape (Supported/Contradicted/Conflicted/Insufficient) via the
  subject-general `LensEvidenceResolution` introduced in A4-4bR. The
  legacy mapping would be approximately:
  `Aligned → Supported`, `Misaligned → Contradicted`,
  `Tension → Conflicted`, `Unknown → Insufficient`. `NotApplicable`
  remains an intent-layer concern and is resolved **before** the
  kernel. Until A4-4M closes, `paradigm_lens/` remains the runtime
  authority for AC7.
- The two reference lenses under
  `crates/sddk-engine/tests/alignment_lens_fixture.rs` are **test-only**,
  not exported in the production API. They exist to exercise the
  kernel's heterogeneity contract and to pin behavior under
  `observation_set_canonical_tag` / real `ObservationSubject`s
  (relations AND units — see A4-4bR).

## Post-acceptance corrections

- **2026-09-16 (A4-4bR preflight):** The "Context" section originally
  attributed `paradigm_lens/` to A4-2. Corrected to **A3-S8 (AC7)**.
  The "Migration notes" section originally said the legacy output was
  `AlignmentState`; that is A4-3's vocabulary, not AC7's. Corrected
  to `LensAssessment + LensStatus` (Aligned / Misaligned / Tension /
  Unknown) with the equivalence mapping to the four-variant
  `EvidencePosture` shape that A4-4bR formalizes. Status remains
  `accepted`; these are factual corrections, not a decision reversal.

## Post-acceptance amendment: A4-4bR — subject-general evidence target

- **2026-09-16 (A4-4bR, cycle
  `p-63676b11dc0ef88f/a4-4br-subject-general-evidence`):** The A4-4M
  preflight revealed that reusing relation-only `EvidenceResolution`
  inside the lens kernel forced unit-target observations into fabricated
  self-relations (`unit A --depends_on--> unit A`) merely to satisfy the
  resolution shape. A4-4bR generalizes the **same algebra** by
  parameterizing it on the observed target:
  `EvidencePosture<Target>` with the unchanged four variants
  (Supported / Contradicted / Conflicted / Insufficient), where
  `EvidenceResolution := EvidencePosture<RelationId>` (type alias —
  `resolve_relation` and every A4-0/A4-2 consumer unchanged, equivalence
  pinned by tests) and
  `LensEvidenceResolution := EvidencePosture<ObservationTargetRef>` with
  `ObservationTargetRef ∈ {Relation, Unit, Contract, Knowledge}`.
  `LensContribution` and `LensContributionId` migrate to the
  subject-general shape; **target kind enters identity**
  (`Unit(foo) ≠ Relation(foo→foo)`, pinned). Lenses consume real
  targets from the `ObservationSet`; synthetic `RelationId` derivation
  from `(intent_id, concern, unit_ref)` is forbidden (doc + grep pins).
  No second epistemic taxonomy was introduced. Status remains
  `accepted`; this amendment records the generalization, not a reversal.

## Post-acceptance amendment: A4-4M — AC7 convergence executed

- **2026-09-16 (A4-4M, cycle
  `p-63676b11dc0ef88f/a4-4m-ac7-alignmentlens-convergence`):** The AC7
  migration referenced above landed behind an **M0 Migration Proof**
  gate (user-mandated): a pinned migration matrix
  (`docs/architecture/a4-4m-migration-matrix.md`) proving the typed,
  lossless `LensObservation → SoftwareObservation` translation before
  any production edit. Outcome: four production `ParadigmLens` family
  lenses (OO/FP/ADT/DSL) now implement the single `AlignmentLens`
  trait and compose via `AlignmentLensRegistry`; AC7 probes feed the
  canonical `ObservationSet` through `paradigm_lens::translation`;
  `evaluate_lens` survives only as a zero-logic compatibility facade
  (projection posture→`LensStatus`, removal trigger = A4-4C);
  `status_from_polarities` and `inferred_lens_assessment` are deleted —
  there is exactly one execution authority. A4-5 encroachment remains
  pinned out: the kernel still produces only `LensContribution[]`.

## Post-acceptance amendment: A4-4C — receipt/UAT closure (this ADR remains accepted)

- **2026-09-17 (A4-4C, cycle
  `p-63676b11dc0ef88f/a4-4c-arch-spec-046-receipt-uat`):** The
  arch-spec-046 milestone closed as `implemented` (v1.169.60) on
  receipt/UAT grounds. The 119-pin test base
  (`a4_4a_intent_universal_concern_integration`,
  `a4_4b_alignment_lens_kernel`, `a4_4br_subject_general_evidence`,
  `a4_4m_convergence_pins`, `a4_4m_m0_migration_proof`, `ac7_lens_over_ac3_profile`,
  `ac8_full_chain_receipt`, `alignment_lens_fixture`, plus the new
  `a4_4c_arch_spec_046_acceptance`) is the witness set. **A4-4C did
  NOT remove the `evaluate_lens` compatibility facade** — its scope
  is receipt/UAT only, not source-code migration. The facade's
  doc-comment in `crates/sddk-engine/src/paradigm_lens/lenses.rs:34`
  now names the AC7 corpus tests (`ac7_lens_over_ac3_profile.rs` and
  `ac8_full_chain_receipt.rs`) as the explicit migration predicate.
  Removal of the facade (and migration of those two test files to
  call `AlignmentLensKernel` directly) is owned by a future cycle.
  ADR status remains `accepted`; this amendment records the deferral,
  not a reversal of the kernel contract.
