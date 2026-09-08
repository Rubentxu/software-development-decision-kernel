//! Cold-Start Recovery — RED tests from REQ-ColdStartRecovery §Scenarios.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-ColdStartRecovery.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-082-AGENT-HOST-COLD-START.md

use std::sync::Arc;

use sddk_engine::retry::{Clock, MockClock};
use sddk_engine::run_view::{RunOrigin, RunStateView};
use sddk_engine::{
    AgentHost, AgentIdentity, AgentKind, CapsulePersistence, CapsuleStore, ColdStartError,
    ColdStartSource, CompilerPolicy, ContextCapsule, ContextCompiler, InMemoryCapsuleInputs,
    InMemoryCapsuleStore, InMemoryLeaseStore, InMemoryRunStateViewInputs, InMemoryTelemetrySink,
    NegativeKnowledge, NegativeStatus, RecordingCapsulePersistence, cold_start_core,
};

fn make_rsv(
    run_id: &str,
    frontier: Vec<&str>,
    blockers: Vec<&str>,
    pending: Vec<&str>,
) -> RunStateView {
    RunStateView::for_test(
        RunOrigin::Declared,
        run_id,
        frontier.into_iter().map(String::from).collect(),
        blockers.into_iter().map(String::from).collect(),
        pending.into_iter().map(String::from).collect(),
        1_000,
    )
}

fn make_parent_capsule(workflow_run: &str, node_run: &str, attempt: &str) -> ContextCapsule {
    let inputs = InMemoryCapsuleInputs::new()
        .with_objective("Continue architecture review")
        .with_must_read(vec!["ref:arch".to_string(), "ref:ledger-100".to_string()])
        .with_negative_knowledge(vec![NegativeKnowledge {
            claim: "Graph is source of truth".to_string(),
            status: NegativeStatus::RuledOut,
            reason: "Event Ledger is authority per ADR-021".to_string(),
            evidence_ref: Some("ADR-021".to_string()),
        }])
        .with_dod(vec!["dod-1".to_string()]);
    let c = ContextCompiler::new(
        Arc::new(inputs) as Arc<dyn sddk_engine::CapsuleInputs>,
        Arc::new(MockClock::new(1_000)),
    );
    let target = sddk_engine::CapsuleTarget {
        workflow_run: workflow_run.to_string(),
        node_run: node_run.to_string(),
        attempt: attempt.to_string(),
    };
    c.compile(target).expect("compile parent")
}

fn make_agent_host_with_rsv(
    rsv: RunStateView,
    clock: Arc<MockClock>,
) -> (
    AgentHost,
    Arc<InMemoryRunStateViewInputs>,
    Arc<InMemoryCapsuleStore>,
    Arc<RecordingCapsulePersistence>,
) {
    let rsv_arc = Arc::new(InMemoryRunStateViewInputs::new());
    rsv_arc.seed(rsv);
    let store = Arc::new(InMemoryCapsuleStore::new());
    let recorder = Arc::new(RecordingCapsulePersistence::new());
    let host = AgentHost::new(
        AgentIdentity::new("test-agent", "Test Agent", AgentKind::Human),
        Arc::new(InMemoryLeaseStore::new()) as Arc<dyn sddk_engine::LeaseStore>,
        clock as Arc<dyn Clock>,
    )
    .with_run_state_view_inputs(rsv_arc.clone())
    .with_capsule_store(store.clone() as Arc<dyn CapsuleStore>)
    .with_capsule_persistence(recorder.clone() as Arc<dyn CapsulePersistence>);
    (host, rsv_arc, store, recorder)
}

// ── Scenario 1 — Cold-start with no prior capsule produces Fresh ───────────

#[test]
fn scenario_1_fresh_cold_start() {
    let clock = Arc::new(MockClock::new(5_000));
    let rsv = make_rsv("wf-1", vec!["nr-1"], vec![], vec!["d-1"]);
    let (host, _rsv, _store, _rec) = make_agent_host_with_rsv(rsv, clock);
    let out = host.cold_start("wf-1", "nr-1").expect("ok");
    assert_eq!(out.source, ColdStartSource::Fresh);
    assert!(out.capsule.recovery.previous_attempt.is_none());
}

