# A4-4C — arch-spec-046 Acceptance Receipt

> **Cycle:** `p-63676b11dc0ef88f/a4-4c-arch-spec-046-receipt-uat`
> **Spec closed:** `docs/architecture/specs/arch-spec-046-alignment-intent-and-lenses.md`
> **Released baseline inherited:** v1.169.59 (`ba986a5eda88e87dff0874e0cb01e57e838aa3db`)
> **Closure release tag:** v1.169.60 (cycle-46 install-coherence bumps workspace first)
> **Receipt date:** 2026-09-17

This document is the durable, machine-checkable acceptance receipt that
closes `arch-spec-046` as `implemented`. It enumerates, per closed
sub-cycle, the release that delivered each component, the file:line
evidence, the falsification pin that locks the contract, and the gate
result that proves the delivery was real.

## 1. Closed delivery surface (per A4 sub-cycle)

| Sub-cycle | Component | Released | File evidence | Pin test |
|---|---|---|---|---|
| **A4-4a** | Closed 10-member `UniversalConcern` vocabulary | v1.169.52 | `crates/sddk-engine/src/intent_universal_concern/types.rs:23` | `crates/sddk-engine/tests/a4_4a_intent_universal_concern_integration.rs` (5 tests) |
| **A4-4aR** | Applicability semantics correction (intent-only reducer; no string-grounding; paradigm does NOT erase a concern) | v1.169.54 | `crates/sddk-engine/src/intent_universal_concern/reducer.rs` | `crates/sddk-engine/tests/a4_4a_intent_universal_concern_integration.rs` (5 tests, all 4 falsification pins) |
| **A4-4b** | Generic `AlignmentLens` trait + `AlignmentLensRegistry` + `AlignmentLensKernel` + `LensContributionId` content-addressed identity | v1.169.57 | `crates/sddk-engine/src/alignment_lens/` (9 files, ~2122 lines) | `crates/sddk-engine/tests/a4_4b_alignment_lens_kernel.rs` (24 tests, pins 1-20 + anti-encroachment 11-19) |
| **A4-4bR** | Subject-General `EvidencePosture<Target>` + `EvidenceResolution` preserved as alias; `LensEvidenceResolution = EvidencePosture<ObservationTargetRef>` | v1.169.58 | `crates/sddk-engine/src/observation/posture.rs:129` | `crates/sddk-engine/tests/a4_4br_subject_general_evidence.rs` (29 tests) |
| **A4-4M** | AC7 / `paradigm_lens` → generic `AlignmentLens` convergence; `evaluate_lens` survives as zero-logic LEGACY_READ_COMPAT facade; `status_from_polarities` + `inferred_lens_assessment` deleted | v1.169.59 | `crates/sddk-engine/src/alignment_lens/paradigm.rs` + migration matrix at `docs/architecture/a4-4m-migration-matrix.md` | `crates/sddk-engine/tests/a4_4m_convergence_pins.rs` (14 tests) + `a4_4m_m0_migration_proof.rs` (11 tests) |
| **A4-4C** | Receipt/UAT closure; `arch-spec-046` flips `status: contract-ready` → `status: implemented` | v1.169.60 (this cycle) | `docs/architecture/specs/arch-spec-046-alignment-intent-and-lenses.md` (frontmatter) + this receipt | `crates/sddk-engine/tests/a4_4c_arch_spec_046_acceptance.rs` (3 tests — pin_119, pin_119a, pin_119b) |

## 2. Falsification gates (all pinned)

