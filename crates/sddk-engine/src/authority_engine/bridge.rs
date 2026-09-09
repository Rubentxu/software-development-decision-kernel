// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// bridge.rs — T-07: Bridge from existing authority components to the engine.
//
// Existing components become inputs/facts to AuthorityEngine per ADR-0102.
// This module provides the conversion utilities.

use crate::authority_engine::{
    ActionKind, ActorKind, ApprovalRequirement, ApproverKind, Capability, Facts, PolicySnapshot,
    RiskBand, infer_capability_from_surface, risk_band_from_policy_level,
};

/// Build a `PolicySnapshot` from existing risk approval policy level.
pub fn policy_snapshot_from_risk_level(policy_id: &str, risk_level: &str) -> PolicySnapshot {
    let mut policy = PolicySnapshot::default_low_risk(policy_id);
    policy.risk_band = risk_band_from_policy_level(risk_level);
    if matches!(policy.risk_band, RiskBand::High) {
        policy
            .approval_required_for
            .insert(ActionKind::CycleSupersede);
        policy
            .approval_required_for
            .insert(ActionKind::MemoryRefMutation);
    }
    policy
}

/// Add a capability for an actor based on a writable surface name.
pub fn capability_for_surface(surface: &str) -> Capability {
    infer_capability_from_surface(surface)
}

/// Build an `ApprovalRequirement` for a high-risk action.
pub fn approval_for_high_risk_action() -> ApprovalRequirement {
    ApprovalRequirement {
        approver_kind: ApproverKind::Human,
        minimum_evidence: vec!["risk_review".to_string()],
        timeout_seconds: 3600,
    }
}

/// Check whether an actor kind is permitted for a given action kind
/// under the default low-risk policy.
pub fn actor_permitted_default(actor: &ActorKind, action: ActionKind) -> bool {
    let cap = match action {
        ActionKind::CycleStart
        | ActionKind::CycleTransition
        | ActionKind::CycleSupersede
        | ActionKind::CycleReplan
        | ActionKind::CyclePause
        | ActionKind::CycleResume => "cycle.lifecycle",
        ActionKind::PlanWorkItem | ActionKind::PlanEvidence | ActionKind::PlanDecision => {
            "plan.write"
        }
        ActionKind::MemoryCommit | ActionKind::MemoryRefMutation | ActionKind::MemoryBranchFork => {
            "memory.write"
        }
        ActionKind::VaultIndex => "vault.write",
        ActionKind::VaultSearch | ActionKind::VaultExport => "vault.read",
        ActionKind::AgentExecute | ActionKind::AgentHandoff | ActionKind::AgentSupersede => {
            "agent.execute"
        }
        ActionKind::PackInstall => "pack.write",
        ActionKind::PackValidate => "pack.read",
        ActionKind::CliRun
        | ActionKind::CliRelease
        | ActionKind::CliShip
        | ActionKind::CliRecover => "cli.execute",
    };
    let is_system = matches!(actor, ActorKind::System { .. });
    let is_human_or_agent = matches!(
        actor,
        ActorKind::Human { .. }
            | ActorKind::Agent { .. }
            | ActorKind::Orchestrator { .. }
            | ActorKind::Coordinator { .. }
            | ActorKind::SecretaryL0
            | ActorKind::SecretaryL1
            | ActorKind::SecretaryL2
    );
    if is_system {
        // System actors only permitted on lifecycle actions.
        return cap == "cycle.lifecycle";
    }
    is_human_or_agent && !is_system
}

/// Build default `Facts` for an admission request.
pub fn default_facts() -> Facts {
    Facts::default()
}

#[cfg(test)]
mod bridge_tests {
    use super::*;
    use crate::authority_engine::{AuthorityEngine, DefaultAuthorityEngine};

    #[test]
    fn policy_snapshot_from_risk_level_low() {
        let p = policy_snapshot_from_risk_level("p", "low");
        assert_eq!(p.risk_band, RiskBand::Low);
        assert!(p.approval_required_for.is_empty());
    }

    #[test]
    fn policy_snapshot_from_risk_level_high_triggers_supersede_approval() {
        let p = policy_snapshot_from_risk_level("p", "high");
        assert_eq!(p.risk_band, RiskBand::High);
        assert!(
            p.approval_required_for
                .contains(&ActionKind::CycleSupersede)
        );
    }

    #[test]
    fn capability_for_surface_uses_prefix() {
        let c = capability_for_surface("memory_write");
        assert_eq!(c, "surface.memory_write");
    }

    #[test]
    fn approval_for_high_risk_action_is_human() {
        let r = approval_for_high_risk_action();
        assert_eq!(r.approver_kind, ApproverKind::Human);
        assert!(r.timeout_seconds > 0);
    }

    #[test]
    fn actor_permitted_default_human_can_cycle() {
        let actor = ActorKind::Human {
            id: "u1".to_string(),
        };
        assert!(actor_permitted_default(&actor, ActionKind::CycleStart));
    }

    #[test]
    fn actor_permitted_default_system_blocked_from_plan() {
        let actor = ActorKind::System {
            service: "k1".to_string(),
        };
        assert!(!actor_permitted_default(&actor, ActionKind::PlanWorkItem));
    }

    #[test]
    fn default_facts_is_empty() {
        let f = default_facts();
        assert!(f.evidence_refs.is_empty());
        assert!(f.decision_refs.is_empty());
        assert!(f.memory_refs.is_empty());
        assert!(f.cycle_refs.is_empty());
    }

    #[test]
    fn bridge_to_engine_admits_low_risk_cycle() {
        let engine = DefaultAuthorityEngine::new();
        let policy = policy_snapshot_from_risk_level("p", "low");
        let actor = crate::authority_engine::Actor {
            kind: ActorKind::Human {
                id: "u1".to_string(),
            },
            capabilities: vec!["cycle.lifecycle".to_string()],
            lease: None,
        };
        let proposal = crate::authority_engine::ActionProposal {
            kind: ActionKind::CycleStart,
            target_id: "c1".to_string(),
            payload_digest: None,
            created_at: crate::authority_engine::OffsetDateTime::now_utc(),
        };
        let d = engine.admit(&proposal, &actor, &default_facts(), &policy);
        assert!(d.is_allow(), "got {:?}", d);
    }
}
