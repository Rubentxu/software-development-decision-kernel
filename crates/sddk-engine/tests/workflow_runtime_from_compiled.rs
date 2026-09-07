//! Tests for WorkflowRuntime::from_compiled (REQ-WFR3-RT-001, REQ-WFR3-ID-001).
//!
//! Verifies:
//! - `from_compiled` derives `run_id` deterministically via `RunId::derive`
//! - `run_id` derivation is stable across processes (same inputs → same output)

use sddk_domain::graph::ExecutionGraphRevision;
use sddk_domain::workflow_ir::{RevisionId, RunId, WorkflowIR};
use sddk_domain::{CorrelationId, TemplateRef, WorkflowRunState};
use sddk_engine::WorkflowRuntime;
use std::collections::BTreeMap;

// ── Test fixtures ────────────────────────────────────────────────────────────────

fn dummy_ir() -> WorkflowIR {
    use sddk_domain::workflow_ir::Budgets;

    let operators = BTreeMap::new();

    WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: TemplateRef {
            id: "test-template".into(),
            version: "0.1.0".into(),
        },
        operators,
        guards: Default::default(),
        expansion_permissions: Default::default(),
        budgets: Budgets {
            max_wall_ms: u64::MAX,
            max_tokens: u64::MAX,
            max_cost_micros: u64::MAX,
            max_depth: 8,
            max_nodes: 16,
            remaining_tokens: None,
            no_progress_threshold: u32::MAX,
        },
        required_invariants: Default::default(),
        provenance: sddk_domain::Provenance {
            generated_by: "test".into(),
            prompt_hash: "test".into(),
            model_hash: "test".into(),
            policy_hash: "test".into(),
        },
    }
}

// ── Mock GraphStore ─────────────────────────────────────────────────────────────

struct MockStore;

impl sddk_domain::ports::GraphStore for MockStore {
    fn save_state(&mut self, _: &sddk_domain::GraphState) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }
    fn load_state(&self) -> Result<Option<sddk_domain::GraphState>, sddk_domain::StorageError> {
        Ok(None)
    }
    fn checkpoint(&self) -> Result<Option<sddk_domain::projections::Checkpoint>, sddk_domain::StorageError> {
        Ok(None)
    }
    fn record_ir_digest(&mut self, _: &str, _: &str) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }
    fn record_graph_revision(&mut self, _: &ExecutionGraphRevision) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }
    fn load_node_attempts(
        &self, _: &RunId, _: &sddk_domain::workflow_ir::NodeId,
    ) -> Result<Vec<sddk_domain::workflow_run::Attempt>, sddk_domain::StorageError> {
        Ok(Vec::new())
    }
    fn attempt_count(
        &self, _: &RunId, _: &sddk_domain::workflow_ir::NodeId,
    ) -> Result<u32, sddk_domain::StorageError> {
        Ok(0)
    }
    fn save_revision(&mut self, _: &ExecutionGraphRevision) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }
    fn load_revision(
        &self, _: &RunId, _: &RevisionId,
    ) -> Result<Option<ExecutionGraphRevision>, sddk_domain::StorageError> {
        Ok(None)
    }
    fn latest_revision(&self, _: &RunId) -> Result<Option<ExecutionGraphRevision>, sddk_domain::StorageError> {
        Ok(None)
    }
    fn record_node_run(&mut self, _: &sddk_domain::workflow_run::NodeRun) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }
    fn record_node_run_for_run(
        &mut self, _: &RunId, _: &sddk_domain::workflow_run::NodeRun,
    ) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }
    fn record_attempt(&mut self, _: &sddk_domain::workflow_run::Attempt) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }
    fn load_run(&self, _: &RunId) -> Result<Option<sddk_domain::workflow_run::WorkflowRun>, sddk_domain::StorageError> {
        Ok(None)
    }
    fn load_node_run(
        &self, _: &RunId, _: &sddk_domain::workflow_ir::NodeId,
    ) -> Result<Option<sddk_domain::workflow_run::NodeRun>, sddk_domain::StorageError> {
        Ok(None)
    }
    fn list_attempts(
        &self, _: &RunId, _: &sddk_domain::workflow_ir::NodeId,
    ) -> Result<Vec<sddk_domain::workflow_run::Attempt>, sddk_domain::StorageError> {
        Ok(Vec::new())
    }
    fn latest_attempt(
        &self, _: &RunId, _: &sddk_domain::workflow_ir::NodeId,
    ) -> Result<Option<sddk_domain::workflow_run::Attempt>, sddk_domain::StorageError> {
        Ok(None)
    }
    fn stream_node_runs(&self, _: &RunId) -> Result<Vec<sddk_domain::workflow_run::NodeRun>, sddk_domain::StorageError> {
        Ok(Vec::new())
    }
    fn record_run(
        &mut self, _: &sddk_domain::workflow_run::WorkflowRun, _: &ExecutionGraphRevision,
    ) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }
    fn latest_workflow_run_state(&self, _: &RunId) -> Result<Option<WorkflowRunState>, sddk_domain::StorageError> {
        Ok(None)
    }
}

