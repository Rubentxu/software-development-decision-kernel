# Handoff — A4-4aR Applicability Semantics Correction + v1.169.54 release

- **Cycle:** `p-63676b11dc0ef88f-a4-4ar-applicability-correction`
- **Goal:** Single-budget correction of the `applicable_concerns()` reducer
  so that Applicability is intent-only, separating it from Grounding and
  Evaluability. Found by the user during the A4-4b pre-flight; opened
  instead of A4-4b so the kernel would not be built on a flawed reducer.
- **Predecessor handoff:** `docs/handoff/HANDOFF-2026-09-16-rel-1-public-release-gate-v1.169.53.md`
  (REL-1 closed 2026-09-16; v1.169.53 / `ccecdc723355b0ecc7e037f2266f465165dc59dc`).
  Carries the REL-1 close-out + the FU-REL-1-BACKFILL obligation that
  lands in this cycle's `chore(release)` commit.

## Goal (single budget)

> Eliminate the Applicability / Grounding / Evaluability frontier collapse
> in the A4-4a reducer so A4-4b can build on a clean surface.

## What was found (and replaced)

The pre-A4-4aR `applicable_concerns()` reducer did this:

```text
applicable_concerns(pi, ui, contracts, decisions) -> Vec<(ApplicableConcern, Option<ApplicableReason>)>

Rules:
1. project_intent.declared_concerns.contains(c)?
2. unit_intent.applies_to_concerns.contains(c)?
3. paradigm_supports_concern(pi.paradigm, c)?   <- (Pipeline, TemporalCoupling) => false
4. d.render().contains(c.canonical_tag())?      <- string-grounding on decisions
5. c.as_str().contains(c.canonical_tag())?      <- string-grounding on contracts
```

This conflates three distinct semantic layers:

```text
APPLICABILITY
  = this question is pertinent under the declared intent

GROUNDING
  = which decisions / contracts substantiate that intent

EVALUABILITY
  = whether we have evidence sufficient to answer the question
```

The reducer was emitting `NotApplicable(NoGroundingDecision)` for concerns
that were perfectly applicable in scope but merely lacked an
incident-specific decision/contract in the test fixtures. A4-4b would
have built a lens kernel on top of that answer — which would have made
"ApplicableButUngrounded" indistinguishable from "NotApplicable". The
user caught it during a review of the A4-4a intent surface before the
A4-4b kernel would have been written on top of it.

## What A4-4aR delivers (MUST list)

1. **Applicability answers to intent only.** The reducer is now a
   two-argument function `fn applicable_concerns(&ProjectIntent, &UnitIntent)`.
2. **`NotApplicableReason` shrinks to four legitimate scope reasons:**
   `NotInProjectIntent`, `NotInUnitIntent`, `ExplicitlyExcludedByProject`,
   `ExplicitlyExcludedByUnit`. The deprecated variants
   `NoGroundingDecision`, `NoContractReference`, `ParadigmIrrelevant`
   are removed outright (no compat shim, no `#[allow(deprecated)]`).
3. **`ApplicableReason` shrinks** to a single variant `ProjectAndUnitIntent`.
4. **No string-grounding.** The reducer signature has no `DecisionRef`
   or `ContractId` parameter. The previous `decision_grounds_concern`
   and `contract_references_concern` helpers are deleted.
5. **Paradigm does not erase a concern.** The `(Pipeline, TemporalCoupling)`
   row in the relevance table is removed; the table itself collapses to
   "every paradigm supports every concern". The function is kept as an
   identity helper for future lens-selection use, but it returns `true`
   for every pair.
6. **ProjectIntent / UnitIntent grow `excluded_concerns: BTreeSet<UniversalConcern>`**
   fields, with `#[serde(default)]` for back-compat.
7. **Removed wrapper types** `DecisionRefs(Vec<DecisionRef>)` and
   `ContractRefs(Vec<ContractId>)`. Verified zero external callers.
8. **Module-level epistemic pin** added to the doc-comment of
   `applicable_concerns()`, `NotApplicableReason`, and the module root.
9. **Pipeline + TemporalCoupling declared in BOTH scopes → Applicable.**
   Falsification pin test (`falsification_paradigm_does_not_erase_concern`).
10. **Determinism preserved** (same `(pi, ui)` → same output, sorted
    by `ApplicableConcern::canonical()`).

## What A4-4aR explicitly does NOT do

No `AlignmentLens`, no lens registry, no OO/FP/ADT/DSL evaluator, no
`paradigm_lens` mutation, no `software_alignment::reduce_alignment`
wiring, no new `Alignment` finding, no CLI change, no
`provider_kind`, no `Governance`, no `AdvisoryContext` integration.
A4-4aR is a correction slice; the kernel stays in A4-4b.

## What stays untouched

- `arch-spec-046` part-1 model surface (vocabularies, intent types,
  `ApplicableConcern`, `ParadigmProfileRef`) — vocabulary & module shape unchanged.
- `arch-spec-045` A4-3 software-alignment surface (`AlignmentState`,
  three finding kinds) — untouched per A4-3 owner.
