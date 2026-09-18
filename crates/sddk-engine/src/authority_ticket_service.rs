// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// authority_ticket_service.rs — A6-3: shared admission+ticket facade.
//
// Combines:
//   * `DefaultAuthorityEngine` — admit step (R4-A)
//   * `AdmissionTicketBus` — issue + consume (R4-B, A6-0 primitive)
// into a single service that:
//   * holds a **process-wide monotonic seq counter** (replaces the
//     `current_seq = 0` hardcoded at A6-1 / A6-2 call sites),
//   * owns a **shared bus** so different call sites (one CLI process can
//     call this service many times) see the same fence / consumed-set,
//   * accepts **live `PolicySnapshot`** registration so `PolicyChanged`
//     is detected when the caller swaps policies between issue and consume
//     (closes the honest limit in A6-2-RECEIPT.md §"Honest limits" #2).
//
// What this service is **not**:
//   * It is NOT the engine event log. The seq is a service-local counter,
//     not the seq from the canonical event log. Wiring the real event log
//     is A6-4 (out of scope for BASE_PRODUCTION_READY).
//   * It does NOT replace `AuthorityEngineRunner`. The runner is a thin
//     admit facade with no tickets. This service adds ticket semantics on
//     top of the same engine.
//   * It does NOT migrate A6-1 / A6-2 call sites. Wiring them is A6-4
//     work; for this cycle A6-1/A6-2 keep their local helpers, and the
//     honest limits named in A6-2-RECEIPT.md remain in force.
//
// See ADR-0133 for the design.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use crate::authority_admission_ticket::{
    AdmissionTicketBus, AdmissionTicketError, AuthorityAdmissionTicket, AuthorityNow,
};
use crate::authority_engine::{
    ActionKind, ActionProposal, Actor, AuthorityEngine, DefaultAuthorityEngine, Facts,
    PolicySnapshot,
};