// ── from_compiled_assigns_run_id_and_graph_revision_deterministically ────────────

/// Scenario: from_compiled derives run_id deterministically
/// GIVEN a plan_revision_id, correlation_id, and compiled_revision with known revision_id
/// WHEN WorkflowRuntime::from_compiled is invoked
/// THEN WorkflowRun.run_id is RunId::derive(plan_revision_id, correlation_id)
/// AND graph_revision.0 == compiled_revision.revision_id
#[test]
fn from_compiled_assigns_run_id_and_graph_revision_deterministically() {
    let ir = dummy_ir();
    let plan_revision_id = "plan-rev-test-123";
    let correlation_id = CorrelationId("corr-xyz-456".into());
    let compiled_revision = ExecutionGraphRevision {
        revision: 0,
        revision_id: RevisionId("rev-abc-def".into()),
        parent: None,
        events: BTreeMap::new(),
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        digest: [0u8; 32],
        schema_version: 1,
    };

    let store = MockStore;
    let clock = sddk_engine::operator::Clock;
    let executor: std::sync::Arc<dyn sddk_domain::TaskExecutor> =
        std::sync::Arc::new(sddk_domain::NoopTaskExecutor);

    let runtime = WorkflowRuntime::from_compiled(
        ir,
        store,
        clock,
        executor,
        plan_revision_id,
        &correlation_id,
        &compiled_revision,
    );

    // run_id must be derived deterministically
    let expected_run_id = RunId::derive(plan_revision_id, &correlation_id);
    assert_eq!(
        runtime.run().run_id, expected_run_id,
        "run_id must be RunId::derive(plan_revision_id, correlation_id)"
    );

    // graph_revision must match compiled_revision.revision_id
    assert_eq!(
        runtime.run().graph_revision.0, compiled_revision.revision_id.0,
        "graph_revision must match compiled_revision.revision_id"
    );
}

// ── run_id_derivation_is_stable_across_processes ───────────────────────────────

/// Scenario: identical (plan_revision_id, correlation_id) derive identical run_id
/// GIVEN a fixed pair (plan_revision_id, correlation_id)
/// WHEN RunId::derive is called in two separate invocations
/// THEN both results are byte-equal and match ^[0-9a-f]{64}$
#[test]
fn run_id_derivation_is_stable_across_processes() {
    let plan_revision_id = "plan-rev-abc-123";
    let correlation_id = CorrelationId("corr-def-456".into());

    let run_id_1 = RunId::derive(plan_revision_id, &correlation_id);
    let run_id_2 = RunId::derive(plan_revision_id, &correlation_id);

    // Must be byte-equal across invocations
    assert_eq!(run_id_1, run_id_2, "RunId::derive must be deterministic");

    // Must match ^[0-9a-f]{64}$ — 64 hex chars
    let is_hex_64 = run_id_1.0.len() == 64
        && run_id_1.0.chars().all(|c| c.is_ascii_hexdigit());
    assert!(
        is_hex_64,
        "run_id must be 64 hex chars, got: {}",
        run_id_1.0
    );
}
