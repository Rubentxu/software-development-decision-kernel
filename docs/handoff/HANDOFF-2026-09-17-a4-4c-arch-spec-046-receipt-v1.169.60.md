# Handoff: A4-4C — arch-spec-046 Receipt/UAT → v1.169.60

**Date:** 2026-09-17
**Cycle:** `p-63676b11dc0ef88f/a4-4c-arch-spec-046-receipt-uat`
**Status:** SHIPPED + RELEASED.
**Release tag:** `v1.169.60` → SHA `ca14e46078b3c9c10c541ec6b24a6483d4f1fc82`
**GH Release URL:** https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.60
**Released baseline inherited:** v1.169.59 / `ba986a5eda88e87dff0874e0cb01e57e838aa3db`.

## Scope

A4-4C is **RECEIPT / UAT — ZERO FEATURE**. It closes
`docs/architecture/specs/arch-spec-046-alignment-intent-and-lenses.md`
as `status: implemented` by issuing the durable acceptance receipt
that proves part-1, part-2 and the A4-4M lens migration are
verifiably present, pinned, and durable. Resolves two pre-existing
doc-level contradictions surfaced by the explore envelope. Does NOT
remove the `evaluate_lens` compatibility facade (out of scope;
documented deferral via ADR-0125 amendment).

## What landed

### Commits (this cycle)

| # | Hash | Subject |
|---|------|---------|
| 1 | 9dd35f2 | test(engine): A4-4C — arch-spec-046 acceptance UAT pin (3 tests, pin_119/119a/119b) |
| 2 | bfaa3df | docs: A4-4C — arch-spec-046 status implemented + ADR-0125 amendment + roadmap + §6 residue fix |
| 3 | ca14e46 | chore(release): bump version 1.169.59 -> 1.169.60 (A4-4C release) |

### Source

- `crates/sddk-engine/tests/a4_4c_arch_spec_046_acceptance.rs` (216 lines):
  - `pin_119_arch_spec_046_acceptance_witness_chain` — end-to-end witness
    chain (universal-concern vocabulary → reducer → registry → kernel →
    identity).
  - `pin_119a_arch_spec_046_anti_encroachment_witness` — compile-time
    witness that the imports don't pull authority/instruction_compiler/
    provider_sdk/paradigm_lens.
  - `pin_119b_lens_id_construction_is_static_str` — identity-stability
    sanity check.
- `crates/sddk-engine/src/paradigm_lens/lenses.rs` — doc-comment update
  on the `evaluate_lens` removal trigger (names the AC7 corpus tests;
  records the deferral).

### Docs

- `docs/architecture/specs/arch-spec-046-alignment-intent-and-lenses.md`:
  frontmatter flips `status: contract-ready → status: implemented`;
  `implemented_by` enumerates the shipped sub-cycles
  (A4-4a, A4-4aR, A4-4b, A4-4bR, A4-4M, A4-4C). Only A4-5 loop
  integration remains honestly open.
- `docs/architecture/adrs/ADR-0125-GENERIC-ALIGNMENT-LENS-KERNEL-REGISTRY.md`:
  third post-acceptance amendment recording (a) A4-4C closure,
  (b) `evaluate_lens` facade NOT removed (out of scope), (c) explicit
  migration predicate for the future cycle.
- `docs/architecture/README.md`: A4-4C roadmap row flips from CURRENT
  to closed — v1.169.60; close-checkpoint block updated.
- `docs/architecture/a4-4c-acceptance-receipt.md`: durable per-cycle
  acceptance receipt (file:line evidence, 7 falsification gates, gate
  results).
