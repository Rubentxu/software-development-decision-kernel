//! Risk-Sensitive Approval Policy substrate — typed policy engine
//! gating `HumanDecisionRequest` before port admission.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-RiskApprovalPolicy.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-087-RISK-APPROVAL-POLICY.md

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::continuation_candidate::Reversibility;

/// Risk tier (strict partial order Trivial<Low<Medium<High<Critical).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RiskTier {
    Trivial,
    Low,
    Medium,
    High,
    Critical,
}

impl RiskTier {
    fn rank(self) -> i32 {
        match self {
            RiskTier::Trivial => 0,
            RiskTier::Low => 1,
            RiskTier::Medium => 2,
            RiskTier::High => 3,
            RiskTier::Critical => 4,
        }
    }
}

/// Verdict of the approval engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ApprovalVerdict {
    /// Pass through with no extra obligations.
    AutoApprove,
    /// Approved given the request data, no human required.
    Clear,
    /// Needs human approval; return evidence.
    NeedsApproval,
    /// Escalate to a higher authority.
    Escalate,
    /// Block outright.
    Block,
}

/// Plain-data policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalPolicy {
    pub policy_id: String,
    pub tier_cap_for_auto: RiskTier,
    pub evidence_required_above: RiskTier,
    pub budget_review_threshold: u64,
    pub irreversible_escalates: bool,
    pub per_cycle_limit: u32,
}

impl ApprovalPolicy {
    pub fn new(policy_id: impl Into<String>) -> Self {
        Self {
            policy_id: policy_id.into(),
            tier_cap_for_auto: RiskTier::Trivial,
            evidence_required_above: RiskTier::High,
            budget_review_threshold: 0,
            irreversible_escalates: false,
            per_cycle_limit: u32::MAX,
        }
    }

    pub fn with_tier_cap_for_auto(mut self, t: RiskTier) -> Self {
        self.tier_cap_for_auto = t;
        self
    }
    pub fn with_evidence_required_above(mut self, t: RiskTier) -> Self {
        self.evidence_required_above = t;
        self
    }
    pub fn with_budget_review_threshold(mut self, t: u64) -> Self {
        self.budget_review_threshold = t;
        self
    }
    pub fn with_irreversible_escalates(mut self, b: bool) -> Self {
        self.irreversible_escalates = b;
        self
    }
    pub fn with_per_cycle_limit(mut self, n: u32) -> Self {
        self.per_cycle_limit = n;
        self
    }
}

/// Context for the engine: per-call data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalContext {
    pub risk_tier: RiskTier,
    pub reversibility: Reversibility,
    pub evidence_refs: Vec<String>,
    pub budget_tokens: u64,
    pub actor: String,
    pub actions_seen_in_cycle: u32,
}

impl ApprovalContext {
    pub fn new(actor: impl Into<String>, risk_tier: RiskTier) -> Self {
        Self {
            risk_tier,
            reversibility: Reversibility::Unknown,
            evidence_refs: Vec::new(),
            budget_tokens: 0,
            actor: actor.into(),
            actions_seen_in_cycle: 0,
        }
    }
}

/// Engine verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalDecision {
    pub verdict: ApprovalVerdict,
    pub reasons: Vec<String>,
    pub required_authority: Option<String>,
}

impl ApprovalDecision {
    pub fn auto() -> Self {
        Self {
            verdict: ApprovalVerdict::AutoApprove,
            reasons: Vec::new(),
            required_authority: None,
        }
    }
}

/// Error taxonomy (closed-set, `#[non_exhaustive]`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
pub enum PolicyError {
    UnknownPolicy { policy_id: String },
    InvalidThreshold { field: String, value: String },
    MissingActor,
}

impl std::fmt::Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyError::UnknownPolicy { policy_id } => {
                write!(f, "unknown policy {policy_id}")
            }
            PolicyError::InvalidThreshold { field, value } => {
                write!(f, "invalid threshold {field}={value}")
            }
            PolicyError::MissingActor => write!(f, "actor required for evaluation"),
        }
    }
}

impl std::error::Error for PolicyError {}

/// The deterministic approval engine.
#[derive(Debug, Clone)]
pub struct ApprovalEngine {
    policy: ApprovalPolicy,
}

