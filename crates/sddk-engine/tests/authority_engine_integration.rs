// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// authority_engine_integration.rs — T-08: integration tests proving that
// governed cycle operations (cycle_pause, cycle_supersede) admit through
// the AuthorityEngine facade and produce the expected admission decisions.

use sddk_engine::authority_engine::{
    ActionKind, ActionProposal, Actor, ActorKind, AdmissionDecision, AuthorityEngine,
    DefaultAuthorityEngine, bridge::default_facts, bridge::policy_snapshot_from_risk_level,
};
use time::OffsetDateTime;

fn human_actor_with_cap(cap: &str) -> Actor {
    Actor {
        kind: ActorKind::Human {
            id: "test-user".to_string(),
        },
        capabilities: vec![cap.to_string()],
        lease: None,
    }
}

fn proposal(kind: ActionKind, target: &str) -> ActionProposal {
    ActionProposal {
        kind,
        target_id: target.to_string(),
        payload_digest: None,
        created_at: OffsetDateTime::now_utc(),
    }
}

#[test]
fn cycle_pause_admits_through_engine() {
    let engine = DefaultAuthorityEngine::new();
    let policy = policy_snapshot_from_risk_level("test-policy", "low");
    let actor = human_actor_with_cap("cycle.lifecycle");
    let decision = engine.admit(
        &proposal(ActionKind::CyclePause, "test-cycle-1"),
        &actor,
        &default_facts(),
        &policy,
    );
    assert!(
        decision.is_allow(),
        "CyclePause should admit under low-risk policy; got {:?}",
        decision
    );
}

#[test]
fn cycle_supersede_admits_through_engine_under_high_risk_requires_approval() {
    let engine = DefaultAuthorityEngine::new();
    let policy = policy_snapshot_from_risk_level("test-policy", "high");
    let actor = human_actor_with_cap("cycle.lifecycle");
    let decision = engine.admit(
        &proposal(ActionKind::CycleSupersede, "test-cycle-2"),
        &actor,
        &default_facts(),
        &policy,
    );
    assert!(
        decision.is_require_approval(),
        "CycleSupersede under high risk should require approval; got {:?}",
        decision
    );
}

#[test]
fn agent_execute_admits_for_agent_with_capability() {
    let engine = DefaultAuthorityEngine::new();
    let policy = policy_snapshot_from_risk_level("test-policy", "low");
    let actor = Actor {
        kind: ActorKind::Agent {
            profile_id: "core.reviewer".to_string(),
            profile_version: 2,
        },
        capabilities: vec!["agent.execute".to_string()],
        lease: None,
    };
    let decision = engine.admit(
        &proposal(ActionKind::AgentExecute, "session-1"),
        &actor,
        &default_facts(),
        &policy,
    );
    assert!(
        decision.is_allow(),
        "AgentExecute should admit; got {:?}",
        decision
    );
}

#[test]
fn deny_reason_is_deterministic_per_actor_kind() {
    let engine = DefaultAuthorityEngine::new();
    let policy = policy_snapshot_from_risk_level("test-policy", "low");
    // Actor without the required capability — engine denies via ActorKindNotPermitted.
    let actor = Actor {
        kind: ActorKind::System {
            service: "scheduler".to_string(),
        },
        capabilities: vec![], // no capabilities
        lease: None,
    };
    let d1 = engine.admit(
        &proposal(ActionKind::PlanWorkItem, "w1"),
        &actor,
        &default_facts(),
        &policy,
    );
    let d2 = engine.admit(
        &proposal(ActionKind::PlanWorkItem, "w1"),
        &actor,
        &default_facts(),
        &policy,
    );
    assert_eq!(d1, d2, "Same inputs must yield same decision");
    assert!(d1.is_deny(), "Actor without capability should be denied");
    if let AdmissionDecision::Deny { reason, .. } = d1 {
        use sddk_engine::authority_engine::DenyReason;
        assert!(matches!(reason, DenyReason::ActorKindNotPermitted));
    }
}

#[test]
fn policy_at_returns_registered_policy() {
    let mut engine = DefaultAuthorityEngine::new();
    let p = policy_snapshot_from_risk_level("p1", "low");
    engine.register_policy(p.clone()).unwrap();
    let fetched = engine.policy_at(1).expect("policy version 1");
    assert_eq!(fetched.policy_id, "p1");
}

#[test]
fn multiple_policies_coexist_by_version() {
    let mut engine = DefaultAuthorityEngine::new();
    let mut p1 = policy_snapshot_from_risk_level("p1", "low");
    p1.policy_version = 1;
    let mut p2 = policy_snapshot_from_risk_level("p2", "high");
    p2.policy_version = 2;
    engine.register_policy(p1).unwrap();
    engine.register_policy(p2).unwrap();
    assert_eq!(engine.policy_at(1).unwrap().policy_id, "p1");
    assert_eq!(engine.policy_at(2).unwrap().policy_id, "p2");
}

#[test]
fn receipt_id_is_unique_per_decision_shape() {
    let engine = DefaultAuthorityEngine::new();
    let policy = policy_snapshot_from_risk_level("test-policy", "low");
    let actor = human_actor_with_cap("memory.write");

    let d_allow = engine.admit(
        &proposal(ActionKind::MemoryCommit, "mem-1"),
        &actor,
        &default_facts(),
        &policy,
    );
    let d_high_risk = engine.admit(
        &proposal(ActionKind::MemoryRefMutation, "mem-2"),
        &actor,
        &default_facts(),
        &policy_snapshot_from_risk_level("test-policy", "high"),
    );

    let allow_id = if let AdmissionDecision::Allow { receipt_id, .. } = d_allow {
        receipt_id
    } else {
        panic!("expected Allow")
    };
    let approval_id = if let AdmissionDecision::RequireApproval { decision_id, .. } = d_high_risk {
        decision_id
    } else {
        panic!("expected RequireApproval")
    };
    assert_ne!(
        allow_id, approval_id,
        "Receipt IDs should differ per decision shape"
    );
}

#[test]
fn facts_with_evidence_admits_low_risk_action() {
    let engine = DefaultAuthorityEngine::new();
    let policy = policy_snapshot_from_risk_level("test-policy", "low");
    let actor = human_actor_with_cap("cli.execute");
    let mut facts = default_facts();
    facts
        .evidence_refs
        .push(sddk_engine::authority_engine::EvidenceRef(
            "ev-1".to_string(),
        ));
    let decision = engine.admit(
        &proposal(ActionKind::CliRun, "verify"),
        &actor,
        &facts,
        &policy,
    );
    assert!(decision.is_allow(), "got {:?}", decision);
}
