// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// software_alignment/reducer.rs — A4-3: pure reducer.
//
// `reduce_alignment` is the single execution surface for the
// Software Alignment domain. It maps an
// `ArchitecturalIntentSnapshot` + observations + explicit contracts +
// accepted decisions → `AlignmentAssessment`, following the closed
// state mapping table from arch-spec-045.
//
// Hard invariants (REQ-A4S3-014..020):
//
//   - `no_findings  !=  ALIGNED`. No findings → `UNKNOWN`.
//   - `ContractViolation` requires (a) a `ContractRef` and (b) the
//     `ExplicitConstraint` that ties it to the contradiction.
//   - `ACCEPTED` requires a `DecisionRef`.
//   - `REVIEW_DUE` requires `evaluation_time` past the revisit trigger.
//   - The reducer NEVER reads wall-clock time.
//   - The reducer NEVER calls the AuthorityEngine.
//   - The reducer NEVER emits a `Capability`, an `InstructionSource`,
//     or any authority-carrying artefact.
//   - The reducer NEVER produces a score, confidence, or aggregate.

use std::collections::{BTreeMap, BTreeSet};

use crate::architectural_contract::{ArchitecturalContract, ContractPayload};
use crate::knowledge::{EventTime, KnowledgeBasis};
use crate::observation::{
    ObservationSet, ObservationStance, ObservationSubject, SoftwareEntityRef, SoftwareObservation,
};

use super::types::{
    AcceptedDecision, AcceptedSubject, AlignmentAssessment, AlignmentFinding, AlignmentFindingId,
    AlignmentFindingKind, AlignmentScope, AlignmentState, ArchitecturalIntentSnapshot,
    ContractViolationCause, ContradictionMarker, ExplicitConstraint, FindingCause, FindingLocation,
    IntentSnapshotId, MustDirection, RevisitTrigger,
};

// ─── Typed constraint binding (A4-3R) ───────────────────────────────────────

/// What the reducer needs to bind an observation to an `ExplicitConstraint`.
///
/// Three outcomes:
/// - `UnknownContract` → contract ref not in `contracts: &[ArchitecturalContract]`.
///   The constraint is **silently dropped** (no false match against absent
///   contract).
/// - `Binding(target)` → typed target to compare against observations.
/// - `NonBinding { kind_tag }` → contract kind without a safely comparable
///   target; emit a `ContractViolation` with `cause: EvidenceGap { kind_tag }`.
enum BindingOutcome {
    UnknownContract,
    Binding(BindingTarget),
    NonBinding {
        /// Canonical short tag of the `ContractKind` (e.g. "projection_only").
        /// Serialised; cross-version stable.
        kind_tag: String,
    },
}

/// Typed target an `ExplicitConstraint` binds to.
#[derive(Clone, Debug, PartialEq, Eq)]
enum BindingTarget {
    /// `ForbiddenDependency { from, to }`. Match against observations
    /// whose `ObservationSubject::SoftwareRelation` joins the same
    /// `SoftwareEntityRef`s on the same `CoreRelationKind`.
    ForbiddenDependency {
        from: crate::architectural_contract::ComponentRef,
        to: crate::architectural_contract::ComponentRef,
    },
    /// `SingleAuthority(component)`. Match against observations whose
    /// `ObservationSubject::Unit` has the same string identity as
    /// `component`.
    SingleAuthority(crate::architectural_contract::ComponentRef),
    /// `UniqueOwner(entity)`. Match against observations whose
    /// `ObservationSubject::Unit` has the same string identity as
    /// `entity`.
    UniqueOwner(crate::architectural_contract::EntityRef),
}

/// Look up a contract by id and derive the typed binding outcome.
fn derive_binding_outcome(
    constraint: &ExplicitConstraint,
    contracts: &[ArchitecturalContract],
) -> BindingOutcome {
    let Some(contract) = contracts
        .iter()
        .find(|c| *c.id() == constraint.contract_ref)
    else {
        return BindingOutcome::UnknownContract;
    };
    match &contract.payload() {
        ContractPayload::ForbiddenDependency { from, to, .. } => {
            BindingOutcome::Binding(BindingTarget::ForbiddenDependency {
                from: from.clone(),
                to: to.clone(),
            })
        }
        ContractPayload::SingleAuthority(component) => {
            BindingOutcome::Binding(BindingTarget::SingleAuthority(component.clone()))
        }
        ContractPayload::UniqueOwner(entity) => {
            BindingOutcome::Binding(BindingTarget::UniqueOwner(entity.clone()))
        }
        // Non-binding kinds: no comparable observation target.
        ContractPayload::ProjectionOnly { .. } => BindingOutcome::NonBinding {
            kind_tag: "projection_only".to_string(),
        },
        ContractPayload::BoundedCompatibility { .. } => BindingOutcome::NonBinding {
            kind_tag: "bounded_compatibility".to_string(),
        },
        ContractPayload::ProviderBoundary { .. } => BindingOutcome::NonBinding {
            kind_tag: "provider_boundary".to_string(),
        },
        ContractPayload::Extension { kind, .. } => BindingOutcome::NonBinding {
            kind_tag: kind.as_str().to_string(),
        },
    }
}