- `docs/handoff/HANDOFF-2026-09-16-a4-4a-intent-universal-concern-foundation-v1.169.52.md`:
  corrects the historical `per arch-spec-046 §6` deferral note
  (documented residue; §6 doesn't exist).

## Gate results (all green pre-release)

| Gate | Result |
|------|--------|
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test -p sddk-engine --test a4_4c_arch_spec_046_acceptance` | 3/3 pass |
| `cargo test` (all 8 A4 scoped test files) | 119/119 pass (5 + 24 + 29 + 3 + 14 + 11 + 3 + 3 + 27 = 119 across `a4_4a`, `a4_4b`, `a4_4br`, `a4_4c`, `a4_4m_convergence_pins`, `a4_4m_m0_migration_proof`, `ac7_lens_over_ac3_profile`, `ac8_full_chain_receipt`, `alignment_lens_fixture`) |
| `cargo test --workspace --offline` (release script step 1) | 0 failed |
| `cargo build --release -p sddk-cli --bin sddk` | clean — `sddk 1.169.60` |
| `bash tests/test_release_public_gate.sh` (release script step 1b) | PASS=11 FAIL=0 |
| `sddk dev manifest --root . --verify` (step 4) | manifest OK |
| `gh release create v1.169.60` (step 9) | 9 canonical assets published |
| **PublicReleaseGate step 9b** (tag SHA anchored, `isDraft=false`, `isPrerelease=false`, 9/9 HTTP 200) | PASS |
| `bash scripts/install.sh --version v1.169.60 --editor none` (step 10) | OK |
| `sddk dev doctor` (step 11) | `binary.bundle_coherence: present` + `all_present: true` |
| `sddk dev update --prune-only --keep 1` (step 12) | OK |
| Distrib round-trip (step 13) | OK (binary + bundle coherent after prune) |
| `scripts/release.sh` final state | `release v1.169.60 shipped and installed locally` |

## Release — DONE

| Step | Value |
|------|-------|
| Tag  | `v1.169.60` |
| SHA  | `ca14e46078b3c9c10c541ec6b24a6483d4f1fc82` |
| Workspace version at release | `1.169.60` |
| GitHub Release | https://github.com/Rubentxu/software-development-decision-kernel/releases/tag/v1.169.60 |
| 9 canonical assets | published (sddk + sddk.sha256 + tar.gz + tar.gz.sha256 + CHECKSUMS + sbom.json + unified tarball + unified tarball.sha256 + gh-release-receipt.json) |
| Install from URL | OK |
| Doctor | OK (`binary.bundle_coherence: present`, `all_present: true`) |
| Prune | OK |
| Distrib round-trip | OK (binary + bundle coherent after prune) |
| `scripts/release.sh` final state | `release v1.169.60 shipped and installed locally` |

## Falsification gates (all pinned)

| # | Gate | How it pins | Test |
|---|------|-------------|------|
| 1 | Lenses are selected by declared intent, never applied universally | `applicable_concerns()` returns `Applicable` only for declared concerns; paradigm does NOT erase | `pin_119` |
| 2 | No lens may produce a `ContractViolation` without an explicit contract | `LensContribution` has no `AuthorityDecision` field | `pin_119a` |
| 3 | `ApplicableConcern + no registered lens` → `NotEvaluated`, NOT `NotApplicable` | typed gap | `a4_4b_alignment_lens_kernel.rs:213` |
| 4 | `LensInput::try_new` refuses `NotApplicable` at the boundary | structural refusal | `a4_4b_alignment_lens_kernel.rs` (multiple pins) |
| 5 | Same `(lens_id, concern, observation_set, evidence_resolution)` → byte-identical `LensContributionId` | `pin_119` re-evaluation identity equality | `a4_4c_arch_spec_046_acceptance.rs` |
| 6 | `evaluate_lens` facade produces identical outputs to the kernel path for AC7 corpus inputs | AC7 corpus passes UNMODIFIED post-convergence | `a4_4m_convergence_pins.rs` |
| 7 | Paradigm does NOT erase a concern (A4-4aR correction) | `(Pipeline, TemporalCoupling)` is now Applicable | `a4_4a_intent_universal_concern_integration.rs` falsification pin |

## Contradictions resolved

### C1 — A4-4C vs A4-CLOSEOUT as `arch-spec-046` promotion authority
- `arch-spec-046` body has 4 sections; the historical "per arch-spec-046 §6" references in `docs/architecture/README.md:145` and `docs/handoff/HANDOFF-2026-09-16-a4-4a-…` were documentation residue. A4-4C closes the spec; A4-CLOSEOUT remains the A4 milestone acceptance gate (blocked_by A4-5), not the spec authority.

### C2 — `evaluate_lens` facade removal
- The doc-comment said "removal trigger = A4-4C". A4-4C is receipt-only, so the trigger is now explicit: removal happens when the AC7 corpus tests (`ac7_lens_over_ac3_profile.rs`, `ac8_full_chain_receipt.rs`) migrate to call `AlignmentLensKernel` directly. A4-4C did NOT remove the facade. ADR-0125 records the deferral.

## Debt notes

- **`FU-A4-3-CONSTRAINT-BINDING`** (P1): `software_alignment::reduce_alignment` still associates `ExplicitConstraint` → observations via `subject.contains(contract_ref)` / `canonical_tag.contains(contract_ref)`. **NOT touched in A4-4C** (receipt-only budget; out of scope). Must close before A4-5.
- **`INC-A4-RELEASE-VERSION-DRIFT`** (P2): open. A4-4C inherits cycle-46 install-coherence contract (workspace bumps BEFORE tag). Expected release tag was v1.169.60; actual = v1.169.60 (no off-by-one this cycle — workspace bumped from 1.169.59 → 1.169.60 in commit `ca14e46` and the tag matches).

## Cycle spec compliance

The scope contract at `.sddk/cycles/p-63676b11dc0ef88f-a4-4c-arch-spec-046-receipt-uat/spec.md` required:
- Frontmatter update of `arch-spec-046` (status + implemented_by). DONE.
- New acceptance UAT test (1). DONE (`pin_119/119a/119b` — 3 tests for cheap anti-encroachment bonus).
- ADR-0125 3rd amendment. DONE.
- README close-checkpoint + roadmap row update. DONE.
- `evaluate_lens` removal-trigger doc-comment clarification. DONE.
- §6 residue fix in A4-4a handoff. DONE.
- Acceptance receipt (`docs/architecture/a4-4c-acceptance-receipt.md`). DONE.
- Release through `scripts/release.sh` step 9b + 10-13. DONE.

Commits: 3 (test, docs, chore(release)) — within the 4-commit budget.

## Roadmap delta

```text
A4-4M   AC7 → AlignmentLens convergence            ✓ v1.169.59
A4-4C   arch-spec-046 closure (receipt/UAT)        ✓ v1.169.60 (this cycle)
A4-5    Intelligence loop wiring                   blocked_by A4-4C
A4-CLOSEOUT  A4 milestone acceptance gate           blocked_by A4-5
A5      BASE_PRODUCTION_READY                      blocked_by A4-CLOSEOUT
```

## STOP

A4-5 does NOT auto-open after A4-4C closes. A4-5 requires a new
ROADMAP-SYNC preflight + scope contract + user green-light.
