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
///
/// Three-state verdict (P4, ADR-077):
/// - `Allow` — admit if preconditions hold (was `Admit` in DEC-PLANE-001).
/// - `Deny` — never admit, even if preconditions hold.
/// - `RequireApproval` — admit only after an approval of the given kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionRule {
    Allow,
    Deny,
    RequireApproval { approver_kind: ApproverKind },
}

/// Kind of approver required when `AdmissionRule::RequireApproval` is in effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApproverKind {
    Human,
    Auto,
    External,
}

/// Immutable policy snapshot used to compute `ActionSurfaceView`.
#[derive(Debug, Clone)]
pub struct PolicySnapshot {
    id: String,
    admit: BTreeMap<ActionKind, AdmissionRule>,
}

impl Default for PolicySnapshot {
    fn default() -> Self {
        let mut admit = BTreeMap::new();
        for kind in closed_taxonomy() {
            admit.insert(*kind, AdmissionRule::Allow);
        }
        Self {
            id: "default".to_string(),
            admit,
        }
    }
}

impl PolicySnapshot {
    /// Test helper: policy that denies `Resume`.
    pub fn deny_resume() -> Self {
        let mut admit = BTreeMap::new();
        for kind in closed_taxonomy() {
            let rule = if *kind == ActionKind::Resume {
                AdmissionRule::Deny
            } else {
                AdmissionRule::Allow
            };
            admit.insert(*kind, rule);
        }
        Self {
            id: "deny-resume".to_string(),
            admit,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    /// Test helper: insert or replace the rule for one `ActionKind`.
    pub fn insert(&mut self, kind: ActionKind, rule: AdmissionRule) {
        self.admit.insert(kind, rule);
    }

    pub fn admits(&self, kind: ActionKind) -> bool {
        matches!(self.admit.get(&kind), Some(AdmissionRule::Allow))
    }

    pub fn admission_rule(&self, kind: ActionKind) -> AdmissionRule {
        self.admit
            .get(&kind)
            .cloned()
            .unwrap_or(AdmissionRule::Deny)
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
                AdmissionRule::Allow => 1u8,
                AdmissionRule::Deny => 2u8,
                AdmissionRule::RequireApproval { .. } => 3u8,
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
///
/// **Deprecated** since DEC-PLANE-002 (v1.91.0). This path uses the heuristic
/// fallback (decisions encoded in `pending_decisions`, `frontier.is_empty()`,
/// etc.) and does not derive from the persisted frontier.
///
/// Use [`build_action_surface_view_with_frontier`] instead. This signature is
/// retained so existing callers and tests keep compiling during the migration
/// window; new code MUST use the frontier variant.
#[deprecated(
    since = "1.91.0",
    note = "use build_action_surface_view_with_frontier — heuristic path is a fallback only"
)]
pub fn build_action_surface_view(
    state: &RunStateView,
    policy: &PolicySnapshot,
) -> Result<ActionSurfaceView, ViewError> {
    // Fallback: delegate to the frontier builder with an empty projection.
    // The empty projection causes `project_frontier_to_actions` to emit only
    // `Abort`, which is not the legacy behaviour. To preserve the heuristic
    // behaviour we replicate it inline here.
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

// =========================================================================
// DEC-PLANE-002 — Frontier projection
// =========================================================================
// Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-NextActionDerivation.md
// ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-076-NEXT-ACTION-DERIVATION.md

/// One entry in a `FrontierProjection` — the durable intersection of a
/// declared transition with its gate receipts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontierEntryRef {
    transition_id: String,
    requires_met: bool,
}

impl FrontierEntryRef {
    /// Test-only constructor.
    pub fn for_test(transition_id: impl Into<String>, requires_met: bool) -> Self {
        Self {
            transition_id: transition_id.into(),
            requires_met,
        }
    }

    pub fn transition_id(&self) -> &str {
        &self.transition_id
    }

    pub fn requires_met(&self) -> bool {
        self.requires_met
    }
}

/// Reference to a declared transition in the workflow manifest.
///
/// Only the fields needed by the projection rules are captured. The full
/// transition (with `requires` body, gates, etc.) lives in
/// `WorkflowManifest.transitions` and is loaded by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredTransitionRef {
    id: String,
    from_status: String,
    to_status: String,
    from_phase: Option<String>,
    pub has_approval_gate: bool,
    pub has_escalation_gate: bool,
    pub has_retry_policy: bool,
    paths: Vec<String>,
}

impl DeclaredTransitionRef {
    /// Test-only constructor.
    #[allow(clippy::too_many_arguments)]
    pub fn for_test(
        id: impl Into<String>,
        from_status: impl Into<String>,
        to_status: impl Into<String>,
        from_phase: Option<String>,
        has_approval_gate: bool,
        has_escalation_gate: bool,
        has_retry_policy: bool,
        paths: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            from_status: from_status.into(),
            to_status: to_status.into(),
            from_phase,
            has_approval_gate,
            has_escalation_gate,
            has_retry_policy,
            paths,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn from_status(&self) -> &str {
        &self.from_status
    }
    pub fn to_status(&self) -> &str {
        &self.to_status
    }
    pub fn from_phase(&self) -> Option<&str> {
        self.from_phase.as_deref()
    }
    pub fn paths(&self) -> &[String] {
        &self.paths
    }
}

/// Durable projection of the persisted frontier onto the `ActionKind` taxonomy.
///
/// `entries` are the gate-satisfied frontier (already filtered by
/// `frontier_for_state`). `declared_transitions` is the lookup index from
/// transition_id to its declared shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontierProjection {
    entries: Vec<FrontierEntryRef>,
    declared_transitions: Vec<DeclaredTransitionRef>,
}

impl FrontierProjection {
    /// Test-only constructor.
    pub fn for_test(
        entries: Vec<FrontierEntryRef>,
        declared_transitions: Vec<DeclaredTransitionRef>,
    ) -> Self {
        Self {
            entries,
            declared_transitions,
        }
    }

