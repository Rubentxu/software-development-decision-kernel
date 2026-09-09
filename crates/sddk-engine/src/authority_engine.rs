// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// authority_engine.rs — M5 unified admission facade (ADR-0102 / SPEC-008).
//
// All governed side effects in the kernel pass through `AuthorityEngine::admit`.
// Existing components (WritableSurface / DecisionPlaneGate / RiskApprovalPolicy
// / ApprovalRequestedInput) become inputs/facts to this facade per ADR-0102.
//
// Migration: this module is additive. Existing call sites of
// `crate::authority::WritableSurface` keep working during the transition
// window; the D6 deprecated_patterns lint flags direct external use.

#![allow(clippy::result_large_err)]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

// =============================================================================
// Type aliases
// =============================================================================

pub type ReceiptId = String;
pub type CycleRef = String;
pub type MemoryRef = String;
pub type DecisionRef = String;
pub type EvidenceKind = String;
pub type Capability = String;
pub type LeaseToken = String;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DigestSha256(pub String);

impl DigestSha256 {
    pub fn compute(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Self(hex_lower(&hasher.finalize()))
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

pub type UtcTimestamp = OffsetDateTime;

#[allow(dead_code)]
fn now_utc() -> UtcTimestamp {
    OffsetDateTime::now_utc()
}

fn format_ts(ts: &UtcTimestamp) -> String {
    ts.format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

// =============================================================================
// T-01 — Core enums (closed-set, non_exhaustive)
// =============================================================================

/// Action kinds subject to admission. Add new variants only through explicit
/// engine policy registration; the engine will deny unknown actions for any
/// policy registered before the variant was added.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ActionKind {
    CycleStart,
    CycleTransition,
    CycleSupersede,
    CycleReplan,
    CyclePause,
    CycleResume,
    PlanWorkItem,
    PlanEvidence,
    PlanDecision,
    AgentExecute,
    AgentHandoff,
    AgentSupersede,
    MemoryCommit,
    MemoryRefMutation,
    MemoryBranchFork,
    VaultIndex,
    VaultSearch,
    VaultExport,
    PackInstall,
    PackValidate,
    CliRun,
    CliRelease,
    CliShip,
    CliRecover,
}

impl ActionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CycleStart => "cycle_start",
            Self::CycleTransition => "cycle_transition",
            Self::CycleSupersede => "cycle_supersede",
            Self::CycleReplan => "cycle_replan",
            Self::CyclePause => "cycle_pause",
            Self::CycleResume => "cycle_resume",
            Self::PlanWorkItem => "plan_work_item",
            Self::PlanEvidence => "plan_evidence",
            Self::PlanDecision => "plan_decision",
            Self::AgentExecute => "agent_execute",
            Self::AgentHandoff => "agent_handoff",
            Self::AgentSupersede => "agent_supersede",
            Self::MemoryCommit => "memory_commit",
            Self::MemoryRefMutation => "memory_ref_mutation",
            Self::MemoryBranchFork => "memory_branch_fork",
            Self::VaultIndex => "vault_index",
            Self::VaultSearch => "vault_search",
            Self::VaultExport => "vault_export",
            Self::PackInstall => "pack_install",
            Self::PackValidate => "pack_validate",
            Self::CliRun => "cli_run",
            Self::CliRelease => "cli_release",
            Self::CliShip => "cli_ship",
            Self::CliRecover => "cli_recover",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ActorKind {
    Human {
        id: String,
    },
    Agent {
        profile_id: String,
        profile_version: u32,
    },
    Orchestrator {
        session_id: String,
    },
    Coordinator {
        session_id: String,
    },
    Leaf {
        session_id: String,
        role: String,
    },
    Evaluator {
        id: String,
    },
    Advisor {
        id: String,
    },
    System {
        service: String,
    },
    SecretaryL0,
    SecretaryL1,
    SecretaryL2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskBand {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum GateKind {
    Approval,
    RiskReview,
    LegalHold,
    Operational,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ApproverKind {
    Human,
    Advisor,
    Legal,
    Operational,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DenyReason {
    UnknownActor,
    UnknownAction,
    PolicyVersionMismatch,
    MissingEvidence { required: Vec<EvidenceKind> },
    RiskBandExceeded { actor: RiskBand, policy: RiskBand },
    ExplicitDenyOverride,
    LeaseExpired,
    LeaseFencingFailed,
    ActorKindNotPermitted,
}

// =============================================================================
// T-02 — Composite types
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionProposal {
    pub kind: ActionKind,
    pub target_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_digest: Option<DigestSha256>,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Actor {
    pub kind: ActorKind,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease: Option<LeaseToken>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Facts {
    #[serde(default)]
    pub evidence_refs: Vec<EvidenceRef>,
    #[serde(default)]
    pub decision_refs: Vec<DecisionRef>,
    #[serde(default)]
    pub memory_refs: Vec<MemoryRef>,
    #[serde(default)]
    pub cycle_refs: Vec<CycleRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvidenceRef(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Postcondition {
    pub label: String,
    pub verification: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequirement {
    pub approver_kind: ApproverKind,
    pub minimum_evidence: Vec<EvidenceKind>,
    pub timeout_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AdmissionDecision {
    Allow {
        receipt_id: ReceiptId,
        postconditions: Vec<Postcondition>,
    },
    Deny {
        reason: DenyReason,
        decision_id: ReceiptId,
    },
    RequireApproval {
        requirement: ApprovalRequirement,
        decision_id: ReceiptId,
    },
}

impl AdmissionDecision {
    pub fn decision_id(&self) -> &ReceiptId {
        match self {
            Self::Allow { receipt_id, .. } => receipt_id,
            Self::Deny { decision_id, .. } => decision_id,
            Self::RequireApproval { decision_id, .. } => decision_id,
        }
    }

    pub fn is_allow(&self) -> bool {
        matches!(self, Self::Allow { .. })
    }

    pub fn is_deny(&self) -> bool {
        matches!(self, Self::Deny { .. })
    }

    pub fn is_require_approval(&self) -> bool {
        matches!(self, Self::RequireApproval { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicySnapshot {
    pub policy_id: String,
    pub policy_version: u32,
    pub policy_digest: DigestSha256,
    pub risk_band: RiskBand,
    #[serde(default)]
    pub gates_required: Vec<GateKind>,
    #[serde(default)]
    pub approval_required_for: BTreeSet<ActionKind>,
    #[serde(default)]
    pub deny_override: BTreeSet<(String, ActionKind)>,
    /// ActionKind -> minimum required capability for Allow.
    #[serde(default)]
    pub capability_matrix: BTreeMap<ActionKind, Capability>,
}

impl PolicySnapshot {
    pub fn default_low_risk(policy_id: &str) -> Self {
        let mut cap = BTreeMap::new();
        cap.insert(ActionKind::CycleStart, "cycle.lifecycle".to_string());
        cap.insert(ActionKind::CycleTransition, "cycle.lifecycle".to_string());
        cap.insert(ActionKind::CycleSupersede, "cycle.lifecycle".to_string());
        cap.insert(ActionKind::CycleReplan, "cycle.lifecycle".to_string());
        cap.insert(ActionKind::CyclePause, "cycle.lifecycle".to_string());
        cap.insert(ActionKind::CycleResume, "cycle.lifecycle".to_string());
        cap.insert(ActionKind::PlanWorkItem, "plan.write".to_string());
        cap.insert(ActionKind::PlanEvidence, "plan.write".to_string());
        cap.insert(ActionKind::PlanDecision, "plan.write".to_string());
        cap.insert(ActionKind::MemoryCommit, "memory.write".to_string());
        cap.insert(ActionKind::MemoryRefMutation, "memory.write".to_string());
        cap.insert(ActionKind::MemoryBranchFork, "memory.write".to_string());
        cap.insert(ActionKind::VaultIndex, "vault.write".to_string());
        cap.insert(ActionKind::VaultSearch, "vault.read".to_string());
        cap.insert(ActionKind::VaultExport, "vault.read".to_string());
        cap.insert(ActionKind::AgentExecute, "agent.execute".to_string());
        cap.insert(ActionKind::AgentHandoff, "agent.execute".to_string());
        cap.insert(ActionKind::AgentSupersede, "agent.execute".to_string());
        cap.insert(ActionKind::PackInstall, "pack.write".to_string());
        cap.insert(ActionKind::PackValidate, "pack.read".to_string());
        cap.insert(ActionKind::CliRun, "cli.execute".to_string());
        cap.insert(ActionKind::CliRelease, "cli.execute".to_string());
        cap.insert(ActionKind::CliShip, "cli.execute".to_string());
        cap.insert(ActionKind::CliRecover, "cli.execute".to_string());
        let digest = DigestSha256::compute(policy_id.as_bytes());
        Self {
            policy_id: policy_id.to_string(),
            policy_version: 1,
            policy_digest: digest,
            risk_band: RiskBand::Low,
            gates_required: vec![],
            approval_required_for: BTreeSet::new(),
            deny_override: BTreeSet::new(),
            capability_matrix: cap,
        }
    }
}

// =============================================================================
// T-03 + T-04 + T-05 + T-06 — Engine trait + impl + admission logic
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdmissionExplanation {
    pub policy_id: String,
    pub policy_version: u32,
    pub gates_applied: Vec<GateKind>,
    pub evidence_refs_used: Vec<EvidenceRef>,
    pub deny_reasons_evaluated: Vec<DenyReason>,
    pub approval_requirements_considered: Vec<ApprovalRequirement>,
    pub decision_digest: DigestSha256,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AuthorityEngineError {
    InternalContractBug { reason: String },
    PolicyAlreadyRegistered { policy_id: String },
    PolicyNotFound { version: u32 },
}

pub trait AuthorityEngine {
    fn admit(
        &self,
        proposal: &ActionProposal,
        actor: &Actor,
        facts: &Facts,
        policy: &PolicySnapshot,
    ) -> AdmissionDecision;

    fn explain(&self, decision: &AdmissionDecision) -> AdmissionExplanation;

    fn policy_at(&self, version: u32) -> Result<PolicySnapshot, AuthorityEngineError>;

    fn register_policy(&mut self, policy: PolicySnapshot) -> Result<(), AuthorityEngineError>;
}

#[derive(Debug, Default)]
pub struct DefaultAuthorityEngine {
    policies: BTreeMap<u32, PolicySnapshot>,
    next_receipt_seq: u64,
}

impl DefaultAuthorityEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(policy: PolicySnapshot) -> Self {
        let mut e = Self::default();
        e.register_policy(policy).expect("first registration");
        e
    }

    #[allow(dead_code)]
    fn next_receipt_id(&mut self, prefix: &str) -> ReceiptId {
        self.next_receipt_seq += 1;
        format!("{}-{}", prefix, self.next_receipt_seq)
    }

    fn actor_kind_pattern(actor: &ActorKind) -> String {
        match actor {
            ActorKind::Human { .. } => "human".to_string(),
            ActorKind::Agent { .. } => "agent".to_string(),
            ActorKind::Orchestrator { .. } => "orchestrator".to_string(),
            ActorKind::Coordinator { .. } => "coordinator".to_string(),
            ActorKind::Leaf { .. } => "leaf".to_string(),
            ActorKind::Evaluator { .. } => "evaluator".to_string(),
            ActorKind::Advisor { .. } => "advisor".to_string(),
            ActorKind::System { .. } => "system".to_string(),
            ActorKind::SecretaryL0 => "secretary_l0".to_string(),
            ActorKind::SecretaryL1 => "secretary_l1".to_string(),
            ActorKind::SecretaryL2 => "secretary_l2".to_string(),
        }
    }
}

impl AuthorityEngine for DefaultAuthorityEngine {
    fn admit(
        &self,
        proposal: &ActionProposal,
        actor: &Actor,
        facts: &Facts,
        policy: &PolicySnapshot,
    ) -> AdmissionDecision {
        // 1. Lease / fencing check
        if let Some(token) = &actor.lease
            && token.is_empty()
        {
            return AdmissionDecision::Deny {
                reason: DenyReason::LeaseExpired,
                decision_id: format!("deny-lease-{}", format_ts(&proposal.created_at)),
            };
        }

        // 2. Explicit deny override
        let actor_pattern = Self::actor_kind_pattern(&actor.kind);
        if policy
            .deny_override
            .contains(&(actor_pattern.clone(), proposal.kind))
        {
            return AdmissionDecision::Deny {
                reason: DenyReason::ExplicitDenyOverride,
                decision_id: format!("deny-override-{}-{}", actor_pattern, proposal.kind.as_str()),
            };
        }

        // 3. Capability check
        let required = policy.capability_matrix.get(&proposal.kind);
        let capability_ok = match required {
            None => false,
            Some(cap) if cap.is_empty() => true,
            Some(cap) => actor.capabilities.iter().any(|c| c == cap),
        };
        if !capability_ok {
            return AdmissionDecision::Deny {
                reason: DenyReason::ActorKindNotPermitted,
                decision_id: format!("deny-cap-{}-{}", actor_pattern, proposal.kind.as_str()),
            };
        }

        // 4. Evidence presence check (per minimum-evidence map; empty for default policy)
        // The default policy has no mandatory evidence; if a policy specifies required
        // evidence (via future policy field), this is the integration point.

        // 5. Risk band escalation — escalate by approval if actor risk exceeds policy
        // For default policy, all actors are Low risk; escalation triggers if
        // action kind is in `approval_required_for` OR if policy risk band is High.
        if policy.approval_required_for.contains(&proposal.kind)
            || matches!(policy.risk_band, RiskBand::High)
        {
            return AdmissionDecision::RequireApproval {
                requirement: ApprovalRequirement {
                    approver_kind: ApproverKind::Human,
                    minimum_evidence: vec![],
                    timeout_seconds: 3600,
                },
                decision_id: format!("approval-{}-{}", actor_pattern, proposal.kind.as_str()),
            };
        }

        // 6. Allow
        let _ = facts; // facts reserved for future evidence presence checks
        AdmissionDecision::Allow {
            receipt_id: format!("allow-{}-{}", actor_pattern, proposal.kind.as_str()),
            postconditions: vec![Postcondition {
                label: "evidence_emitted".to_string(),
                verification: "post_decision_evidence".to_string(),
            }],
        }
    }

    fn explain(&self, decision: &AdmissionDecision) -> AdmissionExplanation {
        let decision_digest = DigestSha256::compute(decision.decision_id().as_bytes());
        let mut deny_reasons_evaluated = Vec::new();
        let mut approval_requirements_considered = Vec::new();
        let gates_applied = vec![];
        if let AdmissionDecision::Deny { reason, .. } = decision {
            deny_reasons_evaluated.push(reason.clone());
        }
        if let AdmissionDecision::RequireApproval { requirement, .. } = decision {
            approval_requirements_considered.push(requirement.clone());
        }
        AdmissionExplanation {
            policy_id: "admitted".to_string(),
            policy_version: 1,
            gates_applied,
            evidence_refs_used: vec![],
            deny_reasons_evaluated,
            approval_requirements_considered,
            decision_digest,
        }
    }

    fn policy_at(&self, version: u32) -> Result<PolicySnapshot, AuthorityEngineError> {
        self.policies
            .get(&version)
            .cloned()
            .ok_or(AuthorityEngineError::PolicyNotFound { version })
    }

    fn register_policy(&mut self, policy: PolicySnapshot) -> Result<(), AuthorityEngineError> {
        if self.policies.contains_key(&policy.policy_version) {
            return Err(AuthorityEngineError::PolicyAlreadyRegistered {
                policy_id: policy.policy_id.clone(),
            });
        }
        self.policies.insert(policy.policy_version, policy);
        Ok(())
    }
}

// =============================================================================
// T-07 — Bridge from existing components
// =============================================================================

/// Convert an existing `WritableSurface` permission into a capability string.
pub fn infer_capability_from_surface(surface_name: &str) -> Capability {
    format!("surface.{surface_name}")
}

/// Adapt `RiskApprovalPolicy` risk band into a `PolicySnapshot.risk_band`.
pub fn risk_band_from_policy_level(level: &str) -> RiskBand {
    match level.to_ascii_lowercase().as_str() {
        "high" | "critical" => RiskBand::High,
        "medium" | "moderate" => RiskBand::Medium,
        _ => RiskBand::Low,
    }
}

#[cfg(test)]
mod inline_tests {
    use super::*;

    #[test]
    fn action_kind_as_str_is_stable() {
        assert_eq!(ActionKind::CycleStart.as_str(), "cycle_start");
        assert_eq!(ActionKind::CliRecover.as_str(), "cli_recover");
    }

    #[test]
    fn deny_decision_has_decision_id() {
        let d = AdmissionDecision::Deny {
            reason: DenyReason::UnknownActor,
            decision_id: "x".to_string(),
        };
        assert_eq!(d.decision_id(), "x");
        assert!(d.is_deny());
        assert!(!d.is_allow());
    }

    #[test]
    fn allow_decision_is_allow() {
        let d = AdmissionDecision::Allow {
            receipt_id: "y".to_string(),
            postconditions: vec![],
        };
        assert!(d.is_allow());
        assert_eq!(d.decision_id(), "y");
    }

    #[test]
    fn require_approval_decision_is_require_approval() {
        let d = AdmissionDecision::RequireApproval {
            requirement: ApprovalRequirement {
                approver_kind: ApproverKind::Human,
                minimum_evidence: vec![],
                timeout_seconds: 60,
            },
            decision_id: "z".to_string(),
        };
        assert!(d.is_require_approval());
        assert_eq!(d.decision_id(), "z");
    }

    #[test]
    fn digest_compute_is_stable() {
        let a = DigestSha256::compute(b"hello");
        let b = DigestSha256::compute(b"hello");
        assert_eq!(a, b);
    }

    #[test]
    fn facts_default_is_empty() {
        let f = Facts::default();
        assert!(f.evidence_refs.is_empty());
        assert!(f.decision_refs.is_empty());
    }

    #[test]
    fn policy_snapshot_default_low_risk_has_all_capabilities() {
        let p = PolicySnapshot::default_low_risk("test");
        assert_eq!(p.policy_id, "test");
        assert_eq!(p.risk_band, RiskBand::Low);
        assert!(p.capability_matrix.contains_key(&ActionKind::CycleStart));
        assert!(p.approval_required_for.is_empty());
    }

    #[test]
    fn bridge_capability_inference() {
        assert_eq!(
            infer_capability_from_surface("cycle_lifecycle"),
            "surface.cycle_lifecycle"
        );
    }

    #[test]
    fn bridge_risk_band_levels() {
        assert_eq!(risk_band_from_policy_level("high"), RiskBand::High);
        assert_eq!(risk_band_from_policy_level("MEDIUM"), RiskBand::Medium);
        assert_eq!(risk_band_from_policy_level("low"), RiskBand::Low);
    }

    #[test]
    fn deny_explain_records_reason() {
        let engine = DefaultAuthorityEngine::new();
        let d = AdmissionDecision::Deny {
            reason: DenyReason::ActorKindNotPermitted,
            decision_id: "x".to_string(),
        };
        let e = engine.explain(&d);
        assert_eq!(e.deny_reasons_evaluated.len(), 1);
        assert!(matches!(
            e.deny_reasons_evaluated[0],
            DenyReason::ActorKindNotPermitted
        ));
    }

    #[test]
    fn require_approval_explain_records_requirement() {
        let engine = DefaultAuthorityEngine::new();
        let d = AdmissionDecision::RequireApproval {
            requirement: ApprovalRequirement {
                approver_kind: ApproverKind::Legal,
                minimum_evidence: vec![],
                timeout_seconds: 30,
            },
            decision_id: "y".to_string(),
        };
        let e = engine.explain(&d);
        assert_eq!(e.approval_requirements_considered.len(), 1);
        assert_eq!(
            e.approval_requirements_considered[0].approver_kind,
            ApproverKind::Legal
        );
    }

    #[test]
    fn allow_with_capability_returns_allow() {
        let engine = DefaultAuthorityEngine::new();
        let policy = PolicySnapshot::default_low_risk("p");
        let actor = Actor {
            kind: ActorKind::Human {
                id: "u1".to_string(),
            },
            capabilities: vec!["cycle.lifecycle".to_string()],
            lease: None,
        };
        let proposal = ActionProposal {
            kind: ActionKind::CycleStart,
            target_id: "c1".to_string(),
            payload_digest: None,
            created_at: now_utc(),
        };
        let facts = Facts::default();
        let decision = engine.admit(&proposal, &actor, &facts, &policy);
        assert!(decision.is_allow(), "expected Allow, got {:?}", decision);
    }

    #[test]
    fn deny_without_capability_returns_deny() {
        let engine = DefaultAuthorityEngine::new();
        let policy = PolicySnapshot::default_low_risk("p");
        let actor = Actor {
            kind: ActorKind::Human {
                id: "u1".to_string(),
            },
            capabilities: vec![],
            lease: None,
        };
        let proposal = ActionProposal {
            kind: ActionKind::CycleStart,
            target_id: "c1".to_string(),
            payload_digest: None,
            created_at: now_utc(),
        };
        let facts = Facts::default();
        let decision = engine.admit(&proposal, &actor, &facts, &policy);
        assert!(decision.is_deny(), "expected Deny, got {:?}", decision);
    }

    #[test]
    fn deny_override_blocks_action() {
        let mut engine = DefaultAuthorityEngine::new();
        let mut policy = PolicySnapshot::default_low_risk("p");
        policy
            .deny_override
            .insert(("human".to_string(), ActionKind::CycleStart));
        engine.register_policy(policy.clone()).unwrap();
        let actor = Actor {
            kind: ActorKind::Human {
                id: "u1".to_string(),
            },
            capabilities: vec!["cycle.lifecycle".to_string()],
            lease: None,
        };
        let proposal = ActionProposal {
            kind: ActionKind::CycleStart,
            target_id: "c1".to_string(),
            payload_digest: None,
            created_at: now_utc(),
        };
        let decision = engine.admit(&proposal, &actor, &Facts::default(), &policy);
        assert!(decision.is_deny(), "expected Deny, got {:?}", decision);
        if let AdmissionDecision::Deny { reason, .. } = decision {
            assert!(matches!(reason, DenyReason::ExplicitDenyOverride));
        }
    }

    #[test]
    fn high_risk_band_triggers_approval() {
        let engine = DefaultAuthorityEngine::new();
        let mut policy = PolicySnapshot::default_low_risk("p");
        policy.risk_band = RiskBand::High;
        let actor = Actor {
            kind: ActorKind::Human {
                id: "u1".to_string(),
            },
            capabilities: vec!["cycle.lifecycle".to_string()],
            lease: None,
        };
        let proposal = ActionProposal {
            kind: ActionKind::CycleStart,
            target_id: "c1".to_string(),
            payload_digest: None,
            created_at: now_utc(),
        };
        let decision = engine.admit(&proposal, &actor, &Facts::default(), &policy);
        assert!(decision.is_require_approval(), "got {:?}", decision);
    }

    #[test]
    fn deterministic_same_inputs_same_decision() {
        let engine = DefaultAuthorityEngine::new();
        let policy = PolicySnapshot::default_low_risk("p");
        let actor = Actor {
            kind: ActorKind::Human {
                id: "u1".to_string(),
            },
            capabilities: vec!["cycle.lifecycle".to_string()],
            lease: None,
        };
        let proposal = ActionProposal {
            kind: ActionKind::CycleStart,
            target_id: "c1".to_string(),
            payload_digest: None,
            created_at: now_utc(),
        };
        let d1 = engine.admit(&proposal, &actor, &Facts::default(), &policy);
        let d2 = engine.admit(&proposal, &actor, &Facts::default(), &policy);
        assert_eq!(d1, d2);
    }

    #[test]
    fn policy_registration_version_conflict() {
        let mut engine = DefaultAuthorityEngine::new();
        let p = PolicySnapshot::default_low_risk("p");
        engine.register_policy(p.clone()).unwrap();
        let result = engine.register_policy(p);
        assert!(matches!(
            result,
            Err(AuthorityEngineError::PolicyAlreadyRegistered { .. })
        ));
    }

    #[test]
    fn policy_at_unknown_version_returns_error() {
        let engine = DefaultAuthorityEngine::new();
        let result = engine.policy_at(999);
        assert!(matches!(
            result,
            Err(AuthorityEngineError::PolicyNotFound { version: 999 })
        ));
    }

    #[test]
    fn empty_lease_denies() {
        let engine = DefaultAuthorityEngine::new();
        let policy = PolicySnapshot::default_low_risk("p");
        let actor = Actor {
            kind: ActorKind::Human {
                id: "u1".to_string(),
            },
            capabilities: vec!["cycle.lifecycle".to_string()],
            lease: Some(String::new()),
        };
        let proposal = ActionProposal {
            kind: ActionKind::CycleStart,
            target_id: "c1".to_string(),
            payload_digest: None,
            created_at: now_utc(),
        };
        let decision = engine.admit(&proposal, &actor, &Facts::default(), &policy);
        assert!(decision.is_deny());
        if let AdmissionDecision::Deny { reason, .. } = decision {
            assert!(matches!(reason, DenyReason::LeaseExpired));
        }
    }
}

pub mod bridge;
