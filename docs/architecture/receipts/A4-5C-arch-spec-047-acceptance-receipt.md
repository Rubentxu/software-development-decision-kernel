# A4-5C — arch-spec-047 Acceptance Receipt

> **Cycle:** `p-63676b11dc0ef88f-a4-5c-arch-spec-047-acceptance`
> **Date:** 2026-09-17
> **Budget:** ACCEPTANCE / RECEIPT / UAT ONLY — zero production semantics.
>
> This receipt is **evidence / projection**. It is **not** a runtime
> authority and introduces no production type (`ArchSpec047Receipt` does
> not exist and must not).

## Identity

| Field | Value |
|---|---|
| Certified spec | `docs/architecture/specs/arch-spec-047-a4-intelligence-loop.md` |
| Certified spec revision (git blob) | `5690b7de0a8b437e9dbc54db358d1e9a4b2c78bf` |
| Certified spec sha256 | `3227edc80c30a4688d960a74bc02b1d375f6b7a2e8d992b484996fe1df86974c` |
| Released baseline | `v1.169.66` → `aa3aa91c11794baf9b5bce34cb26e704d1c7d59b` |
| Development head (cycle open) | `9253a817f4b294b97d5ecd8488ad21ca9bc7b2ef` |
| Scope contract | `.sddk/cycles/p-63676b11dc0ef88f-a4-5c-arch-spec-047-acceptance/spec.md` |
| Corpus | `crates/sddk-engine/tests/a4_5c_arch_spec_047_acceptance.rs` |

## Evidence classes

`OBSERVED` — a runtime test exercised the real behaviour and asserted it.
`STRUCTURAL` — a source/type pin; the property is enforced by construction.
`DERIVED` — follows from OBSERVED/STRUCTURAL evidence, not directly executed.
`DOCUMENTED` — recorded in prose/ADR; no behavioural claim.

A clause with only `STRUCTURAL` evidence is **not** presented as observed.

## Normative-clause matrix

