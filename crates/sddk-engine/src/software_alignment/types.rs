// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// software_alignment/types.rs — A4-3 closed vocabularies.
//
// Every enum here is closed. Adding a variant is a breaking change
// of the public contract surface and requires a new arch-spec
// revision. There is no `Other` variant and no `Score`/`Confidence`
// numeric field by design.

use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

use crate::architectural_contract::{ArchitecturalContract, ContractId, DecisionRef};
use crate::knowledge::{EventTime, KnowledgeBasis};

// ─── Identity prefix ────────────────────────────────────────────────────

const ASSESSMENT_ID_DOMAIN: &str = "sddk.software_alignment.assessment.v1|";

// ─── ArchitecturalIntentSnapshot ────────────────────────────────────────

/// A snapshot of declared architectural intent for a given scope.
///
/// This is **input** to the reducer, never a finding. It captures
/// what the project says it is doing (paradigm, dependency
/// directions, naming conventions, etc.) — enough to evaluate whether
/// observed reality aligns with declared intent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ArchitecturalIntentSnapshot {
    /// Identity (content-addressed from intent inputs only).
    pub id: IntentSnapshotId,
    /// Free-form scope label (e.g. `repo:crates/sddk-engine`,
    /// `unit:auth_core`). The reducer uses this to filter
    /// observations whose subject matches.
    pub scope: AlignmentScope,
    /// The paradigm in force for this scope. A4-3 consumes the tag
    /// only — it does NOT interpret the paradigm. Lens interpretation
    /// is A4-4.
    pub paradigm_tag: ParadigmTag,
    /// Declarations of MUST / MUST_NOT constraints inside the scope.
    /// Each is a contract-style predicate. Empty means "no explicit
    /// constraints": the reducer must NOT promote a heuristic
    /// disagreement to `MISALIGNED` in that case.
    pub explicit_constraints: Vec<ExplicitConstraint>,
}

impl ArchitecturalIntentSnapshot {
    /// Identity hash (content-addressed).
    pub fn derive_id(
        scope: &AlignmentScope,
        paradigm_tag: ParadigmTag,
        constraints: &[ExplicitConstraint],
    ) -> IntentSnapshotId {
        let mut h = Sha256::new();
        h.update(b"sddk.software_alignment.intent.v1|");
        h.update(scope.id().as_bytes());
        h.update(b"|");
        h.update(paradigm_tag.canonical().as_bytes());
        for c in constraints {
            h.update(c.canonical().as_bytes());
            h.update(b";");
        }
        IntentSnapshotId(format!("{:064x}", h.finalize()))
    }
}

/// Tag identifying which paradigm the scope declares. A4-3 only reads
/// the tag; interpretation is A4-4's job.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParadigmTag {
    /// Object-Oriented.
    Oop,
    /// Functional / pure.
    Functional,
    /// Algebraic Data Types / typed DSL.
    Adt,
    /// Hexagonal / ports & adapters.
    Hexagonal,
    /// Reactive / event-driven.
    Reactive,
    /// Imperative / procedural.
    Imperative,
    /// No declared paradigm.
    Undeclared,
}

impl ParadigmTag {
    pub fn canonical(self) -> &'static str {
        match self {
            ParadigmTag::Oop => "oop",
            ParadigmTag::Functional => "functional",
            ParadigmTag::Adt => "adt",
            ParadigmTag::Hexagonal => "hexagonal",
            ParadigmTag::Reactive => "reactive",
            ParadigmTag::Imperative => "imperative",
            ParadigmTag::Undeclared => "undeclared",
        }
    }
}

/// Wraps the identity hash of an intent snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct IntentSnapshotId(pub String);

impl IntentSnapshotId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A scope label for the assessment.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct AlignmentScope(String);

impl AlignmentScope {
    pub fn new(label: impl Into<String>) -> Self {
        Self(label.into())
    }
    pub fn id(&self) -> &str {
        &self.0
    }
}