impl ApprovalEngine {
    /// Construct an engine. Returns `InvalidThreshold` if the policy
    /// is malformed.
    pub fn new(policy: ApprovalPolicy) -> Result<Self, PolicyError> {
        if policy.tier_cap_for_auto.rank() > policy.evidence_required_above.rank() {
            return Err(PolicyError::InvalidThreshold {
                field: "tier_cap_for_auto".to_string(),
                value: format!("{:?}", policy.tier_cap_for_auto),
            });
        }
        Ok(Self { policy })
    }

    pub fn policy(&self) -> &ApprovalPolicy {
        &self.policy
    }

    /// Deterministic per-call evaluation.
    pub fn evaluate(&self, ctx: &ApprovalContext) -> Result<ApprovalDecision, PolicyError> {
        if ctx.actor.trim().is_empty() {
            return Err(PolicyError::MissingActor);
        }

        // Per-cycle limit (highest precedence)
        if ctx.actions_seen_in_cycle >= self.policy.per_cycle_limit {
            return Ok(ApprovalDecision {
                verdict: ApprovalVerdict::Block,
                reasons: vec!["per_cycle_limit".to_string()],
                required_authority: Some(ctx.actor.clone()),
            });
        }

        // Irreversible escalation
        if self.policy.irreversible_escalates
            && matches!(ctx.reversibility, Reversibility::Irreversible)
        {
            return Ok(ApprovalDecision {
                verdict: ApprovalVerdict::Escalate,
                reasons: vec!["irreversible_action".to_string()],
                required_authority: Some(ctx.actor.clone()),
            });
        }

        // Budget review
        if ctx.budget_tokens > self.policy.budget_review_threshold {
            return Ok(ApprovalDecision {
                verdict: ApprovalVerdict::NeedsApproval,
                reasons: vec!["budget_review".to_string()],
                required_authority: Some(ctx.actor.clone()),
            });
        }

        // Evidence required at or above threshold
        if ctx.risk_tier.rank() >= self.policy.evidence_required_above.rank()
            && ctx.evidence_refs.is_empty()
        {
            return Ok(ApprovalDecision {
                verdict: ApprovalVerdict::NeedsApproval,
                reasons: vec!["evidence_required".to_string()],
                required_authority: Some(ctx.actor.clone()),
            });
        }

        // Below tier cap ⇒ auto-approve
        if ctx.risk_tier.rank() <= self.policy.tier_cap_for_auto.rank() {
            return Ok(ApprovalDecision::auto());
        }

        // Between cap and evidence threshold ⇒ clear
        Ok(ApprovalDecision {
            verdict: ApprovalVerdict::Clear,
            reasons: Vec::new(),
            required_authority: None,
        })
    }
}

/// Persist policies.
pub trait PolicyStore: Send + Sync + std::fmt::Debug {
    fn put(&self, policy: ApprovalPolicy);
    fn get(&self, policy_id: &str) -> Option<ApprovalPolicy>;
    fn list(&self) -> Vec<ApprovalPolicy>;
}

#[derive(Debug, Default)]
pub struct InMemoryPolicyStore {
    inner: Mutex<BTreeMap<String, ApprovalPolicy>>,
}

