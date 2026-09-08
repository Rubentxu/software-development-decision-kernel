//! Provider Failover Substrate — RED tests from REQ-ProviderFailover §Scenarios.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-ProviderFailover.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-080-PROVIDER-FAILOVER-SUBSTRATE.md

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use sddk_engine::retry::{Clock, MockClock};
use sddk_engine::{
    CircuitBreakerStore, InMemoryCircuitBreakerStore, InMemoryTelemetrySink, Provider,
    ProviderError, ProviderFailure, ProviderKind, ProviderOutput, ProviderOutputKind,
    ProviderRouter, RouteAttempt, RouteError, RouterPolicy, mock_provider_router_identity,
};

// ── Scripted provider helper ────────────────────────────────────────────────

#[derive(Debug)]
struct ScriptedProvider {
    identity: sddk_engine::ProviderIdentity,
    script: Mutex<Vec<ProviderOutcome>>,
    calls: AtomicU32,
}

#[derive(Debug, Clone)]
enum ProviderOutcome {
    Ok(ProviderOutput),
    Err(ProviderFailure),
}

impl ScriptedProvider {
    fn new(id: &str, script: Vec<ProviderOutcome>) -> Self {
        let identity = mock_provider_router_identity(id, ProviderKind::Mock);
        Self {
            identity,
            script: Mutex::new(script),
            calls: AtomicU32::new(0),
        }
    }

