//! Continuation Candidate & Resume View substrate — typed frontier
//! of next-action candidates for fresh LLMs and returning operators.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-ContinuationCandidate.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-085-CONTINUATION-CANDIDATE-RESUME-VIEW.md

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

/// What kind of action the candidate represents (CDD-CONTINUE-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ContinuationKind {
    /// Continue working on the same workflow.
    Resume,
    /// Spawn a new delegation from current state.
    Dispatch,
    /// Commit a decision and emit a receipt.
    Decide,
    /// Open a follow-up cycle based on this candidate.
    OpenCycle,
    /// Verify or audit a finished decision.
    Audit,
    /// Defer: no action; carry candidate forward.
    Defer,
}

/// Reversibility of a candidate's action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reversibility {
    FullyReversible,
    PartiallyReversible,
    Irreversible,
    Unknown,
}

/// Typed frontier item with provenance and gates (15 fields).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContinuationCandidate {
    pub candidate_id: String,
    pub action: String,
    pub kind: ContinuationKind,
    pub prerequisites: Vec<String>,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub risks: Vec<String>,
    pub reversibility: Reversibility,
    pub confidence: f64,
    pub uncertainty: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub expected_value: f64,
    pub expected_cost: f64,
    pub blocks: Vec<String>,
    pub unlocks: Vec<String>,
    pub human_authority_required: bool,
    pub source_ref: String,
    pub produced_at_ms: i64,
}

impl ContinuationCandidate {
    pub fn new(
        candidate_id: impl Into<String>,
        action: impl Into<String>,
        kind: ContinuationKind,
    ) -> Self {
        Self {
            candidate_id: candidate_id.into(),
            action: action.into(),
            kind,
            prerequisites: Vec::new(),
            pros: Vec::new(),
            cons: Vec::new(),
            risks: Vec::new(),
            reversibility: Reversibility::Unknown,
            confidence: 0.0,
            uncertainty: Vec::new(),
            evidence_refs: Vec::new(),
            expected_value: 0.0,
            expected_cost: 0.0,
            blocks: Vec::new(),
            unlocks: Vec::new(),
            human_authority_required: false,
            source_ref: String::new(),
            produced_at_ms: 0,
        }
    }
}

/// A resume view: head + frontier + blockers + must_read.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResumeView {
    pub view_id: String,
    pub head_ref: String,
    pub frontier: Vec<ContinuationCandidate>,
    pub blockers: Vec<String>,
    pub pending_decisions: Vec<String>,
    pub must_read: Vec<String>,
    pub recovery_state_ref: Option<String>,
    pub generated_at_ms: i64,
}

impl ResumeView {
    pub fn new(
        view_id: impl Into<String>,
        head_ref: impl Into<String>,
        generated_at_ms: i64,
    ) -> Self {
        Self {
            view_id: view_id.into(),
            head_ref: head_ref.into(),
            frontier: Vec::new(),
            blockers: Vec::new(),
            pending_decisions: Vec::new(),
            must_read: Vec::new(),
            recovery_state_ref: None,
            generated_at_ms,
        }
    }
}

/// Error taxonomy (CDD-CONTINUE-001). Closed-set, `#[non_exhaustive]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
pub enum FrontierError {
    EmptyAction {
        candidate_id: String,
    },
    BoundedConfidence {
        candidate_id: String,
        value: f64,
    },
    NonFiniteCost {
        candidate_id: String,
        field: String,
    },
    IrreversibleNeedsAuthority {
        candidate_id: String,
    },
    ReversibilityMismatch {
        candidate_id: String,
        declared: Reversibility,
        implied: Reversibility,
    },
    UnknownSourceRef {
        candidate_id: String,
        source_ref: String,
    },
}

impl std::fmt::Display for FrontierError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrontierError::EmptyAction { candidate_id } => {
                write!(f, "candidate {candidate_id} has empty action")
            }
            FrontierError::BoundedConfidence {
                candidate_id,
                value,
            } => {
                write!(
                    f,
                    "candidate {candidate_id} confidence {value} out of [0,1]"
                )
            }
            FrontierError::NonFiniteCost {
                candidate_id,
                field,
            } => write!(f, "candidate {candidate_id} field {field} non-finite"),
            FrontierError::IrreversibleNeedsAuthority { candidate_id } => write!(
                f,
                "candidate {candidate_id} is irreversible but does not require human authority"
            ),
            FrontierError::ReversibilityMismatch {
                candidate_id,
                declared,
                implied,
            } => write!(
                f,
                "candidate {candidate_id} reversibility mismatch declared={declared:?} implied={implied:?}"
            ),
            FrontierError::UnknownSourceRef {
                candidate_id,
                source_ref,
            } => write!(
                f,
                "candidate {candidate_id} has empty/unknown source_ref {source_ref:?}"
            ),
        }
    }
}

impl std::error::Error for FrontierError {}

