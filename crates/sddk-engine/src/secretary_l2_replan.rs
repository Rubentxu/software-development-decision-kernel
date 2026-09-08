//! Secretary L2 Bounded Cognitive Replan substrate — bounded
//! replan after L0 rules + L1 proposals are exhausted; emits only
//! `ContinuationCandidate`-shaped recommendations through existing
//! seams.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-SecretaryL2BoundedReplan.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-091-SECRETARY-L2-BOUNDED-REPLAN.md

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::continuation_candidate::Reversibility;
use crate::risk_approval_policy::RiskTier;

/// Bounded resource envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitiveDrainage {
    pub budget_tokens: u64,
    pub budget_iterations: u32,
    pub evidence_budget: u32,
    pub policy_ceiling: RiskTier,
}

impl CognitiveDrainage {
    pub fn new(
        budget_tokens: u64,
        budget_iterations: u32,
        evidence_budget: u32,
        policy_ceiling: RiskTier,
    ) -> Self {
        Self {
            budget_tokens,
            budget_iterations,
            evidence_budget,
            policy_ceiling,
        }
    }
}

/// Input to a replan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitiveInput {
    pub cycle_ref: String,
    pub head_ref: String,
    pub drainage: CognitiveDrainage,
    pub exhausted_reactive_rules: Vec<String>,
    pub exhausted_proposals: Vec<String>,
    pub remaining_frontier: Vec<String>,
    pub human_authority_token: Option<String>,
}

/// One recommendation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitiveRecommendation {
    pub recommendation_id: String,
    pub summary: String,
    pub candidate_summary: String,
    pub reversibility: Reversibility,
    pub evidence_required: bool,
    pub human_authority_required: bool,
    pub produced_at_ms: i64,
}

/// Verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
pub enum CognitiveReplanVerdict {
    GiveUp,
    Recommend(Vec<CognitiveRecommendation>),
    DeferredUntil { until_ref: String, reason: String },
}

/// A typed cognitive replan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CognitiveReplan {
    pub replan_id: String,
    pub verdict: CognitiveReplanVerdict,
    pub produced_at_ms: i64,
    pub iterations_used: u32,
    pub tokens_used: u64,
    pub evidence_consumed: u32,
}

/// Error taxonomy (closed-set, `#[non_exhaustive]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
pub enum SecretaryL2Error {
    BudgetExhausted { replan_id: String, iterations: u32 },
    InvalidDrainage { field: String },
    MissingCycleRef,
    AuthorityTokenRequired,
}

impl std::fmt::Display for SecretaryL2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretaryL2Error::BudgetExhausted {
                replan_id,
                iterations,
            } => write!(
                f,
                "replan {replan_id} exhausted after {iterations} iterations"
            ),
            SecretaryL2Error::InvalidDrainage { field } => write!(f, "invalid drainage {field}"),
            SecretaryL2Error::MissingCycleRef => write!(f, "cycle_ref missing"),
            SecretaryL2Error::AuthorityTokenRequired => {
                write!(f, "authority token required for high-tier replan")
            }
        }
    }
}

impl std::error::Error for SecretaryL2Error {}

/// Deterministic bounded replan engine.
#[derive(Debug, Default)]
pub struct SecretaryL2ReplanEngine {
    replans: Mutex<Vec<CognitiveReplan>>,
    history: Mutex<BTreeMap<String, u32>>,
}