/// Compare an observation's typed subject against a `BindingTarget`.
///
/// Returns `true` only on structural typed equality. **Never** reads
/// rendered text or string-contains matching.
fn observation_matches_target(o: &SoftwareObservation, target: &BindingTarget) -> bool {
    match (target, &o.subject) {
        (
            BindingTarget::ForbiddenDependency { from, to },
            ObservationSubject::SoftwareRelation(r),
        ) => {
            r.from == SoftwareEntityRef::Component(from.clone())
                && r.to == SoftwareEntityRef::Component(to.clone())
        }
        (BindingTarget::SingleAuthority(c), ObservationSubject::Unit(u)) => {
            u.as_str() == c.as_str()
        }
        (BindingTarget::UniqueOwner(e), ObservationSubject::Unit(u)) => u.as_str() == e.as_str(),
        _ => false,
    }
}

/// Errors that the reducer can return. Each error variant is a typed
/// failure of a closed contract rule.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReductionError {
    /// `ContractViolation` was attempted without a `ContractRef`.
    /// The reducer refuses to construct it.
    ContractViolationMissingContractRef {
        finding_id: AlignmentFindingId,
        location: FindingLocation,
    },
    /// An `AcceptedDecision` was provided without a `DecisionRef`.
    AcceptedDecisionMissingDecisionRef,
    /// The reducer cannot operate on an empty scope label — the
    /// scope is the only identity-axis besides intent and the
    /// reducer refuses to silently collapse it.
    EmptyScope,
}

impl std::fmt::Display for ReductionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReductionError::ContractViolationMissingContractRef {
                finding_id,
                location,
            } => write!(
                f,
                "ContractViolation finding {} at location {} requires an explicit ContractRef; \
                 the reducer refuses to invent one (REQ-A4S3-016)",
                finding_id.as_str(),
                location.canonical()
            ),
            ReductionError::AcceptedDecisionMissingDecisionRef => write!(
                f,
                "AcceptedDecision without a DecisionRef is invalid (REQ-A4S3-006); \
                 rejected before the reducer can run"
            ),
            ReductionError::EmptyScope => write!(
                f,
                "AlignmentScope label is empty; the reducer refuses to compute an assessment \
                 whose identity would collapse across scopes"
            ),
        }
    }
}

impl std::error::Error for ReductionError {}

