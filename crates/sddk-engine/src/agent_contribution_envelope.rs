//! Agent Contribution Envelope substrate — typed handoff between
//! orchestrator and worker without copying full contexts.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-AgentContributionEnvelope.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-084-AGENT-CONTRIBUTION-ENVELOPE.md

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

/// Immutable/versioned handle binding a `ContextCapsule` rev to a
/// delegation window (CDD-HANDOFF-001 §ContextLease).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextLease {
    pub lease_id: String,
    pub capsule_rev: u64,
    pub digest: String,
    pub issued_to: String,
    pub issued_at_ms: i64,
    pub valid_until_ms: i64,
}

impl ContextLease {
    pub fn new(
        lease_id: impl Into<String>,
        capsule_rev: u64,
        digest: impl Into<String>,
        issued_to: impl Into<String>,
        issued_at_ms: i64,
        valid_until_ms: i64,
    ) -> Self {
        Self {
            lease_id: lease_id.into(),
            capsule_rev,
            digest: digest.into(),
            issued_to: issued_to.into(),
            issued_at_ms,
            valid_until_ms,
        }
    }
}

/// Typed request from a delegator to a delegatee.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationRequest {
    pub delegation_id: String,
    pub from_role: String,
    pub to_role: String,
    pub objective: String,
    pub context_rev: u64,
    pub context_lease: ContextLease,
    pub scope: Vec<String>,
    pub budget_tokens: u64,
    pub created_at_ms: i64,
}

impl DelegationRequest {
    pub fn new(
        delegation_id: impl Into<String>,
        from_role: impl Into<String>,
        to_role: impl Into<String>,
        objective: impl Into<String>,
        context_lease: ContextLease,
        context_rev: u64,
        created_at_ms: i64,
    ) -> Self {
        Self {
            delegation_id: delegation_id.into(),
            from_role: from_role.into(),
            to_role: to_role.into(),
            objective: objective.into(),
            context_rev,
            context_lease,
            scope: Vec::new(),
            budget_tokens: 0,
            created_at_ms,
        }
    }

    pub fn with_scope(mut self, scope: Vec<String>) -> Self {
        self.scope = scope;
        self
    }

    pub fn with_budget(mut self, budget_tokens: u64) -> Self {
        self.budget_tokens = budget_tokens;
        self
    }
}

/// A finding: structured observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub summary: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// A proposal: an alternative or recommendation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Proposal {
    pub id: String,
    pub summary: String,
    pub rationale: String,
}

/// A rejection: alternative the worker considered but discarded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rejection {
    pub id: String,
    pub summary: String,
    pub reason: String,
}

/// Entry describing a portion of the worker's view that no longer
/// matches the parent's `context_rev`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextDeltaEntry {
    pub path: String,
    pub reason: String,
}

/// The 12-field worker→orchestrator projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentContributionEnvelope {
    pub envelope_id: String,
    pub delegation_id: String,
    pub role_id: String,
    pub context_rev: u64,
    pub lease_id: String,
    pub objective: String,
    pub coverage_satisfied: Vec<String>,
    pub coverage_missing: Vec<String>,
    pub findings: Vec<Finding>,
    pub proposals: Vec<Proposal>,
    pub alternatives: Vec<Proposal>,
    pub rejections: Vec<Rejection>,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub assumptions: Vec<String>,
    pub uncertainty: Vec<String>,
    pub risks: Vec<String>,
    pub open_questions: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub artifact_refs: Vec<String>,
    pub context_delta: Vec<ContextDeltaEntry>,
    pub recommendation: String,
    pub confidence: f64,
    pub metrics: BTreeMap<String, f64>,
    pub produced_at_ms: i64,
}

impl AgentContributionEnvelope {
    /// Construct an empty envelope bound to a request.
    pub fn new(
        envelope_id: impl Into<String>,
        role_id: impl Into<String>,
        request: &DelegationRequest,
        produced_at_ms: i64,
    ) -> Self {
        Self {
            envelope_id: envelope_id.into(),
            delegation_id: request.delegation_id.clone(),
            role_id: role_id.into(),
            context_rev: request.context_rev,
            lease_id: request.context_lease.lease_id.clone(),
            objective: request.objective.clone(),
            coverage_satisfied: Vec::new(),
            coverage_missing: Vec::new(),
            findings: Vec::new(),
            proposals: Vec::new(),
            alternatives: Vec::new(),
            rejections: Vec::new(),
            pros: Vec::new(),
            cons: Vec::new(),
            assumptions: Vec::new(),
            uncertainty: Vec::new(),
            risks: Vec::new(),
            open_questions: Vec::new(),
            evidence_refs: Vec::new(),
            artifact_refs: Vec::new(),
            context_delta: Vec::new(),
            recommendation: String::new(),
            confidence: 0.0,
            metrics: BTreeMap::new(),
            produced_at_ms,
        }
    }
}

