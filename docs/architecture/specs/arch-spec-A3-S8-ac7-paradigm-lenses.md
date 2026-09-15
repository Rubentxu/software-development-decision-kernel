---
id: arch-spec-A3-S8-ac7-paradigm-lenses
status: cycle-bounded
supersedes_history: false
proposed_at: 2026-09-15
accepted_at: null
proposed_by_cycle: p-63676b11dc0ef88f/a3-8-ac7-paradigm-lenses
source: arch-spec-035-paradigm-lens-system + ADR-0114-PARADIGMS-AS-ALIGNMENT-LENSES
based_on: docs/SDDK-Architecture-Conformance-Graph-Evolution-2026-09-14/04-PARADIGM-LENSES.md
---

# arch-spec-A3-S8 — Paradigm Lens Assessments

## Intent

AC7 of the Architecture Conformance track (per `09-ROADMAP.md`):

> Implement deterministic/heuristic lenses for OO, functional/pure-functional,
> ADT and DSL design. LLM evaluation is optional and must emit `INFERRED`
> assessment with provenance.

A3-S4 delivered the paradigm **data** and deferred evaluation to AC7:

> AC3 is data-only: assessment status defaults to `UNKNOWN` with
> `EvidenceBasis::Declared`; **lens evaluation belongs to AC7**.

Inherits `arch-spec-035` (stays `proposed`).

## The `INFERRED`-vs-frozen-vocabulary problem

`REQ-AC3-004` pins AC3's `EvidenceBasis` to **exactly 5** variants with a
`ALL.len() == 5` test. The roadmap's `INFERRED` is not among them, and AC7 must
not extend AC3's enum.

**Resolution.** AC7 owns a 2-closed `LensEvaluationBasis`
(`Deterministic | Inferred`) and maps onto AC3's frozen vocabulary:

| AC7 basis | AC3 `EvidenceBasis` | When |
|---|---|---|
| `Deterministic` | `Observed` | a heuristic probe produced the observations |
| `Deterministic` (exhaustive) | `Verified` | the check is total over the input |
| `Inferred` | `Declared` | intent-only; no deterministic observation backs it |

Provenance (evaluator, model, input digest, lens version) travels in a typed
`LensProvenance` plus `notes`.

## Scope (must)

- `LensObservation` closed vocabulary covering the four lens families.
- `ObservationPolarity` (`Supports | Contradicts`) and `Observation::lens()`.
- `LensEvaluationBasis` (2-closed) + `LensProvenance` + `LENS_VERSION`.
- `evaluate_lens(...) -> LensAssessment` producing real statuses.
- Deterministic source probes for the four families.
- `inferred_lens_assessment(...)` that validates provenance and rejects an
  inferred assessment without it.
- AC7 emits AC3's `LensAssessment`; AC3's types are consumed unmodified.

## Scope (must NOT)

- MUST NOT modify AC3's closed enums (`LensStatus`, `EvidenceBasis`,
  `ParadigmLensKind`) or any `paradigm_profile` source file.
- MUST NOT import `authority_engine`, `capability`, `effective_instructions`,
  `provider`, `host_sdk`, `agent_host`, `alignment`, `debverify`.
- MUST NOT call an LLM/provider or perform network IO.
- MUST NOT emit a global paradigm judgment; every assessment is scoped to the
  declared anchor (AC-UAT-012).
- MUST NOT write EffectiveInstructions or grant capability (AC-035-005).

## Requirements (REQ-AC7-NNN)

### Vocabulary

- **REQ-AC7-001** — `LensObservation` is closed and every variant reports the
  `ParadigmLensKind` family that consumes it and an `ObservationPolarity`.
  Pin: `acceptance_observations_have_family_and_polarity`.
- **REQ-AC7-002** — The four lens families have at least three observations each.
  Pin: `acceptance_every_family_has_observations`.
- **REQ-AC7-003** — `LensEvaluationBasis` is closed with exactly 2 variants;
  `to_evidence_basis()` maps onto AC3's frozen `EvidenceBasis` per the table
  above. Pin: `acceptance_basis_maps_onto_ac3`.
- **REQ-AC7-004** — `LENS_VERSION` is a published constant and every
  `LensProvenance` carries it (AC-035-007).
  Pin: `acceptance_provenance_carries_lens_version`.

### Evaluation (AC-035-004, AC-UAT-012, AC-UAT-013)

- **REQ-AC7-005** — A family that is **not declared** by the profile yields
  `LensStatus::NotApplicable`. Pin: `acceptance_absent_declaration_is_not_applicable`
  (AC-UAT-013).
- **REQ-AC7-006** — A declared family with **zero** observations yields
  `LensStatus::Unknown` (declared but unobserved), never `Misaligned`.
  Pin: `acceptance_declared_without_evidence_is_unknown`.
- **REQ-AC7-007** — All observations `Supports` ⇒ `Aligned`; all `Contradicts`
  ⇒ `Misaligned`; mixed ⇒ `Tension`.
  Pin: `acceptance_status_from_polarity`.
- **REQ-AC7-008** — The assessment is scoped to the declared anchor and lens;
  it never generalises (AC-UAT-012).
  Pin: `acceptance_assessment_is_scoped_and_not_global`.
