//! A6-4 — R4-B migration to **shared** `AuthorityTicketService` for the
//! `github_releases` surface.
//!
//! As of A6-3, the `AuthorityTicketService` is the canonical authority
//! facade: one per CLI process, shared monotonic seq, shared bus, shared
//! fence domain. A6-2's standalone `AdmissionTicketBus` is **deleted**
//! from this call site in favour of `process_service()`.
//!
//! This file is a **thin compat facade**: the public function
//! `with_github_releases_ticket(...)` keeps the same signature as A6-2
//! so `run_release_apply` does not have to change. Its body delegates to
//! the process-wide service.
//!
//! Pattern: ADR-0133 §"Scope of this cycle" — A6-4 migrates the call
//! sites, the service is the new authority. The local bus / engine /
//! `current_seq = 0` from A6-2 are GONE from production code.

use sddk_engine::authority_admission_ticket::AdmissionTicketError;
use sddk_engine::authority_engine::{ActionKind, Actor, DigestSha256, Facts, PolicySnapshot};
use sddk_engine::authority_ticket_service::{
    AuthorityTicketService, AuthorityTicketServiceError, process_service,
};

/// Typed error returned by the `github_releases` ticket facade.
#[derive(Debug, thiserror::Error)]
pub enum GithubReleasesTicketError {
    /// Admission denied by the authority engine.
    #[error("github_releases admission denied: {0}")]
    Denied(String),
    /// Admission ticket refused.
    #[error("github_releases ticket refused: {0}")]
    Ticket(#[source] AdmissionTicketError),
    /// Apply chain failed after ticket consume.
    #[error("github_releases apply chain failed: {0}")]
    Apply(#[source] anyhow::Error),
}

/// Helper: build the canonical `github_releases` policy.
pub fn github_releases_policy() -> PolicySnapshot {
    let mut policy = PolicySnapshot::default_low_risk("github_releases");
    policy.policy_digest = DigestSha256::compute(b"github_releases/v1-a6-2");
    policy.policy_version = 1;
    policy
}

/// A6-4 — wrap the entire `apply_release` chain
/// (CreatePr → MergePr → CreateRelease) with the **process-wide**
/// `AuthorityTicketService`. Body runs ONLY after a successful
/// issue + consume against the service.
///
/// The caller passes the `Actor` already constructed. The facade does
/// NOT enrich capabilities — that is the caller's job.
///
/// **A6-4 §8 (helper disposition):** this function is `THIN_COMPAT_FACADE`
/// over the shared service. The standalone engine + bus + `current_seq = 0`
/// from A6-2 are **deleted** from production code.
pub fn with_github_releases_ticket<F, T>(
    actor: Actor,
    target_id: &str,
    body: F,
) -> Result<T, GithubReleasesTicketError>
where
    F: FnOnce() -> anyhow::Result<T>,
{
    with_github_releases_ticket_on(process_service(), actor, target_id, body)
}

/// Variant that accepts an explicit service. See `with_framework_bundle_ticket_on`.
pub fn with_github_releases_ticket_on<F, T>(
    svc: &AuthorityTicketService,
    actor: Actor,
    target_id: &str,
    body: F,
) -> Result<T, GithubReleasesTicketError>
where
    F: FnOnce() -> anyhow::Result<T>,
{
    let policy = github_releases_policy();
    let _ = svc.register_policy(policy.clone());
    svc.set_last_policy_digest(policy.policy_digest.clone());

    let proposal = sddk_engine::authority_engine::ActionProposal {
        kind: ActionKind::CliRelease,
        target_id: target_id.to_string(),
        payload_digest: None,
        created_at: time::OffsetDateTime::now_utc(),
    };
    let ticket = match svc.issue(&actor, &proposal, &policy, &Facts::default()) {
        Ok(t) => t,
        Err(AuthorityTicketServiceError::Denied(msg)) => {
            return Err(GithubReleasesTicketError::Denied(msg));
        }
        Err(other) => {
            return Err(GithubReleasesTicketError::Ticket(match other {
                AuthorityTicketServiceError::Ticket(e) => e,
                other => AdmissionTicketError::EngineBug {
                    reason: format!("{other:?}"),
                },
            }));
        }
    };
    svc.consume_at_live_now(&ticket, &policy).map_err(|e| {
        GithubReleasesTicketError::Ticket(match e {
            AuthorityTicketServiceError::Ticket(e) => e,
            other => AdmissionTicketError::EngineBug {
                reason: format!("{other:?}"),
            },
        })
    })?;

    body().map_err(GithubReleasesTicketError::Apply)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_engine::authority_admission_ticket::{
        AdmissionTicketBus, AdmissionTicketError, AuthorityNow,
    };
    use sddk_engine::authority_engine::{
        ActionProposal, ActorKind, AuthorityEngine, DefaultAuthorityEngine,
    };
    use sddk_engine::authority_ticket_service::AuthorityTicketService;

    fn sys_actor(service: &str) -> Actor {
        Actor {
            kind: ActorKind::System {
                service: service.to_string(),
            },
            capabilities: vec!["cli.execute".to_string()],
            lease: None,
        }
    }

    #[test]
    fn fence_t1_happy_path_runs_body_and_consumes_ticket() {
        let svc = AuthorityTicketService::new();
        let invoked = std::sync::Mutex::new(false);
        let result = with_github_releases_ticket_on::<_, String>(
            &svc,
            sys_actor("a6-4-gr"),
            "vA6-4-test",
            || {
                *invoked.lock().unwrap() = true;
                Ok("released".to_string())
            },
        );
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
        let actor = sys_actor("a6-4-gr");
        let proposal = ActionProposal {
            kind: ActionKind::CliRelease,
            target_id: "github_releases-once".to_string(),
            payload_digest: None,
            created_at: time::OffsetDateTime::now_utc(),
        };
        let facts = Facts::default();
        let ticket = bus
            .issue(&engine, &proposal, &actor, &facts, &p, 0)
            .expect("issue succeeds");
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
        let svc = AuthorityTicketService::new();
        let actor = Actor {
            kind: ActorKind::System {
                service: "no-priv".to_string(),
            },
            capabilities: Vec::new(),
            lease: None,
        };
        let invoked = std::sync::Mutex::new(false);
        let result =
            with_github_releases_ticket_on::<_, ()>(&svc, actor, "github_releases-deny", || {
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
    fn fence_t6_policy_swap_at_service_invalidates_ticket() {
        let svc = AuthorityTicketService::new();
        let policy_a = github_releases_policy();
        svc.register_policy(policy_a.clone()).unwrap();
        let actor = sys_actor("a6-4-gr");
        let proposal = ActionProposal {
            kind: ActionKind::CliRelease,
            target_id: "github_releases-swap".to_string(),
            payload_digest: None,
            created_at: time::OffsetDateTime::now_utc(),
        };
        let ticket = svc
            .issue(&actor, &proposal, &policy_a, &Facts::default())
            .expect("issue with policy_a");
        let mut policy_b = github_releases_policy();
        policy_b.policy_digest = DigestSha256::compute(b"github_releases/v2-a6-4-swapped");
        policy_b.policy_version = 2;
        svc.register_policy(policy_b).unwrap();
        let err = svc
            .consume_at_live_now(&ticket, &policy_a)
            .expect_err("must refuse");
        match err {
            AuthorityTicketServiceError::Ticket(
                AdmissionTicketError::PolicyChanged { .. }
                | AdmissionTicketError::FenceExpired { .. },
            ) => {}
            other => panic!("expected PolicyChanged or FenceExpired, got {:?}", other),
        }
    }
}