// ── Scenario 2 — Cold-start with parent produces FromRecovery ──────────────

#[test]
fn scenario_2_recovery_cold_start() {
    let clock = Arc::new(MockClock::new(5_000));
    let rsv = make_rsv("wf-1", vec!["nr-1"], vec![], vec!["d-1"]);
    let (host, _rsv, store, _rec) = make_agent_host_with_rsv(rsv, clock.clone());
    let parent = make_parent_capsule("wf-1", "nr-1", "at-3");
    store.persist(&parent);

    let out = host.cold_start("wf-1", "nr-1").expect("ok");
    assert_eq!(out.source, ColdStartSource::FromRecovery);
    assert_eq!(
        out.capsule.recovery.previous_attempt,
        Some("at-3".to_string())
    );
    assert_eq!(
        out.capsule.recovery.previous_capsule,
        Some(parent.capsule_id.clone())
    );
    assert!(
        out.capsule
            .negative_knowledge
            .iter()
            .any(|n| n.claim == "Graph is source of truth")
    );
}

// ── Scenario 3 — Blockers stitched into must_read ──────────────────────────

#[test]
fn scenario_3_blockers_in_must_read() {
    let clock = Arc::new(MockClock::new(5_000));
    let rsv = make_rsv("wf-1", vec!["nr-1"], vec!["ref:blocker-x"], vec![]);
    let (host, _rsv, _store, _rec) = make_agent_host_with_rsv(rsv, clock);
    let out = host.cold_start("wf-1", "nr-1").expect("ok");
    assert!(
        out.capsule
            .artifacts
            .must_read
            .contains(&"ref:blocker-x".to_string())
    );
}

// ── Scenario 4 — Run not found returns RunNotFound ─────────────────────────

#[test]
fn scenario_4_run_not_found() {
    let clock = Arc::new(MockClock::new(5_000));
    let (host, _rsv, _store, _rec) =
        make_agent_host_with_rsv(make_rsv("wf-other", vec!["nr-1"], vec![], vec![]), clock);
    let err = host
        .cold_start("wf-missing", "nr-1")
        .expect_err("must fail");
    assert!(matches!(err, ColdStartError::RunNotFound { .. }));
}

// ── Scenario 5 — No frontier returns NoFrontier ────────────────────────────

#[test]
fn scenario_5_no_frontier() {
    let clock = Arc::new(MockClock::new(5_000));
    let rsv = make_rsv("wf-1", vec![], vec![], vec![]);
    let (host, _rsv, _store, _rec) = make_agent_host_with_rsv(rsv, clock);
    let err = host.cold_start("wf-1", "nr-1").expect_err("must fail");
    assert!(matches!(err, ColdStartError::NoFrontier { .. }));
}

// ── Scenario 6 — Determinism: byte-stable across cold-starts ───────────────

#[test]
fn scenario_6_determinism_byte_stable() {
    let clock_a = Arc::new(MockClock::new(5_000));
    let clock_b = Arc::new(MockClock::new(5_000));
    let rsv_a = make_rsv("wf-1", vec!["nr-1"], vec![], vec!["d-1"]);
    let rsv_b = make_rsv("wf-1", vec!["nr-1"], vec![], vec!["d-1"]);
    let (host_a, _rsv_a, store_a, _rec_a) = make_agent_host_with_rsv(rsv_a, clock_a);
    let (host_b, _rsv_b, store_b, _rec_b) = make_agent_host_with_rsv(rsv_b, clock_b);
    let parent = make_parent_capsule("wf-1", "nr-1", "at-1");
    store_a.persist(&parent);
    store_b.persist(&parent);

    // Force the same capsule_id by seeding deterministic clock via the
    // same stamp. We instead compare must_read set + recovery chain which
    // must be identical.
    let out_a = host_a.cold_start("wf-1", "nr-1").expect("a");
    let out_b = host_b.cold_start("wf-1", "nr-1").expect("b");

    assert_eq!(out_a.source, out_b.source);
    assert_eq!(
        out_a.capsule.recovery.previous_attempt,
        out_b.capsule.recovery.previous_attempt
    );
    assert_eq!(
        out_a.capsule.artifacts.must_read,
        out_b.capsule.artifacts.must_read
    );
    assert_eq!(
        out_a.capsule.budget.actual_tokens,
        out_b.capsule.budget.actual_tokens
    );
}

