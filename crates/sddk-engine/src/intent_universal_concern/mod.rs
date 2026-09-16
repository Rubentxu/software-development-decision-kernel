//! Intent + UniversalConcern — A4-4a model surface.
//!
//! **Status:** A4-4a model (per `arch-spec-046` part-1).
//! Implements: closed 10-member `UniversalConcern` vocabulary,
//! typed intent representation (`ProjectIntent`, `UnitIntent`),
//! `ApplicableConcern` answer, `ParadigmProfileRef`,
//! and a pure `applicable_concerns()` reducer.
//!
//! **MISALIGNED ≠ DENY** (architectural invariant from A4-3):
//! this module produces *descriptive* answers (what is worth looking
//! at for this unit, given the declared intent). It cannot grant
//! capability, cannot call into the AuthorityEngine, and imports
//! zero symbols from `authority` / `capability` / `lens` /
//! `UniversalConcern` / `provider_sdk` surfaces.
//!
//! **NOT shipped in A4-4a** (anti-encroachment):
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
    ApplicableConcern, ApplicableReason, ContractRefs, DecisionRefs, NotApplicableReason,
    ParadigmProfileRef, ProjectIntent, UnitIntent, UniversalConcern,
};