    pub fn entries(&self) -> &[FrontierEntryRef] {
        &self.entries
    }

    pub fn declared_transitions(&self) -> &[DeclaredTransitionRef] {
        &self.declared_transitions
    }
}

/// Empty projection — sentinel for callers that want to exercise the
/// fallback (heuristic) path through `build_action_surface_view`.
pub fn empty_projection() -> FrontierProjection {
    FrontierProjection::for_test(vec![], vec![])
}

/// Build an `ActionSurfaceView` for the given `RunStateView` under `policy`,
/// with the authoritative frontier projection.
///
/// This is the recommended path. The projection rules are defined in
/// `REQ-NextActionDerivation.md` §"Mapping contract".
pub fn build_action_surface_view_with_frontier(
    state: &RunStateView,
    policy: &PolicySnapshot,
    frontier: &FrontierProjection,
) -> Result<ActionSurfaceView, ViewError> {
    let projected = project_frontier_to_actions(frontier);
    let mut actions = Vec::with_capacity(projected.len());
    for kind in projected {
        if !policy.admits(kind) {
            continue;
        }
        actions.push(kind);
    }
    // Sort by discriminant for byte-stable determinism.
    actions.sort_by_key(kind_discriminant);
    Ok(ActionSurfaceView {
        origin: state.origin(),
        run_id: state.run_id().to_string(),
        evaluated_at: state.evaluated_at(),
        available_actions: actions,
        policy_digest: policy.digest(),
    })
}

/// Project a `FrontierProjection` onto the closed `ActionKind` taxonomy.
///
/// Implements the seven mapping rules from `REQ-NextActionDerivation.md`.
/// `ActionKind::Abort` is unconditional and always included.
fn project_frontier_to_actions(frontier: &FrontierProjection) -> Vec<ActionKind> {
    let mut out: Vec<ActionKind> = Vec::with_capacity(7);
    out.push(ActionKind::Abort);

    let transitions_by_id: std::collections::HashMap<&str, &DeclaredTransitionRef> = frontier
        .declared_transitions()
        .iter()
        .map(|t| (t.id(), t))
        .collect();

    let mut has_start = false;
    let mut has_resume = false;
    let mut has_approve = false;
    let mut has_escalate = false;
    let mut has_retry = false;
    let mut has_reconcile = false;

    for entry in frontier.entries() {
        let transition = match transitions_by_id.get(entry.transition_id()) {
            Some(t) => *t,
            None => continue,
        };
        if transition.from_status() == "Pending" && transition.from_phase().is_none() {
            has_start = true;
        }
        if entry.requires_met() && transition.from_status() == "Running" {
            has_resume = true;
        }
        if transition.has_approval_gate {
            has_approve = true;
        }
        if transition.has_escalation_gate {
            has_escalate = true;
        }
        if entry.requires_met()
            && transition.from_status() == "Failed"
            && transition.has_retry_policy
        {
            has_retry = true;
        }
        if entry.requires_met() && transition.to_status() == "Drifted" {
            has_reconcile = true;
        }
    }

    if has_start {
        out.push(ActionKind::Start);
    }
    if has_resume {
        out.push(ActionKind::Resume);
    }
    if has_approve {
        out.push(ActionKind::Approve);
    }
    if has_escalate {
        out.push(ActionKind::Escalate);
    }
    if has_retry {
        out.push(ActionKind::Retry);
    }
    if has_reconcile {
        out.push(ActionKind::Reconcile);
    }

    out
}

// =========================================================================
// DEC-PLANE-003 — Typed policy evaluation
// =========================================================================
// Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-TypedPolicyEvaluation.md
// ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-077-TYPED-POLICY-EVALUATION.md

/// Three-state verdict for an action (P4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionVerdict {
    Allow,
    Deny,
    RequireApproval { approver_kind: ApproverKind },
}

