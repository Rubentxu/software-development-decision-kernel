// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// paradigm_lens/types.rs — A3-S8 / AC7 public types.
//
// The lens vocabulary AC7 owns: observation families, polarity, evaluation
// basis and provenance. AC3's closed enums (ParadigmLensKind, LensStatus,
// EvidenceBasis) are consumed unmodified.

use crate::evidence_ref::EvidenceRef;
use crate::paradigm_profile::{EvidenceBasis, ParadigmLensKind};
use serde::{Deserialize, Serialize};

/// The published lens-system version (AC-035-007: cite the lens version).
pub const LENS_VERSION: &str = "ac7.lens.v1";

// ─────────────────────────────────────────────────────────────────────────────
// LensFamily (AC7-owned; AC3's kinds map onto it)
// ─────────────────────────────────────────────────────────────────────────────

/// AC7's own lens-family vocabulary (AC-035-002/003).
///
/// Deliberately separate from AC3's 11-variant `ParadigmLensKind`: AC7 is
/// extensible without touching the profile ontology.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LensFamily {
    /// Object-oriented design lens.
    ObjectOriented,
    /// Functional / pure-functional lens.
    Functional,
    /// Algebraic-data-type / modelling lens.
    Adt,
    /// Typed-DSL design lens.
    Dsl,
}

impl LensFamily {
    /// All families.
    pub const ALL: [Self; 4] = [Self::ObjectOriented, Self::Functional, Self::Adt, Self::Dsl];

    /// Canonical short tag.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::ObjectOriented => "object_oriented",
            Self::Functional => "functional",
            Self::Adt => "adt",
            Self::Dsl => "dsl",
        }
    }

    /// The `ParadigmLensKind` list that activates this family, or `None` when
    /// the declared kind has no AC7 lens.
    pub fn declared_kinds(self) -> &'static [ParadigmLensKind] {
        match self {
            // `ObjectOriented` profiles activate the OO lens.
            Self::ObjectOriented => &[ParadigmLensKind::ObjectOriented],
            // Both functional variants activate the functional lens.
            Self::Functional => &[
                ParadigmLensKind::Functional,
                ParadigmLensKind::FunctionalPure,
            ],
            // Data-oriented profiles activate the ADT/modelling lens.
            Self::Adt => &[ParadigmLensKind::DataOriented],
            // Pipeline and Custom-carrying profiles activate the DSL lens.
            Self::Dsl => &[ParadigmLensKind::Pipeline, ParadigmLensKind::Custom],
        }
    }
}

/// Which AC7 family a declared profile kind activates, if any.
pub fn family_for_kind(kind: ParadigmLensKind) -> Option<LensFamily> {
    LensFamily::ALL
        .into_iter()
        .find(|f| f.declared_kinds().contains(&kind))
}

// ─────────────────────────────────────────────────────────────────────────────
// ObservationPolarity (2-closed)
// ─────────────────────────────────────────────────────────────────────────────

/// Whether an observation is evidence for or against the declared paradigm.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ObservationPolarity {
    /// Evidence consistent with the declared paradigm.
    Supports,
    /// Evidence contradicting the declared paradigm.
    Contradicts,
}

impl ObservationPolarity {
    /// Canonical short tag.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::Supports => "supports",
            Self::Contradicts => "contradicts",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// LensObservation (closed, REQ-AC7-001/002)
// ─────────────────────────────────────────────────────────────────────────────

/// The closed observation vocabulary the four lenses consume.
///
/// Each variant names one dimension from
/// `04-PARADIGM-LENSES.md` §Object-oriented / §Functional / §ADT / §DSL.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LensObservation {
    // ── Object-oriented ────────────────────────────────────────────────────
    /// Encapsulation / invariant ownership is present.
    EncapsulationPresent,
    /// Dependency inversion is used at a boundary.
    DependencyInversionUsed,
    /// Behaviourless data holders (anemic model / transaction script).
    AnemicModelDetected,
    /// Inheritance used where composition would serve.
    InheritanceOverComposition,

