//! Intent + UniversalConcern — A4-4a model surface.
//!
//! **Status:** A4-4a corrected model (A4-4aR — Applicability Semantics Correction).
//! Implements: closed 10-member `UniversalConcern` vocabulary,
//! typed intent representation (`ProjectIntent`, `UnitIntent`, each with
//! `declared_concerns` and explicit `excluded_concerns`),
//! `ApplicableConcern` answer, `ParadigmProfileRef`,
//! and a pure `applicable_concerns()` reducer.
//!
//! **A4-4aR applicability boundary:** `applicable_concerns()` answers
//! only *declared-scope applicability*. It does not inspect decisions,
//! contracts, or paradigm×concern relevance. Grounding and Evaluability
//! belong to A4-4b (AlignmentLens kernel) and A4-5.
//!
//! **MISALIGNED ≠ DENY** (architectural invariant from A4-3):
//! this module produces *descriptive* answers (what is worth looking
//! at for this unit, given the declared intent). It cannot grant
//! capability, cannot call into the AuthorityEngine, and imports
//! zero symbols from `authority` / `capability` / `lens` /
//! `UniversalConcern` / `provider_sdk` surfaces.
//!
//! **NOT shipped in A4-4aR** (anti-encroachment; same surface as A4-4a,
//! reduced by removing the now-unused `DecisionRefs` / `ContractRefs`
//! wrapper types):
//! - `AlignmentLens` trait or ADT (A4-4b)
//! - Any concrete lens strategy (A4-4b / A4-4M)
//! - Any wiring into `software_alignment::reduce_alignment`
//! - Any CLI surface
//! - Any authority / capability wiring
//! - Any mutation of the existing `paradigm_lens` registry (A4-4M)

#![forbid(unsafe_code)]

pub mod reducer;
pub mod types;

#[cfg(test)]
mod tests;

pub use reducer::{ReductionError, applicable_concerns};
pub use types::{
    ApplicableConcern, ApplicableReason, NotApplicableReason, ParadigmProfileRef, ProjectIntent,
    UnitIntent, UniversalConcern,
};