/// Closed taxonomy of reasons for a decision.
///
/// `#[non_exhaustive]` so future variants don't break downstream match
/// arms. Consumers MUST include a wildcard arm.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DecisionReason {
    FrontierEmpty,
    BlockerUnresolved {
        blocker_id: String,
    },
    PolicyDenied {
        policy_id: String,
        kind: ActionKind,
    },
    ApprovalRequired {
        approver_kind: ApproverKind,
    },
    GateUnsatisfied {
        gate_name: String,
        transition_id: String,
    },
    RetryPolicyAbsent,
    DriftNotDetected,
    LifecycleMismatch {
        expected: String,
        actual: String,
    },
    FrontierProjectionMissing,
    Closed {
        reason: String,
    },
}

/// One step in the provenance chain (P5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceStep {
    PolicyAdmission,
    FrontierProjection,
    GateReceipt,
    LifecycleCheck,
    ApprovalGate,
}

/// Ordered link in a decision's provenance chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProvenanceLink {
    step: ProvenanceStep,
    reference: String,
    evaluated_at: u64,
}

impl ProvenanceLink {
    pub fn new(step: ProvenanceStep, reference: impl Into<String>, evaluated_at: u64) -> Self {
        Self {
            step,
            reference: reference.into(),
            evaluated_at,
        }
    }

    pub fn step(&self) -> ProvenanceStep {
        self.step
    }
    pub fn reference(&self) -> &str {
        &self.reference
    }
    pub fn evaluated_at(&self) -> u64 {
        self.evaluated_at
    }
    pub fn is_policy_admission(&self) -> bool {
        self.step == ProvenanceStep::PolicyAdmission
    }
}

/// Per-action evaluation result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DecisionRecord {
    kind: ActionKind,
    verdict: DecisionVerdict,
    reasons: Vec<DecisionReason>,
    provenance: Vec<ProvenanceLink>,
}

impl DecisionRecord {
    pub fn kind(&self) -> ActionKind {
        self.kind
    }
    pub fn verdict(&self) -> &DecisionVerdict {
        &self.verdict
    }
    pub fn reasons(&self) -> &[DecisionReason] {
        &self.reasons
    }
    pub fn provenance(&self) -> &[ProvenanceLink] {
        &self.provenance
    }
}

/// Full action surface with typed decisions (P6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TypedActionSurfaceView {
    origin: RunOrigin,
    run_id: String,
    evaluated_at: u64,
    policy_digest: u64,
    available_actions: Vec<ActionKind>,
    decisions: Vec<DecisionRecord>,
}