/// Error taxonomy. Closed-set, `#[non_exhaustive]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
#[non_exhaustive]
pub enum EnvelopeError {
    ContextRevMismatch {
        envelope_id: String,
        expected: u64,
        actual: u64,
    },
    LeaseMismatch {
        envelope_id: String,
        lease_id: String,
    },
    DelegationMismatch {
        envelope_id: String,
        delegation_id: String,
    },
    InvalidLease {
        reason: String,
    },
    UnknownDelegation {
        delegation_id: String,
    },
    DissentRequired {
        reason: String,
    },
}

impl std::fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvelopeError::ContextRevMismatch {
                envelope_id,
                expected,
                actual,
            } => write!(
                f,
                "envelope {envelope_id} context_rev {actual} != request {expected}"
            ),
            EnvelopeError::LeaseMismatch {
                envelope_id,
                lease_id,
            } => write!(
                f,
                "envelope {envelope_id} lease_id {lease_id} != request lease"
            ),
            EnvelopeError::DelegationMismatch {
                envelope_id,
                delegation_id,
            } => write!(
                f,
                "envelope {envelope_id} delegation_id {delegation_id} != request"
            ),
            EnvelopeError::InvalidLease { reason } => write!(f, "invalid lease: {reason}"),
            EnvelopeError::UnknownDelegation { delegation_id } => {
                write!(f, "unknown delegation: {delegation_id}")
            }
            EnvelopeError::DissentRequired { reason } => {
                write!(f, "dissent required: {reason}")
            }
        }
    }
}

impl std::error::Error for EnvelopeError {}

/// Persists envelopes; structural store, not authoritative (raw artifacts win).
pub trait EnvelopeStore: Send + Sync + std::fmt::Debug {
    fn put(&self, env: AgentContributionEnvelope);
    fn get(&self, envelope_id: &str) -> Option<AgentContributionEnvelope>;
    fn by_delegation(&self, delegation_id: &str) -> Vec<AgentContributionEnvelope>;
    fn all(&self) -> Vec<AgentContributionEnvelope>;
}

/// Reference in-memory implementation.
#[derive(Debug, Default)]
pub struct InMemoryEnvelopeStore {
    inner: Mutex<BTreeMap<String, AgentContributionEnvelope>>,
}

impl InMemoryEnvelopeStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl EnvelopeStore for InMemoryEnvelopeStore {
    fn put(&self, env: AgentContributionEnvelope) {
        let mut g = self
            .inner
            .lock()
            .expect("InMemoryEnvelopeStore mutex poisoned");
        g.insert(env.envelope_id.clone(), env);
    }
    fn get(&self, envelope_id: &str) -> Option<AgentContributionEnvelope> {
        let g = self
            .inner
            .lock()
            .expect("InMemoryEnvelopeStore mutex poisoned");
        g.get(envelope_id).cloned()
    }
    fn by_delegation(&self, delegation_id: &str) -> Vec<AgentContributionEnvelope> {
        let g = self
            .inner
            .lock()
            .expect("InMemoryEnvelopeStore mutex poisoned");
        g.values()
            .filter(|e| e.delegation_id == delegation_id)
            .cloned()
            .collect()
    }
    fn all(&self) -> Vec<AgentContributionEnvelope> {
        let g = self
            .inner
            .lock()
            .expect("InMemoryEnvelopeStore mutex poisoned");
        g.values().cloned().collect()
    }
}

/// Lightweight lookup seam for leases.
pub trait LeaseRegistry: Send + Sync + std::fmt::Debug {
    fn get(&self, lease_id: &str) -> Option<ContextLease>;
}

/// HashMap-backed reference impl.
#[derive(Debug, Default)]
pub struct InMemoryLeaseRegistry {
    inner: Mutex<HashMap<String, ContextLease>>,
}

impl InMemoryLeaseRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, lease: ContextLease) {
        let mut g = self
            .inner
            .lock()
            .expect("InMemoryLeaseRegistry mutex poisoned");
        g.insert(lease.lease_id.clone(), lease);
    }
}

impl LeaseRegistry for InMemoryLeaseRegistry {
    fn get(&self, lease_id: &str) -> Option<ContextLease> {
        let g = self
            .inner
            .lock()
            .expect("InMemoryLeaseRegistry mutex poisoned");
        g.get(lease_id).cloned()
    }
}

