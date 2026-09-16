# HANDOFF — A4-4a Intent + UniversalConcern Foundation shipped as v1.169.52

**Date:** 2026-09-16
**Cycle:** `p-63676b11dc0ef88f/a4-4a-intent-universal-concern-foundation`
**Tag:** `v1.169.52` (published at `2026-09-16T14:19:12Z`)
**HEAD:** `chore(release)` post-bump commit (matches `origin/main` and the GH release tag). The exact SHA is whatever `git rev-parse origin/main` returns at the moment of reading this handoff; the tag `v1.169.52` tracks `main` and is the durable reference.
**Previous released:** `v1.169.50` (A4-3 Software Alignment Core).

## What shipped

A4-4a ships the model surface for the Intent + UniversalConcern kernel
(per `arch-spec-046` part-1). It is the *descriptive* layer that
upcoming cycles (A4-4b generic AlignmentLens, A4-4M paradigm_lens
convergence, A4-5 loop integration) will consume.

**New module:** `crates/sddk-engine/src/intent_universal_concern/` (4 files).

- `types.rs` — closed vocabularies + typed identity (~271 lines).
- `reducer.rs` — pure `applicable_concerns()` + `ReductionError` (~166 lines).
- `tests.rs` — 18 tests (~384 lines).
- `mod.rs` — public surface (11 exports, no implicit leaks).

## Closed UniversalConcern vocabulary (10)

```
Cohesion, Coupling, BoundaryIntegrity, StateSafety, EffectVisibility,
DependencyDirection, SemanticOwnership, TemporalCoupling, Testability, Freshness
```

## Intent representation (typed, not stringly)

- `ProjectIntent { intent_id, paradigm: ParadigmProfileRef, declared_concerns: BTreeSet<UniversalConcern> }`
- `UnitIntent { unit_ref: SoftwareUnitRef, applies_to_concerns: BTreeSet<UniversalConcern> }`
- `IntentId(String)` — content-addressed identity (opaque to consumers).
- `ParadigmProfileRef` — closed 6-variant enum (OO/FP/FPure/Data/Pipeline/Custom).
- `DecisionRefs(Vec<DecisionRef>)` / `ContractRefs(Vec<ContractId>)` — typed accessors.

## Reducer precedence (descriptive, no authority)

For each `UniversalConcern` in the closed 10-member vocabulary:

1. If `project_intent.declared_concerns` does NOT contain the concern →
   `NotApplicable(c, NotInProjectIntent)`.
2. Else if `unit_intent.applies_to_concerns` does NOT contain the concern →
   `NotApplicable(c, NotInUnitIntent)`.
3. Else if the paradigm is irrelevant (Pipeline × TemporalCoupling) →
   `NotApplicable(c, ParadigmIrrelevant)`.
4. Else if no decision grounds the concern →
   `NotApplicable(c, NoGroundingDecision)`.
5. Else if no contract references the concern → branch 5b.
6. Else → `Applicable(c)` with one of three `ApplicableReason`:
   - `ProjectAndUnitIntentAndDecision` (preferred)
   - `ProjectIntentAndParadigm` (no decision, paradigm matches)
   - `UnitIntentAndContract` (unit intent + contract, no project-level — safety net)

Output is sorted by `ApplicableConcern::canonical()`. No wall clock.
No global state. No IO. No label text. Pure function of inputs.

## Architectural pins closed (verifiable)

| Pin | Test |
|---|---|
| Descriptive, not prescriptive | `a4_4a_universal_concern_is_descriptive_not_prescriptive` (asserts no `capability`/`deny`/`DENY` in serialized output) |
| No authority / capability / lens in reducer signature | `a4_4a_does_not_import_authority_or_capability_or_lens_into_public_api`, `a4_4a_no_capability_or_authority_in_reducer_signature` |
| No `AlignmentLens` symbol leaked | `a4_4a_no_alignment_lens_symbol_leaked` |
| No concrete lens strategy | `a4_4a_no_concrete_lens_strategy_leaked` |
| No `software_alignment` mutation | `a4_4a_does_not_change_software_alignment_public_api` |
| No `paradigm_lens` registry mutation | `a4_4a_does_not_mutate_paradigm_lens_registry` |
| Pure / deterministic | `identical_inputs_produce_identical_outputs`, `ordering_is_deterministic_by_canonical`, `canonical_tags_are_stable_strings` |
| Closed vocabularies | `universal_concern_all_has_exactly_ten`, `paradigm_profile_ref_covers_six_variants` |
| Refusal paths | `empty_project_intent_with_no_declared_concerns_is_refused`, `empty_unit_ref_is_refused` |
| UAT pinned | `uat_same_software_different_intent_different_applicable_concerns` |

