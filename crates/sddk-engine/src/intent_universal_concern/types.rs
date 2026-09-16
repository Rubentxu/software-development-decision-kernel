//! Types for the Intent + UniversalConcern model (A4-4a).
//!
//! Closed vocabularies, typed accessors, and identity-stable ids.
//! **Zero policy.** This file is descriptive, not prescriptive.

use serde::Serialize;
use std::collections::BTreeSet;

use crate::architectural_contract::{ContractId, DecisionRef};
use crate::architecture_graph::SoftwareUnitRef;
use crate::knowledge::EventTime;
use crate::paradigm_profile::ParadigmLensKind;

// ─── UniversalConcern (10-member closed enum) ───────────────────────────────

/// Closed vocabulary of *what is worth looking at* for any unit of software.
///
/// These are descriptive categories of concern, not policies. Whether a
/// given concern is *applicable* to a specific unit (given its declared
/// intent and the evidence) is what `applicable_concerns()` answers.
///
/// `arch-spec-046` part-1 + A4-4a scope contract.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub enum UniversalConcern {
    Cohesion,
    Coupling,
    BoundaryIntegrity,
    StateSafety,
    EffectVisibility,
    DependencyDirection,
    SemanticOwnership,
    TemporalCoupling,
    Testability,
    Freshness,
}

impl UniversalConcern {
    /// The closed list of all concerns, in canonical order.
    pub const ALL: [Self; 10] = [
        Self::Cohesion,
        Self::Coupling,
        Self::BoundaryIntegrity,
        Self::StateSafety,
        Self::EffectVisibility,
        Self::DependencyDirection,
        Self::SemanticOwnership,
        Self::TemporalCoupling,
        Self::Testability,
        Self::Freshness,
    ];

    /// Canonical short tag (used in `canonical()` and identity hashing).
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::Cohesion => "cohesion",
            Self::Coupling => "coupling",
            Self::BoundaryIntegrity => "boundary_integrity",
            Self::StateSafety => "state_safety",
            Self::EffectVisibility => "effect_visibility",
            Self::DependencyDirection => "dependency_direction",
            Self::SemanticOwnership => "semantic_ownership",
            Self::TemporalCoupling => "temporal_coupling",
            Self::Testability => "testability",
            Self::Freshness => "freshness",
        }
    }

    /// Stable string identity, equal to `canonical_tag()`.
    pub fn canonical(self) -> String {
        self.canonical_tag().to_string()
    }
}

// ─── Intent representation ──────────────────────────────────────────────────

/// Declared intent for the project as a whole.
///
/// `intent_id` is the stable identity of the intent (content-addressed).
/// `paradigm` is the project's declared paradigm profile (e.g. OO, FP, ADT,
/// Typed DSL). `declared_concerns` is the *project's* declared subset of
/// `UniversalConcern` — the concerns the project itself considers relevant.
/// This is the project's *policy*, separate from the descriptive model in
/// `applicable_concerns()`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct ProjectIntent {
    pub intent_id: IntentId,
    pub paradigm: ParadigmProfileRef,
    pub declared_concerns: BTreeSet<UniversalConcern>,
}

/// Declared intent for a specific unit, within a `ProjectIntent`.
///
/// `unit_ref` is the identity of the unit being declared for.
/// `applies_to_concerns` is the unit's declared subset — narrower than
/// the project's declared_concerns.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct UnitIntent {
    pub unit_ref: SoftwareUnitRef,
    pub applies_to_concerns: BTreeSet<UniversalConcern>,
}

/// Content-addressed identity for an intent.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct IntentId(pub String);

impl IntentId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// ─── Reference types ────────────────────────────────────────────────────────

/// Typed reference to a paradigm profile. A4-4a *references* but does not
/// define — profile definitions live in `paradigm_profile` and the registry
/// in `paradigm_lens` (A4-4M will converge them).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub enum ParadigmProfileRef {
    /// Object-oriented profile.
    ObjectOriented,
    /// Functional / pure-functional profile.
    Functional,
    /// Pure-functional profile (subset of Functional).
    FunctionalPure,
    /// Algebraic-data-type / data-oriented profile.
    DataOriented,
    /// Pipeline-style profile.
    Pipeline,
    /// Custom user-declared profile.
    Custom,
}