/// Structural validator for `AgentContributionEnvelope`s.
#[derive(Debug, Clone)]
#[allow(dead_code)] // envelopes reserved for cross-referencing extensions
pub struct EnvelopeValidator {
    /// Optional envelope store for cross-referencing previously seen envelopes.
    envelopes: Option<Arc<dyn EnvelopeStore>>,
    leases: Arc<dyn LeaseRegistry>,
}

impl EnvelopeValidator {
    /// Construct a validator with leases only.
    pub fn with_leases(leases: Arc<dyn LeaseRegistry>) -> Self {
        Self {
            envelopes: None,
            leases,
        }
    }

    /// Construct a validator with both a store and a lease registry.
    pub fn new(envelopes: Arc<dyn EnvelopeStore>, leases: Arc<dyn LeaseRegistry>) -> Self {
        Self {
            envelopes: Some(envelopes),
            leases,
        }
    }

    /// Construct a validator with no dependencies (envelope/lease lookups
    /// become no-ops).
    pub fn offline() -> Self {
        let leases: Arc<dyn LeaseRegistry> = Arc::new(InMemoryLeaseRegistry::new());
        Self {
            envelopes: None,
            leases,
        }
    }

    /// Validate an envelope against its parent request. Implements
    /// all 8 invariants from REQ-AgentContributionEnvelope.
    pub fn validate_envelope(
        &self,
        env: &AgentContributionEnvelope,
        request: &DelegationRequest,
    ) -> Result<(), EnvelopeError> {
        // (1) context_rev
        if env.context_rev != request.context_rev {
            return Err(EnvelopeError::ContextRevMismatch {
                envelope_id: env.envelope_id.clone(),
                expected: request.context_rev,
                actual: env.context_rev,
            });
        }

        // (2) lease_id
        if env.lease_id != request.context_lease.lease_id {
            return Err(EnvelopeError::LeaseMismatch {
                envelope_id: env.envelope_id.clone(),
                lease_id: env.lease_id.clone(),
            });
        }

        // (3) delegation_id
        if env.delegation_id != request.delegation_id {
            return Err(EnvelopeError::DelegationMismatch {
                envelope_id: env.envelope_id.clone(),
                delegation_id: env.delegation_id.clone(),
            });
        }

        // (4) produced_at_ms non-zero
        if env.produced_at_ms == 0 {
            return Err(EnvelopeError::InvalidLease {
                reason: "produced_at_ms is zero".to_string(),
            });
        }

        // (5) Dissent preservation: alternatives => rejections
        if !env.alternatives.is_empty() && env.rejections.is_empty() {
            return Err(EnvelopeError::DissentRequired {
                reason: "alternatives without rejections".to_string(),
            });
        }

        // (6) evidence refs required (structural)
        // Already guaranteed by coverage_satisfied non-empty? Backlog says
        // structural, but a non-empty coverage map implies evidence.
        // We treat empty evidence_refs as DissentRequired-equivalent.
        if env.evidence_refs.is_empty() {
            return Err(EnvelopeError::DissentRequired {
                reason: "evidence_refs is empty".to_string(),
            });
        }

        // (7) Confidence bounded [0, 1]
        if !env.confidence.is_finite() || !(0.0..=1.0).contains(&env.confidence) {
            return Err(EnvelopeError::InvalidLease {
                reason: "confidence out of [0,1]".to_string(),
            });
        }

        // (8) Metrics scalar-only (no NaN)
        if env.metrics.values().any(|v| !v.is_finite() || v.is_nan()) {
            return Err(EnvelopeError::InvalidLease {
                reason: "metrics NaN".to_string(),
            });
        }

        // Cross-reference lease against the lease registry.
        if self.leases.get(&env.lease_id).is_none() {
            return Err(EnvelopeError::UnknownDelegation {
                delegation_id: env.delegation_id.clone(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_request() -> (DelegationRequest, ContextLease) {
        let lease = ContextLease::new("L1", 7, "digest123", "leaf-x", 1_000, 2_000);
        let req = DelegationRequest::new(
            "D1",
            "coord-y",
            "leaf-x",
            "compute deployment",
            lease.clone(),
            7,
            500,
        );
        (req, lease)
    }

    fn make_envelope(req: &DelegationRequest) -> AgentContributionEnvelope {
        let mut env = AgentContributionEnvelope::new("E1", "leaf-x", req, 1_500);
        env.evidence_refs = vec!["cid://abc".to_string()];
        env.coverage_satisfied = vec!["cov-1".to_string()];
        env.confidence = 0.8;
        env
    }

    fn registry_with_lease() -> (Arc<InMemoryEnvelopeStore>, Arc<InMemoryLeaseRegistry>) {
        let (_, lease) = make_request();
        let store = Arc::new(InMemoryEnvelopeStore::new());
        let leases = Arc::new(InMemoryLeaseRegistry::new());
        leases.register(lease);
        (store, leases)
    }

    #[test]
    fn envelope_rev_matches_request_rev() {
        let (req, _) = make_request();
        let env = make_envelope(&req);
        let (store, leases) = registry_with_lease();
        let v = EnvelopeValidator::new(store, leases);
        v.validate_envelope(&env, &req).unwrap();
    }

    #[test]
    fn envelope_rev_mismatch() {
        let (mut req, _) = make_request();
        req.context_rev = 7;
        let mut env = make_envelope(&req);
        env.context_rev = 8;
        let (store, leases) = registry_with_lease();
        let v = EnvelopeValidator::new(store, leases);
        let err = v.validate_envelope(&env, &req).unwrap_err();
        assert!(matches!(
            err,
            EnvelopeError::ContextRevMismatch {
                expected: 7,
                actual: 8,
                ..
            }
        ));
    }

    #[test]
    fn envelope_lease_mismatch() {
        let (req, _) = make_request();
        let mut env = make_envelope(&req);
        env.lease_id = "L2".to_string();
        let (store, leases) = registry_with_lease();
        let v = EnvelopeValidator::new(store, leases);
        let err = v.validate_envelope(&env, &req).unwrap_err();
        assert!(matches!(err, EnvelopeError::LeaseMismatch { .. }));
    }

    #[test]
    fn envelope_delegation_id_mismatch() {
        let (req, _) = make_request();
        let mut env = make_envelope(&req);
        env.delegation_id = "D2".to_string();
        let (store, leases) = registry_with_lease();
        let v = EnvelopeValidator::new(store, leases);
        let err = v.validate_envelope(&env, &req).unwrap_err();
        assert!(matches!(err, EnvelopeError::DelegationMismatch { .. }));
    }

    #[test]
    fn empty_produced_at_ms_rejected() {
        let (req, _) = make_request();
        let mut env = make_envelope(&req);
        env.produced_at_ms = 0;
        let (store, leases) = registry_with_lease();
        let v = EnvelopeValidator::new(store, leases);
        let err = v.validate_envelope(&env, &req).unwrap_err();
        assert!(matches!(err, EnvelopeError::InvalidLease { .. }));
    }

    #[test]
    fn alternatives_without_rejections() {
        let (req, _) = make_request();
        let mut env = make_envelope(&req);
        env.alternatives = vec![Proposal {
            id: "alt-1".into(),
            summary: "alt".into(),
            rationale: "r".into(),
        }];
        env.rejections.clear();
        let (store, leases) = registry_with_lease();
        let v = EnvelopeValidator::new(store, leases);
        let err = v.validate_envelope(&env, &req).unwrap_err();
        assert!(matches!(err, EnvelopeError::DissentRequired { .. }));
    }

    #[test]
    fn confidence_out_of_bounds() {
        let (req, _) = make_request();
        let mut env = make_envelope(&req);
        env.confidence = 1.5;
        let (store, leases) = registry_with_lease();
        let v = EnvelopeValidator::new(store, leases);
        let err = v.validate_envelope(&env, &req).unwrap_err();
        assert!(matches!(err, EnvelopeError::InvalidLease { .. }));
    }

    #[test]
    fn metrics_nan_rejected() {
        let (req, _) = make_request();
        let mut env = make_envelope(&req);
        env.metrics.insert("x".into(), f64::NAN);
        let (store, leases) = registry_with_lease();
        let v = EnvelopeValidator::new(store, leases);
        let err = v.validate_envelope(&env, &req).unwrap_err();
        assert!(matches!(err, EnvelopeError::InvalidLease { .. }));
    }

    #[test]
    fn store_roundtrip() {
        let (req, _) = make_request();
        let env = make_envelope(&req);
        let store = InMemoryEnvelopeStore::new();
        store.put(env.clone());
        let got = store.get("E1").unwrap();
        assert_eq!(got.envelope_id, "E1");
        let by_deleg = store.by_delegation("D1");
        assert_eq!(by_deleg.len(), 1);
    }

    #[test]
    fn coverage_field_roundtrip() {
        let (req, _) = make_request();
        let env = make_envelope(&req);
        let json = serde_json::to_string(&env).unwrap();
        let back: AgentContributionEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(back.coverage_satisfied, env.coverage_satisfied);
        assert_eq!(back.envelope_id, env.envelope_id);
    }
}
