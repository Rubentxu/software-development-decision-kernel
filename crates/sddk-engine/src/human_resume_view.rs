//! Human Resume View & Rehydration Plan substrate — typed
//! human-facing projection over generic CDD `ResumeView`.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-HumanResumeView.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-088-HUMAN-RESUME-VIEW.md

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// Compact summary of a previous decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionSummary {
    pub decision_ref: String,
    pub verdict: String,
    pub at_ms: i64,
    pub digest: String,
}

impl DecisionSummary {
    pub fn new(
        decision_ref: impl Into<String>,
        verdict: impl Into<String>,
        at_ms: i64,
        digest: impl Into<String>,
    ) -> Self {
        Self {
            decision_ref: decision_ref.into(),
            verdict: verdict.into(),
            at_ms,
            digest: digest.into(),
        }
    }
}

/// Human-facing snapshot of a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeInfo {
    pub info_id: String,
    pub cycle_ref: String,
    pub head_ref: String,
    pub human_id: String,
    pub visited_at_ms: i64,
    pub last_decisions: Vec<DecisionSummary>,
    pub open_blockers: Vec<String>,
    pub pending_asks: Vec<String>,
    pub last_view_view_id: String,
    pub produced_at_ms: i64,
}

impl ResumeInfo {
    pub fn new(
        info_id: impl Into<String>,
        cycle_ref: impl Into<String>,
        head_ref: impl Into<String>,
        human_id: impl Into<String>,
        visited_at_ms: i64,
        last_view_view_id: impl Into<String>,
        produced_at_ms: i64,
    ) -> Self {
        Self {
            info_id: info_id.into(),
            cycle_ref: cycle_ref.into(),
            head_ref: head_ref.into(),
            human_id: human_id.into(),
            visited_at_ms,
            last_decisions: Vec::new(),
            open_blockers: Vec::new(),
            pending_asks: Vec::new(),
            last_view_view_id: last_view_view_id.into(),
            produced_at_ms,
        }
    }
}

/// A rehydration step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
pub enum RehydrationStep {
    ReadContext {
        context_ref: String,
    },
    AcknowledgeDecision {
        decision_ref: String,
        digest: String,
    },
    ProvideApproval {
        request_ref: String,
    },
    SurfaceQuestion {
        question_ref: String,
    },
    WaitFor {
        event_ref: String,
        timeout_ms: i64,
    },
    PersistEvidence {
        evidence_ref: String,
    },
}

/// A typed rehydration plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RehydrationPlan {
    pub plan_id: String,
    pub info_id: String,
    pub steps: Vec<RehydrationStep>,
    pub authority_required: Option<String>,
    pub expires_at_ms: Option<i64>,
    pub produced_at_ms: i64,
}

impl RehydrationPlan {
    pub fn new(
        plan_id: impl Into<String>,
        info_id: impl Into<String>,
        produced_at_ms: i64,
    ) -> Self {
        Self {
            plan_id: plan_id.into(),
            info_id: info_id.into(),
            steps: Vec::new(),
            authority_required: None,
            expires_at_ms: None,
            produced_at_ms,
        }
    }

    pub fn with_steps(mut self, steps: Vec<RehydrationStep>) -> Self {
        self.steps = steps;
        self
    }

    pub fn with_authority(mut self, authority: impl Into<String>) -> Self {
        self.authority_required = Some(authority.into());
        self
    }
}

/// Error taxonomy (closed-set, `#[non_exhaustive]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
pub enum HumanResumeError {
    EmptyHumanId {
        info_id: String,
    },
    EmptyCycleRef {
        info_id: String,
    },
    NoSteps {
        plan_id: String,
    },
    UnknownStep {
        plan_id: String,
        index: usize,
    },
    AuthorityMissing {
        plan_id: String,
    },
    NegativeTimeout {
        plan_id: String,
        step_index: usize,
        timeout_ms: i64,
    },
}

impl std::fmt::Display for HumanResumeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HumanResumeError::EmptyHumanId { info_id } => {
                write!(f, "info {info_id} has empty human_id")
            }
            HumanResumeError::EmptyCycleRef { info_id } => {
                write!(f, "info {info_id} has empty cycle_ref")
            }
            HumanResumeError::NoSteps { plan_id } => {
                write!(f, "plan {plan_id} has no steps")
            }
            HumanResumeError::UnknownStep { plan_id, index } => {
                write!(f, "plan {plan_id} step[{index}] unknown")
            }
            HumanResumeError::AuthorityMissing { plan_id } => {
                write!(f, "plan {plan_id} authority missing")
            }
            HumanResumeError::NegativeTimeout {
                plan_id,
                step_index,
                timeout_ms,
            } => write!(
                f,
                "plan {plan_id} step[{step_index}] timeout {timeout_ms} < 0"
            ),
        }
    }
}

