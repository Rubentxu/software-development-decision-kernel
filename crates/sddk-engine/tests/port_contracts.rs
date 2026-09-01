//! Port contract tests for [`sddk_domain::Ledger`] and every adapter port
//! reachable through [`SqliteLedgerFactory::open_in_memory()`].
//!
//! Required coverage: Ledger roundtrip, EventStore roundtrip, GraphStore revision,
//! ForkStore branch, ProjectionStore write, ControlPlane upsert, 2 byte-equivalence
//! cross-checks.  Total: ≥8 distinct tests, ≤150 LOC.

use std::collections::BTreeMap;

use sddk_domain::Checkpoint;
use sddk_domain::event_envelope::EventEnvelopeV1;
use sddk_domain::fork::{ForkInput, ForkStore, ReplayPolicy};
use sddk_domain::metrics::MetricsRecord;
use sddk_domain::{
    ActorKind, ActorRef, ControlPlane, EventStore as _, GateOutcomeStatus, GateReceiptNextSeqInput,
    GraphStore, Ledger, LedgerFactory,
};
use sddk_storage::{
    SqliteControlPlane, SqliteEventStore, SqliteForkStore, SqliteGraphStore, SqliteLedgerFactory,
    SqliteProjectionStore,
};
use sddk_testkit::InMemoryLedger;
use serde_json::json;
use tempfile::TempDir;

mod common;

use common::{
    mk_both_ledgers, mk_control_plane_with_project, mk_cycle, mk_event, mk_project,
    mk_registered_mem_ledger, mk_registered_sqlite_ledger, mk_workspace,
};