| Clause | Claim | Evidence class | Concrete evidence | Result |
|---|---|---|---|---|
| N1 | The loop composes existing authority outputs, in order, without owning them. | STRUCTURAL | `n1_n7_loop_is_composition_only_and_receives_outputs` (loop module names no kernel; `IntelligenceLoopInputs` carries data) | PASS |
| N2 | Governance enters only after the loop, never inside it. | STRUCTURAL | `n2_governance_is_not_in_the_loop_or_advisory_modules` | PASS |
| N3 | Never `MISALIGNED → DENY`. | OBSERVED + STRUCTURAL | `n3_n6_n17_misaligned_is_advisory_only`, `uat_5_misaligned_without_deny` | PASS |
| N4 | Never `Alignment → Authority`. | STRUCTURAL | `n2_…`, `sec17_n18_…`, `sec4_epistemic_domains_are_distinct_types` | PASS |
| N5 | Never `Alignment → Capability`. | STRUCTURAL | `n2_…`, `sec17_n18_…` | PASS |
| N6 | Never `Alignment → InstructionSource`. | OBSERVED + STRUCTURAL | `sec21_advisory_does_not_change_instruction_identity`, `n2_…` | PASS |
| N7 | Provenance precedes interpretation; conclusions are traceable. | OBSERVED | `sec13_absence_is_not_negation`, `sec14_provenance_axes_are_independent`, `uat_1`, `uat_2` | PASS |
| N8 | Spec-ID reconciliation recorded (042+), no behaviour. | DOCUMENTED | spec §"Spec-ID reconciliation (A4-0 record)" | PASS |
| N9 | One content-addressed receipt; composition receives, never calls. | OBSERVED + STRUCTURAL | `sec20_receipt_is_order_independent_and_semantically_sensitive`, `n1_n7_…`; A4-5a corpus | PASS |
| N10 | Advisory completeness: one item per source; nothing lost. | OBSERVED | `sec9_advisory_cardinality_matches_every_source` | PASS |
| N11 | Typed WHY trace (`basis`/`legs`/`unresolved`/`why_not`). | OBSERVED | `sec12`, `sec13`, `sec18`, `sec19` | PASS |
| N12 | `AdvisoryKind` carries the loop kinds. | OBSERVED | `sec10_advisory_kinds_are_exhaustively_covered` | PASS |
| N13 | `contract_provenance` read-only, two axes. | OBSERVED | `sec14_…`, `sec15_contract_provenance_is_read_only_and_order_independent` | PASS |
| N14 | `SpecifiedBy` does not imply `VerifiedBy`. | OBSERVED | `sec13_…`, `sec14_…`, `uat_2_specified_but_not_verified` | PASS |
| N15 | `absence != negation`. | OBSERVED | `sec13_absence_is_not_negation`, `uat_2_…` | PASS |
| N16 | WHY explains; it never strengthens the conclusion. | OBSERVED | `sec12_why_never_strengthens_the_conclusion` | PASS |
| N17 | `MISALIGNED != DENY`; no authority/capability/instruction coupling. | OBSERVED + STRUCTURAL | `n3_n6_n17_…`, `sec21_…` | PASS |
| N18 | No hidden orchestration. | STRUCTURAL | `sec17_n18_advisory_code_has_no_string_semantics_or_orchestration` | PASS |
| N19 | No global verdict field. | OBSERVED | `sec4_no_overall_verdict_field_anywhere_on_the_surface` | PASS |
| §4 | Epistemic domains are not collapsed. | OBSERVED + STRUCTURAL | `sec4_…` (distinct types + no overall token) | PASS |
| §5 | Verify states survive without reinterpretation. | OBSERVED | `sec5_verify_states_survive_without_reinterpretation` | PASS |
| §6 | DebVerify variants survive; no cross-domain translation. | OBSERVED | `sec6_reconciliation_variants_survive_and_stay_distinct` | PASS |
| §7 | Lens contributions **and** coverage gaps survive. | OBSERVED | `sec7_contributions_and_coverage_gaps_survive` | PASS |
| §8 | Alignment states survive; `UNKNOWN != NOT_APPLICABLE`. | OBSERVED | `sec8_alignment_states_survive_and_stay_distinct` | PASS |
| §9 | Advisory completeness + cardinality; no silent drop. | OBSERVED | `sec9_…` | PASS |
| §10 | `AdvisoryKind` exhaustive; no wildcard/`Unknown` fallback. | STRUCTURAL | `sec10_…` | PASS |
| §11 | Advisory determinism (order-independent). | OBSERVED | `sec11_advisory_is_order_independent` | PASS |
| §12 | WHY is explanatory only. | OBSERVED | `sec12_…` | PASS |
| §13 | `absence != negation` UAT. | OBSERVED | `sec13_…`, `uat_2_…` | PASS |
| §14 | Provenance axes independent (0/1/N). | OBSERVED | `sec14_…` | PASS |
| §15 | `contract_provenance` falsification. | OBSERVED | `sec15_…` | PASS |
| §16 | `EvidenceKind` codec fail-closed. | OBSERVED | `sec16_evidence_kind_codec_is_fail_closed` | PASS |
| §17 | Typed WHY legs; no string reconstruction. | STRUCTURAL | `sec17_n18_…` | PASS |
| §18 | WHY-NOT describes the gap, not an invented cause. | OBSERVED | `sec18_why_not_describes_the_gap` | PASS |
| §19 | Clock stability of the basis. | OBSERVED | `sec19_basis_is_clock_stable` | PASS |
| §20 | Receipt acceptance (order + sensitivity + clock). | OBSERVED | `sec20_…`; A4-5a corpus | PASS |
| §21 | Advisory isolation: capsule hash changes, instruction hash does not. | OBSERVED | `sec21_…` | PASS |
| §22 | `MISALIGNED`-without-`DENY` UAT. | OBSERVED | `uat_5_misaligned_without_deny` | PASS |
| §23 | Conflicting knowledge coexists (no latest-wins). | OBSERVED | `sec23_conflicting_knowledge_coexists` | PASS |
| §24 | Incomplete knowledge stays visible (no false-clean). | OBSERVED | `sec24_incomplete_knowledge_is_not_false_clean` | PASS |
| §25 | Projection rebuild deterministic; stale edges do not survive. | OBSERVED | `sec25_rebuild_is_deterministic_and_stale_edges_do_not_survive` | PASS |
| §26 | State classification unchanged. | DOCUMENTED + STRUCTURAL | `sec26_state_classes_are_unchanged`; ADR-0126 / ADR-0128 | PASS |
| §27 | No hidden orchestration. | STRUCTURAL | `sec17_n18_…` | PASS |
| §28 | Production-path UAT (real kernels). | OBSERVED | `production_path::*` (real `VerifyKernel` + `AlignmentLensKernel` + `reduce_alignment` + `compose_intelligence_loop`) | PASS |
| §29 | UAT-1..6. | OBSERVED | `uat_1_happy_evidenced` … `uat_6_order_and_rebuild_determinism` | PASS |
| §30 | Legacy facade not on the acceptance path. | STRUCTURAL | `sec30_sec31_…` (source pin) + positive `ParadigmLens::ALL` | PASS |
| §31 | No provider required for BASE. | STRUCTURAL | `sec30_sec31_…` | PASS |