impl std::error::Error for HumanResumeError {}

/// Structural validator.
#[derive(Debug, Default, Clone)]
pub struct HumanResumeValidator;

impl HumanResumeValidator {
    pub fn new() -> Self {
        Self
    }

    pub fn validate_info(&self, info: &ResumeInfo) -> Result<(), HumanResumeError> {
        if info.human_id.trim().is_empty() {
            return Err(HumanResumeError::EmptyHumanId {
                info_id: info.info_id.clone(),
            });
        }
        if info.cycle_ref.trim().is_empty() {
            return Err(HumanResumeError::EmptyCycleRef {
                info_id: info.info_id.clone(),
            });
        }
        for d in &info.last_decisions {
            if d.digest.is_empty() {
                return Err(HumanResumeError::AuthorityMissing {
                    plan_id: info.info_id.clone(),
                });
            }
        }
        Ok(())
    }

    pub fn validate_plan(&self, plan: &RehydrationPlan) -> Result<(), HumanResumeError> {
        if plan.steps.is_empty() {
            return Err(HumanResumeError::NoSteps {
                plan_id: plan.plan_id.clone(),
            });
        }
        let mut needs_authority = false;
        for (idx, step) in plan.steps.iter().enumerate() {
            match step {
                RehydrationStep::WaitFor { timeout_ms, .. } if *timeout_ms < 0 => {
                    return Err(HumanResumeError::NegativeTimeout {
                        plan_id: plan.plan_id.clone(),
                        step_index: idx,
                        timeout_ms: *timeout_ms,
                    });
                }
                RehydrationStep::ProvideApproval { .. } => {
                    needs_authority = true;
                }
                _ => {}
            }
        }
        if needs_authority && plan.authority_required.is_none() {
            return Err(HumanResumeError::AuthorityMissing {
                plan_id: plan.plan_id.clone(),
            });
        }
        if let Some(exp) = plan.expires_at_ms
            && exp < plan.produced_at_ms
        {
            return Err(HumanResumeError::AuthorityMissing {
                plan_id: plan.plan_id.clone(),
            });
        }
        Ok(())
    }
}

/// Persistence seam.
pub trait RehydrationStore: Send + Sync + std::fmt::Debug {
    fn put_info(&self, info: ResumeInfo);
    fn info(&self, info_id: &str) -> Option<ResumeInfo>;
    fn put_plan(&self, plan: RehydrationPlan);
    fn plan(&self, plan_id: &str) -> Option<RehydrationPlan>;
    fn list_plans_for_info(&self, info_id: &str) -> Vec<RehydrationPlan>;
}

#[derive(Debug, Default)]
pub struct InMemoryRehydrationStore {
    infos: Mutex<BTreeMap<String, ResumeInfo>>,
    plans: Mutex<BTreeMap<String, RehydrationPlan>>,
}