impl TypedActionSurfaceView {
    pub fn origin(&self) -> RunOrigin {
        self.origin
    }
    pub fn run_id(&self) -> &str {
        &self.run_id
    }
    pub fn evaluated_at(&self) -> u64 {
        self.evaluated_at
    }
    pub fn policy_digest(&self) -> u64 {
        self.policy_digest
    }
    pub fn available_actions(&self) -> &[ActionKind] {
        &self.available_actions
    }
    pub fn decisions(&self) -> &[DecisionRecord] {
        &self.decisions
    }
}

/// Test-only: discriminant helper exposed for cross-process determinism tests.
pub fn kind_discriminant_for_test(kind: ActionKind) -> u64 {
    kind_discriminant(&kind)
}

/// Build the typed action surface for a given run under a policy + frontier.
///
/// This is the canonical builder since DEC-PLANE-003. The legacy
/// `build_action_surface_view_with_frontier` delegates here and projects
/// the `Allow` subset.
pub fn build_action_surface_view_typed(
    state: &RunStateView,
    policy: &PolicySnapshot,
    frontier: &FrontierProjection,
) -> Result<TypedActionSurfaceView, ViewError> {
    let evaluated_at = state.evaluated_at();
    let mut decisions = Vec::with_capacity(7);

    for kind in closed_taxonomy() {
        let (verdict, reasons, provenance) =
            evaluate_action(*kind, state, policy, frontier, evaluated_at);
        decisions.push(DecisionRecord {
            kind: *kind,
            verdict,
            reasons,
            provenance,
        });
    }

    // decisions are already sorted because closed_taxonomy() is sorted by
    // discriminant (Start=1..Reconcile=7).
    let available_actions: Vec<ActionKind> = decisions
        .iter()
        .filter(|d| d.verdict == DecisionVerdict::Allow)
        .map(|d| d.kind)
        .collect();

    Ok(TypedActionSurfaceView {
        origin: state.origin(),
        run_id: state.run_id().to_string(),
        evaluated_at,
        policy_digest: policy.digest(),
        available_actions,
        decisions,
    })
}

fn evaluate_action(
    kind: ActionKind,
    state: &RunStateView,
    policy: &PolicySnapshot,
    frontier: &FrontierProjection,
    evaluated_at: u64,
) -> (DecisionVerdict, Vec<DecisionReason>, Vec<ProvenanceLink>) {
    let mut reasons: Vec<DecisionReason> = Vec::new();
    let mut provenance: Vec<ProvenanceLink> = Vec::new();

    // Policy step is always the first provenance link (causality).
    provenance.push(ProvenanceLink::new(
        ProvenanceStep::PolicyAdmission,
        policy.id(),
        evaluated_at,
    ));

    // Frontier projection step (if relevant) comes next.
    let frontier_includes = frontier_projection_includes(frontier, kind);
    if frontier_includes.is_some() {
        provenance.push(ProvenanceLink::new(
            ProvenanceStep::FrontierProjection,
            frontier_includes.unwrap_or_default(),
            evaluated_at,
        ));
    }

    // Build reasons from the frontier + state view.
    match kind {
        ActionKind::Abort => {
            // Abort is always admissible (closed taxonomy invariant).
            reasons.push(DecisionReason::Closed {
                reason: "abort is unconditional".to_string(),
            });
        }
        ActionKind::Start => {
            if frontier.entries().is_empty() && frontier.declared_transitions().is_empty() {
                reasons.push(DecisionReason::FrontierProjectionMissing);
            } else {
                reasons.push(DecisionReason::LifecycleMismatch {
                    expected: "Pending".to_string(),
                    actual: run_origin_str(state.origin()).to_string(),
                });
            }
        }
        ActionKind::Resume => {
            if state.frontier().is_empty() {
                reasons.push(DecisionReason::FrontierEmpty);
            }
            if !state.blockers().is_empty() {
                for b in state.blockers() {
                    reasons.push(DecisionReason::BlockerUnresolved {
                        blocker_id: b.clone(),
                    });
                }
            }
            if reasons.is_empty() {
                reasons.push(DecisionReason::Closed {
                    reason: "resume preconditions hold".to_string(),
                });
            }
        }
        ActionKind::Approve => {
            let has = state
                .pending_decisions()
                .iter()
                .any(|d| d.starts_with("approval"));
            if !has {
                reasons.push(DecisionReason::FrontierEmpty);
            } else {
                reasons.push(DecisionReason::Closed {
                    reason: "approval pending".to_string(),
                });
            }
        }
        ActionKind::Escalate => {
            let has = state
                .pending_decisions()
                .iter()
                .any(|d| d.starts_with("escalation"));
            if !has {
                reasons.push(DecisionReason::FrontierEmpty);
            } else {
                reasons.push(DecisionReason::Closed {
                    reason: "escalation pending".to_string(),
                });
            }
        }
        ActionKind::Retry => {
            reasons.push(DecisionReason::RetryPolicyAbsent);
        }
        ActionKind::Reconcile => {
            reasons.push(DecisionReason::DriftNotDetected);
        }
    }

    // Policy verdict step.
    let verdict = match policy.admission_rule(kind) {
        AdmissionRule::Allow => {
            if matches!(kind, ActionKind::Abort) {
                DecisionVerdict::Allow
            } else if reasons.iter().any(is_blocking_reason) {
                DecisionVerdict::Deny
            } else {
                DecisionVerdict::Allow
            }
        }
        AdmissionRule::Deny => {
            reasons.insert(
                0,
                DecisionReason::PolicyDenied {
                    policy_id: policy.id().to_string(),
                    kind,
                },
            );
            DecisionVerdict::Deny
        }
        AdmissionRule::RequireApproval { approver_kind } => {
            reasons.push(DecisionReason::ApprovalRequired { approver_kind });
            DecisionVerdict::RequireApproval { approver_kind }
        }
    };

    (verdict, reasons, provenance)
}

