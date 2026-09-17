# FU-A4-4M-CONCERN-PRESERVATION

> **Status:** CLOSED (closed by A4-4MR release v1.169.63 on 2026-09-17)
> **Severity:** P1
> **Origin:** A4-5P preflight §4 production-lens audit (cycle `p-63676b11dc0ef88f/a4-5p-intelligence-loop-entry-gate`) — confirmed independently during A4-4MR scoping.
> **Blocks:** A4-5a (Intelligence Loop composition) — STRUCTURALLY UNBLOCKED.
> **Disposes:** nothing — this is a closed follow-up

## 0. Closure evidence

- Release: **v1.169.63** (release tag `v1.169.63` → SHA
  `6f48e1cb6a2eb1f740e007c1bb99a327b38e7143`).
- Cycle: `p-63676b11dc0ef88f-a4-4mr-concern-preserving`.
- Scope contract: `.sddk/cycles/p-63676b11dc0ef88f-a4-4mr-concern-preserving/spec.md`.
- Handoff: `docs/handoff/HANDOFF-2026-09-17-a4-4mr-concern-preserving.md`.
- PublicReleaseGate: PASS (scenario 9 against the real release at the GH
  API).
- `cargo test --workspace` → **4586 passed, 0 failed**; `cargo fmt
  --check` clean; `cargo clippy --workspace --all-targets -- -D
  warnings` clean.
- Falsification corpus:
  `crates/sddk-engine/tests/a4_4mr_concern_preserving.rs` (16 pins
  across 7 families; verified to fail 13/16 against the pre-fix code
  and pass 16/16 against the post-fix code).
- A4-4M / A4-4b / A4-4bR / A4-4C / A4-3R / A4-3R2 regression: all
  green. The legacy `paradigm_lens::evaluate_lens()` facade is
  unchanged and its tests still pass.
- Production `ParadigmLens::evaluate(input)` now returns one
  `Contribution(c)` with `c == input.concern()` whenever the lens
  declares support for `input.concern()`; otherwise it returns
  `Refused(LensRejected { id, concern: input.concern() })`.
- Legacy facade remains `LEGACY_READ_COMPAT` and is documented as
  such (production-vs-facade boundary section in
  `crates/sddk-engine/src/paradigm_lens/lenses.rs` module header).

## 0. Summary

`crates/sddk-engine/src/alignment_lens/paradigm.rs:209-227` — production
`ParadigmLens::evaluate(&LensInput)` iterates `self.concerns`, overwrites
`out` on every iteration, and returns only the **last** contribution. As a
result the contribution's `concern` field carries the LAST concern
declared by the lens (the last element of the static `concerns` slice),
NOT the concern the caller asked for in `LensInput::applicable`. The
`LensInput.concern()` field is ignored entirely.

Concretely, given the production `ParadigmLens::ALL` array:

- `ParadigmLens::object_oriented` declares
  `[BoundaryIntegrity, StateSafety, DependencyDirection, SemanticOwnership,
    Cohesion, Coupling]`. Any `LensInput` with `applicable = Applicable(c)`
  for `c ∈ {BoundaryIntegrity, StateSafety, DependencyDirection,
    SemanticOwnership, Cohesion}` returns a contribution tagged
  `concern = Coupling`. The caller's request is silently substituted.
- `ParadigmLens::functional` declares
  `[StateSafety, EffectVisibility, SemanticOwnership]`. Any input concern
  returns `SemanticOwnership`.
- `ParadigmLens::adt` declares `[StateSafety, SemanticOwnership]`. Any
  input concern returns `SemanticOwnership`.
- `ParadigmLens::dsl` declares
  `[BoundaryIntegrity, EffectVisibility, StateSafety, TemporalCoupling]`.
  Any input concern returns `TemporalCoupling`.

The same bug affects the contribution identity: `derive_contribution_id`
hashes the (incorrect) concern into the content address, so two
invocations of the same lens with the same observation set but different
`input.applicable` collide on `LensContributionId` if their inputs happen
to land on the same lens's terminal concern. A4-4b's determinism probe
(`kernel.rs:187-196`) does NOT catch this — it only checks for duplicates
**within one** `LensEvaluation`, not across concerns.

## 1. Why this is a kernel-contract violation (not a cosmetic bug)

`LensInput::concern()` exists for the kernel to tell a lens what concern
to evaluate. The kernel already looks up lenses via
`AlignmentLensRegistry::for_concern(concern)` and dispatches ONLY
matching lenses. Inside the lens, the contract is:

> `LensContribution.concern` equals `LensInput.concern()` whenever the
> lens declares support for `LensInput.concern()`. If the lens does not
> declare support for `LensInput.concern()`, it returns
> `LensEvaluationOutcome::Refused(LensError::LensRejected { id, concern })`.

This contract is the bridge between the intent layer's
`ApplicableConcern` (a request) and the lens's epistemic answer. The
intent layer asked: "evaluate DependencyDirection". The lens responded
with: "Coupling". Downstream consumers (A4-5a Intelligence Loop, future
`AlignmentAssessment` derivation) would silently see a different concern
than they asked for. `MISALIGNED ≠ DENY` is fine; `request_X ≠ answer_X`
is not.

The contract is documented in:

- `alignment_lens/types.rs:196-218` — `LensInput::concern()` exists
  specifically so lenses know what to evaluate.
- `alignment_lens/kernel.rs:14-19` — kernel dispatches lenses that
  declared support for `input.concern()` and "Each lens returns
  Contribution(c) or Refused(e)".
- `alignment_lens/lens.rs:78-84` — `evaluate(&self, &LensInput)` is
  documented as pure on the same input; the `LensInput` IS the lens's
  evaluation contract.