    // ── Functional / pure-functional ───────────────────────────────────────
    /// Immutable values dominate.
    ImmutableValuesPresent,
    /// Effects are explicit at boundaries.
    EffectsAtBoundary,
    /// Hidden mutation: interior mutability or `&mut` in a declared-pure unit.
    HiddenMutationDetected,
    /// Sentinel / stringly control flow where a typed alternative exists.
    SentinelControlFlowDetected,

    // ── ADT ───────────────────────────────────────────────────────────────
    /// A typed sum type models the state space.
    TypedSumTypePresent,
    /// Invalid states remain representable (independent optional fields).
    InvalidStatesRepresentable,
    /// Boolean flags stand in for a state machine.
    BooleanBlindness,
    /// Identifiers/statuses/actions are stringly typed.
    StringlyTypedStatus,

    // ── DSL ───────────────────────────────────────────────────────────────
    /// An explicit typed AST/IR exists.
    TypedAstPresent,
    /// Authoring syntax is separated from execution effects.
    SyntaxSeparatedFromEffects,
    /// Validation happens before execution.
    ValidatesBeforeExecution,
    /// Invalid programs remain representable in the syntax.
    InvalidProgramsRepresentable,
}

impl LensObservation {
    /// All variants.
    pub const ALL: [Self; 16] = [
        Self::EncapsulationPresent,
        Self::DependencyInversionUsed,
        Self::AnemicModelDetected,
        Self::InheritanceOverComposition,
        Self::ImmutableValuesPresent,
        Self::EffectsAtBoundary,
        Self::HiddenMutationDetected,
        Self::SentinelControlFlowDetected,
        Self::TypedSumTypePresent,
        Self::InvalidStatesRepresentable,
        Self::BooleanBlindness,
        Self::StringlyTypedStatus,
        Self::TypedAstPresent,
        Self::SyntaxSeparatedFromEffects,
        Self::ValidatesBeforeExecution,
        Self::InvalidProgramsRepresentable,
    ];

    /// The family that consumes this observation (REQ-AC7-001).
    pub fn family(self) -> LensFamily {
        match self {
            Self::EncapsulationPresent
            | Self::DependencyInversionUsed
            | Self::AnemicModelDetected
            | Self::InheritanceOverComposition => LensFamily::ObjectOriented,

            Self::ImmutableValuesPresent
            | Self::EffectsAtBoundary
            | Self::HiddenMutationDetected
            | Self::SentinelControlFlowDetected => LensFamily::Functional,

            Self::TypedSumTypePresent
            | Self::InvalidStatesRepresentable
            | Self::BooleanBlindness
            | Self::StringlyTypedStatus => LensFamily::Adt,

            Self::TypedAstPresent
            | Self::SyntaxSeparatedFromEffects
            | Self::ValidatesBeforeExecution
            | Self::InvalidProgramsRepresentable => LensFamily::Dsl,
        }
    }

    /// Whether this observation supports or contradicts the family's paradigm.
    pub fn polarity(self) -> ObservationPolarity {
        match self {
            Self::EncapsulationPresent
            | Self::DependencyInversionUsed
            | Self::ImmutableValuesPresent
            | Self::EffectsAtBoundary
            | Self::TypedSumTypePresent
            | Self::TypedAstPresent
            | Self::SyntaxSeparatedFromEffects
            | Self::ValidatesBeforeExecution => ObservationPolarity::Supports,

            Self::AnemicModelDetected
            | Self::InheritanceOverComposition
            | Self::HiddenMutationDetected
            | Self::SentinelControlFlowDetected
            | Self::InvalidStatesRepresentable
            | Self::BooleanBlindness
            | Self::StringlyTypedStatus
            | Self::InvalidProgramsRepresentable => ObservationPolarity::Contradicts,
        }
    }

