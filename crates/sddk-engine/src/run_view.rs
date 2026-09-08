//! Current Run View — `RunStateView` and `ActionSurfaceView`.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-CurrentRunView-Shape.md
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-CurrentRunView-Actions.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-075-CURRENT-RUN-VIEW-SHAPE.md

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Origin of a workflow run — declared (planning-ledger cycle) or generated
/// (workflow runtime DAG).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RunOrigin {
    Declared,
    Generated,
}

/// Pure snapshot of a persisted run. No policy, no actions — those live on
/// `ActionSurfaceView`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunStateView {
    origin: RunOrigin,
    run_id: String,
    frontier: Vec<String>,
    blockers: Vec<String>,
    pending_decisions: Vec<String>,
    evaluated_at: u64,
}

impl RunStateView {
    /// Test-only constructor. Real implementations read from the ledger.
    pub fn for_test(
        origin: RunOrigin,
        run_id: impl Into<String>,
        frontier: Vec<String>,
        blockers: Vec<String>,
        pending_decisions: Vec<String>,
        evaluated_at: u64,
    ) -> Self {
        Self {
            origin,
            run_id: run_id.into(),
            frontier,
            blockers,
            pending_decisions,
            evaluated_at,
        }
    }

    pub fn origin(&self) -> RunOrigin {
        self.origin
    }
    pub fn run_id(&self) -> &str {
        &self.run_id
    }
    pub fn frontier(&self) -> &[String] {
        &self.frontier
    }
    pub fn blockers(&self) -> &[String] {
        &self.blockers
    }
    pub fn pending_decisions(&self) -> &[String] {
        &self.pending_decisions
    }
    pub fn evaluated_at(&self) -> u64 {
        self.evaluated_at
    }
}

/// Build a `RunStateView` for the given run id at a given event sequence.
///
/// This is the v0 scaffold: the real implementation reads from the
/// persisted ledger (event log + workflow run table). For now we accept
/// the populated fields directly so callers and tests can compose the
/// view from upstream data.
pub fn build_run_state_view(
    run_id: impl Into<String>,
    evaluated_at: u64,
    origin: RunOrigin,
    frontier: Vec<String>,
    blockers: Vec<String>,
    pending_decisions: Vec<String>,
    _as_of_event_seq: u64,
) -> Result<RunStateView, ViewError> {
    Ok(RunStateView {
        origin,
        run_id: run_id.into(),
        frontier,
        blockers,
        pending_decisions,
        evaluated_at,
    })
}

/// Closed taxonomy of legal next actions on a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ActionKind {
    Start,
    Resume,
    Abort,
    Approve,
    Escalate,
    Retry,
    Reconcile,
}

/// Admission rule per `ActionKind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdmissionRule {
    /// Always admit if preconditions hold.
    Admit,
    /// Never admit, even if preconditions hold.
    Deny,
}

/// Immutable policy snapshot used to compute `ActionSurfaceView`.
#[derive(Debug, Clone)]
pub struct PolicySnapshot {
    id: String,
    admit: BTreeMap<ActionKind, AdmissionRule>,
}

impl PolicySnapshot {
    /// Default policy: admit all variants (preconditions decide).
    pub fn default() -> Self {
        let mut admit = BTreeMap::new();
        admit.insert(ActionKind::Start, AdmissionRule::Admit);
        admit.insert(ActionKind::Resume, AdmissionRule::Admit);
        admit.insert(ActionKind::Abort, AdmissionRule::Admit);
        admit.insert(ActionKind::Approve, AdmissionRule::Admit);
        admit.insert(ActionKind::Escalate, AdmissionRule::Admit);
        admit.insert(ActionKind::Retry, AdmissionRule::Admit);
        admit.insert(ActionKind::Reconcile, AdmissionRule::Admit);
        Self {
            id: "default".to_string(),
            admit,
        }
    }