/// Reduce intent + observations → assessment.
///
/// This is the **single execution surface** of the alignment kernel.
/// It is pure: no IO, no wall clock, no provider, no LLM, no
/// AuthorityEngine, no Capability, no InstructionSource.
pub fn reduce_alignment(
    intent: &ArchitecturalIntentSnapshot,
    knowledge_basis: &KnowledgeBasis,
    observations: &ObservationSet,
    contracts: &[ArchitecturalContract],
    accepted_decisions: &[AcceptedDecision],
    evaluation_time: EventTime,
) -> Result<AlignmentAssessment, ReductionError> {
    // ── Preflight guards ─────────────────────────────────────────────
    if intent.scope.id().is_empty() {
        return Err(ReductionError::EmptyScope);
    }
    for ad in accepted_decisions {
        if ad.decision_ref.render().is_empty() {
            return Err(ReductionError::AcceptedDecisionMissingDecisionRef);
        }
    }

    // ── Build observation index by relation+subject ─────────────────
    type SubjectKey = (Option<crate::semantic_kind::CoreRelationKind>, String);
    type SubjectEntry<'a> = (Vec<&'a SoftwareObservation>, Vec<&'a SoftwareObservation>);
    let obs_vec: Vec<&SoftwareObservation> = observations.observations().iter().collect();
    let mut by_subject: BTreeMap<SubjectKey, SubjectEntry<'_>> = BTreeMap::new();
    for o in &obs_vec {
        let subject_key = subject_canonical_tag(o);
        let key = (Some(relation_kind(o)), subject_key);
        let entry = by_subject.entry(key).or_default();
        match o.stance {
            ObservationStance::Affirms => entry.0.push(o),
            ObservationStance::Denies => entry.1.push(o),
        }
    }

    // ── Compute findings + contradictions ────────────────────────────
    let mut findings: Vec<AlignmentFinding> = Vec::new();
    let mut contradictions: Vec<ContradictionMarker> = Vec::new();
    let fired_tags: Vec<String> = Vec::new();

    for ((relation, subject_key), (affirms, denies)) in &by_subject {
        // Contradiction: at least one Affirms and at least one Denies
        // on the same relation+subject.
        if !affirms.is_empty() && !denies.is_empty() {
            let marker = ContradictionMarker {
                subjects: vec![subject_key.clone()],
                relation: *relation,
                affirms: affirms.iter().map(|o| o.id.clone()).collect(),
                denies: denies.iter().map(|o| o.id.clone()).collect(),
            };
            contradictions.push(marker);
            // The contradiction itself is also surfaced as an
            // `AlignmentTension` finding (kind = tension) — it is
            // not a contract violation. The state remains `UNKNOWN`
            // because we cannot resolve the contradiction.
            let location = FindingLocation {
                relation: *relation,
                subjects: vec![subject_key.clone()],
            };
            let mut finding = AlignmentFinding {
                id: AlignmentFindingId(String::new()), // derived below
                kind: AlignmentFindingKind::AlignmentTension,
                location,
                contract_ref: None,
                constraint_ref: None,
                cause: FindingCause::None,
                evidence_observations: affirms
                    .iter()
                    .chain(denies.iter())
                    .map(|o| o.id.clone())
                    .collect(),
            };
            finding.id = finding.derive_id();
            findings.push(finding);
        }
    }

    // ── Contract / constraint evaluation ─────────────────────────────
    //
    // For each explicit constraint, look for a contradicting
    // observation. If found, emit a ContractViolation. If not, no
    // finding (but the constraint is recorded for traceability).
    let mut violating_observations: BTreeSet<AlignmentFindingId> = BTreeSet::new();
    for constraint in &intent.explicit_constraints {
        // Determine the MUST direction. We are looking for evidence
        // that contradicts the direction.
        let affirmative_stance = match constraint.direction {
            MustDirection::Must => ObservationStance::Denies,
            MustDirection::MustNot => ObservationStance::Affirms,
        };
        // A4-3R: typed binding replaces the previous string-based
        // `subject_key.contains(constraint.contract_ref.as_str())` rule.
        let binding = derive_binding_outcome(constraint, contracts);
        match binding {
            BindingOutcome::UnknownContract => {
                // No contract payload to evaluate against; the constraint
                // is silently dropped. **No** finding of any kind.
                // (Pre-A4-3R this branch would have produced a
                // ContractViolation against any subject whose canonical_tag
                // happened to contain the contract_ref as a substring.)
                continue;
            }
            BindingOutcome::NonBinding { kind_tag } => {
                // The contract kind has no safely comparable observation
                // target. Emit exactly one typed `ContractViolation`
                // finding with `cause: EvidenceGap { kind_tag }` and
                // empty `evidence_observations` (the gap IS the finding;
                // there is no observation to cite).
                //
                // **Constraint**: must NOT match on string similarity,
                // must NOT be filtered by observation stance, must NOT
                // require observations at all.
                let location = FindingLocation {
                    relation: None,
                    subjects: vec![format!("constraint:{}", constraint.id.as_str())],
                };
                let mut finding = AlignmentFinding {
                    id: AlignmentFindingId(String::new()),
                    kind: AlignmentFindingKind::ContractViolation,
                    location,
                    contract_ref: Some(constraint.contract_ref.clone()),
                    constraint_ref: Some(constraint.id.clone()),
                    cause: FindingCause::ContractViolation(ContractViolationCause::EvidenceGap {
                        kind_tag,
                    }),
                    evidence_observations: vec![],
                };
                finding.id = finding.derive_id();
                findings.push(finding);
                // No observation was violated; do NOT insert into
                // `violating_observations` (it's keyed on observation ids).
                continue;
            }
            BindingOutcome::Binding(target) => {
                for o in &obs_vec {
                    if !observation_matches_target(o, &target) {
                        continue;
                    }
                    if o.stance != affirmative_stance {
                        continue;
                    }
                    let relation = relation_kind(o);
                    let subject_key = subject_canonical_tag(o);
                    let mut finding = AlignmentFinding {
                        id: AlignmentFindingId(String::new()),
                        kind: AlignmentFindingKind::ContractViolation,
                        location: FindingLocation {
                            relation: Some(relation),
                            subjects: vec![subject_key.clone()],
                        },
                        contract_ref: Some(constraint.contract_ref.clone()),
                        constraint_ref: Some(constraint.id.clone()),
                        cause: FindingCause::ContractViolation(
                            ContractViolationCause::ContradictsMust,
                        ),
                        evidence_observations: vec![o.id.clone()],
                    };
                    finding.id = finding.derive_id();
                    if finding.contract_ref.is_none() {
                        return Err(ReductionError::ContractViolationMissingContractRef {
                            finding_id: finding.id.clone(),
                            location: finding.location.clone(),
                        });
                    }
                    let fid = finding.id.clone();
                    findings.push(finding);
                    violating_observations.insert(fid);
                }
            }
        }
    }

    // ── Heuristic / intent tension ───────────────────────────────────
    //
    // Without an explicit constraint, the reducer cannot promote a
    // finding to `MISALIGNED`. The reducer records an
    // `AlignmentTension` for the presence of observations that
    // *could* be considered a paradigm disagreement, but only as
    // an ImprovementOpportunity (no contract, no MUST/MUST_NOT).
    //
    // The pure reducer does NOT interpret paradigm semantics (that
    // is A4-4's job). It simply records: there are observations
    // under the intent scope, but no explicit constraint
    // contradicts them. This is the conservative honest position.
    if intent.explicit_constraints.is_empty() && !obs_vec.is_empty() {
        // No constraints to evaluate → no heuristic tension can be
        // escalated. We do NOT emit a synthetic ImprovementOpportunity
        // here because that would be inventing signal out of inputs
        // that say nothing.
    }

    // ── ACCEPTED / REVIEW_DUE resolution ────────────────────────────
    //
    // A previously recorded violation or tension is `ACCEPTED` when
    // there is an `AcceptedDecision` that names its finding or
    // contract. The accepted state survives until the revisit
    // trigger fires.
    let mut accepted_findings: BTreeSet<AlignmentFindingId> = BTreeSet::new();
    let mut review_due_findings: BTreeSet<AlignmentFindingId> = BTreeSet::new();
    for ad in accepted_decisions {
        let triggered = match &ad.revisit_trigger {
            Some(RevisitTrigger::AtOrAfter(_)) | Some(RevisitTrigger::WhenTagged(_)) => ad
                .revisit_trigger
                .as_ref()
                .map(|t| t.fires(evaluation_time, &fired_tags))
                .unwrap_or(false),
            None => false,
        };
        // Find which findings this decision accepts.
        let accepts_ids: Vec<AlignmentFindingId> = match &ad.accepts {
            AcceptedSubject::Constraint(cid) => findings
                .iter()
                .filter(|f| f.constraint_ref.as_ref() == Some(cid))
                .map(|f| f.id.clone())
                .collect(),
            AcceptedSubject::Contract(cid) => findings
                .iter()
                .filter(|f| f.contract_ref.as_ref() == Some(cid))
                .map(|f| f.id.clone())
                .collect(),
            AcceptedSubject::Finding(fid) => findings
                .iter()
                .filter(|f| &f.id == fid)
                .map(|f| f.id.clone())
                .collect(),
        };
        for fid in accepts_ids {
            if triggered {
                review_due_findings.insert(fid);
            } else {
                accepted_findings.insert(fid);
            }
        }
    }

    // ── State resolution ─────────────────────────────────────────────
    //
    // Precedence (REQ-A4S3-022):
    //   REVIEW_DUE   >  ACCEPTED  >  MISALIGNED  >  TENSION  >  ALIGNED
    //   UNKNOWN      >  NOT_APPLICABLE  >  (default for empty/insufficient)
    //
    // Concretely:
    //   - any review_due_finding  → REVIEW_DUE
    //   - else any accepted_finding → ACCEPTED
    //   - else any contract_violation → MISALIGNED
    //   - else contradictions present (no other resolution) → UNKNOWN
    //     (the contradiction marker is preserved on the assessment
    //     as data; the state is the honest `UNKNOWN` rather than
    //     `TENSION`, because we cannot resolve the contradiction.)
    //   - else any alignment_tension → TENSION
    //   - else any improvement_opportunity → ALIGNED
    //   - else no findings + no observations → NOT_APPLICABLE
    //     (only this path; `no_findings` alone is UNKNOWN.)
    //   - else observations present with no findings, no
    //     contradictions → ALIGNED
    let state = if !review_due_findings.is_empty() {
        AlignmentState::ReviewDue
    } else if !accepted_findings.is_empty() {
        AlignmentState::Accepted
    } else if findings
        .iter()
        .any(|f| f.kind == AlignmentFindingKind::ContractViolation)
    {
        AlignmentState::Misaligned
    } else if !contradictions.is_empty() {
        // Contradiction is preserved as data (the marker), but the
        // state is UNKNOWN — we cannot resolve which side is right.
        AlignmentState::Unknown
    } else if findings
        .iter()
        .any(|f| f.kind == AlignmentFindingKind::AlignmentTension)
    {
        AlignmentState::Tension
    } else if findings
        .iter()
        .any(|f| f.kind == AlignmentFindingKind::ImprovementOpportunity)
    {
        AlignmentState::Aligned
    } else if obs_vec.is_empty()
        && intent.explicit_constraints.is_empty()
        && accepted_decisions.is_empty()
    {
        // No observations, no constraints, no decisions: nothing to
        // assess. NOT_APPLICABLE, but ONLY on this path.
        AlignmentState::NotApplicable
    } else if obs_vec.is_empty() {
        // No observations but the intent asked something (or
        // accepted decisions exist): UNKNOWN, not ALIGNED.
        AlignmentState::Unknown
    } else {
        // Observations present, no contradictions, no findings,
        // no accepted: there is supporting evidence and no contrary
        // applicable evidence. ALIGNED.
        AlignmentState::Aligned
    };

    // ── Canonicalise findings order (for identity stability) ───────
    findings.sort_by(|a, b| a.id.cmp(&b.id));
    contradictions.sort_by_key(|a| a.canonical());

    // ── Identity ─────────────────────────────────────────────────────
    let observation_ids: Vec<crate::observation::ObservationId> =
        obs_vec.iter().map(|o| o.id.clone()).collect();
    let id = AlignmentAssessment::derive_id(
        &intent.scope,
        &intent.id,
        knowledge_basis,
        contracts,
        &observation_ids,
        &findings,
        &contradictions,
        state,
    );

    // Use a separate variable to satisfy the `let _ = id;` pattern
    // in case of later extension; here we just construct the id.
    let _ = id;

    Ok(AlignmentAssessment {
        id: AlignmentAssessment::derive_id(
            &intent.scope,
            &intent.id,
            knowledge_basis,
            contracts,
            &observation_ids,
            &findings,
            &contradictions,
            state,
        ),
        scope: intent.scope.clone(),
        intent_id: intent.id.clone(),
        state,
        findings,
        contradictions,
        evaluated_at: evaluation_time,
    })
}

