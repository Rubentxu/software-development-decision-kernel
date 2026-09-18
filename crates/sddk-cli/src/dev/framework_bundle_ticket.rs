//! A6-4 — R4-B migration to **shared** `AuthorityTicketService` for the
//! `framework_bundle` surface.
//!
//! As of A6-3, the `AuthorityTicketService` is the canonical authority
//! facade: one per CLI process, shared monotonic seq, shared bus, shared
//! fence domain. A6-1's standalone `AdmissionTicketBus` is **deleted**
//! from this call site in favour of `process_service()`.
//!
//! This file is therefore a **thin compat facade**: the public function
//! `with_framework_bundle_ticket(...)` keeps the same signature as A6-1
//! so `run_dev_install` does not have to change. Its body delegates to
//! the process-wide service.
//!
//! Pattern: ADR-0133 §"Scope of this cycle" — A6-4 migrates the call
//! sites, the service is the new authority. The local bus / engine /
//! `current_seq = 0` from A6-1 are GONE from production code.

use sddk_engine::authority_admission_ticket::AdmissionTicketError;
use sddk_engine::authority_engine::{ActionKind, Actor, DigestSha256, Facts, PolicySnapshot};
use sddk_engine::authority_ticket_service::{
    AuthorityTicketService, AuthorityTicketServiceError, process_service,
};

/// Typed error returned by the `framework_bundle` ticket facade.
///
/// Kept stable so `run_dev_install` does not change its error mapping.
#[derive(Debug, thiserror::Error)]
pub enum FrameworkBundleTicketError {
    /// Admission denied by the authority engine.
    #[error("framework_bundle admission denied: {0}")]
    Denied(String),
    /// Admission ticket refused (PolicyChanged / FenceExpired / TicketAlreadyConsumed).
    #[error("framework_bundle ticket refused: {0}")]
    Ticket(#[source] AdmissionTicketError),
    /// Side-effect body returned an error after a valid ticket was consumed.
    #[error("framework_bundle effect failed: {0}")]
    Effect(#[source] anyhow::Error),
}

/// Helper: build the canonical `framework_bundle` policy.
///
/// The ticket's `policy_digest` is anchored to this snapshot. The
/// service re-validates `current_policy_digest` against it at consume
/// time.
pub fn framework_bundle_policy() -> PolicySnapshot {
    let mut policy = PolicySnapshot::default_low_risk("framework_bundle");
    policy.policy_digest = DigestSha256::compute(b"framework_bundle/v1-a6-1");
    policy.policy_version = 1;
    policy
}

/// A6-4 — wrap a `framework_bundle` side effect with the
/// **process-wide** `AuthorityTicketService`.
///
/// Body runs ONLY after a successful issue + consume against the
/// service. If the service denies or consume refuses (PolicyChanged /
/// FenceExpired / TicketAlreadyConsumed / Deny), the body never runs
/// and no prefix mutation happens.
///
/// Pattern: ADR-0133 §"Implementation".
///
/// **A6-4 §8 (helper disposition):** this function is `THIN_COMPAT_FACADE`
/// over the shared service. The standalone engine + bus + `current_seq = 0`
/// from A6-1 are **deleted** from production code.
pub fn with_framework_bundle_ticket<F, T>(
    actor: Actor,
    target_id: &str,
    body: F,
) -> Result<T, FrameworkBundleTicketError>
where
    F: FnOnce() -> anyhow::Result<T>,
{
    with_framework_bundle_ticket_on(process_service(), actor, target_id, body)
}

/// Variant that accepts an explicit service. Production uses
/// `with_framework_bundle_ticket` (the shared singleton). Tests use
/// this variant with a fresh local service to keep parallel test
/// runs isolated.
pub fn with_framework_bundle_ticket_on<F, T>(
    svc: &AuthorityTicketService,
    actor: Actor,
    target_id: &str,
    body: F,
) -> Result<T, FrameworkBundleTicketError>
where
    F: FnOnce() -> anyhow::Result<T>,
{
    let policy = framework_bundle_policy();
    // First try to register the canonical policy (idempotent if already
    // present). Whether or not registration succeeds, restore the
    // canonical digest on the service so a re-entry doesn't see a
    // leftover digest from a different surface.
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
            return Err(FrameworkBundleTicketError::Denied(msg));
        }
        Err(other) => {
            return Err(FrameworkBundleTicketError::Ticket(match other {
                AuthorityTicketServiceError::Ticket(e) => e,
                other => AdmissionTicketError::EngineBug {
                    reason: format!("{other:?}"),
                },
            }));
        }
    };
    svc.consume_at_live_now(&ticket, &policy).map_err(|e| {
        FrameworkBundleTicketError::Ticket(match e {
            AuthorityTicketServiceError::Ticket(e) => e,
            other => AdmissionTicketError::EngineBug {
                reason: format!("{other:?}"),
            },
        })
    })?;