/// A single explicit constraint declared by the intent snapshot.
///
/// Constraints are the **only** path to a `ContractViolation`. A
/// heuristic disagreement (a smell, a paradigm preference) without an
/// `ExplicitConstraint` cannot produce `MISALIGNED` — it produces
/// `TENSION` at most.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExplicitConstraint {
    /// Stable id for this constraint (typically derived from the
    /// decision that introduced it).
    pub id: ConstraintId,
    /// The contract or invariant this constraint realises.
    pub contract_ref: ContractId,
    /// Whether this constraint is `MUST` or `MUST_NOT`.
    pub direction: MustDirection,
    /// Optional human-readable label (NOT included in identity — there
    /// for operators only; the alignment module is renderer-text-free
    /// in its identity computation).
    pub label: Option<String>,
}

impl ExplicitConstraint {
    pub fn canonical(&self) -> String {
        format!(
            "{}|{}|{}",
            self.id.as_str(),
            self.contract_ref.as_str(),
            match self.direction {
                MustDirection::Must => "must",
                MustDirection::MustNot => "must_not",
            }
        )
    }
}

/// Stable id for an explicit constraint (typically equal to a decision
/// reference).
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct ConstraintId(String);

impl ConstraintId {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MustDirection {
    Must,
    MustNot,
}

// ─── AcceptedDecision ───────────────────────────────────────────────────

/// A decision that explicitly accepts a previously identified tension
/// or violation. Used by the reducer to determine `ACCEPTED` vs
/// `MISALIGNED` vs `TENSION`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AcceptedDecision {
    pub decision_ref: DecisionRef,
    /// What the decision accepts (a constraint id, a contract id, or
    /// an `AlignmentFinding` id).
    pub accepts: AcceptedSubject,
    /// Optional revisit trigger. When present and `evaluation_time` is
    /// past the trigger, the assessment becomes `REVIEW_DUE` instead
    /// of `ACCEPTED`. When absent, the assessment stays `ACCEPTED`.
    pub revisit_trigger: Option<RevisitTrigger>,
    /// What the decision says about the subject (free-form summary,
    /// NOT in identity).
    pub summary: Option<String>,
}

/// What an `AcceptedDecision` accepts.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub enum AcceptedSubject {
    Constraint(ConstraintId),
    Contract(ContractId),
    Finding(AlignmentFindingId),
}

/// The moment (or condition) at which an accepted decision must be
/// revisited. The reducer receives `evaluation_time` as input and
/// compares it against this trigger — never against a hidden clock.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum RevisitTrigger {
    /// Revisit at or after the given event time.
    AtOrAfter(EventTime),
    /// Revisit when a named condition fires (the reducer does not
    /// interpret the condition; it only carries the tag).
    WhenTagged(String),
}

impl RevisitTrigger {
    /// Decide whether the trigger fires for the given evaluation
    /// time. `WhenTagged` triggers do NOT fire automatically — the
    /// reducer needs an explicit tag list from the caller.
    pub fn fires(&self, evaluation_time: EventTime, fired_tags: &[String]) -> bool {
        match self {
            RevisitTrigger::AtOrAfter(t) => evaluation_time.0 >= t.0,
            RevisitTrigger::WhenTagged(tag) => fired_tags.iter().any(|f| f == tag),
        }
    }
}

// ─── Findings ───────────────────────────────────────────────────────────

/// Findings produced by the reducer. Three kinds, closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentFindingKind {
    /// Explicit MUST / MUST_NOT contradiction.
    ContractViolation,
    /// Heuristic / intent tension without explicit contract violation.
    AlignmentTension,
    /// Suggestion arising from observations.
    ImprovementOpportunity,
}

impl AlignmentFindingKind {
    pub fn canonical(self) -> &'static str {
        match self {
            AlignmentFindingKind::ContractViolation => "contract_violation",
            AlignmentFindingKind::AlignmentTension => "alignment_tension",
            AlignmentFindingKind::ImprovementOpportunity => "improvement_opportunity",
        }
    }
}

/// A finding id, content-addressed.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct AlignmentFindingId(pub String);

impl AlignmentFindingId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A location a finding refers to: the subject it implicates.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
pub struct FindingLocation {
    /// The relation kind implicated, if any (e.g. `DependsOn`).
    pub relation: Option<crate::semantic_kind::CoreRelationKind>,
    /// The subjects involved.
    pub subjects: Vec<String>,
}