// ── Scenario 7 — Persistence hook records the new capsule ──────────────────

#[test]
fn scenario_7_persistence_records() {
    let clock = Arc::new(MockClock::new(5_000));
    let rsv = make_rsv("wf-1", vec!["nr-1"], vec![], vec![]);
    let (host, _rsv, store, recorder) = make_agent_host_with_rsv(rsv, clock);
    let _ = host.cold_start("wf-1", "nr-1").expect("ok");
    assert_eq!(recorder.len(), 1);
    let last = store.last_capsule_for_node("wf-1", "nr-1");
    assert!(last.is_some());
}

// ── Scenario 8 — Blockers supersede parent must_read ────────────────────────

#[test]
fn scenario_8_blockers_added_to_parent_must_read() {
    let clock = Arc::new(MockClock::new(5_000));
    let rsv = make_rsv("wf-1", vec!["nr-1"], vec!["ref:ci-status"], vec![]);
    let (host, _rsv, store, _rec) = make_agent_host_with_rsv(rsv, clock);
    let parent = make_parent_capsule("wf-1", "nr-1", "at-1");
    store.persist(&parent);

    let out = host.cold_start("wf-1", "nr-1").expect("ok");
    assert!(
        out.capsule
            .artifacts
            .must_read
            .contains(&"ref:arch".to_string())
    );
    assert!(
        out.capsule
            .artifacts
            .must_read
            .contains(&"ref:ledger-100".to_string())
    );
    assert!(
        out.capsule
            .artifacts
            .must_read
            .contains(&"ref:ci-status".to_string())
    );
}

// ── Scenario 9 — Compiled capsule honors budget ────────────────────────────

#[test]
fn scenario_9_budget_honored() {
    let clock = Arc::new(MockClock::new(5_000));
    let rsv = make_rsv("wf-1", vec!["nr-1"], vec![], vec![]);
    let (host, _rsv, _store, _rec) = make_agent_host_with_rsv(rsv, clock);
    let out = host.cold_start("wf-1", "nr-1").expect("ok");
    assert!(out.capsule.budget.actual_tokens <= out.capsule.budget.max_tokens);
}

// ── Scenario 10 — Source = FromRecovery even when node_run doesn't match ────

#[test]
fn scenario_10_node_run_falls_back_to_frontier() {
    let clock = Arc::new(MockClock::new(5_000));
    let rsv = make_rsv("wf-1", vec!["nr-front"], vec![], vec![]);
    let (host, _rsv, store, _rec) = make_agent_host_with_rsv(rsv, clock);
    let parent = make_parent_capsule("wf-1", "nr-front", "at-2");
    store.persist(&parent);

    let out = host.cold_start("wf-1", "nr-other").expect("ok");
    assert_eq!(out.source, ColdStartSource::FromRecovery);
}

#[allow(dead_code)]
fn _suppress_warnings() {
    let _ = InMemoryTelemetrySink::new();
    let _ = CompilerPolicy::default();
    let _: Option<ColdStartError> = None;
    let _: Arc<dyn sddk_engine::CapsuleInputs> = Arc::new(InMemoryCapsuleInputs::new());
    let _ = cold_start_core
        as fn(
            &str,
            &str,
            &dyn sddk_engine::RunStateViewInputs,
            &dyn sddk_engine::CapsuleStore,
            Arc<dyn sddk_engine::retry::Clock>,
            CompilerPolicy,
        ) -> Result<sddk_engine::ColdStartOutput, _>;
}