impl InMemoryPolicyStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl PolicyStore for InMemoryPolicyStore {
    fn put(&self, policy: ApprovalPolicy) {
        let mut g = self
            .inner
            .lock()
            .expect("InMemoryPolicyStore mutex poisoned");
        g.insert(policy.policy_id.clone(), policy);
    }
    fn get(&self, policy_id: &str) -> Option<ApprovalPolicy> {
        let g = self
            .inner
            .lock()
            .expect("InMemoryPolicyStore mutex poisoned");
        g.get(policy_id).cloned()
    }
    fn list(&self) -> Vec<ApprovalPolicy> {
        let g = self
            .inner
            .lock()
            .expect("InMemoryPolicyStore mutex poisoned");
        g.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn permissive_policy() -> ApprovalPolicy {
        ApprovalPolicy::new("permissive")
            .with_tier_cap_for_auto(RiskTier::Trivial)
            .with_evidence_required_above(RiskTier::High)
            .with_budget_review_threshold(1_000)
            .with_irreversible_escalates(true)
            .with_per_cycle_limit(3)
    }

    fn ctx(tier: RiskTier, ev: Vec<&str>, budget: u64) -> ApprovalContext {
        let mut c = ApprovalContext::new("alice", tier);
        c.evidence_refs = ev.into_iter().map(String::from).collect();
        c.budget_tokens = budget;
        c
    }

    #[test]
    fn auto_approve_below_cap() {
        let p = permissive_policy();
        let e = ApprovalEngine::new(p).unwrap();
        let c = ctx(RiskTier::Trivial, vec!["cid://a"], 0);
        let d = e.evaluate(&c).unwrap();
        assert_eq!(d.verdict, ApprovalVerdict::AutoApprove);
    }

    #[test]
    fn clear_between_cap_and_threshold() {
        let p = permissive_policy();
        let e = ApprovalEngine::new(p).unwrap();
        let c = ctx(RiskTier::Medium, vec!["cid://a"], 0);
        let d = e.evaluate(&c).unwrap();
        assert_eq!(d.verdict, ApprovalVerdict::Clear);
    }

    #[test]
    fn evidence_required_above_threshold() {
        let p = permissive_policy();
        let e = ApprovalEngine::new(p).unwrap();
        let c = ctx(RiskTier::High, Vec::new(), 0);
        let d = e.evaluate(&c).unwrap();
        assert_eq!(d.verdict, ApprovalVerdict::NeedsApproval);
        assert!(d.required_authority.is_some());
    }

    #[test]
    fn budget_review() {
        let p = permissive_policy();
        let e = ApprovalEngine::new(p).unwrap();
        let c = ctx(RiskTier::Low, vec!["cid://a"], 5_000);
        let d = e.evaluate(&c).unwrap();
        assert_eq!(d.verdict, ApprovalVerdict::NeedsApproval);
    }

    #[test]
    fn irreversible_escalates() {
        let p = permissive_policy();
        let e = ApprovalEngine::new(p).unwrap();
        let mut c = ctx(RiskTier::Trivial, vec!["cid://a"], 0);
        c.reversibility = Reversibility::Irreversible;
        let d = e.evaluate(&c).unwrap();
        assert_eq!(d.verdict, ApprovalVerdict::Escalate);
    }

    #[test]
    fn per_cycle_limit_blocks() {
        let p = permissive_policy();
        let e = ApprovalEngine::new(p).unwrap();
        let mut c = ctx(RiskTier::Trivial, vec!["cid://a"], 0);
        c.actions_seen_in_cycle = 3;
        let d = e.evaluate(&c).unwrap();
        assert_eq!(d.verdict, ApprovalVerdict::Block);
    }

    #[test]
    fn invalid_threshold_at_construction() {
        let p = ApprovalPolicy::new("bad")
            .with_tier_cap_for_auto(RiskTier::High)
            .with_evidence_required_above(RiskTier::Low);
        let err = ApprovalEngine::new(p).unwrap_err();
        assert!(matches!(err, PolicyError::InvalidThreshold { .. }));
    }

    #[test]
    fn missing_actor() {
        let p = permissive_policy();
        let e = ApprovalEngine::new(p).unwrap();
        let mut c = ctx(RiskTier::Trivial, vec!["cid://a"], 0);
        c.actor = "".into();
        let err = e.evaluate(&c).unwrap_err();
        assert!(matches!(err, PolicyError::MissingActor));
    }

    #[test]
    fn store_roundtrip() {
        let store = InMemoryPolicyStore::new();
        let p = permissive_policy();
        store.put(p.clone());
        let back = store.get("permissive").unwrap();
        assert_eq!(back.policy_id, "permissive");
        assert_eq!(store.list().len(), 1);
    }

    #[test]
    fn needs_approval_carries_required_authority() {
        let p = permissive_policy();
        let e = ApprovalEngine::new(p).unwrap();
        let c = ctx(RiskTier::High, Vec::new(), 0);
        let d = e.evaluate(&c).unwrap();
        assert_eq!(d.verdict, ApprovalVerdict::NeedsApproval);
        assert!(d.required_authority.is_some());
    }
}