impl FindingLocation {
    pub fn canonical(&self) -> String {
        let mut s = String::new();
        if let Some(r) = self.relation {
            s.push_str(r.domain_tag());
        }
        s.push('|');
        let mut sorted = self.subjects.clone();
        sorted.sort();
        for sub in sorted {
            s.push_str(&sub);
            s.push(',');
        }
        s
    }
}

/// A single finding produced by the reducer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AlignmentFinding {
    pub id: AlignmentFindingId,
    pub kind: AlignmentFindingKind,
    /// The contract / constraint / observation this finding refers to.
    pub location: FindingLocation,
    /// The optional contract ref (only set for `ContractViolation`).
    pub contract_ref: Option<ContractId>,
    /// The optional constraint ref (only set when this finding ties
    /// back to an explicit constraint).
    pub constraint_ref: Option<ConstraintId>,
    /// The observations that motivated this finding. **Set, ordered**
    /// by canonical observation id — but identity does not depend on
    /// the order (the reducer sorts before hashing).
    pub evidence_observations: Vec<crate::observation::ObservationId>,
}

impl AlignmentFinding {
    /// Identity (sorted set of observation ids + location + kind).
    pub fn derive_id(&self) -> AlignmentFindingId {
        let mut h = Sha256::new();
        h.update(b"sddk.software_alignment.finding.v1|");
        h.update(self.kind.canonical().as_bytes());
        h.update(b"|");
        h.update(self.location.canonical().as_bytes());
        h.update(b"|");
        let mut obs: Vec<&str> = self
            .evidence_observations
            .iter()
            .map(|o| o.as_str())
            .collect();
        obs.sort();
        for o in obs {
            h.update(o.as_bytes());
            h.update(b";");
        }
        if let Some(c) = &self.contract_ref {
            h.update(b"|c|");
            h.update(c.as_str().as_bytes());
        }
        if let Some(c) = &self.constraint_ref {
            h.update(b"|k|");
            h.update(c.as_str().as_bytes());
        }
        AlignmentFindingId(format!("{:064x}", h.finalize()))
    }
}

// ─── ContradictionMarker ────────────────────────────────────────────────

/// Marks that a contradiction was detected among the observations
/// backing the assessment. Carried as data, never collapsed to a
/// verdict.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ContradictionMarker {
    pub subjects: Vec<String>,
    pub relation: Option<crate::semantic_kind::CoreRelationKind>,
    /// The observation ids on each side.
    pub affirms: Vec<crate::observation::ObservationId>,
    pub denies: Vec<crate::observation::ObservationId>,
}

impl ContradictionMarker {
    pub fn canonical(&self) -> String {
        let mut s = String::new();
        if let Some(r) = self.relation {
            s.push_str(r.domain_tag());
        }
        s.push('|');
        let mut subjects = self.subjects.clone();
        subjects.sort();
        for sub in subjects {
            s.push_str(&sub);
            s.push(',');
        }
        s.push('|');
        let mut a = self
            .affirms
            .iter()
            .map(|x| x.as_str().to_string())
            .collect::<Vec<_>>();
        a.sort();
        let mut d = self
            .denies
            .iter()
            .map(|x| x.as_str().to_string())
            .collect::<Vec<_>>();
        d.sort();
        s.push_str("affirms=");
        for x in a {
            s.push_str(&x);
            s.push(';');
        }
        s.push_str("|denies=");
        for x in d {
            s.push_str(&x);
            s.push(';');
        }
        s
    }
}

// ─── Assessment ─────────────────────────────────────────────────────────

/// Closed state vocabulary for Alignment assessments (REQ-A4S3-001).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AlignmentState {
    Aligned,
    Tension,
    Misaligned,
    Accepted,
    ReviewDue,
    Unknown,
    NotApplicable,
}

impl AlignmentState {
    /// All states, in canonical order. Useful for tests + serde audits.
    pub const ALL: &'static [AlignmentState] = &[
        AlignmentState::Aligned,
        AlignmentState::Tension,
        AlignmentState::Misaligned,
        AlignmentState::Accepted,
        AlignmentState::ReviewDue,
        AlignmentState::Unknown,
        AlignmentState::NotApplicable,
    ];

    pub fn canonical(self) -> &'static str {
        match self {
            AlignmentState::Aligned => "aligned",
            AlignmentState::Tension => "tension",
            AlignmentState::Misaligned => "misaligned",
            AlignmentState::Accepted => "accepted",
            AlignmentState::ReviewDue => "review_due",
            AlignmentState::Unknown => "unknown",
            AlignmentState::NotApplicable => "not_applicable",
        }
    }
}

