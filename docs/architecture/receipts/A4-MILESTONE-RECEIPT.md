# A4 Milestone Receipt

> **Milestone:** A4 — Observations, Evidence, Verify, DebVerify, Software
> Alignment, Intelligence Loop.
> **Certification cycle:** `p-63676b11dc0ef88f-a4-closeout-milestone-audit`
> **Date:** 2026-09-17
> **Budget:** AUDIT / RECONCILIATION / CERTIFICATION ONLY.
>
> This receipt is a **PROJECTION / certification artifact**. It is **not**
> a runtime authority and introduces no production type
> (`A4MilestoneReceipt` does not exist and must not).

## Freeze

| Field | Value |
|---|---|
| `released_baseline` | `v1.169.67` → `1949fa8448b636ffdc4fe2fea02339d82922481c` |
| `development_head` (at cycle open, full) | `730f855428f89c7232b8390ce0d96e791e6a68b0` |
| `workspace_version` (at open) | `1.169.67` |
| `cycle_id` | `p-63676b11dc0ef88f-a4-closeout-milestone-audit` |
| Milestone | `A4` |
| `next_if_pass` | `A5-PLAN` |

## Certified spec revisions (frozen at audit open)

The audit certifies **revisions**, not floating names.

| Spec | Frozen blob SHA | Frozen status |
|---|---|---|
| `arch-spec-042` | `5419294e88320efc852da9fb4f1469ec35cb4f44` | `accepted` |
| `arch-spec-043` | `80c5511ada8779145b45c928282d259524a37e4a` | `implemented` |
| `arch-spec-044` | `70567f2cb781ffdeecc5f87f93da72d0b953a351` | `implemented` |
| `arch-spec-045` | `a4c18bfa95fffa879cd83ecc08e0e89ed29a1f76` | `implemented` (body drift) |
| `arch-spec-046` | `3c1c6b63e29c552049e01533c34805dd626bcadf` | `implemented` (body drift) |
| `arch-spec-047` | `7d8a5fee29f58cb7b8970a6dba833dc943b30a11` | `implemented` |
| A4-5C acceptance receipt | `bd76c368974f542e2e0722c3a521c6f08c4ffe6a` | — |

Reconciled revisions produced by this cycle are recorded in the closeout
commit (see the archive manifest); only documentation changed.

## §2 Spec coherence audit — 042 → 047

| Spec | Claim | Owner (code) | Acceptance evidence | Drift | Result |
|---|---|---|---|---|---|
| 042 | Evidence / Observation / Provenance substrate | `observation/`, `evidence_ref.rs`, `evidence_relation_mapping.rs` | A4-0 handoff; consumed by A4-1..A4-5 | frontmatter said `accepted` with no `implemented_by` | **DOC_DRIFT → fixed** (→ `implemented`) |
| 043 | Generic Verify | `verify_kernel` | A4-1 handoff; `architecture_conformance::compute_conformance_delta_core` single-execution core | none | **PASS** |
| 044 | Generic DebVerify | `debverify_kernel` | A4-2 handoff; adapters AC5/observation | none | **PASS** |
| 045 | Software Alignment domain | `software_alignment::reducer::reduce_alignment` | A4-3 handoff | body said "Contract-ready, NOT implemented" | **DOC_DRIFT → fixed** |
| 046 | Intent / UniversalConcern / AlignmentLens | `intent_universal_concern`, `alignment_lens::kernel` | A4-4a/4aR/4b/4bR/4M/4C handoffs | body said "Contract-ready, NOT implemented"; `implemented_by` said "Loop integration (A4-5) remains open" | **DOC_DRIFT → fixed** |
| 047 | Intelligence Loop + Advisory/WHY | `intelligence_loop`, `intelligence_advisory` | A4-5C acceptance receipt (all clauses PASS) | none | **PASS** |

No `RUNTIME_GAP`, no `NOT_PROVEN`.

### 042 status semantics

There is no formal spec-status taxonomy document. The observed scale is
`proposed` < `accepted` / `contract-ready` < `implemented`, consistent with
ADR-0001's ADR lifecycle (`proposed → accepted → released`), where
`accepted` precedes implementation. `accepted` carried **no**
`implemented_by`, and 043–047 (which depend on 042) are `implemented`. A
delivered substrate tagged `accepted` while its dependents are
`implemented` is an inconsistent story, so 042 was promoted to
`implemented` with explicit `implemented_by`.