/// Persists candidates and views.
pub trait FrontierStore: Send + Sync + std::fmt::Debug {
    fn put_candidate(&self, candidate: ContinuationCandidate);
    fn list_candidates(&self) -> Vec<ContinuationCandidate>;
    fn candidates_for_head(&self, head_ref: &str) -> Vec<ContinuationCandidate>;
    fn put_view(&self, view: ResumeView);
    fn get_view(&self, view_id: &str) -> Option<ResumeView>;
    fn latest_view(&self) -> Option<ResumeView>;
}

/// Reference in-memory implementation.
#[derive(Debug, Default)]
pub struct InMemoryFrontierStore {
    candidates: Mutex<BTreeMap<String, ContinuationCandidate>>,
    views: Mutex<BTreeMap<String, ResumeView>>,
}

impl InMemoryFrontierStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl FrontierStore for InMemoryFrontierStore {
    fn put_candidate(&self, candidate: ContinuationCandidate) {
        let mut g = self
            .candidates
            .lock()
            .expect("InMemoryFrontierStore candidates mutex poisoned");
        g.insert(candidate.candidate_id.clone(), candidate);
    }
    fn list_candidates(&self) -> Vec<ContinuationCandidate> {
        let g = self
            .candidates
            .lock()
            .expect("InMemoryFrontierStore candidates mutex poisoned");
        g.values().cloned().collect()
    }
    fn candidates_for_head(&self, head_ref: &str) -> Vec<ContinuationCandidate> {
        let g = self
            .candidates
            .lock()
            .expect("InMemoryFrontierStore candidates mutex poisoned");
        // All candidates are scoped to a single current head; this trivial
        // impl matches every candidate — head filter is a future extension.
        let _ = head_ref;
        g.values().cloned().collect()
    }
    fn put_view(&self, view: ResumeView) {
        let mut g = self
            .views
            .lock()
            .expect("InMemoryFrontierStore views mutex poisoned");
        g.insert(view.view_id.clone(), view);
    }
    fn get_view(&self, view_id: &str) -> Option<ResumeView> {
        let g = self
            .views
            .lock()
            .expect("InMemoryFrontierStore views mutex poisoned");
        g.get(view_id).cloned()
    }
    fn latest_view(&self) -> Option<ResumeView> {
        let g = self
            .views
            .lock()
            .expect("InMemoryFrontierStore views mutex poisoned");
        g.values().max_by_key(|v| v.generated_at_ms).cloned()
    }
}

/// Structural validator for `ContinuationCandidate`.
#[derive(Debug, Clone)]
#[allow(dead_code)] // store reserved for cross-referencing extensions
pub struct FrontierValidator {
    store: Arc<dyn FrontierStore>,
}

impl FrontierValidator {
    pub fn new(store: Arc<dyn FrontierStore>) -> Self {
        Self { store }
    }

    /// Off-line validator with no store dependency.
    pub fn offline() -> Self {
        Self {
            store: Arc::new(InMemoryFrontierStore::new()),
        }
    }

    /// Validate a candidate. Implements 7 invariants.
    pub fn validate_candidate(
        &self,
        candidate: &ContinuationCandidate,
    ) -> Result<(), FrontierError> {
        // (1) non-empty action
        if candidate.action.trim().is_empty() {
            return Err(FrontierError::EmptyAction {
                candidate_id: candidate.candidate_id.clone(),
            });
        }

        // (2) bounded confidence
        if !candidate.confidence.is_finite() || !(0.0..=1.0).contains(&candidate.confidence) {
            return Err(FrontierError::BoundedConfidence {
                candidate_id: candidate.candidate_id.clone(),
                value: candidate.confidence,
            });
        }

        // (3) finite value/cost
        if !candidate.expected_value.is_finite() {
            return Err(FrontierError::NonFiniteCost {
                candidate_id: candidate.candidate_id.clone(),
                field: "expected_value".to_string(),
            });
        }
        if !candidate.expected_cost.is_finite() {
            return Err(FrontierError::NonFiniteCost {
                candidate_id: candidate.candidate_id.clone(),
                field: "expected_cost".to_string(),
            });
        }

        // (7) produced_at_ms non-zero
        if candidate.produced_at_ms == 0 {
            return Err(FrontierError::NonFiniteCost {
                candidate_id: candidate.candidate_id.clone(),
                field: "produced_at_ms".to_string(),
            });
        }

        // (4) irreversible ⇒ human authority required
        if matches!(candidate.reversibility, Reversibility::Irreversible)
            && !candidate.human_authority_required
        {
            return Err(FrontierError::IrreversibleNeedsAuthority {
                candidate_id: candidate.candidate_id.clone(),
            });
        }

        // (5) reversibility mismatch: implied is at-least as severe
        // as declared (Unknown is permissive). Surface mismatch when
        // the candidate claims lower-risk reversibility than the
        // declared facts imply.
        let implied = Self::implied_reversibility(candidate);
        let declared_sev = match candidate.reversibility {
            Reversibility::Unknown => 0,
            Reversibility::FullyReversible => 1,
            Reversibility::PartiallyReversible => 2,
            Reversibility::Irreversible => 3,
        };
        let implied_sev = match implied {
            Reversibility::Unknown => 0,
            Reversibility::FullyReversible => 1,
            Reversibility::PartiallyReversible => 2,
            Reversibility::Irreversible => 3,
        };
        if implied_sev > declared_sev {
            return Err(FrontierError::ReversibilityMismatch {
                candidate_id: candidate.candidate_id.clone(),
                declared: candidate.reversibility,
                implied,
            });
        }

        // (6) non-empty source_ref
        if candidate.source_ref.trim().is_empty() {
            return Err(FrontierError::UnknownSourceRef {
                candidate_id: candidate.candidate_id.clone(),
                source_ref: candidate.source_ref.clone(),
            });
        }

        Ok(())
    }