- A4-4bR §1 (post-acceptance amendment): the `LensInput.concern()`
  pin is the load-bearing invariant for the generic kernel.

## 2. Why A4-4M did not catch this

A4-4M (`p-63676b11dc0ef88f/a4-4m-ac7-alignmentlens-convergence`) shipped
the four production lenses with the following corpus
(`crates/sddk-engine/tests/a4_4m_convergence_pins.rs`):

- **M3** (`m3_four_lenses_compose_via_registry`): only checks registry
  composition (`reg.len() == 4`, lens ids prefix). Does NOT exercise
  `AlignmentLensKernel::evaluate`.
- **M5** (textual probes): checks removal of legacy motors.
- **M7** (corpus equivalence): calls `evaluate_lens(...)` (the LEGACY
  compatibility facade) and compares its output against the substrate
  posture projection. The facade does NOT go through
  `AlignmentLensKernel` — it calls `observation::resolve_subject` directly
  and projects. The bug is invisible to the facade because the facade
  never instantiates a `ParadigmLens` via the kernel.
- **M9 / M10**: provenance / inferred-path deletion probes.

Net: no pin in A4-4M invokes the four production `ParadigmLens`
implementations through `AlignmentLensKernel::evaluate`. The bug was
unobservable by the existing corpus.

## 3. Scope of the fix (A4-4MR)

Single change budget: **PRESERVE INPUT CONCERN THROUGH PRODUCTION LENS.**

`ParadigmLens::evaluate(&LensInput)` is rewritten so that:

1. `let concern = input.concern();` is the ONLY concern the lens
   considers.
2. If `!self.concerns.contains(&concern)`, the lens returns
   `LensEvaluationOutcome::Refused(LensError::LensRejected { id: self.id,
   concern })`.
3. Otherwise, exactly ONE `LensContribution` is assembled with
   `concern = concern` (the requested one) and `id =
   derive_contribution_id(self.id, versions::V1, concern, set_digest,
   resolution, evidence_refs)` — both inputs to the hash match the
   request.
4. The legacy `paradigm_lens::evaluate_lens()` compatibility facade is
   UNCHANGED. It already projects directly from the substrate posture
   (not through `AlignmentLensKernel`), so the bug never reached it.
   This is documented as the **single execution authority** boundary:
   the generic kernel is the only path A4-5a must consume.

## 4. Exit gate (A4-4MR release blocker)

- Production `AlignmentLens::evaluate(input)` returns `Contribution(c)`
  with `c == input.concern()` whenever the lens declares support for
  `input.concern()`.
- Production `AlignmentLens::evaluate(input)` returns
  `Refused(LensRejected { id, concern: input.concern() })` whenever it
  does not.
- Exhaustive corpus: for each `l ∈ ParadigmLens::ALL` and each `c ∈
  l.supported_concerns`, invoking `AlignmentLensKernel::evaluate` with
  `LensInput { applicable: Applicable(c), ... }` returns a single
  contribution whose `concern == c` and whose `id` derives from `c`.
- For each `l ∈ ParadigmLens::ALL` and each `c ∈ UniversalConcern::ALL`
  with `c ∉ l.supported_concerns`, invoking the kernel with
  `applicable = Applicable(c)` returns `NotEvaluated { reason:
  LensExistsButRefused }` (kernel aggregate) — the refusal is silent at
  the per-lens level.
- Multi-lens invariant: same concern + multiple matching lenses →
  one contribution per successful lens, each preserving the concern.
- A4-4b / A4-4bR / A4-4M / A4-4C / A4-3R / A4-3R2 regression: all green.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
  warnings`, `cargo test --workspace` all clean.
- PublicReleaseGate scenario 9 PASS against the new release.

## 5. After closure

A4-5a (Intelligence Loop composition) becomes structurally unblocked
again. The downstream contract is: every contribution coming out of
`AlignmentLensKernel::evaluate` carries the concern that was asked for.
The kernel does not pick; the lens does not pick; the request determines
the answer's `concern` field.

## 6. References

- `crates/sddk-engine/src/alignment_lens/paradigm.rs:150-237` — the
  production lens implementation that contains the bug.
- `crates/sddk-engine/src/alignment_lens/id.rs:87-101` —
  `derive_contribution_id` hashes the (incorrect) concern into the
  identity, so the bug also destabilises the contribution id.
- `crates/sddk-engine/src/alignment_lens/kernel.rs:143-214` — the
  kernel that consumes `LensContribution`s; not the bug site, but its
  determinism probe does NOT cover cross-concern identity collisions.
- `crates/sddk-engine/src/alignment_lens/types.rs:196-251` —
  `LensInput` definition, `concern()` accessor, and
  `try_new` (applicability gate).
- `crates/sddk-engine/src/alignment_lens/error.rs:54-57` —
  `LensError::LensRejected { id, concern }`, the typed refusal for
  unsupported concerns.
- `crates/sddk-engine/src/paradigm_lens/lenses.rs:11-42` —
  compatibility facade header documenting `LEGACY_READ_COMPAT` and the
  A4-4C deferral.
- `crates/sddk-engine/tests/a4_4b_alignment_lens_kernel.rs` — kernel
  pin corpus; no existing pin asserts `c.concern == input.concern()`.
- `crates/sddk-engine/tests/a4_4m_convergence_pins.rs` — M3/M5/M7/M9/M10
  pins; explains why A4-4M did not catch the bug.
- `.sddk/followups/a4-followups.md` — registered as `A4-4MR` row.
- `docs/architecture/a4-4m-migration-matrix.md` — referenced by the
  bug site; will receive a §M11 entry recording the corrective slice.