/// Typed error returned by `AuthorityTicketService`.
///
/// Variants are split so callers can route by reason without parsing the
/// `Display` text. Mirrors the per-call-site pattern used in
/// `framework_bundle_ticket` and `github_releases_ticket`.
#[derive(Debug, thiserror::Error)]
pub enum AuthorityTicketServiceError {
    #[error("authority ticket service: deny at admit — {0}")]
    Denied(String),
    #[error("authority ticket service: ticket refused — {0}")]
    Ticket(#[source] AdmissionTicketError),
    #[error("authority ticket service: unknown surface {0:?}")]
    UnknownSurface(String),
    #[error("authority ticket service: internal lock poisoned")]
    LockPoisoned,
}

/// A6-3 — process-wide authority admission + ticket facade.
///
/// Cheap to clone (everything inside is `Arc`/`Atomic`). Construct one
/// per CLI process. Do NOT construct one per call site — that would
/// regress to the A6-0 primitive's per-bus-local behaviour.
#[derive(Debug, Clone)]
pub struct AuthorityTicketService {
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    engine: Mutex<DefaultAuthorityEngine>,
    bus: AdmissionTicketBus,
    seq: AtomicU64,
    /// Most recent policy registered for each `(action_kind, target_id)` is
    /// tracked by **policy_id** so the caller can swap policies between
    /// issue and consume without forcing the service to re-derive digests.
    last_policy_digest: Mutex<Option<crate::authority_engine::DigestSha256>>,
}

impl Default for AuthorityTicketService {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthorityTicketService {
    /// Construct an empty service. The caller registers policies via
    /// `register_policy` (or relies on the per-call `policy` arg in
    /// `issue_and_consume`).
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                engine: Mutex::new(DefaultAuthorityEngine::new()),
                bus: AdmissionTicketBus::new(),
                seq: AtomicU64::new(0),
                last_policy_digest: Mutex::new(None),
            }),
        }
    }

    /// Register a policy on the engine AND advance the fence so any
    /// previously-issued ticket is invalidated against this new fence.
    ///
    /// **Honest limit:** the engine's `register_policy` does not return a
    /// digest; we recompute the policy's own `policy_digest` here so the
    /// next `AuthorityNow` can return it without re-registering.
    pub fn register_policy(
        &self,
        policy: PolicySnapshot,
    ) -> Result<(), AuthorityTicketServiceError> {
        let mut engine = self
            .inner
            .engine
            .lock()
            .map_err(|_| AuthorityTicketServiceError::LockPoisoned)?;
        engine.register_policy(policy.clone()).map_err(|e| {
            AuthorityTicketServiceError::Ticket(AdmissionTicketError::EngineBug {
                reason: format!("register_policy: {e:?}"),
            })
        })?;
        let mut last = self
            .inner
            .last_policy_digest
            .lock()
            .map_err(|_| AuthorityTicketServiceError::LockPoisoned)?;
        *last = Some(policy.policy_digest.clone());
        self.inner.bus.advance_fence();
        Ok(())
    }

    /// Read the service's monotonic seq (the next seq an issue will use).
    pub fn next_seq(&self) -> u64 {
        self.inner.seq.load(Ordering::SeqCst)
    }

    /// Read the service's current fence token.
    pub fn current_fence(&self) -> u64 {
        self.inner.bus.current_fence()
    }

    /// Build the `AuthorityNow` for consume. Captures the live policy
    /// digest (most recently registered) + live fence + live seq.
    pub fn authority_now(
        &self,
        policy: &PolicySnapshot,
    ) -> Result<AuthorityNow, AuthorityTicketServiceError> {
        let _ = policy; // policy arg is for symmetry with future "live policy" overload
        let last = self
            .inner
            .last_policy_digest
            .lock()
            .map_err(|_| AuthorityTicketServiceError::LockPoisoned)?;
        Ok(AuthorityNow {
            current_policy_digest: last.clone().unwrap_or(policy.policy_digest.clone()),
            current_fence: self.inner.bus.current_fence(),
            current_seq: self.inner.seq.load(Ordering::SeqCst),
        })
    }

    /// Issue a ticket against the live engine + bus. Returns the ticket
    /// and bumps the service's monotonic seq (so the **next** issue sees
    /// `seq+1`). The returned ticket is consumed via
    /// `consume_at_live_now` to enforce fence / digest / seq invariants.
    pub fn issue(
        &self,
        actor: &Actor,
        proposal: &ActionProposal,
        policy: &PolicySnapshot,
        facts: &Facts,
    ) -> Result<AuthorityAdmissionTicket, AuthorityTicketServiceError> {
        let engine = self
            .inner
            .engine
            .lock()
            .map_err(|_| AuthorityTicketServiceError::LockPoisoned)?;
        let decision = engine.admit(proposal, actor, facts, policy);
        if !decision.is_allow() {
            return Err(AuthorityTicketServiceError::Denied(format!(
                "{:?}",
                decision
            )));
        }
        let seq = self.inner.seq.fetch_add(1, Ordering::SeqCst);
        let ticket = self
            .inner
            .bus
            .issue(&*engine, proposal, actor, facts, policy, seq)
            .map_err(AuthorityTicketServiceError::Ticket)?;
        Ok(ticket)
    }

    /// Consume a ticket at the service's live `AuthorityNow`.
    pub fn consume_at_live_now(
        &self,
        ticket: &AuthorityAdmissionTicket,
        policy: &PolicySnapshot,
    ) -> Result<(), AuthorityTicketServiceError> {
        let now = self.authority_now(policy)?;
        self.inner
            .bus
            .consume(ticket, &now)
            .map_err(AuthorityTicketServiceError::Ticket)
    }

    /// One-shot helper: issue a ticket and consume it at the live now.
    /// Mirrors the wrapper shape used by A6-1 / A6-2 but routes through
    /// the shared service (so seq / fence / digest are live).
    ///
    /// Returns the ticket so callers can include it in receipts. The body
    /// returns a `String` error so this module does not need to pull in
    /// `anyhow`; callers map the string back to their own error type.
    pub fn issue_and_consume<F, T>(
        &self,
        actor: Actor,
        action: ActionKind,
        target_id: &str,
        policy: PolicySnapshot,
        facts: Facts,
        body: F,
    ) -> Result<(AuthorityAdmissionTicket, T), AuthorityTicketServiceError>
    where
        F: FnOnce() -> Result<T, String>,
    {
        let proposal = ActionProposal {
            kind: action,
            target_id: target_id.to_string(),
            payload_digest: None,
            created_at: time::OffsetDateTime::now_utc(),
        };
        let ticket = self.issue(&actor, &proposal, &policy, &facts)?;
        self.consume_at_live_now(&ticket, &policy)?;
        let value = body().map_err(|msg| {
            AuthorityTicketServiceError::Ticket(AdmissionTicketError::EngineBug {
                reason: format!("body returned error: {msg}"),
            })
        })?;
        Ok((ticket, value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authority_engine::{ActorKind, DigestSha256, PolicySnapshot};

    fn low_risk_policy(policy_id: &str, version_label: &str, version: u32) -> PolicySnapshot {
        let mut p = PolicySnapshot::default_low_risk(policy_id);
        p.policy_digest = DigestSha256::compute(version_label.as_bytes());
        p.policy_version = version;
        p
    }

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
    fn fence_t1_admit_issue_consume_returns_ticket_and_value() {
        let svc = AuthorityTicketService::new();
        let policy = low_risk_policy("svc-t1", "svc-t1/v1", 1);
        svc.register_policy(policy.clone()).unwrap();
        let actor = sys_actor("svc-t1");
        let (ticket, value) = svc
            .issue_and_consume::<_, String>(
                actor,
                ActionKind::CliRelease,
                "v1.0.0",
                policy.clone(),
                Facts::default(),
                || Ok("ok".to_string()),
            )
            .expect("issue+consume");
        assert_eq!(value, "ok");
        assert!(!ticket.ticket_id.is_empty());
        assert_eq!(svc.next_seq(), 1, "seq bumped exactly once");
    }

    #[test]
    fn fence_t2_swapped_policy_is_rejected_at_consume() {
        let svc = AuthorityTicketService::new();
        let policy = low_risk_policy("svc-t2", "svc-t2/v1", 1);
        svc.register_policy(policy.clone()).unwrap();
        let actor = sys_actor("svc-t2");
        let proposal = ActionProposal {
            kind: ActionKind::CliRelease,
            target_id: "v2.0.0".to_string(),
            payload_digest: None,
            created_at: time::OffsetDateTime::now_utc(),
        };
        let ticket = svc
            .issue(&actor, &proposal, &policy, &Facts::default())
            .unwrap();
        // Register a new policy under a DIFFERENT policy_id so the engine
        // accepts it. digest changes ⇒ PolicyChanged at consume.
        let swapped = low_risk_policy("svc-t2-v2", "svc-t2/v2-swapped", 2);
        svc.register_policy(swapped).unwrap();
        let err = svc
            .consume_at_live_now(&ticket, &policy)
            .expect_err("must refuse");
        match err {
            AuthorityTicketServiceError::Ticket(AdmissionTicketError::PolicyChanged { .. }) => {}
            other => panic!("expected PolicyChanged, got {:?}", other),
        }
    }

    #[test]
    fn fence_t3_advance_fence_via_register_policy_invalidates_in_flight() {
        let svc = AuthorityTicketService::new();
        let policy_a = low_risk_policy("svc-t3-a", "svc-t3/v1", 1);
        svc.register_policy(policy_a.clone()).unwrap();
        let actor = sys_actor("svc-t3");
        let proposal = ActionProposal {
            kind: ActionKind::CliRelease,
            target_id: "v3.0.0".to_string(),
            payload_digest: None,
            created_at: time::OffsetDateTime::now_utc(),
        };
        let ticket = svc
            .issue(&actor, &proposal, &policy_a, &Facts::default())
            .unwrap();
        // Re-registering with a new policy_id bumps the fence.
        let policy_b = low_risk_policy("svc-t3-b", "svc-t3/v2-bumped", 2);
        svc.register_policy(policy_b.clone()).unwrap();
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

    #[test]
    fn fence_t4_shared_bus_across_two_issues_distinct_seq() {
        let svc = AuthorityTicketService::new();
        let policy = low_risk_policy("svc-t4", "svc-t4/v1", 1);
        svc.register_policy(policy.clone()).unwrap();
        let actor = sys_actor("svc-t4");

        let proposal_a = ActionProposal {
            kind: ActionKind::CliRelease,
            target_id: "a".to_string(),
            payload_digest: None,
            created_at: time::OffsetDateTime::now_utc(),
        };
        let proposal_b = ActionProposal {
            kind: ActionKind::CliRelease,
            target_id: "b".to_string(),
            payload_digest: None,
            created_at: time::OffsetDateTime::now_utc(),
        };
        let t_a = svc
            .issue(&actor, &proposal_a, &policy, &Facts::default())
            .unwrap();
        let t_b = svc
            .issue(&actor, &proposal_b, &policy, &Facts::default())
            .unwrap();
        assert_ne!(t_a.issued_at_seq, t_b.issued_at_seq, "seq must advance");
        svc.consume_at_live_now(&t_a, &policy).unwrap();
        svc.consume_at_live_now(&t_b, &policy).unwrap();
    }

    #[test]
    fn fence_t5_deny_yields_no_ticket() {
        let svc = AuthorityTicketService::new();
        let policy = low_risk_policy("svc-t5", "svc-t5/v1", 1);
        svc.register_policy(policy.clone()).unwrap();
        let actor = Actor {
            kind: ActorKind::System {
                service: "no-cap".to_string(),
            },
            capabilities: Vec::new(),
            lease: None,
        };
        let proposal = ActionProposal {
            kind: ActionKind::CliRelease,
            target_id: "v5.0.0".to_string(),
            payload_digest: None,
            created_at: time::OffsetDateTime::now_utc(),
        };
        let err = svc
            .issue(&actor, &proposal, &policy, &Facts::default())
            .expect_err("must refuse");
        match err {
            AuthorityTicketServiceError::Denied(_) => {}
            other => panic!("expected Denied, got {:?}", other),
        }
        assert_eq!(svc.next_seq(), 0, "deny must not bump seq");
    }
}