    fn call_count(&self) -> u32 {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Provider for ScriptedProvider {
    fn identity(&self) -> &sddk_engine::ProviderIdentity {
        &self.identity
    }
    fn execute(
        &self,
        _kind: sddk_engine::run_view::ActionKind,
        _cmd: &str,
    ) -> Result<ProviderOutput, ProviderError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let mut g = self.script.lock().expect("poisoned");
        if g.is_empty() {
            Ok(ProviderOutput {
                kind: ProviderOutputKind::Ok,
                wall_ms: 0,
                input_tokens: None,
                output_tokens: None,
                cost: None,
                currency: None,
            })
        } else {
            let outcome = g.remove(0);
            match outcome {
                ProviderOutcome::Ok(o) => Ok(o),
                ProviderOutcome::Err(f) => Err(ProviderError {
                    failure: f,
                    retries_used: 0,
                    last_attempt_at_ms: 0,
                }),
            }
        }
    }
}

use std::sync::Mutex;

fn clock() -> Arc<MockClock> {
    Arc::new(MockClock::new(1_000))
}

fn policy() -> RouterPolicy {
    RouterPolicy::default()
}

fn make_router(
    providers: Vec<Arc<dyn Provider>>,
    clock: Arc<MockClock>,
) -> (
    ProviderRouter,
    Arc<InMemoryCircuitBreakerStore>,
    Arc<InMemoryTelemetrySink>,
) {
    let breaker = Arc::new(InMemoryCircuitBreakerStore::new());
    let telemetry = Arc::new(InMemoryTelemetrySink::new());
    let router = ProviderRouter::new(
        providers,
        breaker.clone(),
        telemetry.clone(),
        clock as Arc<dyn Clock>,
        policy(),
    );
    (router, breaker, telemetry)
}

fn kind() -> sddk_engine::run_view::ActionKind {
    sddk_engine::run_view::ActionKind::Retry
}

// ── Scenario 1 — Quota exhaustion triggers failover ────────────────────────

#[test]
fn scenario_1_quota_exhaustion_triggers_failover() {
    let primary = Arc::new(ScriptedProvider::new(
        "primary",
        vec![ProviderOutcome::Err(ProviderFailure::QuotaExhausted)],
    ));
    let secondary = Arc::new(ScriptedProvider::new(
        "secondary",
        vec![ProviderOutcome::Ok(ProviderOutput {
            kind: ProviderOutputKind::Ok,
            wall_ms: 50,
            input_tokens: Some(10),
            output_tokens: Some(20),
            cost: None,
            currency: None,
        })],
    ));
    let (router, breaker, _tel) = make_router(vec![primary.clone(), secondary.clone()], clock());
    let out = router.route(kind(), "do_thing").expect("route ok");
    assert_eq!(out.output_tokens, Some(20));
    assert_eq!(
        breaker.state(primary.identity()),
        sddk_engine::CircuitState::Open,
        "primary must be tripped after QuotaExhausted"
    );
    assert_eq!(
        breaker.state(secondary.identity()),
        sddk_engine::CircuitState::Closed,
        "secondary must remain Closed"
    );
    assert_eq!(primary.call_count(), 1);
    assert_eq!(secondary.call_count(), 1);
}

// ── Scenario 2 — Transient timeout retries then succeeds ────────────────────

#[test]
fn scenario_2_timeout_retries_then_succeeds() {
    let primary = Arc::new(ScriptedProvider::new(
        "primary",
        vec![
            ProviderOutcome::Err(ProviderFailure::Timeout { elapsed_ms: 30_000 }),
            ProviderOutcome::Ok(ProviderOutput {
                kind: ProviderOutputKind::Ok,
                wall_ms: 10,
                input_tokens: None,
                output_tokens: None,
                cost: None,
                currency: None,
            }),
        ],
    ));
    let secondary = Arc::new(ScriptedProvider::new(
        "secondary",
        vec![ProviderOutcome::Ok(ProviderOutput {
            kind: ProviderOutputKind::Ok,
            wall_ms: 10,
            input_tokens: None,
            output_tokens: None,
            cost: None,
            currency: None,
        })],
    ));
    let (router, breaker, _tel) = make_router(vec![primary.clone(), secondary.clone()], clock());
    let _ = router.route(kind(), "c").expect("ok");
    assert_eq!(primary.call_count(), 2);
    assert_eq!(secondary.call_count(), 0);
    assert_eq!(
        breaker.state(primary.identity()),
        sddk_engine::CircuitState::Closed,
        "Timeout retry success closes the breaker"
    );
}

// ── Scenario 3 — All providers fail ─────────────────────────────────────────

#[test]
fn scenario_3_all_providers_fail() {
    let primary = Arc::new(ScriptedProvider::new(
        "primary",
        vec![ProviderOutcome::Err(ProviderFailure::AuthenticationFailed)],
    ));
    let secondary = Arc::new(ScriptedProvider::new(
        "secondary",
        vec![ProviderOutcome::Err(ProviderFailure::ModelUnavailable)],
    ));
    let (router, breaker, _tel) = make_router(vec![primary.clone(), secondary.clone()], clock());
    let err = router.route(kind(), "c").expect_err("must fail");
    match err {
        RouteError::AllProvidersFailed { attempts } => {
            assert_eq!(attempts.len(), 2);
            assert_eq!(attempts[0].identity.id, "primary");
            assert_eq!(attempts[1].identity.id, "secondary");
        }
        other => panic!("expected AllProvidersFailed, got {:?}", other),
    }
    assert_eq!(
        breaker.state(primary.identity()),
        sddk_engine::CircuitState::Disabled
    );
    assert_eq!(
        breaker.state(secondary.identity()),
        sddk_engine::CircuitState::Open
    );
}

// ── Scenario 4 — Open circuit is skipped ────────────────────────────────────

#[test]
fn scenario_4_open_circuit_is_skipped() {
    let breaker = Arc::new(InMemoryCircuitBreakerStore::new());
    let primary = Arc::new(ScriptedProvider::new(
        "primary",
        vec![ProviderOutcome::Err(ProviderFailure::QuotaExhausted)],
    ));
    let secondary = Arc::new(ScriptedProvider::new(
        "secondary",
        vec![ProviderOutcome::Ok(ProviderOutput {
            kind: ProviderOutputKind::Ok,
            wall_ms: 5,
            input_tokens: None,
            output_tokens: None,
            cost: None,
            currency: None,
        })],
    ));
    let telemetry = Arc::new(InMemoryTelemetrySink::new());
    let router = ProviderRouter::new(
        vec![primary.clone(), secondary.clone()],
        breaker.clone(),
        telemetry,
        clock(),
        policy(),
    );
    breaker.trip(primary.identity(), 1_000, 60_000);
    let _ = router.route(kind(), "c").expect("ok");
    assert_eq!(primary.call_count(), 0);
    assert_eq!(secondary.call_count(), 1);
}

// ── Scenario 5 — Cooldown transitions Open to HalfOpen ──────────────────────

#[test]
fn scenario_5_cooldown_transitions_open_to_halfopen() {
    use sddk_engine::refresh_open_to_half_open;
    let state = sddk_engine::CircuitState::Open;
    let opened_at = 1_000;
    let cooldown = 500;

    let s1 = refresh_open_to_half_open(state, opened_at, cooldown, 1_400);
    assert_eq!(s1, sddk_engine::CircuitState::Open);

    let s2 = refresh_open_to_half_open(state, opened_at, cooldown, 1_500);
    assert_eq!(s2, sddk_engine::CircuitState::HalfOpen);

    let s3 = refresh_open_to_half_open(
        sddk_engine::CircuitState::Closed,
        opened_at,
        cooldown,
        99_999,
    );
    assert_eq!(s3, sddk_engine::CircuitState::Closed);
}

// ── Scenario 6 — No eligible provider ──────────────────────────────────────

#[test]
fn scenario_6_no_eligible_provider() {
    let (router, _b, _t) = make_router(vec![], clock());
    let err = router.route(kind(), "c").expect_err("must fail");
    assert!(matches!(err, RouteError::NoEligibleProvider { .. }));
}

// ── Scenario 7 — Disabled route returns Disabled ─────────────────────────────

#[test]
fn scenario_7_disabled_route_returns_disabled() {
    let breaker = Arc::new(InMemoryCircuitBreakerStore::new());
    let primary = Arc::new(ScriptedProvider::new(
        "primary",
        vec![ProviderOutcome::Err(ProviderFailure::AuthenticationFailed)],
    ));
    let telemetry = Arc::new(InMemoryTelemetrySink::new());
    let router = ProviderRouter::new(
        vec![primary.clone()],
        breaker.clone(),
        telemetry,
        clock(),
        policy(),
    );
    breaker.disable(primary.identity(), 1_000);
    let err = router.route(kind(), "c").expect_err("must fail");
    assert!(matches!(err, RouteError::Disabled { .. }));
    assert_eq!(primary.call_count(), 0);
}

// ── Scenario 8 — Telemetry records both attempts and failovers ───────────────

#[test]
fn scenario_8_telemetry_records_both_attempts() {
    let primary = Arc::new(ScriptedProvider::new(
        "primary",
        vec![ProviderOutcome::Err(ProviderFailure::QuotaExhausted)],
    ));
    let secondary = Arc::new(ScriptedProvider::new(
        "secondary",
        vec![ProviderOutcome::Ok(ProviderOutput {
            kind: ProviderOutputKind::Ok,
            wall_ms: 50,
            input_tokens: None,
            output_tokens: Some(5),
            cost: None,
            currency: None,
        })],
    ));
    let (router, _b, tel) = make_router(vec![primary.clone(), secondary.clone()], clock());
    let _ = router.route(kind(), "c").expect("ok");
    let records = tel.records();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].identity.id, "primary");
    assert_eq!(
        records[0].result,
        sddk_engine::ProviderResult::TerminalFailure
    );
    assert_eq!(records[1].identity.id, "secondary");
    assert_eq!(records[1].result, sddk_engine::ProviderResult::Ok);
    assert_eq!(records[1].failovers, 1);
}

