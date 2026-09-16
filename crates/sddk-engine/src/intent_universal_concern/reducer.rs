//! Pure reducer for `applicable_concerns()`.
//!
//! A4-4aR reducer rules (intent-only applicability; no string-grounding;
//! paradigm does NOT erase a concern):
//!
//! For each `UniversalConcern` in the closed 10-member vocabulary:
//!
//! 1. If `project_intent.excluded_concerns` contains the concern,
//!    emit `NotApplicable(c, ExplicitlyExcludedByProject)`.
//! 2. Else if `project_intent.declared_concerns` does NOT contain the concern,
//!    emit `NotApplicable(c, NotInProjectIntent)`.
//! 3. Else if `unit_intent.excluded_concerns` contains the concern,
//!    emit `NotApplicable(c, ExplicitlyExcludedByUnit)`.
//! 4. Else if `unit_intent.applies_to_concerns` does NOT contain the concern,
//!    emit `NotApplicable(c, NotInUnitIntent)`.
//! 5. Else emit `Applicable(c)` with reason `ProjectAndUnitIntent`.
//!
//! **Anti-knowledge:** this reducer NEVER inspects `DecisionRef`,
//! `ContractId`, paradigm×concern relevance tables, or any textual
//! representation. Grounding and Evaluability are *not* Applicability —
//! see A4-4b (Lens kernel) and A4-4aR §10 epistemic pin.

use crate::intent_universal_concern::types::{
    ApplicableConcern, ApplicableReason, NotApplicableReason, ProjectIntent, UnitIntent,
    UniversalConcern,
};

/// Typed refusal paths for `applicable_concerns()`. Mirrors the A4-3
/// `ReductionError` style (closed enum, no strings).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ReductionError {
    /// The provided `ProjectIntent` has no declared concerns, no excluded
    /// concerns, and a degenerate `Custom` paradigm. Refusing to emit an
    /// empty `Vec<ApplicableConcern>` because the answer is structurally
    /// undefined.
    EmptyProjectIntent,
    /// The provided `UnitIntent.unit_ref` is empty.
    EmptyUnitRef,
}

/// Compute the list of applicable concerns for a unit, given its declared
/// intent and the surrounding project intent.
///
/// The output is **deterministic** for a given `(project_intent, unit_intent)`
/// pair: same inputs → same output. **No wall clock.** **No label text.**
/// Sorted by `ApplicableConcern::canonical()`.
///
/// Length is bounded by `|UniversalConcern| = 10`.
///
/// A4-4aR: the signature dropped `unit_contracts` and `unit_decisions` —
/// Applicability is *intent-only*. Grounding concerns are A4-4b.
pub fn applicable_concerns(
    project_intent: &ProjectIntent,
    unit_intent: &UnitIntent,
) -> Result<Vec<(ApplicableConcern, Option<ApplicableReason>)>, ReductionError> {
    // ── Refusal: empty inputs ──────────────────────────────────────────
    let pi_fully_empty = project_intent.declared_concerns.is_empty()
        && project_intent.excluded_concerns.is_empty()
        && project_intent.paradigm
            == crate::intent_universal_concern::types::ParadigmProfileRef::Custom;
    if pi_fully_empty {
        return Err(ReductionError::EmptyProjectIntent);
    }
    if unit_intent.unit_ref.0.is_empty() {
        return Err(ReductionError::EmptyUnitRef);
    }

    let mut out: Vec<(ApplicableConcern, Option<ApplicableReason>)> =
        Vec::with_capacity(UniversalConcern::ALL.len());

    for concern in UniversalConcern::ALL {
        // Rule 1: project explicit exclusion (overrides everything else).
        if project_intent.excluded_concerns.contains(&concern) {
            out.push((
                ApplicableConcern::NotApplicable(
                    concern,
                    NotApplicableReason::ExplicitlyExcludedByProject,
                ),
                None,
            ));
            continue;
        }

        // Rule 2: project declared_concerns gate.
        if !project_intent.declared_concerns.contains(&concern) {
            out.push((
                ApplicableConcern::NotApplicable(concern, NotApplicableReason::NotInProjectIntent),
                None,
            ));
            continue;
        }

        // Rule 3: unit explicit exclusion.
        if unit_intent.excluded_concerns.contains(&concern) {
            out.push((
                ApplicableConcern::NotApplicable(
                    concern,
                    NotApplicableReason::ExplicitlyExcludedByUnit,
                ),
                None,
            ));
            continue;
        }

        // Rule 4: unit applies_to_concerns gate.
        if !unit_intent.applies_to_concerns.contains(&concern) {
            out.push((
                ApplicableConcern::NotApplicable(concern, NotApplicableReason::NotInUnitIntent),
                None,
            ));
            continue;
        }

        // Rule 5: applicable.
        out.push((
            ApplicableConcern::Applicable(concern),
            Some(ApplicableReason::ProjectAndUnitIntent),
        ));
    }

    // Deterministic sort by canonical().
    out.sort_by(|a, b| a.0.canonical().cmp(&b.0.canonical()));

    Ok(out)
}

#[cfg(test)]
mod tests {
    //! Pure-function tests in `tests.rs` exercise the contract; this in-file
    //! module is intentionally empty to keep the reducer file focused.
}
