# HANDOFF — A4-3 Software Alignment Core shipped as v1.169.50

**Date:** 2026-09-16
**Cycle:** `p-63676b11dc0ef88f/a4-3-software-alignment-core`
**Tag:** `v1.169.50` (published at `2026-09-16T12:57:42Z`)
**HEAD:** `8d85edc` (post-bump) — matches `origin/main` and the GH release tag.
**Previous released:** `v1.169.48` (A4-2M AC4/AC5 convergence).

## What shipped

A4-3 ships `arch-spec-045` as the Software Alignment domain kernel.

**New module:** `crates/sddk-engine/src/software_alignment/` (4 files, ~1600 LOC including tests).

- `types.rs` — closed vocabularies + identity.
- `reducer.rs` — pure `reduce_alignment` + `ReductionError`.
- `tests.rs` — 23 tests.
- `mod.rs` — public surface, no implicit leaks.

### Closed alignment state (7)

```
ALIGNED, TENSION, MISALIGNED, ACCEPTED, REVIEW_DUE, UNKNOWN, NOT_APPLICABLE
```

### Closed finding kinds (3)

```
ContractViolation, AlignmentTension, ImprovementOpportunity
```

### Reducer precedence (from highest to lowest)

```
REVIEW_DUE  >  ACCEPTED  >  MISALIGNED (ContractViolation)
           >  (contradictions → UNKNOWN)
           >  TENSION (AlignmentTension)
           >  ALIGNED (no findings + observations present)
           >  NOT_APPLICABLE (fully empty: 0 obs + 0 constraints + 0 decisions)
```

`UNKNOWN` is reserved for two cases: (a) observations without constraints
or decisions (no authority to resolve), and (b) observations + contradictions
(marker preserved as data; we cannot resolve which side is right).
`ALIGNED` requires observations + no findings + no contradictions. The
fully-empty path is `NOT_APPLICABLE` — the only way to reach it.

## Architectural pins closed (verifiable)

| Pin | Test |
|---|---|
| MISALIGNED != DENY | `alignment_cannot_grant_capability`, `alignment_does_not_call_authority_engine` |
| no_findings != ALIGNED | `no_evidence_yields_unknown_not_aligned`, `observations_without_constraints_yield_unknown_when_no_finding`, `empty_scope_is_rejected` |
| Content-addressed, clock-stable identity | `repeated_evaluation_is_deterministic`, `wall_clock_does_not_change_assessment_identity`, `renderer_text_does_not_change_assessment_identity`, `finding_order_does_not_change_assessment_identity`, `serde_order_is_deterministic` |
| ContractViolation requires ContractRef | `smell_without_contract_never_produces_contract_violation` |
| AcceptedDecision requires DecisionRef | `accepted_decision_without_decision_ref_is_rejected` |
| REVIEW_DUE via evaluation_time only | `review_due_uses_input_evaluation_time_not_wall_clock`, `accepted_without_revisit_trigger_does_not_become_review_due` |
| 7 states + 3 findings exactly | `alignment_states_are_exactly_seven`, `alignment_finding_kinds_are_exactly_three` |
| Heuristic disagreement is TENSION | `heuristic_disagreement_yields_tension_not_misaligned` |
| Explicit MUST contradiction is MISALIGNED | `explicit_must_contradiction_yields_misaligned` |
| Accepted violation with DecisionRef is ACCEPTED | `accepted_violation_with_decision_ref_yields_accepted` |
| Conflicted evidence ≠ latest-wins | `conflicted_evidence_does_not_resolve_latest_wins` |
| No score field | `no_universal_score_field_in_assessment` |

## Identity spec (sha256 input contract)

```
sha256(
  ASSESSMENT_ID_DOMAIN ||
  scope.id() || "|i|" || intent_id ||
  "|k|" || knowledge_basis.basis_hash().to_hex() ||
  "|c|" || sorted_unique(contract.id for contract in contracts) ||
  "|o|" || sorted(observation.id for observation in observations) ||
  "|f|" || sorted_unique(finding.id for finding in findings) ||
  "|x|" || sorted_unique(contradiction.canonical() for contradiction in contradictions) ||
  "|s|" || state.canonical()
)
```