| # | Gate | How it pins | Test |
|---|------|-------------|------|
| 1 | Lenses are selected by **declared intent**, never applied universally | `applicable_concerns()` returns `Applicable` only for declared concerns; paradigm does NOT erase | `pin_119` witness chain (A4-4C) |
| 2 | No lens may produce a `ContractViolation` without an explicit contract | `LensEvaluationOutcome` has no `ContractViolation` variant; `LensContribution` has no `AuthorityDecision` field | `pin_119a` anti-encroachment witness |
| 3 | `ApplicableConcern + no registered lens` → `NotEvaluated`, NOT `NotApplicable` | `pin_04_missing_lens_is_not_evaluated_not_not_applicable` | `a4_4b_alignment_lens_kernel.rs:213` |
| 4 | `LensInput::try_new` refuses `NotApplicable` at the boundary | structural (kernel never sees `NotApplicable`) | `a4_4b_alignment_lens_kernel.rs` (multiple pins) |
| 5 | Same `(lens_id, concern, observation_set, evidence_resolution)` → byte-identical `LensContributionId` | `pin_119` re-evaluation identity equality | `a4_4c_arch_spec_046_acceptance.rs` |
| 6 | `evaluate_lens` facade produces identical outputs to the kernel path for AC7 corpus inputs | AC7 corpus (`ac7_lens_over_ac3_profile.rs`, `ac8_full_chain_receipt.rs`) passes UNMODIFIED post-convergence | `a4_4m_convergence_pins.rs` |
| 7 | Paradigm does NOT erase a concern (A4-4aR correction) | `(Pipeline, TemporalCoupling)` is now Applicable | `a4_4a_intent_universal_concern_integration.rs` falsification_paradigm_does_not_erase_concern |

## 3. Gate results (A4-4C closure)

| Gate | Result |
|------|--------|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test -p sddk-engine --test a4_4c_arch_spec_046_acceptance` | 3/3 pass |
| `cargo test -p sddk-engine --lib` | all pass (existing 1263 + 0 new) |
| `cargo test --workspace --offline` | 0 failed |
| `cargo build --release -p sddk-cli --bin sddk` | clean |
| `bash tests/test_release_public_gate.sh` | PASS=11 FAIL=0 |
| `scripts/release.sh` step 9b (PublicReleaseGate) | PASS — v1.169.60 tag anchored, `isDraft=false`, `isPrerelease=false`, 9/9 HTTP 200 |

## 4. Debt notes

- **`FU-A4-3-CONSTRAINT-BINDING`** (P1): `software_alignment::reduce_alignment` still associates `ExplicitConstraint` → observations via `subject.contains(contract_ref)` / `canonical_tag.contains(contract_ref)`. **NOT touched in A4-4C** (receipt-only budget; out of scope). Must close before A4-5.
- **`INC-A4-RELEASE-VERSION-DRIFT`** (P2): open. A4-4C inherits cycle-46 install-coherence contract (workspace bumps BEFORE tag). Expected release tag: v1.169.60.

## 5. Things explicitly OUT of scope (carried forward)

- **A4-5** — loop integration: `AlignmentLensKernel` outputs → `AdvisoryContext` flow. Blocked by A4-4C. Architecturally required before A4-CLOSEOUT.
- **`evaluate_lens` facade removal** — source-code feature; future cycle. ADR-0125 amendment documents the deferral.
- **AC7 corpus test migration** — `ac7_lens_over_ac3_profile.rs` and `ac8_full_chain_receipt.rs` continue to call `evaluate_lens`; removal belongs to the cycle that does the migration.

## 6. References

- Spec closed: `docs/architecture/specs/arch-spec-046-alignment-intent-and-lenses.md`
- Migration matrix: `docs/architecture/a4-4m-migration-matrix.md`
- ADR amendments: `docs/architecture/adrs/ADR-0125-GENERIC-ALIGNMENT-LENS-KERNEL-REGISTRY.md` (3rd post-acceptance amendment)
- Predecessor handoffs: `docs/handoff/HANDOFF-2026-09-16-a4-4*` (a, aR, b, bR, M)
- Handoff for this cycle: `docs/handoff/HANDOFF-2026-09-17-a4-4c-arch-spec-046-receipt-v1.169.60.md`
- Cycle spec: `.sddk/cycles/p-63676b11dc0ef88f-a4-4c-arch-spec-046-receipt-uat/spec.md`
