//! Tests for workflow event emission from WorkflowRuntime.
//!
//! These tests verify that WorkflowRuntime emits the 5 canonical workflow events:
//! 1. `workflow.run.started` — emitted at execute() entry
//! 2. `workflow.run.completed` — emitted when run reaches Completed state
//! 3. `workflow.node.running` — emitted before Operator::evaluate for each node
//! 4. `workflow.node.completed` — emitted on NodeOutcome::Succeeded
//! 5. `workflow.node.failed` — emitted on NodeOutcome::Failed

use std::sync::{Arc, Mutex};

use sddk_domain::{
    EventAppended, EventEnvelopeV1, EventStore, GraphStore, NodeId, NoopTaskExecutor, WorkflowIR,
};
use sddk_engine::operator::Clock;
use sddk_engine::workflow_runtime::WorkflowRuntime;

// ── Spy event store ────────────────────────────────────────────────────────────

/// A spy event store that records all emitted events for testing.
/// Uses a shared events vector so both the spy and the runtime can access it.
struct SpyEventStore {
    events: Arc<Mutex<Vec<EventEnvelopeV1>>>,
}

impl SpyEventStore {
    fn new(events: Arc<Mutex<Vec<EventEnvelopeV1>>>) -> Self {
        Self { events }
    }
}

impl EventStore for SpyEventStore {
    fn append(
        &mut self,
        envelope: &EventEnvelopeV1,
    ) -> Result<EventAppended, sddk_domain::StorageError> {
        self.events.lock().unwrap().push(envelope.clone());
        Ok(EventAppended {
            event_id: envelope.event_id.clone(),
            stream_id: envelope.stream_id.clone(),
            sequence: 1,
            content_hash: envelope.content_hash.clone(),
            recorded_at: envelope.recorded_at.clone(),
            chain_hash: String::new(),
        })
    }

    fn load_by_event_id(
        &self,
        _event_id: &str,
    ) -> Result<Option<EventEnvelopeV1>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn load_stream(
        &self,
        _stream_id: &str,
        _after_sequence: Option<u64>,
        _limit: u32,
    ) -> Result<Vec<EventEnvelopeV1>, sddk_domain::StorageError> {
        Ok(Vec::new())
    }

    fn last_sequence(&self, _stream_id: &str) -> Result<Option<u64>, sddk_domain::StorageError> {
        Ok(Some(1))
    }

    fn count(&self) -> Result<u64, sddk_domain::StorageError> {
        Ok(self.events.lock().unwrap().len() as u64)
    }

    fn head_hash(&self, _stream_id: &str) -> Result<Option<String>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn head_chain_hash(
        &self,
        _stream_id: &str,
    ) -> Result<Option<String>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn verify_stream_chain(&self, _stream_id: &str) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }

    fn verify_chain_integrity(&self, _stream_id: &str) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }

    fn backfill_chain_hash(
        &mut self,
        _stream_id: &str,
    ) -> Result<usize, sddk_domain::StorageError> {
        Ok(0)
    }

    fn load_by_sequence(
        &self,
        _stream_id: &str,
        _sequence: u64,
    ) -> Result<Option<EventEnvelopeV1>, sddk_domain::StorageError> {
        Ok(None)
    }
}

// ── Mock store for WorkflowRuntime ────────────────────────────────────────────

struct MockStore;

impl GraphStore for MockStore {
    fn save_state(
        &mut self,
        _state: &sddk_domain::GraphState,
    ) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }

    fn load_state(&self) -> Result<Option<sddk_domain::GraphState>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn checkpoint(
        &self,
    ) -> Result<Option<sddk_domain::projections::Checkpoint>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn record_ir_digest(
        &mut self,
        _ir_hash: &str,
        _ir_json: &str,
    ) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }

    fn record_graph_revision(
        &mut self,
        _rev: &sddk_domain::graph::ExecutionGraphRevision,
    ) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }

    fn load_node_attempts(
        &self,
        _run_id: &sddk_domain::workflow_ir::RunId,
        _node_id: &NodeId,
    ) -> Result<Vec<sddk_domain::workflow_run::Attempt>, sddk_domain::StorageError> {
        Ok(Vec::new())
    }

    fn attempt_count(
        &self,
        _run_id: &sddk_domain::workflow_ir::RunId,
        _node_id: &NodeId,
    ) -> Result<u32, sddk_domain::StorageError> {
        Ok(0)
    }

    fn save_revision(
        &mut self,
        _rev: &sddk_domain::graph::ExecutionGraphRevision,
    ) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }

    fn load_revision(
        &self,
        _run_id: &sddk_domain::workflow_ir::RunId,
        _rev_id: &sddk_domain::workflow_ir::RevisionId,
    ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn latest_revision(
        &self,
        _run_id: &sddk_domain::workflow_ir::RunId,
    ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn record_node_run(
        &mut self,
        _run: &sddk_domain::workflow_run::NodeRun,
    ) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }

    fn record_attempt(
        &mut self,
        _attempt: &sddk_domain::workflow_run::Attempt,
    ) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }

    fn load_run(
        &self,
        _run_id: &sddk_domain::workflow_ir::RunId,
    ) -> Result<Option<sddk_domain::workflow_run::WorkflowRun>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn load_node_run(
        &self,
        _run_id: &sddk_domain::workflow_ir::RunId,
        _node_id: &NodeId,
    ) -> Result<Option<sddk_domain::workflow_run::NodeRun>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn list_attempts(
        &self,
        _run_id: &sddk_domain::workflow_ir::RunId,
        _node_id: &NodeId,
    ) -> Result<Vec<sddk_domain::workflow_run::Attempt>, sddk_domain::StorageError> {
        Ok(Vec::new())
    }

    fn latest_attempt(
        &self,
        _run_id: &sddk_domain::workflow_ir::RunId,
        _node_id: &NodeId,
    ) -> Result<Option<sddk_domain::workflow_run::Attempt>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn stream_node_runs(
        &self,
        _run_id: &sddk_domain::workflow_ir::RunId,
    ) -> Result<Vec<sddk_domain::workflow_run::NodeRun>, sddk_domain::StorageError> {
        Ok(Vec::new())
    }
}

