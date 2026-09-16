# Handoff — A4-4aR Applicability Semantics Correction + v1.169.54 release

- **Cycle:** `p-63676b11dc0ef88f-a4-4ar-applicability-correction`
- **Released:** v1.169.54 (tag `v1.169.54` →
  `219de3f16a067819125054b89abb88d9dc73cc57`, origin/main HEAD identical)
- **GitHub Release:** https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.54
- **Commits (origin):** feat `df3ea1f`, chore(release) `219de3f16a067819125054b89abb88d9dc73cc57`
- **PublicReleaseGate:** PASS (first non-REL regression of the REL-1 gate)
- **Goal:** Single-budget correction of the `applicable_concerns()` reducer
  so that Applicability is intent-only, separating it from Grounding and
  Evaluability. Found by the user during the A4-4b pre-flight; opened
  instead of A4-4b so the kernel would not be built on a flawed reducer.
- **Predecessor handoff:** `docs/handoff/HANDOFF-2026-09-16-rel-1-public-release-gate-v1.169.53.md`
  (REL-1 closed 2026-09-16; v1.169.53 / `ccecdc723355b0ecc7e037f2266f465165dc59dc`).
  Carries the REL-1 close-out + the FU-REL-1-BACKFILL obligation that
  lands in this cycle's `chore(release)` commit.

## Status

**CLOSED 2026-09-16.** All MUST satisfied, all MUST_NOT observed, all
falsification pins landed. v1.169.54 shipped to GH, end-to-end install
verified, framework bundle pruned. A4-4b is unblocked for user green-light.

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

> **Outcome (2026-09-16T16:22Z).** The single `chore(release)`
> (`219de3f`) landed and is pushed to origin. Both SHA backfills (REL-1
> handoff) and both followup closures (`FU-A4-4A-STRING-GROUNDING`,
> `FU-REL-1-BACKFILL`) are folded into this handoff + the on-origin
> followups table. See "Live release evidence" below.

## Live release evidence — v1.169.54 (2026-09-16)

Source of truth: `bash scripts/release.sh` run captured to
`/tmp/release-1.169.54.log`. **First non-REL cycle to exercise the
new PublicReleaseGate in regression mode.**

| Step | Marker | Result |
|------|--------|--------|
| 0 | preflight | on main, clean tree, HEAD is a release commit |
| 1 | fmt + clippy + test + doctests | workspace green (1101 passed, doctests ok) |
| 1b | shell contract tests | all green (release-receipt authority + 8 cross-crate/M9+ tests) |
| 1c | sync HEAD to origin/main | `HEAD==origin/main (219de3f16a067819125054b89abb88d9dc73cc57)` |
| 2 | read version | 1.169.54 → tag v1.169.54 |
| 3 | cargo build --release --bin sddk | OK |
| 4 | regenerate MANIFEST.sha256 | OK |
| 5 | bundle tarball | OK |
| 6 | BUNDLE.toml (schema v2) | `manifest_sha256=608c6d9ced950456e9d453d8e54529b6c3dc06e45302189c3c15c01c738fd39f` |
| 7 | unified tarball | OK |
| 8 | sha256 + CHECKSUMS + sbom | OK |
| 8b | vault ADR mirror sync | OK |
| 9 | gh release create v1.169.54 | URL `https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.54` |
| **9b** | **PublicReleaseGate** | **PASS** — tag SHA anchored `219de3f16a067819125054b89abb88d9dc73cc57`, isDraft=false, isPrerelease=false, asset set matches 9-asset contract, 9/9 canonical assets reachable from public CDN (HTTP 200) |
| 10 | install from URL | CDN served correct binary sha256 after 10s |
| 11 | sddk dev doctor | binary.bundle_coherence present, all_present true |
| 12 | sddk dev update --prune-only --keep 1 | removed 1 stale bundle (1.169.53); current -> 1.169.54 |
| 13 | re-install from URL (distrib smoke test) | OK (binary=bundle=1.169.54) |
| 14 | final state | binary=1.169.54, bundle=1.169.54, current=1.169.54 |

**Pinned reference points (SHA discipline):**

| Identifier | Value |
|---|---|
| released_baseline | v1.169.54 |
| release_commit | `219de3f16a067819125054b89abb88d9dc73cc57` |
| release_commit_subject | `chore(release): bump version 1.169.53 -> 1.169.54 (A4-4aR + REL-1 backfill)` |
| feat_commit_subject | `feat(engine): A4-4aR — separate Applicability from Grounding and Evaluability` |
| feat_commit | `df3ea1f` |
| origin_main_HEAD | `219de3f16a067819125054b89abb88d9dc73cc57` (matches release) |
| tag | `v1.169.54` → `219de3f16a067819125054b89abb88d9dc73cc57` (per `git ls-remote origin v1.169.54`) |
| released_baseline_origin | v1.169.53 (`ccecdc723355b0ecc7e037f2266f465165dc59dc`) |
| previous_released_baseline | v1.169.52 (HEAD `9785288`) |
| gh_release_url | https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.54 |
| local_installed_binary | `/home/rubentxu/.local/bin/sddk` = 1.169.54 |
| local_bundle | `/home/rubentxu/.local/share/sddk/framework/1.169.54` |
| tag_drift_anchor | `git ls-remote origin $TAG` (NOT `origin/main`) |

**Key cross-check (proves release SHA == origin/main, anchored on the tag,
not on a fresh HEAD push):**

```
$ git ls-remote origin v1.169.54
219de3f16a067819125054b89abb88d9dc73cc57	refs/tags/v1.169.54

$ git rev-parse origin/main
219de3f16a067819125054b89abb88d9dc73cc57
```

**First non-REL PublicReleaseGate regression.** Step 9b PASS proves the
REL-1 fix from v1.169.53 is durable: it does not depend on REL-1's
specific 10-scenario contract test passing; it exercises the live GH API
end-to-end (release-create → release-view → 9 asset probes on the public
CDN). v1.169.54 is the first cycle since REL-1 to do so.

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