fn is_blocking_reason(r: &DecisionReason) -> bool {
    matches!(
        r,
        DecisionReason::FrontierEmpty
            | DecisionReason::BlockerUnresolved { .. }
            | DecisionReason::GateUnsatisfied { .. }
            | DecisionReason::RetryPolicyAbsent
            | DecisionReason::DriftNotDetected
            | DecisionReason::LifecycleMismatch { .. }
            | DecisionReason::FrontierProjectionMissing
    )
}

fn frontier_projection_includes(frontier: &FrontierProjection, kind: ActionKind) -> Option<String> {
    if frontier.entries().is_empty() {
        return None;
    }
    for entry in frontier.entries() {
        let t = frontier
            .declared_transitions()
            .iter()
            .find(|t| t.id() == entry.transition_id())?;
        match kind {
            ActionKind::Start if t.from_status() == "Pending" && t.from_phase().is_none() => {
                return Some(entry.transition_id().to_string());
            }
            ActionKind::Resume if entry.requires_met() && t.from_status() == "Running" => {
                return Some(entry.transition_id().to_string());
            }
            ActionKind::Approve if t.has_approval_gate => {
                return Some(entry.transition_id().to_string());
            }
            ActionKind::Escalate if t.has_escalation_gate => {
                return Some(entry.transition_id().to_string());
            }
            ActionKind::Retry
                if entry.requires_met() && t.from_status() == "Failed" && t.has_retry_policy =>
            {
                return Some(entry.transition_id().to_string());
            }
            ActionKind::Reconcile if entry.requires_met() && t.to_status() == "Drifted" => {
                return Some(entry.transition_id().to_string());
            }
            _ => continue,
        }
    }
    None
}

fn run_origin_str(origin: RunOrigin) -> &'static str {
    match origin {
        RunOrigin::Declared => "declared",
        RunOrigin::Generated => "generated",
    }
}

// =========================================================================
// DEC-PLANE-004 — Decision Plane CLI parity
// =========================================================================
// Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-DecisionPlaneCLIParity.md
// ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-078-DECISION-PLANE-CLI-PARITY.md

/// Context required to assemble a CLI action command from a `DecisionRecord`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionCommandContext {
    cycle_id: String,
    lease_owner: String,
    fencing_token: i64,
    gate_name: Option<String>,
    transition_id: Option<String>,
}

