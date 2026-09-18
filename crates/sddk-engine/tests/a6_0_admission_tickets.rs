//! Integration test (A6-0 / FENCE matrix) — AuthorityAdmissionTicket over the
//! real `DefaultAuthorityEngine`.
//!
//! Purpose: pin the contract that the ticket primitive reuses the engine's
//! own `admit` decision (no shadow authority) AND that a real engine call
//! produces a ticket that survives a meaningful sequence of policy reads.
//!
//! Complements `authority_admission_ticket::tests` (the unit FENCE matrix) by
//! exercising the engine module, not a `DummyEngine`.

use sddk_engine::authority_admission_ticket::{AdmissionTicketBus, AuthorityNow};
use sddk_engine::authority_engine::{
    ActionKind, ActionProposal, Actor, ActorKind, AuthorityEngine, DefaultAuthorityEngine,
    DigestSha256, Facts, PolicySnapshot, Postcondition,
};
use time::OffsetDateTime;

fn low_risk_policy(policy_id: &str, version: u32, payload: &[u8]) -> PolicySnapshot {
    let mut p = PolicySnapshot::default_low_risk(policy_id);
    p.policy_version = version;
    p.policy_digest = DigestSha256::compute(payload);
    p
}

fn proposal() -> ActionProposal {
    ActionProposal {
        kind: ActionKind::CycleStart,
        target_id: "ticket-integration".to_string(),
        payload_digest: Some(DigestSha256::compute(b"proposal-payload")),
        created_at: OffsetDateTime::now_utc(),
    }
}

fn actor() -> Actor {
    Actor {
        kind: ActorKind::Human {
            id: "ticket-test".to_string(),
        },
        // The default low-risk policy requires "cycle.lifecycle" for CycleStart.
        capabilities: vec!["cycle.lifecycle".to_string()],
        lease: None,
    }
}

fn facts() -> Facts {
    Facts::default()
}

/// T1 (extended): a ticket issued via the real engine over a Low-risk policy
/// survives a consume at `seq = issued + 1` with unchanged policy.
#[test]
fn fence_t1_real_engine_low_risk_ticket_survives_same_state() {
    let mut engine = DefaultAuthorityEngine::new();
    let policy = low_risk_policy("ticket-policy", 1, b"v1");
    engine
        .register_policy(policy.clone())
        .expect("register succeeds");
    let bus = AdmissionTicketBus::new();
    let ticket = bus
        .issue(&engine, &proposal(), &actor(), &facts(), &policy, 100)
        .expect("Allow policy must issue a ticket");
    let now = AuthorityNow {
        current_policy_digest: policy.policy_digest.clone(),
        current_fence: ticket.fence_token,
        current_seq: 101,
    };
    bus.consume(&ticket, &now).expect("T1 fence ok");
}

/// T2 (extended): after a *new* policy is registered, an old ticket
/// refuses consumption with `PolicyChanged`.
#[test]
fn fence_t2_real_engine_registers_new_policy_refuses_old_ticket() {
    let mut engine = DefaultAuthorityEngine::new();
    let policy_v1 = low_risk_policy("ticket-policy", 1, b"v1");
    engine
        .register_policy(policy_v1.clone())
        .expect("register v1");
    let bus = AdmissionTicketBus::new();
    let ticket = bus
        .issue(&engine, &proposal(), &actor(), &facts(), &policy_v1, 200)
        .expect("v1 issues");

    // Advance the bus fence (simulating a register_policy that invalidates
    // any in-flight tickets — the policy change is the trigger we test for).
    let policy_v2 = low_risk_policy("ticket-policy", 2, b"v2");
    engine
        .register_policy(policy_v2.clone())
        .expect("register v2");
    bus.advance_fence();

    let now = AuthorityNow {
        current_policy_digest: policy_v2.policy_digest.clone(),
        current_fence: bus.current_fence(),
        current_seq: 201,
    };
    let err = bus
        .consume(&ticket, &now)
        .expect_err("T2 must refuse after policy change");
    match err {
        sddk_engine::authority_admission_ticket::AdmissionTicketError::PolicyChanged {
            ticket_digest,
            current_digest,
            ..
        } => {
            assert_eq!(ticket_digest, policy_v1.policy_digest);
            assert_eq!(current_digest, policy_v2.policy_digest);
        }
        other => panic!("expected PolicyChanged, got {:?}", other),
    }
}