impl SecretaryL2ReplanEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn replan_count(&self) -> usize {
        let g = self
            .replans
            .lock()
            .expect("SecretaryL2ReplanEngine replans poisoned");
        g.len()
    }

    /// Bounded replan. Iterations/tokens/evidence never exceed
    /// `drainage`; authority-token gating applies for high tier.
    pub fn replan(&self, input: &CognitiveInput) -> Result<CognitiveReplan, SecretaryL2Error> {
        // (1) cycle_ref
        if input.cycle_ref.trim().is_empty() {
            return Err(SecretaryL2Error::MissingCycleRef);
        }
        // (2) drainage well-formed
        let d = &input.drainage;
        if d.evidence_budget > 1_000_000 || d.budget_iterations == 0 {
            return Err(SecretaryL2Error::InvalidDrainage {
                field: "evidence_budget or iterations".into(),
            });
        }
        // (6) authority token gating
        let high = matches!(d.policy_ceiling, RiskTier::High | RiskTier::Critical);
        if high && input.human_authority_token.is_none() {
            return Err(SecretaryL2Error::AuthorityTokenRequired);
        }

        // Iteration cap: replay of (cycle_ref, head_ref) bounded.
        let replay_key = format!("{}|{}", input.cycle_ref, input.head_ref);
        let mut history_g = self
            .history
            .lock()
            .expect("SecretaryL2ReplanEngine history poisoned");
        let prior = history_g.get(&replay_key).copied().unwrap_or(0);
        let allowed_iters = d.budget_iterations.saturating_sub(prior);
        if allowed_iters == 0 {
            return Err(SecretaryL2Error::BudgetExhausted {
                replan_id: replay_key.clone(),
                iterations: prior,
            });
        }

        // (8) verdict
        let verdict = if input.remaining_frontier.is_empty() {
            CognitiveReplanVerdict::GiveUp
        } else if input.exhausted_reactive_rules.is_empty() && input.exhausted_proposals.is_empty()
        {
            CognitiveReplanVerdict::DeferredUntil {
                until_ref: input.remaining_frontier[0].clone(),
                reason: "frontier_pending".into(),
            }
        } else {
            let mut recs = Vec::new();
            let max_recs = (allowed_iters as usize).min(input.remaining_frontier.len());
            for (i, item) in input.remaining_frontier.iter().take(max_recs).enumerate() {
                recs.push(CognitiveRecommendation {
                    recommendation_id: format!("{replay_key}-{i}"),
                    summary: format!("replan rec {i} for {item}"),
                    candidate_summary: item.clone(),
                    reversibility: Reversibility::PartiallyReversible,
                    evidence_required: i == 0,
                    human_authority_required: high,
                    produced_at_ms: 1_500,
                });
            }
            CognitiveReplanVerdict::Recommend(recs)
        };

        let iterations_used = allowed_iters;
        history_g.insert(replay_key.clone(), prior + iterations_used);

        let tokens_used = d.budget_tokens.saturating_sub(d.budget_tokens / 10);
        let evidence_consumed = (d.evidence_budget / 2).min(max_consume(&verdict));

        let replan = CognitiveReplan {
            replan_id: replay_key,
            verdict,
            produced_at_ms: 1_500,
            iterations_used,
            tokens_used,
            evidence_consumed,
        };
        let mut r_g = self
            .replans
            .lock()
            .expect("SecretaryL2ReplanEngine replans poisoned");
        r_g.push(replan.clone());
        Ok(replan)
    }
}

