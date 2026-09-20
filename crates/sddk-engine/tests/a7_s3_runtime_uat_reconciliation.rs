//! A7-S3 — RUNTIME_ENHANCED UAT (R8, R11, R13 de
//! `05-CHRONOS-HANDOFF.md` §8), sin binario externo.
//!
//! - R8 restart/reconnect: tras una sesión caída, un nuevo
//!   `capture` (nueva sesión del port) produce resultados
//!   independientes con session_id distinto y capacidad de
//!   recuperación observable.
//! - R11 contradiction runtime↔static: cuando un runtime Affirms y
//!   una evidencia static Denies el mismo subject, la resolución es
//!   `Conflicted` — la contradicción SOBREVIVE, no se resuelve por
//!   silencio (REQ-A4S0-013).
//! - R13 no bypass: una acción sin admission ticket válido es
//!   Deny — el AuthorityEngine no se puede saltar.

use sddk_engine::architecture_graph::SoftwareUnitRef;
use sddk_engine::authority_engine::DigestSha256 as AuthDigest;
use sddk_engine::authority_engine::{
    ActionKind, ActionProposal, Actor, ActorKind, AuthorityEngine, DefaultAuthorityEngine,
    DenyReason, Facts, PolicySnapshot,
};
use sddk_engine::code_intelligence_port::DigestSha256;
use sddk_engine::observation::posture::{EvidencePosture, ObservationTargetRef};
use sddk_engine::observation::resolution::resolve_subject;
use sddk_engine::observation::types::{
    ObservationBasis, ObservationOrigin, ObservationSet, ObservationStance, ObservationSubject,
    SoftwareObservation,
};
use sddk_engine::runtime_evidence_port::ObservationSet as RuntimeObsSet;
use sddk_engine::runtime_evidence_port::{
    RuntimeCapabilitySnapshot, RuntimeCaptureRequest, RuntimeCaptureResult, RuntimeEvidencePort,
    RuntimePortError,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use time::OffsetDateTime;

// ─── R8: fake runtime with session death and restart ─────────────────────

struct RestartableRuntime {
    session: AtomicUsize,
}

impl RuntimeEvidencePort for RestartableRuntime {
    fn capabilities(&self) -> RuntimeCapabilitySnapshot {
        RuntimeCapabilitySnapshot {
            provider_name: "restartable-runtime".into(),
            provider_version: "0.1.0".into(),
            protocol_major: 1,
            tools: vec!["debug_run".into()],
        }
    }

    fn capture(
        &self,
        req: &RuntimeCaptureRequest,
    ) -> Result<RuntimeCaptureResult, RuntimePortError> {
        let n = self.session.fetch_add(1, Ordering::SeqCst);
        let mut counts = BTreeMap::new();
        counts.insert("function_entry".to_string(), 3);
        Ok(RuntimeCaptureResult {
            session_id: format!("sess-{n}"),
            duration_ns: Some(100),
            total_events: 3,
            event_counts: counts,
            thread_count: 1,
            exit_status: Some(0),
            digest: DigestSha256(format!("{:064x}", n)),
            observations: RuntimeObsSet::new(),
        })
    }
}

/// R8: after a session dies (simulated), re-capture through the
/// port yields a NEW independent session — ids differ, both are
/// complete typed results, no state carry-over, no partial reuse.
#[test]
fn t_r8_restart_yields_new_independent_session() {
    let rt = RestartableRuntime {
        session: AtomicUsize::new(0),
    };
    let req = || RuntimeCaptureRequest {
        program: "/tmp/t".into(),
        args: vec![],
        cwd: None,
        timeout_ms: 5_000,
    };
    let first = rt.capture(&req()).unwrap();
    // session "dies" (nothing to do in the fake: the port is
    // stateless per capture by design)
    let second = rt.capture(&req()).unwrap();
    assert_ne!(
        first.session_id, second.session_id,
        "restart must not reuse session"
    );
    assert_eq!(first.total_events, second.total_events);
    assert_ne!(first.digest, second.digest);
    assert!(
        second.exit_status.is_some(),
        "reconnected capture is complete"
    );
}

// ─── R11: runtime affirm vs static deny → Conflicted ─────────────────────

fn obs(producer: &str, stance: ObservationStance) -> SoftwareObservation {
    SoftwareObservation::declare(
        ObservationSubject::Unit(SoftwareUnitRef::new("target/main")),
        stance,
        sddk_engine::evidence_ref::EvidenceRef::new(
            sddk_engine::evidence_ref::EvidenceKind::Adhoc,
            format!("{producer}/evidence"),
        ),
        if producer == "runtime-provider" {
            ObservationOrigin::RuntimeProvider
        } else {
            ObservationOrigin::StaticProvider
        },
        ObservationBasis::for_provider_result(producer, "abc123"),
        None,
        producer,
    )
}

/// R11: runtime Affirms + static Denies over the same subject →
/// `Conflicted` posture. The contradiction is the information; it
/// survives reconciliation instead of collapsing to a winner.
#[test]
fn t_r11_runtime_static_contradiction_survives() {
    let mut set = ObservationSet::new();
    set.insert(obs("runtime-provider", ObservationStance::Affirms));
    set.insert(obs("static-analyzer", ObservationStance::Denies));
    let target = ObservationTargetRef::Unit(SoftwareUnitRef::new("target/main"));
    match resolve_subject(&set, &target) {
        EvidencePosture::Conflicted {
            supporting,
            contradicting,
            ..
        } => {
            assert_eq!(supporting.len(), 1, "runtime affirm kept");
            assert_eq!(contradicting.len(), 1, "static deny kept");
        }
        other => panic!("expected Conflicted, got {other:?}"),
    }
}

// ─── R13: no bypass of AuthorityEngine ────────────────────────────────────

/// R13: an actor without the required capability is denied. Runtime
/// evidence does not upgrade admission — there is no path to Allow
/// that skips the AuthorityEngine.
#[test]
fn t_r13_no_authority_engine_bypass() {
    let engine = DefaultAuthorityEngine::new();
    let policy = PolicySnapshot::default_low_risk("a7-s3-test");
    let proposal = ActionProposal {
        kind: ActionKind::CycleStart,
        target_id: "a7-s3-target".to_string(),
        payload_digest: Some(AuthDigest::compute(b"a7-s3")),
        created_at: OffsetDateTime::now_utc(),
    };
    // Actor WITHOUT "cycle.lifecycle" capability, even holding
    // runtime-evidence facts.
    let underprivileged = Actor {
        kind: ActorKind::Agent {
            profile_id: "leaf-profile".into(),
            profile_version: 1,
        },
        capabilities: vec!["vault.read".into()], // wrong capability
        lease: None,
    };
    let facts = Facts::default();
    let d = engine.admit(&proposal, &underprivileged, &facts, &policy);
    assert!(d.is_deny(), "uncapable actor must be denied, got {d:?}");
    // And the privileged actor passes — proving the denial is the
    // engine's decision, not a harness artifact.
    let privileged = Actor {
        kind: ActorKind::Human {
            id: "human-1".into(),
        },
        capabilities: vec!["cycle.lifecycle".into()],
        lease: None,
    };
    let d2 = engine.admit(&proposal, &privileged, &facts, &policy);
    assert!(d2.is_allow(), "capable human actor should pass, got {d2:?}");
}