/// T5 (extended): a `Deny` decision from the real engine must not issue a
/// ticket. We induce `Deny` by registering a policy whose deny_override
/// matches the `(actor.kind, proposal.kind)` pair so the engine short-circuits.
#[test]
fn fence_t5_real_engine_deny_does_not_issue_ticket() {
    let mut engine = DefaultAuthorityEngine::new();
    let mut policy = low_risk_policy("ticket-deny-policy", 1, b"deny1");
    // Force a deny by overriding all (human, *) actions for CycleStart.
    policy
        .deny_override
        .insert(("human".to_string(), ActionKind::CycleStart));
    engine.register_policy(policy.clone()).expect("register");
    let bus = AdmissionTicketBus::new();
    let err = bus
        .issue(&engine, &proposal(), &actor(), &facts(), &policy, 300)
        .expect_err("Deny must not issue a ticket");
    assert!(
        matches!(
            err,
            sddk_engine::authority_admission_ticket::AdmissionTicketError::NotAllow { .. }
        ),
        "got {:?}",
        err
    );
}

/// Smoke: ensure the `postconditions` field of an `Allow` decision can be
/// captured without disturbing ticket construction. The dummy test version
/// already covers the rest of the enum.
#[test]
fn fence_smoke_allow_decision_carries_postconditions_through_ticket() {
    // We construct an Allow decision by hand so postconditions are non-empty.
    use sddk_engine::authority_engine::{AdmissionDecision, AuthorityEngine};

    struct PostconditionEngine;

    impl AuthorityEngine for PostconditionEngine {
        fn admit(
            &self,
            _proposal: &ActionProposal,
            _actor: &Actor,
            _facts: &Facts,
            _policy: &PolicySnapshot,
        ) -> AdmissionDecision {
            AdmissionDecision::Allow {
                receipt_id: "allow-pc".to_string(),
                postconditions: vec![Postcondition {
                    label: "policy.v2.must_be_observed".to_string(),
                    verification: "must_consult_v2_at_effect_time".to_string(),
                }],
            }
        }
        fn explain(
            &self,
            _decision: &AdmissionDecision,
        ) -> sddk_engine::authority_engine::AdmissionExplanation {
            sddk_engine::authority_engine::AdmissionExplanation {
                policy_id: String::new(),
                policy_version: 0,
                gates_applied: Vec::new(),
                evidence_refs_used: Vec::new(),
                deny_reasons_evaluated: Vec::new(),
                approval_requirements_considered: Vec::new(),
                decision_digest: DigestSha256::compute(b"pc"),
            }
        }
        fn policy_at(
            &self,
            _version: u32,
        ) -> Result<PolicySnapshot, sddk_engine::authority_engine::AuthorityEngineError> {
            Ok(low_risk_policy("pc", 1, b"pc1"))
        }
        fn register_policy(
            &mut self,
            _policy: PolicySnapshot,
        ) -> Result<(), sddk_engine::authority_engine::AuthorityEngineError> {
            Ok(())
        }
    }

    let bus = AdmissionTicketBus::new();
    let policy = low_risk_policy("pc", 1, b"pc1");
    let ticket = bus
        .issue(
            &PostconditionEngine,
            &proposal(),
            &actor(),
            &facts(),
            &policy,
            400,
        )
        .expect("issue succeeds");
    // The ticket's decision carries the postconditions; we access it through
    // the matching arm to verify nothing was dropped.
    match &ticket.decision {
        AdmissionDecision::Allow { postconditions, .. } => {
            assert_eq!(postconditions.len(), 1);
            assert_eq!(postconditions[0].label, "policy.v2.must_be_observed");
        }
        _ => panic!("expected Allow"),
    }
}
