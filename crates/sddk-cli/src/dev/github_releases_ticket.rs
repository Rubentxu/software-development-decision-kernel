//! A6-2 — R4-B migration helper for the `github_releases` surface (Forge route).
//!
//! Wraps `apply_release(...)` (which performs `pr.create` → `pr.merge` →
//! `release.create` through the `Forge` trait) with an `AdmissionTicketBus`
//! issue + consume, per
//! `docs/architecture/adrs/ADR-0132-GITHUB-RELEASES-MIGRATION-PATTERN.md`
//! (which references ADR-0131).
//!
//! The ticket covers the **whole chain**: if `consume` refuses, none of the
//! three forge steps run, because they all live behind `apply_release(...)`.
//! `GitHubForge` and `MockForge` are not modified.

use sddk_engine::authority_admission_ticket::{
    AdmissionTicketBus, AdmissionTicketError, AuthorityNow,
};
use sddk_engine::authority_engine::{
    ActionKind, ActionProposal, Actor, AuthorityEngine as _, DefaultAuthorityEngine, DigestSha256,
    Facts, PolicySnapshot,
};
use time::OffsetDateTime;

/// Typed error returned by the `github_releases` ticket wrapper.
#[derive(Debug, thiserror::Error)]
pub enum GithubReleasesTicketError {
    #[error("github_releases admission denied: {0}")]
    Denied(String),
    #[error("github_releases ticket refused: {0}")]
    Ticket(#[source] AdmissionTicketError),
    #[error("github_releases apply chain failed: {0}")]
    Apply(#[source] anyhow::Error),
}

impl From<AdmissionTicketError> for GithubReleasesTicketError {
    fn from(value: AdmissionTicketError) -> Self {
        Self::Ticket(value)
    }
}

/// Shared policy for `github_releases`. Anchors the ticket; the engine
/// re-validates `current_policy_digest` against it at consume time.
pub fn github_releases_policy() -> PolicySnapshot {
    let mut policy = PolicySnapshot::default_low_risk("github_releases");
    policy.policy_digest = DigestSha256::compute(b"github_releases/v1-a6-2");
    policy.policy_version = 1;
    policy
}

/// A6-2 — Wrap a `github_releases` apply chain with `AdmissionTicketBus`
/// issue + consume. The body runs ONLY after the ticket is consumed; if
/// consume fails the body never runs.
///
/// Pattern is documented in
/// `docs/architecture/adrs/ADR-0132-GITHUB-RELEASES-MIGRATION-PATTERN.md`.
///
/// Honest limits (inherited from ADR-0131): T2 (PolicyChanged via
/// policy_digest) is NOT directly pinned at this call site. The
/// primitive-level T2 is pinned in
/// `crates/sddk-engine/tests/a6_0_admission_tickets.rs`. A6-3 will
/// thread the live snapshot.
///
/// The caller passes the `Actor` already constructed (with whatever
/// `capabilities` it holds). The helper does **not** add capabilities
/// on its own — that is the caller's job. This lets tests construct
/// actors whose capability set will be denied by the helper's policy.
pub fn with_github_releases_ticket<F, T>(
    actor: Actor,
    target_id: &str,
    body: F,
) -> Result<T, GithubReleasesTicketError>
where
    F: FnOnce() -> anyhow::Result<T>,
{
    let mut engine = DefaultAuthorityEngine::new();
    let policy = github_releases_policy();
    engine.register_policy(policy.clone()).map_err(|e| {
        GithubReleasesTicketError::Apply(anyhow::anyhow!("register github_releases policy: {e:?}"))
    })?;

    let proposal = ActionProposal {
        kind: ActionKind::CliRelease,
        target_id: target_id.to_string(),
        payload_digest: None,
        created_at: OffsetDateTime::now_utc(),
    };
    let facts = Facts::default();

    // admit step (R4-A: deny ⇒ no ticket).
    let decision = engine.admit(&proposal, &actor, &facts, &policy);
    if !decision.is_allow() {
        return Err(GithubReleasesTicketError::Denied(format!("{:?}", decision)));
    }

    // Issue ticket (R4-B wired in A6-0).
    let bus = AdmissionTicketBus::new();
    let ticket = bus.issue(&engine, &proposal, &actor, &facts, &policy, 0)?;

    // Consume anchor — same honest limit as A6-1 (`seq = 0`).
    let now = AuthorityNow {
        current_policy_digest: policy.policy_digest.clone(),
        current_fence: bus.current_fence(),
        current_seq: 0,
    };
    bus.consume(&ticket, &now)?;

    body().map_err(GithubReleasesTicketError::Apply)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_engine::authority_admission_ticket::AdmissionTicketBus;
    use sddk_engine::authority_engine::{
        ActorKind, AuthorityEngine, DefaultAuthorityEngine, DigestSha256, Facts,
    };

    fn system_actor() -> Actor {
        Actor {
            kind: ActorKind::System {
                service: "a6-2-test".to_string(),
            },
            capabilities: vec!["cli.execute".to_string()],
            lease: None,
        }
    }

    #[test]
    fn fence_t1_happy_path_runs_body_and_consumes_ticket() {
        let invoked = std::sync::Mutex::new(false);
        let result = with_github_releases_ticket::<_, String>(system_actor(), "vA6-2-test", || {
            *invoked.lock().unwrap() = true;
            Ok("released".to_string())
        });
        let value = result.expect("happy-path wrapper returns Ok");
        assert_eq!(value, "released");
        assert!(*invoked.lock().unwrap(), "body must have run");
    }

    #[test]
    fn fence_t4_one_shot_ticket_does_not_reissue_within_a_bus() {
        let bus = AdmissionTicketBus::new();
        let mut engine = DefaultAuthorityEngine::new();
        let p = github_releases_policy();
        engine.register_policy(p.clone()).unwrap();
        let actor = system_actor();
        let proposal = ActionProposal {
            kind: sddk_engine::authority_engine::ActionKind::CliRelease,
            target_id: "github_releases-once".to_string(),
            payload_digest: None,
            created_at: time::OffsetDateTime::now_utc(),
        };

        let facts = Facts::default();
        let ticket = bus
            .issue(&engine, &proposal, &actor, &facts, &p, 0)
            .unwrap();
        let now = AuthorityNow {
            current_policy_digest: p.policy_digest.clone(),
            current_fence: bus.current_fence(),
            current_seq: 0,
        };
        bus.consume(&ticket, &now).expect("first consume");
        let err = bus
            .consume(&ticket, &now)
            .expect_err("second consume must refuse");
        match err {
            AdmissionTicketError::TicketAlreadyConsumed { .. } => {}
            other => panic!("expected TicketAlreadyConsumed, got {:?}", other),
        }
    }

    #[test]
    fn fence_t5_deny_yields_no_ticket_and_body_does_not_run() {
        let invoked = std::sync::Mutex::new(false);
        // To force Deny inside the helper (which constructs the policy
        // internally), we use an actor whose actor_kind would be denied by
        // a policy that denies all System entries. The helper's policy
        // accepts System actors for `CliRelease` when the capability is
        // present, so we instead force Deny by submitting an actor kind
        // that the engine denies. Agent with a profile_id is the safe
        // choice: agent actions on a low-risk policy without a recorded
        // approval_ref are denied by default.
        // Construct an actor without any capabilities so the engine denies
        // the action. The caller now has direct control over the actor.
        let actor = Actor {
            kind: ActorKind::System {
                service: "no-priv".to_string(),
            },
            capabilities: Vec::new(),
            lease: None,
        };
        let result = with_github_releases_ticket::<_, ()>(actor, "github_releases-deny", || {
            *invoked.lock().unwrap() = true;
            Ok(())
        });
        match result {
            Err(GithubReleasesTicketError::Denied(_)) => {}
            other => panic!("expected Denied, got {:?}", other),
        }
        assert!(!*invoked.lock().unwrap(), "body must NOT run when Deny");
    }

    #[test]
    fn fence_t6_ticket_with_swapped_policy_is_rejected_at_consume() {
        let bus = AdmissionTicketBus::new();
        let mut engine = DefaultAuthorityEngine::new();
        let mut swapped = github_releases_policy();
        swapped.policy_digest = DigestSha256::compute(b"github_releases/v2-a6-2-swapped");
        swapped.policy_version = 2;
        engine.register_policy(swapped.clone()).unwrap();
        let actor = system_actor();
        let proposal = ActionProposal {
            kind: sddk_engine::authority_engine::ActionKind::CliRelease,
            target_id: "github_releases-swap".to_string(),
            payload_digest: None,
            created_at: time::OffsetDateTime::now_utc(),
        };

        let facts = Facts::default();
        let ticket = bus
            .issue(&engine, &proposal, &actor, &facts, &swapped, 0)
            .unwrap();
        // Now consume using the wrapper's real anchor — must refuse.
        let p_real = github_releases_policy();
        let now = AuthorityNow {
            current_policy_digest: p_real.policy_digest.clone(),
            current_fence: bus.current_fence(),
            current_seq: 0,
        };
        let err = bus
            .consume(&ticket, &now)
            .expect_err("consume against the wrapper's anchor must refuse");
        match err {
            AdmissionTicketError::PolicyChanged { .. } => {}
            other => panic!("expected PolicyChanged, got {:?}", other),
        }
    }
}