**Every normative clause is PASS. No MUST is `NOT_PROVEN`.**

## UAT commands

```bash
cargo test -p sddk-engine --test a4_5c_arch_spec_047_acceptance
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Test evidence

- A4-5C acceptance corpus: **34 tests** (all green).
- Full workspace: `PASSED=4678 FAILED=0 IGNORED=14`.

## Negative evidence (the clauses that assert what must NOT happen)

- `sec4` / §4: no `OverallStatus`/`QualityScore`/`ConfidenceScore`/`verdict`
  token on the advisory surface.
- `sec12` / N16: no `Failed`/`Denied`/`Misaligned`/`Unsafe` strengthening.
- `sec13` / N15: no "not verified"/"unverified" positive claim from an empty
  evidence axis.
- `sec16` / §16: unknown tag → `None`, never a nearest/default match.
- `sec17_n18` / N18: the advisory module names no kernel and uses no
  `contains`/`starts_with`/`split`/`parse`.
- `sec18` / §18: no invented causal narrative ("forgot", "never tested").
- `sec23` / §23: contradictory + conflicted + misaligned all survive together.
- `sec24` / §24: incomplete knowledge never reports clean/aligned.
- `sec21` / N6: advisory changes never move `effective_instruction_set_hash`.
- `sec30_sec31` / §30/§31: no legacy facade, no provider on the path.

## Anti-encroachment evidence

- The loop and advisory modules contain no reference to
  `Governance`, `AuthorityEngine`, `AuthorityDecision`, `Capability`,
  `InstructionSource`, `InstructionCompiler`, `VerifyKernel`,
  `DebVerifyKernel`, `AlignmentLensKernel`, `reduce_alignment`,
  `paradigm_lens`, `CogniCode`, `Chronos`, `JCode`, `reqwest`
  (`n1_n7_…`, `n2_…`, `sec17_n18_…`, `sec30_sec31_…`).
- `derive_advisory_context`/`explain_advisory` receive their inputs; they
  never call a producer.
- `ContextCapsule` proves the instructions/advisory separation at runtime
  (`sec21_…`).

## Scope discipline

- Budget honoured: `ACCEPTANCE / RECEIPT / UAT ONLY`.
- No production semantics changed. No new state, finding, advisory type,
  graph relation, provider, orchestration, authority, or verdict.
- No `STOP` was triggered: every clause was demonstrable against the
  **current** behaviour. No corrective slice is required.

## Deferred debt (unchanged; NOT absorbed)

`FU-A3-CO-1`, `FU-A3-CO-3`, `FU-A3-S15-4`, `ASC-MA-1`, the stale UAT
flake and `INC-A4-RELEASE-VERSION-DRIFT` stay `DEFER_A5`. None blocked
acceptance.

## Promotion

Because every MUST clause PASSes, `arch-spec-047` is promoted:

```yaml
status: implemented
implemented_by: A4-5a (v1.169.64); A4-S15R (v1.169.65); A4-5b (v1.169.66); A4-5C acceptance (this receipt)
```

## Release

| Field | Value |
|---|---|
| Released baseline | `v1.169.66` → `aa3aa91c…` |
| Development head | `9253a817…` |
| Workspace version | `<filled at release>` |
| Actual release tag | `<filled at release>` |
| Release SHA | `<filled at release>` |

## Unresolved items

None blocking. A4-CLOSEOUT (cross-cutting A4 audit 042→047) is the next
cycle and is **not** auto-opened.