impl ActionCommandContext {
    /// Test-only constructor.
    pub fn for_test(
        cycle_id: impl Into<String>,
        lease_owner: impl Into<String>,
        fencing_token: i64,
        gate_name: Option<String>,
        transition_id: Option<String>,
    ) -> Self {
        Self {
            cycle_id: cycle_id.into(),
            lease_owner: lease_owner.into(),
            fencing_token,
            gate_name,
            transition_id,
        }
    }

    pub fn cycle_id(&self) -> &str {
        &self.cycle_id
    }
    pub fn lease_owner(&self) -> &str {
        &self.lease_owner
    }
    pub fn fencing_token(&self) -> i64 {
        self.fencing_token
    }
    pub fn gate_name(&self) -> Option<&str> {
        self.gate_name.as_deref()
    }
    pub fn transition_id(&self) -> Option<&str> {
        self.transition_id.as_deref()
    }
    /// Clear `transition_id` (used by tests to exercise the missing-context path).
    pub fn clear_transition_id(&mut self) {
        self.transition_id = None;
    }
}

/// Errors produced by `build_typed_action_command`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CommandBuildError {
    #[error("missing required context field: {field}")]
    MissingContext { field: String },
}

/// Errors produced by `check_decision_plane_parity`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParityError {
    #[error("command mismatch: expected `{expected}`, got `{actual}`")]
    CommandMismatch { expected: String, actual: String },
    #[error("unexpected command `{cli_command}` for decision kind `{decision_kind:?}`")]
    UnexpectedCommand {
        cli_command: String,
        decision_kind: ActionKind,
    },
    #[error("missing required context field: {field}")]
    MissingContext { field: String },
}

impl DecisionRecord {
    /// Test-only constructor.
    pub fn for_test(
        kind: ActionKind,
        verdict: DecisionVerdict,
        reasons: Vec<DecisionReason>,
        provenance: Vec<ProvenanceLink>,
    ) -> Self {
        Self {
            kind,
            verdict,
            reasons,
            provenance,
        }
    }
}

/// Build the CLI action command for a `DecisionRecord` under the given context.
///
/// Returns `None` when the verdict is not directly actionable (`Deny`,
/// `RequireApproval`). Returns `Err(CommandBuildError::MissingContext)` when
/// a required context field is missing.
pub fn build_typed_action_command(
    decision: &DecisionRecord,
    context: &ActionCommandContext,
) -> Result<Option<String>, CommandBuildError> {
    if !matches!(decision.verdict, DecisionVerdict::Allow) {
        return Ok(None);
    }
    let transition_id =
        context
            .transition_id
            .as_deref()
            .ok_or_else(|| CommandBuildError::MissingContext {
                field: "transition_id".to_string(),
            })?;

    let gate_flag = match decision.kind {
        ActionKind::Approve => Some("approval"),
        ActionKind::Escalate => Some("escalation"),
        _ => None,
    };

    let mut cmd = format!(
        "sddk cycle transition --cycle {} --transition {} --lease-owner {} --fencing-token {}",
        context.cycle_id, transition_id, context.lease_owner, context.fencing_token,
    );
    if let Some(gate) = gate_flag {
        cmd.push_str(&format!(" --gate {gate}"));
    }
    Ok(Some(cmd))
}

/// Check that a CLI command matches the Decision Plane output for the
/// given decision + context. Used as a regression net by CLI tests.
pub fn check_decision_plane_parity(
    cli_command: &str,
    decision: &DecisionRecord,
    context: &ActionCommandContext,
) -> Result<(), ParityError> {
    let expected = build_typed_action_command(decision, context).map_err(|e| match e {
        CommandBuildError::MissingContext { field } => ParityError::MissingContext { field },
    })?;
    match expected {
        Some(expected_cmd) => {
            if cli_command == expected_cmd {
                Ok(())
            } else {
                Err(ParityError::CommandMismatch {
                    expected: expected_cmd,
                    actual: cli_command.to_string(),
                })
            }
        }
        None => Err(ParityError::UnexpectedCommand {
            cli_command: cli_command.to_string(),
            decision_kind: decision.kind,
        }),
    }
}