## Identity spec (no hash yet — A4-4a does not mint ids)

A4-4a is *descriptive* and does not mint content-addressed identifiers.
`IntentId` is opaque to consumers; `ApplicableConcern::canonical()`
provides a stable string for sorting and display. Identity is built
upstream by A4-4b/4M when the lens kernel consumes these answers.

## The UAT (pin)

> Same software evidence + different declared intent = different
> applicable concerns possible.

Test: `uat_same_software_different_intent_different_applicable_concerns`.
Same unit, same contracts, same decisions; project intent A declares
all 10 concerns (10 applicable); project intent B declares 3 (3
applicable). The two outputs differ in *which* concerns are applicable.

## Drift state

| Source | Value |
|---|---|
| `released_baseline` | v1.169.50 → **v1.169.52** |
| `development_head` | 1672b1e → **post-bump chore(release)** (matches `origin/main` and the GH release tag `v1.169.52`; use `git rev-parse origin/main` for the live SHA) |
| `workspace_version` | 1.169.51 → **1.169.52** (matches bump) |
| `cycle_id` | `p-63676b11dc0ef88f/a4-4a-intent-universal-concern-foundation` |
| `predecessor_cycle` | `p-63676b11dc0ef88f/a4-3-software-alignment-core` |
| `successor_blocked_by` | `p-63676b11dc0ef88f/a4-4b-generic-alignment-lens-kernel-registry` |

## ROADMAP delta (delta-only)

**Before A4-4a:** A4-4a was `current` in the README A4 table;
`arch-spec-046` part-1 surfaces were pending (A4-5 dependency).

**After A4-4a:**
- `arch-spec-046` part-1 (closed vocabularies + typed intent + pure
  reducer) → **`implemented`** as a model surface (kernel).
- The `contract-ready → implemented` promotion of the **spec** is
  deferred to A4-CLOSEOUT per arch-spec-046 §6, because the kernel is
  only complete when the loop integration (A4-5) closes.
- README A4 table: A4-4a moves from `current` to `closed`.
- A4-4b, A4-4M, A4-4C remain blocked_by their own scope contracts;
  A4-5 remains blocked_by A4-4; A4-CLOSEOUT remains blocked_by A4-5.

## Blocked_by / next cycle (Rule 6 STOP)

**STOP is mandatory.** A4-4a is closed. No further commits under
`a4-4a-*` are permitted without a new ROADMAP-SYNC preflight.

Next cycle candidates (in dependency order, all blocked_by A4-4a):

- **A4-4b — generic AlignmentLens kernel/registry.** Defines the trait
  that *consumes* `ApplicableConcern` answers; closes the kernel half
  of arch-spec-046.
- **A4-4M — paradigm_lens convergence.** Converges the existing
  `paradigm_lens` registry to use the A4-4a `ParadigmProfileRef` as
  its reference surface; zero new features.
- **A4-4C — receipt/UAT.** Wires the applicable_concerns output into
  the receipt/UAT pipeline.

Whichever is opened next requires:
1. A new scope contract (8 MUST / 7+ MUST_NOT).
2. A new frozen baseline update before any code work.
3. ROADMAP-SYNC preflight (verify arch-spec-046 promotion conditions).

## See also

- `.sddk/cycles/p-63676b11dc0ef88f-a4-4a-intent-universal-concern-foundation/spec.md` — scope contract (MUST/MUST_NOT/inputs/outputs/anti-encroachment).
- `.sddk/cycles/p-63676b11dc0ef88f-a4-4a-intent-universal-concern-foundation/archive-manifest.md` — durable ground-truth close-out.
- `docs/architecture/specs/arch-spec-046-*.md` — part-1 model surface (now `implemented`).
- `docs/architecture/adrs/ADR-0124-ALIGNMENT-IS-ADVISORY.md` — addendum (A4-4a MISALIGNED ≠ DENY restatement).
- `docs/handoff/HANDOFF-2026-09-16-a4-3-software-alignment-core-v1.169.50.md` — predecessor handoff.