    /// A concrete locator string used as the assessment's evidence citation.
    pub fn locator(self) -> &'static str {
        match self {
            Self::EncapsulationPresent => "lens.oo.encapsulation",
            Self::DependencyInversionUsed => "lens.oo.dependency_inversion",
            Self::AnemicModelDetected => "lens.oo.anemic_model",
            Self::InheritanceOverComposition => "lens.oo.inheritance_over_composition",
            Self::ImmutableValuesPresent => "lens.fp.immutable_values",
            Self::EffectsAtBoundary => "lens.fp.effects_at_boundary",
            Self::HiddenMutationDetected => "lens.fp.hidden_mutation",
            Self::SentinelControlFlowDetected => "lens.fp.sentinel_control_flow",
            Self::TypedSumTypePresent => "lens.adt.typed_sum_type",
            Self::InvalidStatesRepresentable => "lens.adt.invalid_states",
            Self::BooleanBlindness => "lens.adt.boolean_blindness",
            Self::StringlyTypedStatus => "lens.adt.stringly_typed_status",
            Self::TypedAstPresent => "lens.dsl.typed_ast",
            Self::SyntaxSeparatedFromEffects => "lens.dsl.syntax_separation",
            Self::ValidatesBeforeExecution => "lens.dsl.validate_before_execute",
            Self::InvalidProgramsRepresentable => "lens.dsl.invalid_programs_representable",
        }
    }

    /// An `EvidenceRef` citing this observation (AC-035-007).
    ///
    /// Lens observations are planning-analysis evidence, hence
    /// `EvidenceKind::Planning` (the only kind that fits a heuristic
    /// architecture observation).
    pub fn evidence_ref(self) -> EvidenceRef {
        EvidenceRef::new(crate::evidence_ref::EvidenceKind::Planning, self.locator())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// LensEvaluationBasis (2-closed) + provenance
// ─────────────────────────────────────────────────────────────────────────────

/// How an assessment was produced (REQ-AC7-003).
///
/// AC7's own basis: AC3's `EvidenceBasis` is frozen at 5 variants, so the
/// roadmap's `INFERRED` lives here and maps onto AC3's vocabulary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LensEvaluationBasis {
    /// A deterministic heuristic produced the observations.
    Deterministic,
    /// An (optional) LLM assessment with provenance.
    Inferred,
}

impl LensEvaluationBasis {
    /// All variants.
    pub const ALL: [Self; 2] = [Self::Deterministic, Self::Inferred];

    /// Map onto AC3's frozen `EvidenceBasis`.
    pub fn to_evidence_basis(self) -> EvidenceBasis {
        match self {
            Self::Deterministic => EvidenceBasis::Observed,
            Self::Inferred => EvidenceBasis::Declared,
        }
    }

    /// Canonical short tag.
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::Deterministic => "deterministic",
            Self::Inferred => "inferred",
        }
    }
}

/// Provenance for an inferred (LLM) assessment (AC-035-007).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LensProvenance {
    /// Who produced the assessment (must be non-empty).
    pub evaluator: String,
    /// The model identifier; required for inferred assessments.
    pub model: Option<String>,
    /// Digest of the input the inference saw (must be non-zero for inferred).
    pub input_digest: [u8; 32],
    /// The lens-system version.
    pub lens_version: String,
}

impl LensProvenance {
    /// A deterministic provenance (no model, zero digest allowed).
    pub fn deterministic(evaluator: impl Into<String>) -> Self {
        Self {
            evaluator: evaluator.into(),
            model: None,
            input_digest: [0u8; 32],
            lens_version: LENS_VERSION.to_string(),
        }
    }

    /// Whether the provenance is complete for an *inferred* assessment.
    pub fn is_complete_for_inference(&self) -> bool {
        !self.evaluator.trim().is_empty()
            && self.model.as_ref().is_some_and(|m| !m.trim().is_empty())
            && self.input_digest != [0u8; 32]
            && !self.lens_version.trim().is_empty()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Errors
// ─────────────────────────────────────────────────────────────────────────────

/// Errors surfaced while producing lens assessments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LensError {
    /// An inferred assessment was requested without complete provenance.
    MissingProvenance {
        /// Which field is missing or empty.
        field: &'static str,
    },
}

impl std::fmt::Display for LensError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingProvenance { field } => write!(
                f,
                "paradigm_lens: inferred assessment is missing provenance field `{field}`"
            ),
        }
    }
}

impl std::error::Error for LensError {}