Observation (out of A4 scope, NOT changed): `arch-spec-A3-S13/S14/S15`
also carry `status: accepted`. Their reconciliation belongs to an
A3-scope audit, not A4-CLOSEOUT. Recorded here so it is not lost.

## §3 A4 authority map

Verified in code (one semantic authority per concept):

| Concern | Canonical authority | Facade(s) | Projection(s) | Forbidden competing authority | Gate |
|---|---|---|---|---|---|
| observations / evidence | `observation/` + `evidence_ref.rs` | — | `ObservationSet` digest | any second evidence store | `inv_epistemology_…` |
| Verify | `verify_kernel::VerifyKernel` | — | `VerificationResult` | second verify engine | `sec5` (A4-5C) |
| DebVerify | `debverify_kernel::DebVerifyKernel` | — | `ReconciliationSummary` | second reconcile engine | `sec6` (A4-5C) |
| Alignment assessment | `software_alignment::reduce_alignment` | — | `AlignmentAssessment` | second reducer | `sec8` (A4-5C) |
| Lens evaluation | `alignment_lens::AlignmentLensKernel` | `paradigm_lens::evaluate_lens` (LEGACY_READ_COMPAT, **no runtime consumer**) | `LensEvaluation` | second lens kernel | `inv_lens_concern_…` |
| intent / applicability | `intent_universal_concern` | — | `ApplicableConcern` | — | `inv_applicability_…` |
| architecture graph | `semantic_graph` (one `SemanticGraphProjection`, one `InMemorySemanticGraph` impl) | — | `ArchitectureGraphOverlay` | second graph/store | §3 shell pin |
| intelligence composition | `compose_intelligence_loop` | — | `IntelligenceLoopReceipt` | second loop engine | `inv_composition_…` |
| advisory projection | `intelligence_advisory::derive_advisory_context` | — | `AdvisoryContext` (A3) | second advisory model | `inv_no_overall_state_…` |
| architecture provenance WHY | `architecture_why::explain` | — | `ArchitectureWhy` | second explanation subsystem | `inv_advisory_…` |
| policy effects | `AuthorityEngine` / Governance only | — | — | Alignment→Authority | `inv_misaligned_…`, `false_authority_…` |

**Exactly one semantic authority per concept.** No competing engine
decides the same thing.

<shell evidence for the single-graph claim>
```
$ grep -rc 'impl SemanticGraphProjection for' crates/sddk-engine/src/semantic_graph.rs
1
```

## §4 Legacy / compatibility audit

| Facade | Disposition | Evidence |
|---|---|---|
| `paradigm_lens::evaluate_lens()` | **NO_RUNTIME_CONSUMER** + `DEFER_REMOVAL_A5` | Only referenced by tests (`paradigm_lens/tests.rs`, `architecture_receipt/tests.rs` — a `mod tests;` module, `a4_4m_convergence_pins.rs`) and by a **comment** in `intelligence_loop/mod.rs`. No production consumer anywhere in `crates/*/src/`. Marked `LEGACY_READ_COMPAT`. |

```text
$ grep -rl 'evaluate_lens' crates/*/src/ | grep -v 'paradigm_lens/'
crates/sddk-engine/src/intelligence_loop/mod.rs        # comment only
crates/sddk-engine/src/architecture_receipt/tests.rs   # mod tests;
```

Never `SECOND_AUTHORITY`. The production path is
`AlignmentLensRegistry → AlignmentLensKernel → production ParadigmLens`,
and `LensInput.concern == LensContribution.concern` is pinned
(`inv_lens_concern_preservation`).

## §5 Cross-spec invariant matrix

`crates/sddk-engine/tests/a4_closeout_milestone_audit.rs` (19 tests):

