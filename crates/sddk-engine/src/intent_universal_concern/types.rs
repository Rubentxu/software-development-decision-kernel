//! Types for the Intent + UniversalConcern model (A4-4a).
//!
//! Closed vocabularies, typed accessors, and identity-stable ids.
//! **Zero policy.** This file is descriptive, not prescriptive.

use serde::Serialize;
use std::collections::BTreeSet;

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
    /// Concerns the project explicitly excludes. A concern in this set
    /// is `NotApplicable(c, NotApplicableReason::ExplicitlyExcludedByProject)`
    /// irrespective of whether it is also in `declared_concerns`.
    /// (A4-4aR: explicit exclusion is the only declaratively-legitimate
    /// reason beyond the absence-of-declaration gate.)
    #[serde(default)]
    pub excluded_concerns: BTreeSet<UniversalConcern>,
}

/// Declared intent for a specific unit, within a `ProjectIntent`.
///
/// `unit_ref` is the identity of the unit being declared for.
/// `applies_to_concerns` is the unit's declared subset — narrower than
/// the project's declared_concerns. `excluded_concerns` is the unit's
/// explicit exclusion list (A4-4aR).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct UnitIntent {
    pub unit_ref: SoftwareUnitRef,
    pub applies_to_concerns: BTreeSet<UniversalConcern>,
    #[serde(default)]
    pub excluded_concerns: BTreeSet<UniversalConcern>,
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
///
/// **A4-4aR semantics:** these reasons describe *declared-scope applicability
/// only*. The presence or absence of grounding decisions, contracts, or
/// paradigm-specific observations is OUT OF SCOPE for applicability — that
/// state belongs to Grounding and Evaluability (A4-4b, A4-5), not to
/// `ApplicableConcern`.
///
/// Epistemic pin (do not collapse these notions in future cycles):
///
/// ```text
/// NotApplicable         != Unknown
///                        != Ungrounded
///                        != InsufficientEvidence
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub enum NotApplicableReason {
    /// The project's `declared_concerns` does not include this concern.
    NotInProjectIntent,
    /// The unit's `applies_to_concerns` does not include this concern.
    NotInUnitIntent,
    /// The project explicitly excludes this concern via `excluded_concerns`.
    ExplicitlyExcludedByProject,
    /// The unit explicitly excludes this concern via `excluded_concerns`.
    ExplicitlyExcludedByUnit,
}

impl NotApplicableReason {
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::NotInProjectIntent => "not_in_project_intent",
            Self::NotInUnitIntent => "not_in_unit_intent",
            Self::ExplicitlyExcludedByProject => "explicitly_excluded_by_project",
            Self::ExplicitlyExcludedByUnit => "explicitly_excluded_by_unit",
        }
    }
}

/// The typed reason a particular `ApplicableConcern::Applicable` answer
/// was produced — i.e. the *positive* reason alongside the boolean answer.
///
/// A4-4aR collapsed the prior three-variant enum (which referenced
/// decisions, contracts, or paradigms) into a single **intent-only** reason,
/// because the prior variants conflated Applicability with Grounding.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub enum ApplicableReason {
    /// Project intent includes the concern AND unit intent includes the
    /// concern AND neither scope declares an exclusion.
    ProjectAndUnitIntent,
}

impl ApplicableReason {
    pub fn canonical_tag(self) -> &'static str {
        match self {
            Self::ProjectAndUnitIntent => "project_unit_intent",
        }
    }
}

// ─── Optional: time/event marker (deterministic identity) ────────────────────

/// Optional `EventTime` for the call. **Not wall-clock.** Identity is
/// stable regardless of `evaluation_time` — `evaluation_time` only
/// affects *display* and ordering of any temporal reason tags.
pub type EvaluationTime = EventTime;