    /// Test helper: policy that denies `Resume`.
    pub fn deny_resume() -> Self {
        let mut p = Self::default();
        p.id = "deny-resume".to_string();
        p.admit.insert(ActionKind::Resume, AdmissionRule::Deny);
        p
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn admits(&self, kind: ActionKind) -> bool {
        matches!(self.admit.get(&kind), Some(AdmissionRule::Admit))
    }

    pub fn digest(&self) -> u64 {
        // Stable hash of (id, admit entries)
        let mut h: u64 = 1469598103934665603; // FNV-1a 64 offset basis
        for b in self.id.bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(1099511628211);
        }
        // Walk sorted keys for determinism (BTreeMap already sorted).
        for (kind, rule) in &self.admit {
            let kind_byte = kind_discriminant(kind);
            h ^= kind_byte;
            h = h.wrapping_mul(1099511628211);
            let rule_byte = match rule {
                AdmissionRule::Admit => 1u8,
                AdmissionRule::Deny => 2u8,
            };
            h ^= rule_byte as u64;
            h = h.wrapping_mul(1099511628211);
        }
        h
    }
}

fn kind_discriminant(kind: &ActionKind) -> u64 {
    match kind {
        ActionKind::Start => 1,
        ActionKind::Resume => 2,
        ActionKind::Abort => 3,
        ActionKind::Approve => 4,
        ActionKind::Escalate => 5,
        ActionKind::Retry => 6,
        ActionKind::Reconcile => 7,
    }
}

/// Eagerly-computed action surface for a `RunStateView` under a `PolicySnapshot`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionSurfaceView {
    origin: RunOrigin,
    run_id: String,
    evaluated_at: u64,
    available_actions: Vec<ActionKind>,
    policy_digest: u64,
}

impl ActionSurfaceView {
    pub fn origin(&self) -> RunOrigin {
        self.origin
    }
    pub fn run_id(&self) -> &str {
        &self.run_id
    }
    pub fn evaluated_at(&self) -> u64 {
        self.evaluated_at
    }
    pub fn available_actions(&self) -> &[ActionKind] {
        &self.available_actions
    }
    pub fn policy_digest(&self) -> u64 {
        self.policy_digest
    }
}

/// Build an `ActionSurfaceView` for the given `RunStateView` under `policy`.
/// Eager: `available_actions` is fully populated at construction.
pub fn build_action_surface_view(
    state: &RunStateView,
    policy: &PolicySnapshot,
) -> Result<ActionSurfaceView, ViewError> {
    let mut actions = Vec::new();
    for kind in closed_taxonomy() {
        if !policy.admits(*kind) {
            continue;
        }
        if preconditions_hold(*kind, state) {
            actions.push(*kind);
        }
    }
    Ok(ActionSurfaceView {
        origin: state.origin(),
        run_id: state.run_id().to_string(),
        evaluated_at: state.evaluated_at(),
        available_actions: actions,
        policy_digest: policy.digest(),
    })
}

fn closed_taxonomy() -> &'static [ActionKind] {
    &[
        ActionKind::Start,
        ActionKind::Resume,
        ActionKind::Abort,
        ActionKind::Approve,
        ActionKind::Escalate,
        ActionKind::Retry,
        ActionKind::Reconcile,
    ]
}

fn preconditions_hold(kind: ActionKind, state: &RunStateView) -> bool {
    match kind {
        ActionKind::Start => false, // lifecycle-level; deferred to declared origin
        ActionKind::Resume => !state.frontier().is_empty() && state.blockers().is_empty(),
        ActionKind::Abort => true,
        ActionKind::Approve => state
            .pending_decisions()
            .iter()
            .any(|d| d.starts_with("approval")),
        ActionKind::Escalate => state
            .pending_decisions()
            .iter()
            .any(|d| d.starts_with("escalation")),
        ActionKind::Retry => false, // requires retry_policy context; deferred
        ActionKind::Reconcile => false, // requires drift indicator; deferred
    }
}

/// Errors from view construction. Reserved for future storage-backed
/// implementations; the current scaffold is infallible.
#[derive(Debug, thiserror::Error)]
pub enum ViewError {
    #[error("storage error: {0}")]
    Storage(String),
    #[error("policy `{0}` not found")]
    PolicyNotFound(String),
    #[error("invariant violation: {0}")]
    Invariant(String),
}