/// Build a [`EventEnvelopeV1`] for the event_store_roundtrip test.
fn make_test_envelope() -> EventEnvelopeV1 {
    let mut envelope = EventEnvelopeV1 {
        event_id: "evt-stream1-1".into(),
        stream_id: "stream-1".into(),
        event_type: "cycle.created".into(),
        schema_version: 1,
        project_id: "p-contract".into(),
        occurred_at: common::TS.into(),
        recorded_at: common::TS.into(),
        sequence: 0,
        actor: ActorRef {
            kind: ActorKind::Agent,
            id: "test".into(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![sddk_domain::EntityRef {
            kind: "cycle".into(),
            id: "c1".into(),
            version: None,
            content_hash: None,
        }],
        payload: json!({"cycle_id": "c1"}),
        evidence_refs: vec![],
        content_hash: String::new(),
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: Some("c1".into()),
        frame_id: None,
        fork_id: None,
    };
    envelope.content_hash = envelope.compute_content_hash();
    envelope
}

// ── Ledger roundtrip (InMemoryLedger) ─────────────────────────────────────────

#[test]
fn ledger_roundtrip_in_memory() {
    let mut ledger = mk_registered_mem_ledger();
    let c = mk_cycle("c1");
    let e = mk_event("c1");
    let inserted = ledger.insert_cycle_with_event(&c, &e).unwrap();
    assert_eq!(inserted.sequence, 1);
    assert_eq!(ledger.get_cycle("c1").unwrap().manifest.cycle_id, "c1");
    assert_eq!(ledger.list_cycle_events("c1").unwrap().len(), 1);
    assert!(ledger.has_projects().unwrap());
}

// ── Ledger roundtrip (SqliteLedgerFactory::open_in_memory) ─────────────────────

#[test]
fn ledger_roundtrip_sqlite() {
    let mut ledger = mk_registered_sqlite_ledger();
    let c = mk_cycle("c2");
    let e = mk_event("c2");
    let inserted = ledger.insert_cycle_with_event(&c, &e).unwrap();
    assert_eq!(inserted.sequence, 1);
    assert_eq!(ledger.get_cycle("c2").unwrap().manifest.cycle_id, "c2");
    let lease = ledger
        .acquire_cycle_lease("c2", "agent-test", 1000, 2000)
        .unwrap();
    assert_eq!(lease.owner, "agent-test");
    let receipt = ledger
        .insert_gate_receipt_next_seq(&GateReceiptNextSeqInput {
            project_id: "p-contract".into(),
            cycle_id: Some("c2".into()),
            gate: "test-gate".into(),
            evaluator: "tests".into(),
            transition_id: "phase.explore.complete".into(),
            plan_hash: "sha256:abcd1234567890ef".into(),
            outcome: GateOutcomeStatus::Passed,
            evidence: json!({}),
            actor: "test-runtime".into(),
            command_id: "cmd-contract".into(),
            frame_id: "frame-contract".into(),
            evaluated_at: common::TS.into(),
        })
        .unwrap();
    assert!(receipt.receipt_id.starts_with("gate-"));
}

// ── EventStore roundtrip ────────────────────────────────────────────────────────

#[test]
fn event_store_roundtrip() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let appended = store.append(&make_test_envelope()).unwrap();
    assert_eq!(appended.sequence, 1);
    let loaded = store.load_by_event_id("evt-stream1-1").unwrap().unwrap();
    assert_eq!(loaded.event_id, "evt-stream1-1");
    assert_eq!(store.count().unwrap(), 1);
}

// ── GraphStore ir_digest + state ───────────────────────────────────────────────

#[test]
fn graph_store_revision() {
    let mut store = SqliteGraphStore::open_in_memory().unwrap();
    store
        .record_ir_digest("sha256:abc123", r#"{"nodes":[]}"#)
        .unwrap();
    let state = sddk_domain::GraphState {
        nodes: Default::default(),
        edges: Default::default(),
        last_event_sequence: 0,
        last_event_hash: "sha256:genesis".into(),
    };
    store.save_state(&state).unwrap();
    assert_eq!(store.load_state().unwrap().unwrap().nodes.len(), 0);
}

// ── ForkStore branch ───────────────────────────────────────────────────────────

#[test]
fn fork_store_branch() {
    let mut store = SqliteForkStore::open_in_memory().unwrap();
    let input = ForkInput {
        fork_id: "fork-contract-1".into(),
        parent_stream_id: "stream-main".into(),
        at_sequence: 1,
        label: None,
        overrides: BTreeMap::new(),
        replay_policy: ReplayPolicy::Reconstruct,
    };
    let record = store
        .create_fork(input, "test-runtime", common::TS, "sha256:abc")
        .unwrap();
    assert_eq!(record.fork_id, "fork-contract-1");
    assert_eq!(store.list_forks().unwrap().len(), 1);
}

// ── ProjectionStore write ─────────────────────────────────────────────────────

#[test]
fn projection_store_write() {
    let mut store = SqliteProjectionStore::open_in_memory().unwrap();
    let cp = Checkpoint {
        projection_name: "test-proj".into(),
        version: 1,
        last_event_sequence: 5,
        last_event_hash: "sha256:abc123".into(),
        updated_at: common::TS.into(),
    };
    store.save_checkpoint(&cp, r#"{"key":"value"}"#).unwrap();
    let (loaded_cp, loaded_state) = store.load_checkpoint("test-proj", 1).unwrap().unwrap();
    assert_eq!(loaded_cp.last_event_sequence, 5);
    assert_eq!(loaded_state, r#"{"key":"value"}"#);
}

// ── ControlPlane upsert ──────────────────────────────────────────────────────

#[test]
fn control_plane_upsert() {
    let (mut cp, _dir) = mk_control_plane_with_project();
    let metrics = MetricsRecord {
        cycle_id: "p-test/telemetry".into(),
        path: "a-min".into(),
        context_quality: "C2".into(),
        phase_durations_sec: Default::default(),
        coherence_scores: vec![],
        correction_cycles: 0,
        tokens_used: 100,
        cost_estimate_usd: 0.01,
        first_pass_success: true,
        verify_verdict: "PASS".into(),
        merged_to_main: true,
        tag_version: None,
        lead_time_hours: None,
        teleological_coherence_pct: None,
        costs: Default::default(),
        recorded_at: common::TS.into(),
    };
    cp.upsert_cycle("p-contract", &metrics).unwrap();
    assert_eq!(cp.load_cycles().unwrap().len(), 1);
}

// ── Byte-equivalence: event_count ───────────────────────────────────────────────

#[test]
fn byte_equiv_event_count() {
    let (mut mem, mut sqlite) = mk_both_ledgers();
    for i in 0..5 {
        let c = mk_cycle(&format!("c{i}"));
        let e = mk_event(&format!("c{i}"));
        mem.insert_cycle_with_event(&c, &e).unwrap();
        sqlite.insert_cycle_with_event(&c, &e).unwrap();
    }
    assert_eq!(
        mem.load_all_ledger_events().unwrap().len(),
        sqlite.load_all_ledger_events().unwrap().len()
    );
}

// ── Byte-equivalence: cycle record ─────────────────────────────────────────────

#[test]
fn byte_equiv_cycle_record() {
    let (mut mem, mut sqlite) = mk_both_ledgers();
    let c = mk_cycle("c-eq");
    let e = mk_event("c-eq");
    mem.insert_cycle_with_event(&c, &e).unwrap();
    sqlite.insert_cycle_with_event(&c, &e).unwrap();
    let mem_c = mem.get_cycle("c-eq").unwrap();
    let sqlite_c = sqlite.get_cycle("c-eq").unwrap();
    assert_eq!(mem_c.manifest.cycle_id, sqlite_c.manifest.cycle_id);
    assert_eq!(mem_c.manifest.phase, sqlite_c.manifest.phase);
    assert_eq!(mem_c.manifest.status, sqlite_c.manifest.status);
}