impl ParadigmProfileRef {
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::ObjectOriented => "object_oriented",
            Self::Functional => "functional",
            Self::FunctionalPure => "functional_pure",
            Self::DataOriented => "data_oriented",
            Self::Pipeline => "pipeline",
            Self::Custom => "custom",
        }
    }

    /// Map to the existing `ParadigmLensKind`. A4-4a only references the
    /// 6 closed variants; `EventDriven`, `Reactive`, `Hexagonal`, `Ddd`,
    /// `ActorLike` are extension surfaces (A4-4b will bridge them).
    /// Returns `None` for the A4-4a closed set.
    pub fn as_paradigm_lens_kind(self) -> Option<ParadigmLensKind> {
        match self {
            Self::ObjectOriented => Some(ParadigmLensKind::ObjectOriented),
            Self::Functional => Some(ParadigmLensKind::Functional),
            Self::FunctionalPure => Some(ParadigmLensKind::FunctionalPure),
            Self::DataOriented => Some(ParadigmLensKind::DataOriented),
            Self::Pipeline => Some(ParadigmLensKind::Pipeline),
            Self::Custom => Some(ParadigmLensKind::Custom),
        }
    }
}

/// Typed accessor for the `AcceptedDecision` references a concern decision
/// was grounded on. A4-4a *references* — `AcceptedDecision` itself lives in
/// `software_alignment` (A4-3).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
pub struct DecisionRefs(pub Vec<DecisionRef>);

/// Typed accessor for the `ArchitecturalContract` references a concern
/// decision was grounded on. A4-4a *references* — contracts live in
/// `architectural_contract`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
pub struct ContractRefs(pub Vec<ContractId>);

// ─── ApplicableConcern answer ───────────────────────────────────────────────

/// The descriptive answer to "is this concern applicable to this unit,
/// given the declared intent?".
///
/// A4-4a is descriptive, not prescriptive: applicability is *not* a
/// capability, denial, or authority decision.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum ApplicableConcern {
    /// This concern is applicable to the unit.
    Applicable(UniversalConcern),
    /// This concern is not applicable, for the given reason.
    NotApplicable(UniversalConcern, NotApplicableReason),
}

impl ApplicableConcern {
    /// The concern this answer is about.
    pub fn concern(&self) -> UniversalConcern {
        match self {
            Self::Applicable(c) | Self::NotApplicable(c, _) => *c,
        }
    }

    /// True if the answer is Applicable.
    pub fn is_applicable(&self) -> bool {
        matches!(self, Self::Applicable(_))
    }

    /// True if the answer is NotApplicable.
    pub fn is_not_applicable(&self) -> bool {
        matches!(self, Self::NotApplicable(_, _))
    }

    /// Canonical (concern, reason) string for sorting and identity.
    pub fn canonical(&self) -> String {
        match self {
            Self::Applicable(c) => format!("{}:applicable", c.canonical_tag()),
            Self::NotApplicable(c, r) => {
                format!("{}:not_applicable:{}", c.canonical_tag(), r.canonical_tag())
            }
        }
    }
}

/// Reason a concern is not applicable to a given unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub enum NotApplicableReason {
    /// The project's declared_concerns does not include this concern.
    NotInProjectIntent,
    /// The unit's applies_to_concerns does not include this concern.
    NotInUnitIntent,
    /// No accepted decision grounds this concern for this unit.
    NoGroundingDecision,
    /// No contract references this concern for this unit.
    NoContractReference,
    /// The paradigm profile does not produce observations of this concern.
    ParadigmIrrelevant,
}

impl NotApplicableReason {
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::NotInProjectIntent => "not_in_project_intent",
            Self::NotInUnitIntent => "not_in_unit_intent",
            Self::NoGroundingDecision => "no_grounding_decision",
            Self::NoContractReference => "no_contract_reference",
            Self::ParadigmIrrelevant => "paradigm_irrelevant",
        }
    }
}

/// The typed reason a particular `ApplicableConcern::Applicable` answer
/// was produced — i.e. the *positive* reason alongside the boolean answer.
/// (A4-4a emits this in the `applicable_reason` side channel for
/// traceability.)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub enum ApplicableReason {
    /// Project intent includes the concern AND unit intent includes the
    /// concern AND there is at least one grounding decision.
    ProjectAndUnitIntentAndDecision,
    /// Project intent includes the concern AND the unit's paradigm profile
    /// produces observations of this concern.
    ProjectIntentAndParadigm,
    /// Unit intent includes the concern AND there is a contract reference.
    UnitIntentAndContract,
}

impl ApplicableReason {
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::ProjectAndUnitIntentAndDecision => "project_unit_intent_and_decision",
            Self::ProjectIntentAndParadigm => "project_intent_and_paradigm",
            Self::UnitIntentAndContract => "unit_intent_and_contract",
        }
    }
}

// ─── Optional: time/event marker (deterministic identity) ────────────────────

/// Optional `EventTime` for the call. **Not wall-clock.** Identity is
/// stable regardless of `evaluation_time` — `evaluation_time` only
/// affects *display* and ordering of any temporal reason tags.
pub type EvaluationTime = EventTime;