/// The single output type of `reduce_alignment`. Pure projection.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AlignmentAssessment {
    /// Deterministic, content-addressed id.
    pub id: AlignmentAssessmentId,
    /// The scope the assessment applies to.
    pub scope: AlignmentScope,
    /// The intent snapshot the assessment was reduced against.
    pub intent_id: IntentSnapshotId,
    /// The final state. **Never carries score, confidence, or any
    /// numeric aggregate.** (REQ-A4S3-013)
    pub state: AlignmentState,
    /// Findings supporting the state. Order is canonical (sorted by id).
    pub findings: Vec<AlignmentFinding>,
    /// Contradiction markers discovered while reducing (carried as
    /// data, not collapsed into the verdict).
    pub contradictions: Vec<ContradictionMarker>,
    /// The `evaluation_time` argument. Recorded for reproducibility.
    /// NOT in identity.
    pub evaluated_at: EventTime,
}

impl AlignmentAssessment {
    /// Compute identity. Clock-stable, content-addressed, excludes
    /// everything cosmetic.
    ///
    /// All 8 inputs are deliberately part of the identity spec
    /// (see `arch-spec-045` §3 "Identity"): scope, intent_id,
    /// knowledge_basis, contracts, observations, findings,
    /// contradictions, and the resolved state. Removing any input
    /// would either (a) collapse collisions between assessments that
    /// differ on that dimension, or (b) break the
    /// `no_findings != ALIGNED` invariant's identity contract.
    /// Bundling the inputs into a struct would shift the per-field
    /// cost from call-sites (1 line per field) to construction
    /// (1 line per field, plus a struct name and a docstring per
    /// field); for a deterministic identity function this is
    /// net-negative on readability, so we accept the lint here.
    #[allow(clippy::too_many_arguments)]
    pub fn derive_id(
        scope: &AlignmentScope,
        intent_id: &IntentSnapshotId,
        knowledge_basis: &KnowledgeBasis,
        contracts: &[ArchitecturalContract],
        observations: &[crate::observation::ObservationId],
        findings: &[AlignmentFinding],
        contradictions: &[ContradictionMarker],
        state: AlignmentState,
    ) -> AlignmentAssessmentId {
        let mut h = Sha256::new();
        h.update(ASSESSMENT_ID_DOMAIN.as_bytes());
        h.update(scope.id().as_bytes());
        h.update(b"|i|");
        h.update(intent_id.as_str().as_bytes());
        h.update(b"|k|");
        h.update(knowledge_basis.basis_hash().to_hex().as_bytes());
        h.update(b"|c|");
        let mut ids: Vec<&str> = contracts.iter().map(|c| c.id().as_str()).collect();
        ids.sort();
        ids.dedup();
        for cid in ids {
            h.update(cid.as_bytes());
            h.update(b";");
        }
        h.update(b"|o|");
        let mut obs: Vec<&str> = observations.iter().map(|o| o.as_str()).collect();
        obs.sort();
        for o in obs {
            h.update(o.as_bytes());
            h.update(b";");
        }
        h.update(b"|f|");
        let mut fids: BTreeSet<&str> = BTreeSet::new();
        for f in findings {
            fids.insert(f.id.as_str());
        }
        for fid in fids {
            h.update(fid.as_bytes());
            h.update(b";");
        }
        h.update(b"|x|");
        let mut cm: BTreeSet<String> = BTreeSet::new();
        for c in contradictions {
            cm.insert(c.canonical());
        }
        for c in cm {
            h.update(c.as_bytes());
            h.update(b";");
        }
        h.update(b"|s|");
        h.update(state.canonical().as_bytes());
        AlignmentAssessmentId(format!("{:064x}", h.finalize()))
    }
}

/// Wraps the identity hash of an alignment assessment.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
pub struct AlignmentAssessmentId(pub String);

impl AlignmentAssessmentId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