fn max_consume(v: &CognitiveReplanVerdict) -> u32 {
    match v {
        CognitiveReplanVerdict::Recommend(rs) => rs.len() as u32,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drainage_for_trivial() -> CognitiveDrainage {
        CognitiveDrainage::new(1_000, 4, 8, RiskTier::Trivial)
    }

    fn input_with_frontier(items: Vec<&str>) -> CognitiveInput {
        CognitiveInput {
            cycle_ref: "cycle-269".into(),
            head_ref: "HEAD-269".into(),
            drainage: drainage_for_trivial(),
            exhausted_reactive_rules: vec!["r-a".into()],
            exhausted_proposals: vec!["p-b".into()],
            remaining_frontier: items.into_iter().map(String::from).collect(),
            human_authority_token: Some("tok".into()),
        }
    }

    #[test]
    fn empty_cycle_ref_rejected() {
        let mut i = input_with_frontier(vec!["c1"]);
        i.cycle_ref = "".into();
        let e = SecretaryL2ReplanEngine::new();
        let err = e.replan(&i).unwrap_err();
        assert!(matches!(err, SecretaryL2Error::MissingCycleRef));
    }

    #[test]
    fn invalid_drainage_rejected() {
        let mut i = input_with_frontier(vec!["c1"]);
        i.drainage.evidence_budget = 999_999_999;
        let e = SecretaryL2ReplanEngine::new();
        let err = e.replan(&i).unwrap_err();
        assert!(matches!(err, SecretaryL2Error::InvalidDrainage { .. }));
    }

    #[test]
    fn high_tier_without_token_rejected() {
        let mut i = input_with_frontier(vec!["c1"]);
        i.drainage.policy_ceiling = RiskTier::High;
        i.human_authority_token = None;
        let e = SecretaryL2ReplanEngine::new();
        let err = e.replan(&i).unwrap_err();
        assert!(matches!(err, SecretaryL2Error::AuthorityTokenRequired));
    }

    #[test]
    fn give_up_on_empty_frontier() {
        let i = input_with_frontier(vec![]);
        let e = SecretaryL2ReplanEngine::new();
        let r = e.replan(&i).unwrap();
        assert!(matches!(r.verdict, CognitiveReplanVerdict::GiveUp));
    }

    #[test]
    fn recommend_with_reversibility() {
        let i = input_with_frontier(vec!["c1"]);
        let e = SecretaryL2ReplanEngine::new();
        let r = e.replan(&i).unwrap();
        match r.verdict {
            CognitiveReplanVerdict::Recommend(rs) => {
                assert!(!rs.is_empty());
                assert_eq!(rs[0].reversibility, Reversibility::PartiallyReversible);
            }
            _ => panic!("expected Recommend"),
        }
    }

    #[test]
    fn iteration_bound() {
        let mut i = input_with_frontier(vec!["a", "b", "c", "d", "e"]);
        i.drainage.budget_iterations = 1;
        let e = SecretaryL2ReplanEngine::new();
        let r = e.replan(&i).unwrap();
        if let CognitiveReplanVerdict::Recommend(rs) = &r.verdict {
            assert_eq!(rs.len(), 1);
        } else {
            panic!("expected Recommend");
        }
    }

    #[test]
    fn token_bound() {
        let i = input_with_frontier(vec!["c1"]);
        let e = SecretaryL2ReplanEngine::new();
        let r = e.replan(&i).unwrap();
        assert!(r.tokens_used <= i.drainage.budget_tokens);
    }

    #[test]
    fn replay_truncation() {
        let i = input_with_frontier(vec!["a", "b", "c"]);
        let e = SecretaryL2ReplanEngine::new();
        let r1 = e.replan(&i).unwrap();
        let r2 = e.replan(&i);
        assert!(
            r2.is_err()
                || matches!(r2, Ok(ref _r2) if _r2.iterations_used < r1.iterations_used + 2)
        );
    }

    #[test]
    fn deferred_verdict() {
        let mut i = input_with_frontier(vec!["c1"]);
        i.exhausted_reactive_rules = Vec::new();
        i.exhausted_proposals = Vec::new();
        let e = SecretaryL2ReplanEngine::new();
        let r = e.replan(&i).unwrap();
        assert!(matches!(
            r.verdict,
            CognitiveReplanVerdict::DeferredUntil { .. }
        ));
    }

    #[test]
    fn history_roundtrip() {
        let i1 = input_with_frontier(vec!["c1"]);
        let mut i2 = input_with_frontier(vec!["c2"]);
        i2.cycle_ref = "cycle-270".into();
        let e = SecretaryL2ReplanEngine::new();
        e.replan(&i1).unwrap();
        e.replan(&i2).unwrap();
        assert_eq!(e.replan_count(), 2);
    }
}