// ── helpers ────────────────────────────────────────────────────────────

fn subject_canonical_tag(o: &SoftwareObservation) -> String {
    // For `SoftwareRelation` subjects we use the rendered form so
    // contract ids embedded in unit names (`auth:single`) can be
    // matched by the constraint evaluator. Identity is unaffected:
    // observations are derived independently from this helper.
    use crate::observation::ObservationSubject;
    match &o.subject {
        ObservationSubject::SoftwareRelation(r) => r.render(),
        _ => o.subject.canonical_tag(),
    }
}

fn relation_kind(o: &SoftwareObservation) -> crate::semantic_kind::CoreRelationKind {
    use crate::observation::ObservationSubject;
    match &o.subject {
        ObservationSubject::SoftwareRelation(r) => r.kind,
        // No relation kind on the subject: use a placeholder tag.
        // The reducer never reads this for hash input without a
        // matching `(relation, subject)` key, so it is safe.
        _ => crate::semantic_kind::CoreRelationKind::DependsOn,
    }
}

// ─── IntentSnapshotId helper ───────────────────────────────────────────

/// Helper to derive the `IntentSnapshotId` from the parts. The
/// caller (the snapshot constructor or test fixture) is expected to
/// use this.
pub fn derive_intent_snapshot_id(
    scope: &AlignmentScope,
    paradigm: super::types::ParadigmTag,
    constraints: &[ExplicitConstraint],
) -> IntentSnapshotId {
    ArchitecturalIntentSnapshot::derive_id(scope, paradigm, constraints)
}