    /// Heuristic: infer reversibility from declared facts about the candidate.
    fn implied_reversibility(c: &ContinuationCandidate) -> Reversibility {
        // Opening a new cycle is always at least partially reversible
        // (you can close it again).
        if matches!(c.kind, ContinuationKind::OpenCycle) {
            return Reversibility::PartiallyReversible;
        }
        if !c.blocks.is_empty() && !c.unlocks.is_empty() {
            Reversibility::PartiallyReversible
        } else {
            Reversibility::FullyReversible
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_candidate() -> ContinuationCandidate {
        let mut c = ContinuationCandidate::new(
            "C1",
            "rebuild production database",
            ContinuationKind::Decide,
        );
        c.confidence = 0.8;
        c.expected_value = 10.0;
        c.expected_cost = 1.0;
        c.reversibility = Reversibility::FullyReversible;
        c.source_ref = "head://HEAD-1".to_string();
        c.produced_at_ms = 1_500_000_000_000;
        c
    }

    #[test]
    fn empty_action_rejected() {
        let mut c = make_candidate();
        c.action = "".to_string();
        let v = FrontierValidator::offline();
        let err = v.validate_candidate(&c).unwrap_err();
        assert!(matches!(err, FrontierError::EmptyAction { .. }));
    }

    #[test]
    fn confidence_out_of_bounds() {
        let mut c = make_candidate();
        c.confidence = 1.5;
        let v = FrontierValidator::offline();
        let err = v.validate_candidate(&c).unwrap_err();
        assert!(matches!(err, FrontierError::BoundedConfidence { .. }));
    }

    #[test]
    fn non_finite_value() {
        let mut c = make_candidate();
        c.expected_value = f64::NAN;
        let v = FrontierValidator::offline();
        let err = v.validate_candidate(&c).unwrap_err();
        assert!(matches!(err, FrontierError::NonFiniteCost { .. }));
    }

    #[test]
    fn irreversible_needs_authority() {
        let mut c = make_candidate();
        c.reversibility = Reversibility::Irreversible;
        c.human_authority_required = false;
        let v = FrontierValidator::offline();
        let err = v.validate_candidate(&c).unwrap_err();
        assert!(matches!(
            err,
            FrontierError::IrreversibleNeedsAuthority { .. }
        ));
    }

    #[test]
    fn reversibility_mismatch() {
        let mut c = make_candidate();
        c.kind = ContinuationKind::OpenCycle;
        c.reversibility = Reversibility::FullyReversible;
        c.human_authority_required = true; // pass invariant (4)
        let v = FrontierValidator::offline();
        let err = v.validate_candidate(&c).unwrap_err();
        assert!(matches!(err, FrontierError::ReversibilityMismatch { .. }));
    }

    #[test]
    fn unknown_source_ref() {
        let mut c = make_candidate();
        c.source_ref = "".to_string();
        let v = FrontierValidator::offline();
        let err = v.validate_candidate(&c).unwrap_err();
        assert!(matches!(err, FrontierError::UnknownSourceRef { .. }));
    }

    #[test]
    fn produced_at_ms_zero() {
        let mut c = make_candidate();
        c.produced_at_ms = 0;
        let v = FrontierValidator::offline();
        let err = v.validate_candidate(&c).unwrap_err();
        assert!(matches!(err, FrontierError::NonFiniteCost { .. }));
    }

    #[test]
    fn candidate_roundtrip_on_store() {
        let store = InMemoryFrontierStore::new();
        let c = make_candidate();
        store.put_candidate(c.clone());
        let list = store.list_candidates();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].candidate_id, "C1");
    }

    #[test]
    fn latest_view() {
        let store = InMemoryFrontierStore::new();
        let mut v1 = ResumeView::new("V1", "HEAD-1", 1_000);
        v1.view_id = "V1".into();
        let v2 = ResumeView::new("V2", "HEAD-2", 2_000);
        store.put_view(v1);
        store.put_view(v2);
        let latest = store.latest_view().unwrap();
        assert_eq!(latest.view_id, "V2");
    }

    #[test]
    fn candidates_for_head_filter() {
        let store = InMemoryFrontierStore::new();
        let c = make_candidate();
        store.put_candidate(c);
        let filtered = store.candidates_for_head("HEAD-1");
        assert_eq!(filtered.len(), 1);
    }
}