- **REQ-AC7-009** — The assessment cites the contradicting observations as
  evidence (AC-035-007). Pin: `acceptance_evidence_cites_observations`.
- **REQ-AC7-010** — Deterministic: identical inputs ⇒ identical assessment.
  Pin: `acceptance_evaluation_is_deterministic`.

### Probes (AC-UAT-014)

- **REQ-AC7-011** — `probe_adt_observations(source)` detects stringly-typed
  status/kind fields, two-or-more optional fields on one struct, and boolean
  flags. Pin: `acceptance_adt_probe_stringly_and_invalid_state`.
- **REQ-AC7-012** — `probe_functional_observations(source)` detects hidden
  mutation (`&mut self`, `let mut`, `Cell<`, `RefCell<`) and immutable values.
  Pin: `acceptance_fp_probe_detects_hidden_mutation` (AC-UAT-013).
- **REQ-AC7-013** — `probe_oo_observations(source)` detects encapsulation and
  anemic models. Pin: `acceptance_oo_probe_detects_anemic_model`.
- **REQ-AC7-014** — `probe_dsl_observations(source)` detects a typed AST/IR and
  a validate-before-execute ordering. Pin: `acceptance_dsl_probe_typed_ast_and_validation`
  (AC-UAT-015).
- **REQ-AC7-015** — Every probe has a negative control: a clean source yields no
  contradicting observation. Pin: `acceptance_probes_have_negative_controls`.

### Inferred assessments (AC-035-007)

- **REQ-AC7-016** — `inferred_lens_assessment(...)` accepts a non-empty
  provenance and emits `basis = Inferred → EvidenceBasis::Declared` with the
  provenance in `notes`. Pin: `acceptance_inferred_requires_provenance`.
- **REQ-AC7-017** — An inferred assessment with empty evaluator or zero
  `input_digest` is rejected with `LensError::MissingProvenance`.
  Pin: `acceptance_inferred_without_provenance_is_rejected`.
- **REQ-AC7-018** — Deterministic evaluations do not require a model id;
  inferred ones do (`model: Option<String>` must be `Some`).
  Pin: `acceptance_inferred_requires_model_id`.

### Advisory boundary (AC-UAT-011, AC-035-005)

- **REQ-AC7-019** — The module imports no authority/capability/instruction
  types. Pin: `anti_encroachment_no_authority_or_instruction_imports`.
- **REQ-AC7-020** — AC3's `paradigm_profile` sources are unmodified by this
  cycle: the closed vocabularies still have 7/5/11 variants.
  Pin: `anti_encroachment_ac3_vocabularies_unchanged`.
- **REQ-AC7-021** — No LLM/provider/network call. Pin:
  `anti_encroachment_no_provider_calls`.
- **REQ-AC7-022** — The output is an advisory `LensAssessment`; no
  policy/instruction object is produced. Pin: `anti_encroachment_advisory_only`.

## Acceptance tests (planned, 22)

| # | Test | REQ |
|---|---|---|
| 1 | `acceptance_observations_have_family_and_polarity` | 001 |
| 2 | `acceptance_every_family_has_observations` | 002 |
| 3 | `acceptance_basis_maps_onto_ac3` | 003 |
| 4 | `acceptance_provenance_carries_lens_version` | 004 |
| 5 | `acceptance_absent_declaration_is_not_applicable` | 005 |
| 6 | `acceptance_declared_without_evidence_is_unknown` | 006 |
| 7 | `acceptance_status_from_polarity` | 007 |
| 8 | `acceptance_assessment_is_scoped_and_not_global` | 008 |
| 9 | `acceptance_evidence_cites_observations` | 009 |
| 10 | `acceptance_evaluation_is_deterministic` | 010 |
| 11 | `acceptance_adt_probe_stringly_and_invalid_state` | 011 |
| 12 | `acceptance_fp_probe_detects_hidden_mutation` | 012 |
| 13 | `acceptance_oo_probe_detects_anemic_model` | 013 |
| 14 | `acceptance_dsl_probe_typed_ast_and_validation` | 014 |
| 15 | `acceptance_probes_have_negative_controls` | 015 |
| 16 | `acceptance_inferred_requires_provenance` | 016 |
| 17 | `acceptance_inferred_without_provenance_is_rejected` | 017 |
| 18 | `acceptance_inferred_requires_model_id` | 018 |
| 19 | `anti_encroachment_no_authority_or_instruction_imports` | 019 |
| 20 | `anti_encroachment_ac3_vocabularies_unchanged` | 020 |
| 21 | `anti_encroachment_no_provider_calls` | 021 |
| 22 | `anti_encroachment_advisory_only` | 022 |

## Determinism contract

Identical `(declared_kind, anchor, observations, evaluated_at_ms)` yields an
identical `LensAssessment`.

## State classes (ADR-0095)

| Value | Class |
|---|---|
| `LensAssessment` | PROJECTION (AC3-owned shape; AC7 produces values) |
| `LensObservation`, `ObservationPolarity`, `LensEvaluationBasis`, `LensProvenance` | EPHEMERAL computed values / closed enums |