| Invariant | Test | Class |
|---|---|---|
| Evidence ≠ truth; Observation ≠ decision | `inv_epistemology_evidence_is_a_reference_not_a_truth` | OBSERVED |
| NotApplicable ≠ Unknown ≠ Insufficient ≠ NotEvaluated | `inv_epistemic_states_are_mutually_distinct` | OBSERVED |
| contradictory evidence coexists (no latest-wins/majority) | `inv_contradiction_coexists_without_aggregation` | OBSERVED |
| Applicability ≠ Grounding ≠ Evaluability | `inv_applicability_grounding_evaluability_are_distinct` | OBSERVED |
| Unit("x") ≠ Component("x") ≠ Entity("x") | `inv_namespace_identity_is_strict` | OBSERVED |
| requested concern C → contribution concern C | `inv_lens_concern_preservation` | OBSERVED |
| SpecifiedBy ≠ VerifiedBy; absence ≠ negative | `inv_provenance_axes_and_absence` | OBSERVED |
| one SemanticGraphProjection (no second graph) | §3 shell pin + authority map | STRUCTURAL |
| composition ≠ orchestration authority | `inv_composition_is_not_orchestration_authority` | OBSERVED |
| advisory does not mutate instruction hash | `inv_advisory_does_not_mutate_instructions` | OBSERVED |
| MISALIGNED ↛ Authority/Capability/InstructionSource | `inv_misaligned_never_escalates_authority` | STRUCTURAL |

## §6 End-to-end milestone UAT

Real chain (no mocks): `KnowledgeBasis → ObservationSet → VerifyKernel →
DebVerifyKernel → AlignmentLensKernel → reduce_alignment →
compose_intelligence_loop → derive_advisory_context → explain_advisory`.

| Scenario | Test | Result |
|---|---|---|
| Clean: Verified + ConfirmedBaseline + Aligned | `uat_milestone_clean_chain_is_preserved` | OBSERVED PASS |
| Adversarial: Unknown + EvidenceGap + Conflicted lens + MISALIGNED | `uat_milestone_adversarial_chain_is_not_collapsed` | OBSERVED PASS (all four epistemologies survive; not collapsed to FAIL/DENY) |

Governance is outside the chain.

## §7 False-clean audit

| Path probed | Test | Result |
|---|---|---|
| missing evidence never → aligned/confirmed | `false_clean_unknown_never_becomes_aligned_or_confirmed` | 0 reachable |
| missing lens never → supported | `false_clean_missing_lens_never_becomes_supported` | 0 reachable |
| unknown contract never → confirmed/verified | `false_clean_unknown_contract_never_becomes_confirmed_verified` | 0 reachable |
| unsupported target → NotApplicable/Insufficient, not Supported | `false_clean_unsupported_target_is_not_applicable_not_supported` | 0 reachable |

**0 false-clean paths proven reachable** (OBSERVED, not grep-only).

## §8 False-authority audit

| Surface | Test | Result |
|---|---|---|
| Verify / DebVerify / Alignment / Lens / Loop / Advisory modules | `false_authority_no_provider_or_governance_dependency` | no provider/Governance/AuthorityDecision |
| `AlignmentAssessment` | `false_authority_alignment_has_no_authority_conversion` | no conversion to authority |

**No authority escalation.** arch-spec-047's claim still holds across the
milestone.

## §9 Receipt chain audit

| Artifact | State class |
|---|---|
| `docs/architecture/a4-4c-acceptance-receipt.md` | PROJECTION |
| `docs/architecture/receipts/A4-5C-arch-spec-047-acceptance-receipt.md` | PROJECTION |
| 20 per-cycle A4 handoffs (`docs/history/handoffs/all-handoffs/HANDOFF-*-a4-*`) | DOCUMENTED / handoff (not authority) |
| Release tags `v1.169.40..v1.169.67` | canonical Git facts (FACT) |
| PublicReleaseGate evidence (release.sh step 9b) | OBSERVED at each release |
| **This A4 Milestone Receipt** | PROJECTION / certification artifact |

receipt ≠ handoff ≠ release ≠ canonical fact. None of these is a runtime
authority.

## §10 Follow-up reconciliation

| Item | Invalidates A4 semantics? | Breaks A4 acceptance? | A5 disposition |
|---|---|---|---|
| `FU-A3-CO-1` (relation payload encoding) | no | no | DEFER_A5 |
| `FU-A3-CO-3` (rename) | no | no | DEFER_A5 |
| `FU-A3-S15-4` (fitness rule / CLI lint) | no | no | DEFER_A5 |
| `ASC-MA-1` (`--help` UX) | no | no | DEFER_A5 |
| `uat_stale_tests::stale_detects_geometry_change` (flake) | no | no | DEFER_A5 |
| `INC-A4-RELEASE-VERSION-DRIFT` (low) | no | no | DEFER_A5 (remediation already enforced by release.sh 1c + 9b) |

