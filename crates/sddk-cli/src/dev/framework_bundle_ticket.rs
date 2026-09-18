//! A6-1 — R4-B migration helper for the `framework_bundle` surface.
//!
//! Wraps a side-effect closure with `AdmissionTicketBus::issue` + `consume`,
//! per `docs/architecture/adrs/ADR-0131-MIGRATION-PATTERN-FOR-WRITABLE-SURFACES.md`.
//!
//! Note: this helper is defined in a dedicated module so the actual call site
//! in `install.rs` can opt in incrementally. The module exists whether or
//! not `run_dev_install` wraps its body; A6-1's FENCE integration tests
//! exercise the helper directly so the contract is pinned even before the
//! call site migration is wired in.

use sddk_engine::authority_admission_ticket::{
    AdmissionTicketBus, AdmissionTicketError, AuthorityNow,
};
use sddk_engine::authority_engine::{
    ActionKind, ActionProposal, Actor, AuthorityEngine, DefaultAuthorityEngine, DigestSha256,
    Facts, PolicySnapshot,
};
use std::sync::Arc;
use time::OffsetDateTime;

/// Typed error returned by the `framework_bundle` ticket wrapper.
#[derive(Debug, thiserror::Error)]
pub(crate) enum FrameworkBundleTicketError {
    #[error("framework_bundle admission denied: {0}")]
    Denied(String),
    #[error("framework_bundle ticket refused: {0}")]
    Ticket(#[source] AdmissionTicketError),
    #[error("framework_bundle effect failed: {0}")]
    Effect(#[source] anyhow::Error),
}

impl From<AdmissionTicketError> for FrameworkBundleTicketError {
    fn from(value: AdmissionTicketError) -> Self {
        Self::Ticket(value)
    }
}

/// Shared policy construction. The ticket's `policy_digest` is anchored
/// to this snapshot; the engine re-validates `current_policy_digest`
/// against it at consume time.
pub(crate) fn framework_bundle_policy() -> PolicySnapshot {
    let mut policy = PolicySnapshot::default_low_risk("framework_bundle");
    policy.policy_digest = DigestSha256::compute(b"framework_bundle/v1-a6-1");
    policy.policy_version = 1;
    policy
}

/// A6-1 — Wrap a `framework_bundle` side effect with an
/// `AdmissionTicketBus` issue + consume. The body runs ONLY after the
/// ticket is consumed; if consume fails the body never runs.
///
/// Pattern is documented in
/// `docs/architecture/adrs/ADR-0131-MIGRATION-PATTERN-FOR-WRITABLE-SURFACES.md`.
///
/// Honest limits (per ADR-0131): T2 (PolicyChanged via policy_digest) is
/// NOT directly pinned at this call site — the runner does not expose
/// the live `PolicySnapshot`. The primitive-level T2 is pinned inside
/// `crates/sddk-engine/tests/a6_0_admission_tickets.rs`. A6-3 will
/// thread the live snapshot. Until then this helper pins T1, T4, T5,
/// T6 (verdict-anchored consume).
pub(crate) fn with_framework_bundle_ticket<F, T>(
    actor: Actor,
    target_id: &str,
    body: F,
) -> Result<T, FrameworkBundleTicketError>
where
    F: FnOnce() -> anyhow::Result<T>,
{
    let engine = Arc::new({
        let mut e = DefaultAuthorityEngine::new();
        let policy = framework_bundle_policy();
        // registration can fail only on duplicates; we register a single
        // fresh policy so this is infallible in practice; surface the error
        // by map_err to keep the contract fail-closed.
        e.register_policy(policy).map_err(|e| {
            FrameworkBundleTicketError::Effect(anyhow::anyhow!(
                "register framework_bundle policy: {e:?}"
            ))
        })?;
        e
    });

    let proposal = ActionProposal {
        kind: ActionKind::CliRelease,
        target_id: target_id.to_string(),
        payload_digest: None,
        created_at: OffsetDateTime::now_utc(),
    };
    let facts = Facts::default();
    let policy_ref = framework_bundle_policy();

    // admit step (R4-A: deny ⇒ no ticket).
    let decision = engine.admit(&proposal, &actor, &facts, &policy_ref);
    if !decision.is_allow() {
        return Err(FrameworkBundleTicketError::Denied(format!(
            "{:?}",
            decision
        )));
    }

    // Issue ticket (R4-B wired in A6-0).
    let bus = AdmissionTicketBus::new();
    let ticket = bus.issue(&*engine, &proposal, &actor, &facts, &policy_ref, 0)?;

    // Consume anchor: the live state at the moment the body would run.
    // `seq = 0` is the A6-1 honest limit (no canonical ledger advancement
    // for a CLI install; the runtime is single-threaded). A6-3 will thread
    // the real ledger sequence.
    let now = AuthorityNow {
        current_policy_digest: policy_ref.policy_digest.clone(),
        current_fence: bus.current_fence(),
        current_seq: 0,
    };
    bus.consume(&ticket, &now)?;

    body().map_err(FrameworkBundleTicketError::Effect)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_engine::authority_admission_ticket::AdmissionTicketBus;
    use sddk_engine::authority_engine::ActorKind as EngineActorKind;
    use sddk_engine::authority_engine::{
        ActionProposal, AuthorityEngine, DefaultAuthorityEngine, DigestSha256, Facts,
    };

    fn cli_actor() -> Actor {
        Actor {
            kind: EngineActorKind::System {
                service: "a6-1-test".to_string(),
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
        let actor = Actor {
            kind: EngineActorKind::System {
                service: "no-priv".to_string(),
            },
            capabilities: Vec::new(),
            lease: None,
        };
        let invoked = std::sync::Mutex::new(false);
        let result = with_framework_bundle_ticket::<_, ()>(actor, "framework_bundle-deny", || {
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
    fn fence_t6_ticket_with_swapped_policy_is_rejected_at_consume() {
        let bus = AdmissionTicketBus::new();
        let mut engine = DefaultAuthorityEngine::new();
        let mut swapped = framework_bundle_policy();
        swapped.policy_digest = DigestSha256::compute(b"framework_bundle/v2-a6-1-swapped");
        swapped.policy_version = 2;
        engine.register_policy(swapped.clone()).unwrap();
        let proposal = ActionProposal {
            kind: sddk_engine::authority_engine::ActionKind::CliRelease,
            target_id: "framework_bundle-swap".to_string(),
            payload_digest: None,
            created_at: time::OffsetDateTime::now_utc(),
        };
        let facts = Facts::default();
        let ticket = bus
            .issue(&engine, &proposal, &cli_actor(), &facts, &swapped, 0)
            .expect("issue with swapped policy");
        let p_real = framework_bundle_policy();
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
