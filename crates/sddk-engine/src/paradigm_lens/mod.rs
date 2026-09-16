// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// paradigm_lens/mod.rs — A3-S8 / AC7 public surface.
//
// Paradigm lenses: deterministic/heuristic evaluation of OO, functional,
// ADT/modelling and typed-DSL design intent against concrete observations.
//
// Cycle: `p-63676b11dc0ef88f/a3-8-ac7-paradigm-lenses` (A3-S8)
// Spec: `docs/architecture/specs/arch-spec-A3-S8-ac7-paradigm-lenses.md`
// Upstream: `arch-spec-035-paradigm-lens-system` (remains `proposed`)
// ADR: `ADR-0118-PARADIGM-LENS-EVALUATION`
//
// # What AC7 adds to AC3
//
// A3-S4 (AC3) delivered the paradigm **data** and deferred evaluation:
// assessments defaulted to `LensStatus::Unknown` / `EvidenceBasis::Declared`.
// AC7 is that deferred evaluation: it produces AC3 `LensAssessment` values with
// a real status and concrete evidence citations, without modifying AC3's closed
// vocabularies.
//
// # The INFERRED problem
//
// AC3's `EvidenceBasis` is frozen at exactly 5 variants (REQ-AC3-004) and does
// not contain `INFERRED`. AC7 therefore owns a 2-closed
// `LensEvaluationBasis` (`Deterministic | Inferred`) and maps onto AC3's
// vocabulary: `Deterministic -> Observed`, `Inferred -> Declared` (intent-only),
// with provenance carried in a typed `LensProvenance` and in `notes`.
//
// # Advisory boundary (AC-035-005, AC-UAT-011)
//
// Lens status is advisory. This module imports no authority/capability/
// instruction types, writes no `EffectiveInstructions`, grants no capability,
// performs no IO, calls no LLM/provider, and mutates no graph. `evaluate_lens`
// is scoped to the declared anchor and emits exactly one assessment — it never
// produces a global paradigm judgment (AC-UAT-012).
//
// # State classes (ADR-0095)
//
// - `LensAssessment` — PROJECTION (AC3-owned shape; AC7 produces values).
// - `LensEvaluation`, `LensObservation`, `ObservationPolarity`,
//   `LensEvaluationBasis`, `LensProvenance` — EPHEMERAL computed values.
//
// # Submodules
//
// - `types` — families, observations, polarity, basis, provenance.
// - `lenses` — `evaluate_lens` compatibility facade (A4-4M M4/M5).
// - `translation` — typed AC7→substrate bridge (A4-4M M2).
// - `probes` — deterministic source heuristics.
// - `tests` — acceptance (AC-UAT-011..015) + anti-encroachment pins.

pub mod lenses;
pub mod probes;
pub mod types;

#[cfg(test)]
mod tests;

pub mod translation;

pub use lenses::{LensEvaluation, evaluate_lens};
pub use probes::{
    probe_adt_observations, probe_dsl_observations, probe_functional_observations,
    probe_oo_observations,
};
pub use types::{
    LENS_VERSION, LensError, LensEvaluationBasis, LensFamily, LensObservation, LensProvenance,
    ObservationPolarity, family_for_kind,
};