// ── Scenario 9 — Deterministic ordering ─────────────────────────────────────

#[test]
fn scenario_9_deterministic_ordering() {
    let mk = |id: &str| -> Arc<ScriptedProvider> {
        Arc::new(ScriptedProvider::new(
            id,
            vec![ProviderOutcome::Ok(ProviderOutput {
                kind: ProviderOutputKind::Ok,
                wall_ms: 0,
                input_tokens: None,
                output_tokens: None,
                cost: None,
                currency: None,
            })],
        ))
    };
    let p1 = mk("p1");
    let p2 = mk("p2");
    let p3 = mk("p3");
    let (router, _b, _t) = make_router(vec![p1.clone(), p2.clone(), p3.clone()], clock());
    let _ = router.route(kind(), "c").expect("ok");
    assert_eq!(p1.call_count(), 1);
    assert_eq!(p2.call_count(), 0);
    assert_eq!(p3.call_count(), 0);
}

// ── Scenario 10 — Lease-bound routing via AgentHost::execute_through_router ──

#[test]
fn scenario_10_decision_receipt_carries_provider_route() {
    // Just verifies DecisionReceipt carries provider_route and is None
    // when not used by execute_decision.
    use sddk_engine::DecisionReceipt;
    let receipt: DecisionReceipt = serde_json::from_value(serde_json::json!({
        "decision": {
            "verdict": "allow",
            "kind": "Retry",
            "reasons": [],
            "provenance": []
        },
        "fencing_token": 1,
        "executed_at_ms": 1000,
        "retries_used": 0,
        "agent_id": "test"
    }))
    .expect("deserialize receipt");
    assert!(receipt.provider_route.is_none());
}

#[allow(dead_code)]
fn _route_attempt_marker(_: RouteAttempt) {}