- `UniversalConcern::ALL` 10-member closed set.
- `ApplicableConcern::canonical()` sort + deterministic identity.
- `ReductionError::EmptyProjectIntent` / `EmptyUnitRef` typed refusal paths.

## UAT-falsification pins (test names)

In `crates/sddk-engine/src/intent_universal_concern/tests.rs` and
`crates/sddk-engine/tests/a4_4a_intent_universal_concern_integration.rs`:

1. `falsification_signature_no_decisions_or_contracts` — type-level
   enforcement: `applicable_concerns(pi, ui)` arity-2; this test would
   fail to compile if the signature regressed.
2. `falsification_no_deprecated_reasons` — every emitted
   `NotApplicableReason` canonical_tag is in the legitimate 4-element
   set. (NoGroundingDecision, NoContractReference, ParadigmIrrelevant
   variants are gone.)
3. `falsification_paradigm_does_not_erase_concern` — for every paradigm
   in `{ObjectOriented, Functional, FunctionalPure, DataOriented,
   Pipeline}`, declared concerns ⇒ applicable. Pre-A4-4aR
   `(Pipeline, TemporalCoupling) → NotApplicable` would have broken this.
4. `falsification_same_applicability_with_or_without_evidence` —
   applicability is independent of any external evidence (the function
   literally doesn't take evidence).
5. Plus six additional tests for project/unit explicit exclusion,
   determinism, ordering, identity-stability, MISALIGNED ≠ DENY.

## Verification (falsifiable, not just green)

| Property | Check | Outcome |
|---|---|---|
| Rust compiles with the new signature | `cargo check --workspace` | clean |
| All intent_universal_concern lib tests pass | `cargo test -p sddk-engine --lib intent_universal_concern` | 22 passed; 0 failed |
| Integration test passes | `cargo test -p sddk-engine --test a4_4a_intent_universal_concern_integration` | 5 passed; 0 failed |
| Full workspace tests pass | `cargo test --workspace --offline` | **1101 passed; 0 failed** |
| `cargo fmt --check` clean | `cargo fmt --all -- --check` | clean |
| `cargo clippy -D warnings` clean | `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| PublicReleaseGate test still green | `bash tests/test_release_public_gate.sh` | PASS=11, FAIL=0 |

## Out-of-scope commits to land atomically (single `chore(release)`)

The A4-4aR commit fan-in is bounded: **one** `chore(release)` commit
ships the version bump + the REL-1 carry-over + closes
`FU-REL-1-BACKFILL`. Per the user's carry-over instruction, the
REL-1 handoff backfill (`docs/handoff/HANDOFF-2026-09-16-rel-1-...md`)
gets its two deferred SHA rows replaced by `ccecdc7…` in that same
commit. No new REL cycle opens.

## Roadmap Delta (live in `docs/architecture/README.md`)

| Cycle | Before | After |
|---|---|---|
| A4-4a | `closed — v1.169.52` | unchanged |
| **A4-4aR** | (inserted as NEXT) | new corrective slice |
| A4-4b | `blocked_by A4-4a` | `blocked_by A4-4aR` |
| A4-4M | `blocked_by A4-4b` | unchanged |
| A4-4C | `blocked_by A4-4M` | unchanged |
| A4-5 | `blocked_by A4-4` | `blocked_by A4-4C` |

Disposition: `A4-4a string-grounding applicability → CLOSED by A4-4aR`.

## STOP

A4-4b does NOT auto-open. The user must explicitly green-light A4-4b
per the 6-rule mandatory STOP rule. This handoff is the durable receipt
that A4-4aR is done and that A4-4b is now safe to build on a clean
reducer surface.

## Relevant files (durable references)

- `crates/sddk-engine/src/intent_universal_concern/mod.rs` — module root
  + `pub use` re-exports (DecisionRefs/ContractRefs removed).
- `crates/sddk-engine/src/intent_universal_concern/types.rs` — ProjectIntent /
  UnitIntent gain `excluded_concerns`; `NotApplicableReason` reduced
  to 4 variants; `ApplicableReason` reduced to 1 variant;
  DecisionRefs/ContractRefs deleted.
- `crates/sddk-engine/src/intent_universal_concern/reducer.rs` —
  `applicable_concerns()` rewritten (intent-only, 2-arg signature);
  string-grounding helpers deleted; paradigm table collapsed.
- `crates/sddk-engine/src/intent_universal_concern/tests.rs` — full
  rewrite of tests under A4-4aR semantics; 22 tests.
- `crates/sddk-engine/tests/a4_4a_intent_universal_concern_integration.rs`
  — full rewrite; 5 integration tests.
- `docs/architecture/README.md` — A4 cycle table + new A4-4aR row;
  REL-1 checkpoint; A4-4aR open checkpoint.
- `.sddk/cycles/p-63676b11dc0ef88f-a4-4ar-applicability-correction/spec.md`
  — scope contract (gitignored, durable local ground truth).
- `.sddk/cycles/p-63676b11dc0ef88f-a4-4ar-applicability-correction/archive-manifest.md`
  — durable ground-truth close-out (written post-release).
- `.sddk/followups/a4-followups.md` — A4-4aR-disposition entry
  (FU-A4-4A-STRING-GROUNDING → CLOSED) + REL-1 backfill row closure.