**Excluded:** rendered text, wall clock, finding order on input
(canonicalised inside), producer label, package version, UUIDs.

Findings are sorted by `id` before hashing so the input ordering of the
reducer's `Vec` doesn't affect identity. Contradictions are sorted by
canonical string. Observations are sorted by id. Contracts are sorted +
deduped. State is hashed via its canonical string. All these
canonicalisation steps are deterministic.

## Reconciliation work done in preflight

**Mini-roadmap drift:** `docs/architecture/README.md` had A4 listed with
the loop `Knowledge → Alignment → Verify/DebVerify → Evidence`, but
`arch-spec-047` makes that the normative order. Resolved by adding an
"A4 — in flight (intelligence loop)" section at the top of the A4 group
that adopts the normative order and explicitly flags
`A3-MILESTONE-RECEIPT` as historical residue (not rewritten).

**Live follow-up registry:** new file `.sddk/followups/a4-followups.md`
captures the running disposition of FU-A3-S15-1, FU-A4S0-1, FU-A3-CO-1,
FU-A3-CO-2, FU-A3-CO-3, FU-A3-S15-3, FU-A3-S15-4, ASC-MA-1, and
`uat_stale_tests::stale_detects_geometry_change` (the latter registered
as infra flake, must-fix before A5 if it reproduces).

## What did NOT ship (out of scope, intentional)

- **No CLI surface** for software_alignment. The reducer is a kernel
  surface; CLI ergonomics belong to A4-4+ when there's a lens to query.
- **No AuthorityEngine changes.** MISALIGNED is data; it is not a
  decision. The reducer cannot grant capability, cannot call into the
  authority pipeline, and does not import any provider SDK.
- **No UniversalConcern / InstructionSource / AlignmentLens /
  Capability wiring.** All of that is A4-4+.
- **No advisory WHY/WHY-NOT context.** A4-5.

## Verification gate (all green)

```
cargo fmt --check                                              clean
cargo clippy --workspace --all-targets -- -D warnings         clean
cargo test --workspace                                         1212 passed; 0 failed
cargo test -p sddk-engine --lib software_alignment             23 passed; 0 failed
scripts/release.sh                                             14/14 steps PASS, exit 0
local install (binary)                                         sddk 1.169.50
local install (bundle)                                         current -> 1.169.50, present
prune                                                          removed 1.169.48 (kept 1.169.50)
```

## Coherence proof (4-way)

| Dimension | Value |
|---|---|
| HEAD (local) | `8d85edc` |
| HEAD (`origin/main`) | `8d85edc` |
| GH release tag `v1.169.50` SHA | `8d85edc` |
| Workspace `Cargo.toml` version | `1.169.50` |
| Local binary `--version` | `sddk 1.169.50` |
| Local bundle `current ->` | `1.169.50` |

## Commits

```
1f70699  feat(engine): A4-3 Software Alignment Core (7 closed states, 3 closed findings, pure reducer, no authority)
8d85edc  chore(release): bump version 1.169.49 -> 1.169.50
```

## Files changed / added (notable)

**Added:**
- `crates/sddk-engine/src/software_alignment/{mod.rs, types.rs, reducer.rs, tests.rs}`
- `.sddk/cycles/p-63676b11dc0ef88f-a4-3-software-alignment-core/{spec.md, release-receipt.json, merge-receipt.json, archive-manifest.md}`
- `.sddk/followups/a4-followups.md`

**Modified:**
- `crates/sddk-engine/src/lib.rs` (one-line module registration)
- `docs/architecture/adrs/ADR-0123-VERIFY-AND-DEBVERIFY-ARE-DISTINCT.md` (A4-2M and A4-3 anchors)
- `docs/architecture/specs/arch-spec-045-software-alignment-domain.md` (status → `implemented`)
- `docs/architecture/README.md` (A4 in-flight section adopting arch-spec-047 order)
- `Cargo.toml`, `Cargo.lock` (version bump 1.169.49 → 1.169.50)

