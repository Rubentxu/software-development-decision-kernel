// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// paradigm_lens/lenses.rs — A3-S8 / AC7 evaluation.
//
// `evaluate_lens` turns a declared paradigm profile plus deterministic
// observations into an AC3 `LensAssessment` with a real status and concrete
// evidence citations. It is scoped to the declared anchor and never emits a
// global paradigm judgment (AC-UAT-012).
//
// Rules (AC-035-004):
//   undeclared family                  -> NotApplicable
//   declared family, zero observations -> Unknown
//   all Supports                       -> Aligned
//   all Contradicts                    -> Misaligned
//   mixed                              -> Tension

use crate::paradigm_profile::{
    EvidenceBasis, LensAssessment, LensStatus, ParadigmAnchorRef, ParadigmLensKind,
};

use super::types::{
    LENS_VERSION, LensError, LensEvaluationBasis, LensObservation, LensProvenance,
    ObservationPolarity, family_for_kind,
};

/// The result of a deterministic lens evaluation: the AC3 assessment plus the
/// AC7-level basis/provenance that AC3's frozen vocabulary cannot express.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LensEvaluation {
    /// The AC3-shaped assessment (status + basis + evidence).
    pub assessment: LensAssessment,
    /// AC7's own basis (`Deterministic` | `Inferred`).
    pub basis: LensEvaluationBasis,
    /// Provenance (always populated; AC-035-007).
    pub provenance: LensProvenance,
    /// The observations that contributed, in input order.
    pub used_observations: Vec<LensObservation>,
}

fn notes_for(family_tag: &str, used: usize, basis: LensEvaluationBasis) -> String {
    format!(
        "lens_version={LENS_VERSION}; family={family_tag}; basis={}; observations={used}",
        basis.canonical_tag()
    )
}

fn status_from_polarities(used: &[LensObservation]) -> LensStatus {
    if used.is_empty() {
        return LensStatus::Unknown;
    }
    let supports = used
        .iter()
        .filter(|o| o.polarity() == ObservationPolarity::Supports)
        .count();
    let contradicts = used.len() - supports;
    match (supports > 0, contradicts > 0) {
        (_, false) => LensStatus::Aligned,
        (false, true) => LensStatus::Misaligned,
        (true, true) => LensStatus::Tension,
    }
}

/// Evaluate a lens deterministically.
///
/// `declared_kind` is the profile's declared lens kind (AC3 vocabulary);
/// `observations` is whatever the caller gathered (extra families are ignored).
pub fn evaluate_lens(
    declared_kind: ParadigmLensKind,
    anchor: ParadigmAnchorRef,
    observations: &[LensObservation],
    evaluated_at_ms: i64,
) -> LensEvaluation {
    let family = family_for_kind(declared_kind);
    let used: Vec<LensObservation> = match family {
        Some(f) => observations
            .iter()
            .copied()
            .filter(|o| o.family() == f)
            .collect(),
        None => Vec::new(),
    };

    let status = match family {
        // Undeclared family: nothing to assess (AC-UAT-013).
        None => LensStatus::NotApplicable,
        Some(_) => status_from_polarities(&used),
    };

    let basis = LensEvaluationBasis::Deterministic;
    let provenance = LensProvenance::deterministic("sddk.paradigm_lens.deterministic");
    let family_tag = family.map(|f| f.canonical_tag()).unwrap_or("none");

    let assessment = LensAssessment {
        lens: declared_kind,
        anchor,
        status,
        basis: basis.to_evidence_basis(),
        // Cite only the contradicting/ supporting observations that were used.
        evidence_refs: used.iter().map(|o| o.evidence_ref()).collect(),
        notes: Some(notes_for(family_tag, used.len(), basis)),
        evaluated_at_ms,
    };

    LensEvaluation {
        assessment,
        basis,
        provenance,
        used_observations: used,
    }
}

/// Build an *inferred* (LLM-supplied) assessment (AC-035-007).
///
/// AC7 performs no inference itself: it validates the caller's provenance and
/// shapes the result. Incomplete provenance is rejected (REQ-AC7-016..018).
pub fn inferred_lens_assessment(
    declared_kind: ParadigmLensKind,
    anchor: ParadigmAnchorRef,
    observations: &[LensObservation],
    provenance: LensProvenance,
    evaluated_at_ms: i64,
) -> Result<LensEvaluation, LensError> {
    if provenance.evaluator.trim().is_empty() {
        return Err(LensError::MissingProvenance { field: "evaluator" });
    }
    if provenance
        .model
        .as_ref()
        .map(|m| m.trim().is_empty())
        .unwrap_or(true)
    {
        return Err(LensError::MissingProvenance { field: "model" });
    }
    if provenance.input_digest == [0u8; 32] {
        return Err(LensError::MissingProvenance {
            field: "input_digest",
        });
    }
    if provenance.lens_version.trim().is_empty() {
        return Err(LensError::MissingProvenance {
            field: "lens_version",
        });
    }

    let family = family_for_kind(declared_kind);
    let used: Vec<LensObservation> = match family {
        Some(f) => observations
            .iter()
            .copied()
            .filter(|o| o.family() == f)
            .collect(),
        None => Vec::new(),
    };
    let status = match family {
        None => LensStatus::NotApplicable,
        Some(_) => status_from_polarities(&used),
    };
    let basis = LensEvaluationBasis::Inferred;
    let family_tag = family.map(|f| f.canonical_tag()).unwrap_or("none");

    let assessment = LensAssessment {
        lens: declared_kind,
        anchor,
        status,
        // `Inferred` maps onto AC3's `Declared` (intent-only, no deterministic
        // observation backs it).
        basis: EvidenceBasis::Declared,
        evidence_refs: used.iter().map(|o| o.evidence_ref()).collect(),
        notes: Some(format!(
            "{}; evaluator={}; model={}",
            notes_for(family_tag, used.len(), basis),
            provenance.evaluator,
            provenance.model.as_deref().unwrap_or("<none>")
        )),
        evaluated_at_ms,
    };

    Ok(LensEvaluation {
        assessment,
        basis,
        provenance,
        used_observations: used,
    })
}
