//! DM-02: Budget enforcement — wall-clock budget fires BudgetExceeded.
//!
//! Verifies REQ-WF-RT-017: when wall-clock budget is exhausted at a tick
//! boundary, execute() returns `Err(RuntimeError::BudgetExceeded)` with the
//! elapsed and max values captured.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use sddk_domain::{
    CapabilityId, EventAppended, EventEnvelopeV1, EventStore, GraphStore, NoopTaskExecutor,
    Operator, OperatorId, WorkflowIR, WorkflowRunState,
};
use sddk_engine::RuntimeError;
use sddk_engine::operator::Clock;
use sddk_engine::workflow_runtime::WorkflowRuntime;

// ── Spy event store ────────────────────────────────────────────────────────────

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

    fn load_by_sequence(
        &self,
        _stream_id: &str,
        _sequence: u64,
    ) -> Result<Option<EventEnvelopeV1>, sddk_domain::StorageError> {
        Ok(None)
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
}

// ── Mock GraphStore ────────────────────────────────────────────────────────────

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
        _node_id: &sddk_domain::NodeId,
    ) -> Result<Vec<sddk_domain::workflow_run::Attempt>, sddk_domain::StorageError> {
        Ok(Vec::new())
    }

    fn attempt_count(
        &self,
        _run_id: &sddk_domain::workflow_ir::RunId,
        _node_id: &sddk_domain::NodeId,
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
}

// ── Minimal single-task IR with 1ms wall budget ───────────────────────────────

/// Builds a minimal single-task IR with the given wall-clock budget in ms.
fn build_tiny_ir(max_wall_ms: u64) -> WorkflowIR {
    let op = Operator::Task {
        capability: CapabilityId("noop".into()),
        inputs: Default::default(),
    };
    let mut operators = BTreeMap::new();
    operators.insert(OperatorId("task".into()), op);

    WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: sddk_domain::TemplateRef {
            id: "sddk-budget-test".into(),
            version: "1.0.0".into(),
        },
        operators,
        guards: Default::default(),
        expansion_permissions: Default::default(),
        budgets: sddk_domain::Budgets {
            max_wall_ms,
            max_tokens: u64::MAX,
            max_cost_micros: u64::MAX,
            max_depth: u64::MAX,
            max_nodes: 100,
            remaining_tokens: None,
            no_progress_threshold: u32::MAX,
        },
        required_invariants: Default::default(),
        provenance: sddk_domain::Provenance {
            generated_by: "sddk-budget-test".into(),
            prompt_hash: "canonical-min-sequence".into(),
            model_hash: "test".into(),
            policy_hash: "test".into(),
        },
    }
}

// ── Test ──────────────────────────────────────────────────────────────────────

#[test]
fn dm02_budget_exceeded_fires_when_wall_clock_expires() {
    let events = Arc::new(Mutex::new(Vec::new()));
    #[allow(clippy::arc_with_non_send_sync)] // ADR-0064: test-only helper, single-thread usage
    let spy = Arc::new(Mutex::new(
        Box::new(SpyEventStore::new(events.clone())) as Box<dyn EventStore>
    ));
    let store = MockStore;
    let clock = Clock;
    // 0 ms budget — pre_tick() fires immediately on the first tick since
    // elapsed_ms (≥0) >= wall_budget_ms (0) is always true.
    let ir = build_tiny_ir(0);

    let event_store: Arc<Mutex<Box<dyn EventStore>>> = spy;
    let mut runtime = WorkflowRuntime::new_with_event_store(
        ir,
        store,
        clock,
        event_store,
        Arc::new(NoopTaskExecutor),
    );

    let result = runtime.execute();

    // BudgetExceeded must be returned, not Ok
    assert!(
        result.is_err(),
        "execute() should return Err when wall budget is exceeded, got: {:?}",
        result
    );

    let err = result.unwrap_err();
    let is_budget_exceeded = matches!(err, RuntimeError::BudgetExceeded { .. });
    assert!(
        is_budget_exceeded,
        "expected RuntimeError::BudgetExceeded, got: {:?}",
        err
    );

    // Workflow state must still be Running (not Completed) since we exited via error
    assert_eq!(
        runtime.run().state,
        WorkflowRunState::Running,
        "workflow should still be Running after BudgetExceeded"
    );
}