impl InMemoryRehydrationStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl RehydrationStore for InMemoryRehydrationStore {
    fn put_info(&self, info: ResumeInfo) {
        let mut g = self
            .infos
            .lock()
            .expect("InMemoryRehydrationStore infos poisoned");
        g.insert(info.info_id.clone(), info);
    }
    fn info(&self, info_id: &str) -> Option<ResumeInfo> {
        let g = self
            .infos
            .lock()
            .expect("InMemoryRehydrationStore infos poisoned");
        g.get(info_id).cloned()
    }
    fn put_plan(&self, plan: RehydrationPlan) {
        let mut g = self
            .plans
            .lock()
            .expect("InMemoryRehydrationStore plans poisoned");
        g.insert(plan.plan_id.clone(), plan);
    }
    fn plan(&self, plan_id: &str) -> Option<RehydrationPlan> {
        let g = self
            .plans
            .lock()
            .expect("InMemoryRehydrationStore plans poisoned");
        g.get(plan_id).cloned()
    }
    fn list_plans_for_info(&self, info_id: &str) -> Vec<RehydrationPlan> {
        let g = self
            .plans
            .lock()
            .expect("InMemoryRehydrationStore plans poisoned");
        g.values()
            .filter(|p| p.info_id == info_id)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info_with_human(human_id: &str) -> ResumeInfo {
        ResumeInfo::new("I1", "cycle-269", "HEAD-269", human_id, 1_000, "V1", 1_500)
    }

    #[test]
    fn empty_human_id_rejected() {
        let i = info_with_human("");
        let v = HumanResumeValidator::new();
        let err = v.validate_info(&i).unwrap_err();
        assert!(matches!(err, HumanResumeError::EmptyHumanId { .. }));
    }

    #[test]
    fn empty_cycle_ref_rejected() {
        let mut i = info_with_human("alice");
        i.cycle_ref = "".into();
        let v = HumanResumeValidator::new();
        let err = v.validate_info(&i).unwrap_err();
        assert!(matches!(err, HumanResumeError::EmptyCycleRef { .. }));
    }

    #[test]
    fn plan_no_steps() {
        let p = RehydrationPlan::new("P1", "I1", 1_500);
        let v = HumanResumeValidator::new();
        let err = v.validate_plan(&p).unwrap_err();
        assert!(matches!(err, HumanResumeError::NoSteps { .. }));
    }

    #[test]
    fn negative_waitfor_timeout() {
        let p =
            RehydrationPlan::new("P1", "I1", 1_500).with_steps(vec![RehydrationStep::WaitFor {
                event_ref: "E1".into(),
                timeout_ms: -1,
            }]);
        let v = HumanResumeValidator::new();
        let err = v.validate_plan(&p).unwrap_err();
        assert!(matches!(err, HumanResumeError::NegativeTimeout { .. }));
    }

    #[test]
    fn provide_approval_without_authority() {
        let p = RehydrationPlan::new("P1", "I1", 1_500).with_steps(vec![
            RehydrationStep::ProvideApproval {
                request_ref: "R1".into(),
            },
        ]);
        let v = HumanResumeValidator::new();
        let err = v.validate_plan(&p).unwrap_err();
        assert!(matches!(err, HumanResumeError::AuthorityMissing { .. }));
    }

    #[test]
    fn expires_at_ms_before_produced() {
        let mut p = RehydrationPlan::new("P1", "I1", 1_000).with_steps(vec![
            RehydrationStep::ReadContext {
                context_ref: "C1".into(),
            },
        ]);
        p.expires_at_ms = Some(500);
        let v = HumanResumeValidator::new();
        let err = v.validate_plan(&p).unwrap_err();
        assert!(matches!(err, HumanResumeError::AuthorityMissing { .. }));
    }

    #[test]
    fn empty_digest_decision_summary() {
        let mut i = info_with_human("alice");
        i.last_decisions = vec![DecisionSummary::new("D1", "approved", 500, "")];
        let v = HumanResumeValidator::new();
        let err = v.validate_info(&i).unwrap_err();
        assert!(matches!(err, HumanResumeError::AuthorityMissing { .. }));
    }

    #[test]
    fn info_roundtrip() {
        let store = InMemoryRehydrationStore::new();
        let i = info_with_human("alice");
        store.put_info(i.clone());
        let back = store.info("I1").unwrap();
        assert_eq!(back.info_id, "I1");
        assert_eq!(back.human_id, "alice");
    }

    #[test]
    fn plan_roundtrip() {
        let store = InMemoryRehydrationStore::new();
        let p = RehydrationPlan::new("P1", "I1", 1_500).with_steps(vec![
            RehydrationStep::ReadContext {
                context_ref: "C1".into(),
            },
        ]);
        store.put_plan(p.clone());
        let back = store.plan("P1").unwrap();
        assert_eq!(back.plan_id, "P1");
    }

    #[test]
    fn list_plans_for_info() {
        let store = InMemoryRehydrationStore::new();
        let p1 = RehydrationPlan::new("P1", "I1", 1_500).with_steps(vec![
            RehydrationStep::ReadContext {
                context_ref: "C1".into(),
            },
        ]);
        let p2 = RehydrationPlan::new("P2", "I1", 1_600).with_steps(vec![
            RehydrationStep::ReadContext {
                context_ref: "C2".into(),
            },
        ]);
        store.put_plan(p1);
        store.put_plan(p2);
        let plans = store.list_plans_for_info("I1");
        assert_eq!(plans.len(), 2);
    }
}