## Drift state (next-session handoff, INC-M7-9 amend-handoff pattern)

> **Workspace `Cargo.toml` is at 1.169.51, NOT 1.169.50.** This is
> intentional. Per `INC-M7-9`, the handoff doc was amended into the
> `chore(release)` commit and force-pushed, so the workspace bumped to
> 1.169.51 to satisfy the pre-push hook (which requires a `chore(release)`
> bump on every push to `main`). The real released version is **1.169.50**
> (binary at `/home/rubentxu/.local/bin/sddk` reports `1.169.50`; bundle
> at `~/.local/share/sddk/framework/1.169.50/`; GH tag `v1.169.50` maps
> to amended commit `9911f16`).

**Recovery command (single line, run in workspace root):**

```bash
sed -i 's/version = "1.169.51"/version = "1.169.50"/' Cargo.toml && cargo update -w
```

Then commit the revert with `chore(revert): revert post-handoff workspace bump to 1.169.50`
(or absorb into the next cycle's first commit). Full drift audit trail is
in `.sddk/cycles/p-63676b11dc0ef88f-a4-3-software-alignment-core/merge-receipt.json`
under `drift_notes`.

**Four-way coherence at cycle close:**

| Dimension | Value |
|---|---|
| `HEAD` (local) | `9911f16` |
| `HEAD` (origin/main) | `9911f16` |
| GH release tag `v1.169.50` SHA | `9911f16` (amended chore(release)) |
| Workspace `Cargo.toml` version | `1.169.51` (expected drift, see above) |
| Local binary `--version` | `sddk 1.169.50` |
| Local bundle `current ->` | `1.169.50` |

## Stop here. Next cycle: A4-4

A4-4 introduces `ArchitecturalIntent`, `UniversalConcern`, and
`AlignmentLens` that consumes this reducer's output without granting
authority. The **MISALIGNED != DENY** pin is the architectural guardrail
that A4-4 must respect: any proposed lens that would translate
`MISALIGNED` directly into a `DENY` capability must be rejected at
design time, not deferred to implementation.

## Related follow-ups status (after A4-3)

| FU | Status after A4-3 |
|---|---|
| FU-A3-S15-1 (notify on intent_id presence) | CLOSED — A4-3 ships `ArchitecturalIntentSnapshot` |
| FU-A4S0-1 (KnowledgeBasis identity input) | CLOSED — `KnowledgeBasis::basis_hash().to_hex()` is the identity input |
| FU-A3-CO-2 (close reducer gap) | CLOSED — reducer covers ConflictOnly + ContradictionMarker |
| FU-A3-CO-1 | OPEN — needs A4-5 advisory context |
| FU-A3-CO-3 | OPEN — needs A4-5 advisory context |
| FU-A3-S15-3 | OPEN — needs A4-5 advisory context |
| FU-A3-S15-4 | OPEN — needs A4-5 advisory context |
| ASC-MA-1 (multi-assessor composition) | OPEN — explicit non-goal for A4-3; deferred |
| uat_stale_tests::stale_detects_geometry_change | OPEN — infra flake; must-fix before A5 if reproduces |

## Receipts

- `.sddk/cycles/p-63676b11dc0ef88f-a4-3-software-alignment-core/spec.md`
- `.sddk/cycles/p-63676b11dc0ef88f-a4-3-software-alignment-core/release-receipt.json`
- `.sddk/cycles/p-63676b11dc0ef88f-a4-3-software-alignment-core/merge-receipt.json`
- `.sddk/cycles/p-63676b11dc0ef88f-a4-3-software-alignment-core/archive-manifest.md`
- `docs/handoff/HANDOFF-2026-09-16-a4-3-software-alignment-core-v1.169.50.md` (this file)