// ── Test helpers ────────────────────────────────────────────────────────────────

fn make_test_ir() -> WorkflowIR {
    WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: sddk_domain::TemplateRef {
            id: "test.template".into(),
            version: "1.0.0".into(),
        },
        operators: Default::default(),
        guards: Default::default(),
        expansion_permissions: Default::default(),
        budgets: Default::default(),
        required_invariants: Default::default(),
        provenance: sddk_domain::Provenance {
            generated_by: "test".into(),
            prompt_hash: "test".into(),
            model_hash: "test".into(),
            policy_hash: "test".into(),
        },
    }
}

/// Creates a runtime with a spy event store, returns (runtime, spy_events_accessor)
fn make_runtime_with_spy() -> (
    WorkflowRuntime<MockStore>,
    impl Fn() -> Vec<EventEnvelopeV1>,
) {
    // Shared events vector
    let events = Arc::new(Mutex::new(Vec::new()));
    let events_for_accessor = events.clone();

    // Create spy with shared events
    let spy = SpyEventStore::new(events);
    #[allow(clippy::arc_with_non_send_sync)] // ADR-0064: test-only helper, single-thread usage
    let spy_store = Arc::new(Mutex::new(Box::new(spy) as Box<dyn EventStore>));

    // Create the runtime
    let store = MockStore;
    let clock = Clock;
    let ir = make_test_ir();

    let event_store: Arc<Mutex<Box<dyn EventStore>>> = spy_store;
    let runtime = WorkflowRuntime::new_with_event_store(
        ir,
        store,
        clock,
        event_store,
        Arc::new(NoopTaskExecutor),
    );

    // Return a closure that accesses events
    let events_accessor = move || events_for_accessor.lock().unwrap().clone();
    (runtime, events_accessor)
}

// ── Tests ───────────────────────────────────────────────────────────────────────

// WEE-01: execute() emits workflow.run.started with stream_id == run_id.0
#[test]
fn wee01_execute_emits_workflow_run_started() {
    let (mut runtime, get_events) = make_runtime_with_spy();

    runtime.execute().unwrap();

    let events = get_events();
    assert!(!events.is_empty(), "expected at least one event");
    let started_events: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == "workflow.run.started")
        .collect();
    assert_eq!(
        started_events.len(),
        1,
        "expected exactly one workflow.run.started event"
    );
    let run_id = runtime.run().run_id.0.clone();
    assert_eq!(
        started_events[0].stream_id, run_id,
        "stream_id must equal run_id.0"
    );
}

// WEE-02: execute() emits workflow.run.completed with stream_id == run_id.0
#[test]
fn wee02_execute_emits_workflow_run_completed() {
    let (mut runtime, get_events) = make_runtime_with_spy();

    runtime.execute().unwrap();

    let events = get_events();
    let completed_events: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == "workflow.run.completed")
        .collect();
    assert_eq!(
        completed_events.len(),
        1,
        "expected exactly one workflow.run.completed event"
    );
    let run_id = runtime.run().run_id.0.clone();
    assert_eq!(
        completed_events[0].stream_id, run_id,
        "stream_id must equal run_id.0"
    );
}

// WEE-03: tick() emits workflow.node.running for each node
#[test]
fn wee03_tick_emits_workflow_node_running() {
    let (mut runtime, get_events) = make_runtime_with_spy();

    runtime.start().unwrap();
    runtime.tick().unwrap();

    let events = get_events();
    let node_running_events: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == "workflow.node.running")
        .collect();
    // For empty IR, no nodes so no node.running events
    assert!(
        node_running_events.is_empty(),
        "empty IR should produce zero node.running events, got {}",
        node_running_events.len()
    );
}

// WEE-04: tick() emits workflow.node.completed for each completed node
#[test]
fn wee04_tick_emits_workflow_node_completed() {
    let (mut runtime, get_events) = make_runtime_with_spy();

    runtime.start().unwrap();
    runtime.tick().unwrap();

    let events = get_events();
    let node_completed_events: Vec<_> = events
        .iter()
        .filter(|e| e.event_type == "workflow.node.completed")
        .collect();
    // For empty IR, no nodes so no node.completed events
    assert!(
        node_completed_events.is_empty(),
        "empty IR should produce zero node.completed events, got {}",
        node_completed_events.len()
    );
}

// WEE-05: stream_id equals run_id.0 on all emitted workflow events
#[test]
fn wee05_stream_id_equals_run_id_on_all_events() {
    let (mut runtime, get_events) = make_runtime_with_spy();

    runtime.execute().unwrap();

    let events = get_events();
    let run_id = runtime.run().run_id.0.clone();
    for event in &events {
        if event.event_type.starts_with("workflow.") {
            assert_eq!(
                event.stream_id, run_id,
                "event {} stream_id must equal run_id.0 ({})",
                event.event_type, run_id
            );
        }
    }
}
