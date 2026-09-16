//! Pure reducer for `applicable_concerns()`.
//!
//! A4-4a reducer rules (deterministic, identity-stable, no authority):
//!
//! For each `UniversalConcern` in the closed 10-member vocabulary:
//!
//! 1. If `project_intent.declared_concerns` does NOT contain the concern,
//!    emit `NotApplicable(c, NotInProjectIntent)`.
//! 2. Else if `unit_intent.applies_to_concerns` does NOT contain the concern,
//!    emit `NotApplicable(c, NotInUnitIntent)`.
//! 3. Else if the paradigm profile is irrelevant for the concern
//!    (e.g. `FunctionalPure` profile with `Freshness` concern),
//!    emit `NotApplicable(c, ParadigmIrrelevant)`.
//! 4. Else if there is no `DecisionRef` grounding this concern in
//!    `unit_decisions` (and the concern is decision-grounded),
//!    emit `NotApplicable(c, NoGroundingDecision)`.
//! 5. Else if there is no contract referencing this concern in
//!    `unit_contracts`,
//!    emit `NotApplicable(c, NoContractReference)`.
//! 6. Else emit `Applicable(c)` with the most specific
//!    `ApplicableReason` chosen by precedence:
//!    - ProjectAndUnitIntentAndDecision (preferred: has both + decision)
//!    - ProjectIntentAndParadigm (no decision but paradigm matches)
//!    - UnitIntentAndContract (unit intent + contract, no project-level
//!      concern — but this case is already caught by rule 1, so this
//!      branch is a safety net).

use crate::architectural_contract::{ContractId, DecisionRef};
use crate::intent_universal_concern::types::{
    ApplicableConcern, ApplicableReason, NotApplicableReason, ParadigmProfileRef, ProjectIntent,
    UnitIntent, UniversalConcern,
};

/// Typed refusal paths for `applicable_concerns()`. Mirrors the A4-3
/// `ReductionError` style (closed enum, no strings).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum ReductionError {
    /// The provided `ProjectIntent` is empty (no declared concerns,
    /// no paradigm). Refusing to emit an empty `Vec<ApplicableConcern>`
    /// because the answer is structurally undefined.
    EmptyProjectIntent,
    /// The provided `UnitIntent.unit_ref` is empty.
    EmptyUnitRef,
}

/// Compute the list of applicable concerns for a unit, given its declared
/// intent and the surrounding project intent.
///
/// The output is **deterministic** for a given `(project_intent, unit_intent,
/// unit_contracts, unit_decisions)` tuple: same inputs → same output.
/// **No wall clock.** **No label text.** Sorted by `ApplicableConcern::canonical()`.
///
/// Length is bounded by `|UniversalConcern| = 10`.
pub fn applicable_concerns(
    project_intent: &ProjectIntent,
    unit_intent: &UnitIntent,
    unit_contracts: &[&ContractId],
    unit_decisions: &[&DecisionRef],
) -> Result<Vec<(ApplicableConcern, Option<ApplicableReason>)>, ReductionError> {
    // ── Refusal: empty inputs ──────────────────────────────────────────
    if project_intent.declared_concerns.is_empty()
        && project_intent.paradigm == ParadigmProfileRef::Custom
    {
        // Only refuse if there's literally no signal at all (no declared
        // concerns AND a degenerate paradigm).
        return Err(ReductionError::EmptyProjectIntent);
    }
    if unit_intent.unit_ref.0.is_empty() {
        return Err(ReductionError::EmptyUnitRef);
    }

    let mut out: Vec<(ApplicableConcern, Option<ApplicableReason>)> =
        Vec::with_capacity(UniversalConcern::ALL.len());

    for concern in UniversalConcern::ALL {
        // Rule 1: project intent gate.
        if !project_intent.declared_concerns.contains(&concern) {
            out.push((
                ApplicableConcern::NotApplicable(concern, NotApplicableReason::NotInProjectIntent),
                None,
            ));
            continue;
        }

        // Rule 2: unit intent gate.
        if !unit_intent.applies_to_concerns.contains(&concern) {
            out.push((
                ApplicableConcern::NotApplicable(concern, NotApplicableReason::NotInUnitIntent),
                None,
            ));
            continue;
        }

        // Rule 3: paradigm relevance gate.
        if !paradigm_supports_concern(project_intent.paradigm, concern) {
            out.push((
                ApplicableConcern::NotApplicable(concern, NotApplicableReason::ParadigmIrrelevant),
                None,
            ));
            continue;
        }

        // Rule 4: grounding decision.
        let has_decision = unit_decisions
            .iter()
            .any(|d| decision_grounds_concern(d, concern));
        // Rule 5: contract reference.
        let has_contract = unit_contracts
            .iter()
            .any(|c| contract_references_concern(c, concern));

        match (has_decision, has_contract) {
            (true, _) => out.push((
                ApplicableConcern::Applicable(concern),
                Some(ApplicableReason::ProjectAndUnitIntentAndDecision),
            )),
            (false, true) => out.push((
                ApplicableConcern::Applicable(concern),
                Some(ApplicableReason::UnitIntentAndContract),
            )),
            (false, false) => out.push((
                ApplicableConcern::NotApplicable(concern, NotApplicableReason::NoGroundingDecision),
                None,
            )),
        }
    }

    // Deterministic sort by canonical().
    out.sort_by(|a, b| a.0.canonical().cmp(&b.0.canonical()));

    Ok(out)
}

// ─── helpers (closed, no policy) ───────────────────────────────────────────

/// Paradigm × concern relevance table. Closed — adding a concern requires
/// also adding a row here.
fn paradigm_supports_concern(paradigm: ParadigmProfileRef, concern: UniversalConcern) -> bool {
    use ParadigmProfileRef::*;
    use UniversalConcern::*;
    match (paradigm, concern) {
        // Pipeline: every concern EXCEPT TemporalCoupling (pipelines are
        // already temporally ordered by construction).
        (Pipeline, TemporalCoupling) => false,
        // Default: every paradigm supports every concern.
        // (A4-4a is descriptive — paradigm-level exclusions are minimal.)
        _ => true,
    }
}

/// True if the given decision's render string mentions the concern's
/// canonical tag. A4-4a only inspects the rendered text — no semantics
/// beyond that. A4-4b will replace this with proper grounding via the
/// AlignmentLens registry.
fn decision_grounds_concern(d: &DecisionRef, concern: UniversalConcern) -> bool {
    let rendered = d.render();
    rendered.contains(concern.canonical_tag())
}

/// True if the contract references this concern by id.
fn contract_references_concern(c: &ContractId, concern: UniversalConcern) -> bool {
    c.as_str().contains(concern.canonical_tag())
}