    body().map_err(FrameworkBundleTicketError::Effect)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_engine::authority_admission_ticket::{
        AdmissionTicketBus, AdmissionTicketError, AuthorityNow,
    };
    use sddk_engine::authority_engine::{
        ActionProposal, ActorKind as EngineActorKind, AuthorityEngine, DefaultAuthorityEngine,
    };
    use sddk_engine::authority_ticket_service::AuthorityTicketService;

    fn cli_actor() -> Actor {
        Actor {
            kind: EngineActorKind::System {
                service: "a6-4-test".to_string(),
            },
            capabilities: vec!["cli.execute".to_string()],
            lease: None,
        }
    }

    #[test]
    fn fence_t1_happy_path_runs_body_and_consumes_ticket() {
        let invoked = std::sync::Mutex::new(false);
        let result = with_framework_bundle_ticket::<_, String>(
            cli_actor(),
            "framework_bundle-install",
            || {
                *invoked.lock().unwrap() = true;
                Ok("installed".to_string())
            },
        );
        let value = result.expect("happy-path wrapper returns Ok");
        assert_eq!(value, "installed");
        assert!(*invoked.lock().unwrap(), "body must have run");
    }

    #[test]
    fn fence_t4_one_shot_ticket_does_not_reissue_within_a_bus() {
        // Construct a fresh bus to confirm TicketAlreadyConsumed semantics
        // still hold at primitive level (regression for the A6-0 primitive).
        let bus = AdmissionTicketBus::new();
        let mut engine = DefaultAuthorityEngine::new();
        let p = framework_bundle_policy();
        engine.register_policy(p.clone()).unwrap();
        let proposal = ActionProposal {
            kind: sddk_engine::authority_engine::ActionKind::CliRelease,
            target_id: "framework_bundle-once".to_string(),
            payload_digest: None,
            created_at: time::OffsetDateTime::now_utc(),
        };
        let facts = Facts::default();
        let ticket = bus
            .issue(&engine, &proposal, &cli_actor(), &facts, &p, 0)
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
    fn fence_t5_deny_decision_yields_no_ticket_and_body_does_not_run() {
        let svc = AuthorityTicketService::new();
        let actor = Actor {
            kind: EngineActorKind::System {
                service: "no-priv".to_string(),
            },
            capabilities: Vec::new(),
            lease: None,
        };
        let invoked = std::sync::Mutex::new(false);
        let result =
            with_framework_bundle_ticket_on::<_, ()>(&svc, actor, "framework_bundle-deny", || {
                *invoked.lock().unwrap() = true;
                Ok(())
            });
        match result {
            Err(FrameworkBundleTicketError::Denied(_)) => {}
            other => panic!("expected Denied, got {:?}", other),
        }
        assert!(!*invoked.lock().unwrap(), "body must NOT run when Deny");
    }

    #[test]
    fn fence_t6_policy_swap_at_service_invalidates_ticket() {
        // Use a FRESH service so this test is independent of ordering.
        // The singleton is exercised by A6-3's own tests; the facade
        // asserts that its delegate to the service still rejects on
        // policy change.
        let svc = AuthorityTicketService::new();
        let policy_a = framework_bundle_policy();
        svc.register_policy(policy_a.clone()).unwrap();
        let actor = cli_actor();
        let proposal = ActionProposal {
            kind: sddk_engine::authority_engine::ActionKind::CliRelease,
            target_id: "framework_bundle-swap-a6-4".to_string(),
            payload_digest: None,
            created_at: time::OffsetDateTime::now_utc(),
        };
        let ticket = svc
            .issue(&actor, &proposal, &policy_a, &Facts::default())
            .expect("issue succeeds");
        let mut policy_b = framework_bundle_policy();
        policy_b.policy_digest = DigestSha256::compute(b"framework_bundle/v2-a6-4");
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
