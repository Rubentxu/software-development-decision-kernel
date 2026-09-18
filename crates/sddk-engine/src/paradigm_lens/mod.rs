// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// paradigm_lens/mod.rs — A3-S8 / AC7 data path + A4-4M bridge.
//
// Paradigm lens data: deterministic/heuristic *evidence* for OO, functional,
// ADT/modelling and typed-DSL design intent. The typed observations and the
// translation to the canonical observation substrate are the production data
// path consumed by `alignment_lens::paradigm` lenses.
//
// Cycle: `p-63676b11dc0ef88f/a3-8-ac7-paradigm-lenses` (A3-S8) + A4-4M M2
// Spec: `docs/architecture/specs/arch-spec-A3-S8-ac7-paradigm-lenses.md`
// Bridge spec: `docs/architecture/a4-4m-migration-matrix.md` §M0/M2
// ADR: `ADR-0118-PARADIGM-LENS-EVALUATION`, `ADR-0125-GENERIC-ALIGNMENT-LENS-KERNEL-REGISTRY`
//
// # A5-4a disposition
//
// The legacy `evaluate_lens()` compatibility facade and the legacy
// `LensEvaluation` wrapper were retired in A5-4a:
//   - `paradigm_lens::lenses` module is DELETED.
//   - `evaluate_lens()` is gone.
//   - `LensEvaluation` struct is gone; the receipt composer now
//     takes `&[paradigm_profile::LensAssessment]` (the AC3 shape).
// The AC7-specific deterministic-projection semantics that the facade
// produced live on: the kernel pathway consumes the same
// `SoftwareObservation`s via the M0 translation, and the substrate
// posture + projection that the facade implemented is the SAME
// projection the kernel uses (pinned in `a4_4m_m0_migration_proof.rs`).
//
// # State classes (ADR-0095)
//
// - `LensObservation`, `LensFamily`, `ObservationPolarity`,
//   `LensEvaluationBasis`, `LensProvenance` — EPHEMERAL computed values
//   used by the bridge to land typed observations in the canonical
//   substrate. They are NOT semantic state of any architecture; they
//   describe each individual deterministic observation emitted by a
//   probe.
// - `probe_*` — EPHEMERAL computation that emits typed observations.
// - `translation::translate[_batch]` — pure transformation between two
//   representation layers (typed observations → `SoftwareObservation`).
//
// No `LensAssessment` is produced here: that is the AC3 shape and is
// built by callers (receipt composer, test fixtures) directly.
//
// # Submodules
//
// - `types` — families, observations, polarity, basis, provenance.
// - `probes` — deterministic source heuristics.
// - `translation` — typed AC7→substrate bridge (A4-4M M2).

pub mod probes;
pub mod translation;
pub mod types;

pub use probes::{
    probe_adt_observations, probe_dsl_observations, probe_functional_observations,
    probe_oo_observations,
};
pub use types::{
    LENS_VERSION, LensError, LensEvaluationBasis, LensFamily, LensObservation, LensProvenance,
    ObservationPolarity, family_for_kind,
};