None is an A4 blocker. None was fixed here.

## §11 No hidden P1

- Open debt records: `INC-A4-RELEASE-VERSION-DRIFT` (low),
  `INC-DEBT-023` (low). `INCIDENCE-TEMPLATE.md` is a template.
- `INC-HX-AUTH-001/003/004` are `status: closed`.
- **0 undisposed P1 blockers** relevant to A4.

## Test evidence (§17)

```text
cargo build --release -p sddk-cli                   OK
cargo fmt --all -- --check                          OK
cargo clippy --workspace --all-targets -D warnings  OK
cargo test --workspace                              PASSED=4697 FAILED=0 IGNORED=14
cargo test -p sddk-engine --test a4_closeout_milestone_audit   19 passed
```

The A4-CLOSEOUT corpus is run **explicitly**, not only as part of the
workspace run.

## §12 Spec status final

| Spec | Status |
|---|---|
| 042 | `implemented` |
| 043 | `implemented` |
| 044 | `implemented` |
| 045 | `implemented` |
| 046 | `implemented` |
| 047 | `implemented` |

frontmatter == body claims == runtime reality.

## §14 Claim → evidence

```text
OBSERVED:  a real milestone UAT crosses Knowledge → Observation → Verify →
           DebVerify → AlignmentLens → reduce_alignment → loop → advisory.
OBSERVED:  the adversarial chain preserves Unknown/EvidenceGap/Conflicted/
           MISALIGNED without collapsing to a single verdict.
OBSERVED:  advisory content leaves effective_instruction_set_hash unchanged.
OBSERVED:  0 false-clean paths reachable across the probed routes.

STRUCTURAL: Governance is outside the Intelligence Loop.
STRUCTURAL: one SemanticGraphProjection; no second graph/store.
STRUCTURAL: no authority-driven or provider-coupled module in the A4 core.

OBSERVED + STRUCTURAL: MISALIGNED never becomes DENY.
OBSERVED + STRUCTURAL: SpecifiedBy and VerifiedBy remain distinct.

DOCUMENTED: three specs had documentation drift (042/045/046) reconciled
            here without touching runtime; deferred A5 debt has explicit
            disposition.

DERIVED:    A5 may inherit A4 as its certified semantic baseline. This does
            NOT mean "production ready" — that is A5's job.
```

## §15 A5 readiness gate

**Can A5-PLAN start from a semantically certified A4 baseline?** Yes:
final disposition is `A4_CERTIFIED`.

Roadmap disposition:

```text
A4-CLOSEOUT  CLOSED
A4           CLOSED / CERTIFIED
A5-PLAN      NEXT
```

`BASE_PRODUCTION_READY` is **not** claimed; A5 still owns hardening.

## §16 Release (certified immutable baseline for A5)

| Field | Value |
|---|---|
| `released_baseline` | `v1.169.67` → `1949fa8448b636ffdc4fe2fea02339d82922481c` |
| `development_head` (at open, full) | `730f855428f89c7232b8390ce0d96e791e6a68b0` |
| `workspace_version` | `1.169.68` |
| `release_target` | `main` |
| `actual_release_tag` | `v1.169.68` |
| `release_sha` | `3bad25275212f77c3d0d4d664f4d49293aa779c9` |
| `binary_sha256` | `349e8f3d22ae9fce4eb1a088157411f00635f16481f9a945d6866de826eed9b2` |
| PublicReleaseGate | PASS (draft=false, prerelease=false, 9/9 assets, doctor `all_present: true`) |

The productive diff of this cycle is **zero** (docs + tests only). The
release exists because A5 must inherit one immutable certified point.

## Final disposition

# `A4_CERTIFIED`

Meaning (exactly): all A4 normative contracts proven; single authorities
preserved; no unresolved A4 blocker; deferred debt explicitly belongs to
A5. It does **not** mean software is production-ready.
