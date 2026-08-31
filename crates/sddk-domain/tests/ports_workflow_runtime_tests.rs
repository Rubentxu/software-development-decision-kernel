//! Tests verifying that GraphStore port methods have default-implemented
//! bodies that return StorageError::Other (as NotImplemented) for adapters
//! that haven't implemented them yet.

use sddk_domain::{GraphStore, NodeId, RunId, StorageError};

/// Verifies that record_node_run has a default implementation.
#[test]
fn record_node_run_default_exists() {
    // This test verifies the trait method exists and has a default body.
    let mut store = MockGraphStore::new();
    let node_run = sddk_domain::NodeRun {
        node_id: NodeId("node-1".into()),
        state: sddk_domain::NodeRunState::Pending,
        dependencies: Default::default(),
        attempts: vec![],
        expansion_permissions: Default::default(),
        schema_version: 1,
    };
    let result = store.record_node_run(&node_run);
    assert!(
        result.is_err(),
        "Expected error for unimplemented method, got: {:?}",
        result
    );
}

/// Verifies that record_attempt has a default implementation.
#[test]
fn record_attempt_default_exists() {
    let mut store = MockGraphStore::new();
    let attempt = sddk_domain::Attempt {
        attempt_id: sddk_domain::AttemptId("attempt-1".into()),
        node_id: NodeId("node-1".into()),
        route: sddk_domain::Route {
            provider: "test".into(),
            model: "test".into(),
            host: "test".into(),
        },
        started_at: "2026-08-23T00:00:00Z".into(),
        ended_at: None,
        outcome: None,
        usage: sddk_domain::Usage {
            tokens_in: 0,
            tokens_out: 0,
            cost_micros: 0,
            wall_ms: 0,
        },
        context_capsule: sddk_domain::ContextCapsuleRef::Pointer { cid: "test".into() },
        idempotency_key: sddk_domain::IdempotencyKey {
            project_id: "p-1".into(),
            run_id: RunId("run-1".into()),
            node_id: NodeId("node-1".into()),
            attempt_seq: 1,
        },
        schema_version: 1,
    };
    let result = store.record_attempt(&attempt);
    assert!(result.is_err(), "Expected error for unimplemented method");
}

/// Verifies that load_run has a default implementation.
#[test]
fn load_run_default_exists() {
    let store = MockGraphStore::new();
    let result = store.load_run(&RunId("run-1".into()));
    assert!(result.is_err(), "Expected error for unimplemented method");
}

/// Verifies that load_node_run has a default implementation.
#[test]
fn load_node_run_default_exists() {
    let store = MockGraphStore::new();
    let result = store.load_node_run(&RunId("run-1".into()), &NodeId("node-1".into()));
    assert!(result.is_err(), "Expected error for unimplemented method");
}

/// Verifies that list_attempts has a default implementation.
#[test]
fn list_attempts_default_exists() {
    let store = MockGraphStore::new();
    let result = store.list_attempts(&RunId("run-1".into()), &NodeId("node-1".into()));
    assert!(result.is_err(), "Expected error for unimplemented method");
}

/// Verifies that latest_attempt has a default implementation.
#[test]
fn latest_attempt_default_exists() {
    let store = MockGraphStore::new();
    let result = store.latest_attempt(&RunId("run-1".into()), &NodeId("node-1".into()));
    assert!(result.is_err(), "Expected error for unimplemented method");
}

/// Verifies that stream_node_runs has a default implementation.
#[test]
fn stream_node_runs_default_exists() {
    let store = MockGraphStore::new();
    let result = store.stream_node_runs(&RunId("run-1".into()));
    assert!(result.is_err(), "Expected error for unimplemented method");
}

// Minimal mock to satisfy the GraphStore trait bounds
struct MockGraphStore;

impl MockGraphStore {
    fn new() -> Self {
        Self
    }
}

impl GraphStore for MockGraphStore {
    fn save_state(&mut self, _state: &sddk_domain::GraphState) -> Result<(), StorageError> {
        Err(StorageError::Other("save_state not implemented".into()))
    }

    fn load_state(&self) -> Result<Option<sddk_domain::GraphState>, StorageError> {
        Err(StorageError::Other("load_state not implemented".into()))
    }

    fn checkpoint(&self) -> Result<Option<sddk_domain::Checkpoint>, StorageError> {
        Err(StorageError::Other("checkpoint not implemented".into()))
    }

    fn record_ir_digest(&mut self, _ir_hash: &str, _ir_json: &str) -> Result<(), StorageError> {
        Err(StorageError::Other(
            "record_ir_digest not implemented".into(),
        ))
    }

    fn record_graph_revision(
        &mut self,
        _rev: &sddk_domain::ExecutionGraphRevision,
    ) -> Result<(), StorageError> {
        Err(StorageError::Other(
            "record_graph_revision not implemented".into(),
        ))
    }

    fn load_node_attempts(
        &self,
        _run_id: &RunId,
        _node_id: &NodeId,
    ) -> Result<Vec<sddk_domain::Attempt>, StorageError> {
        Err(StorageError::Other(
            "load_node_attempts not implemented".into(),
        ))
    }

    fn attempt_count(&self, _run_id: &RunId, _node_id: &NodeId) -> Result<u32, StorageError> {
        Err(StorageError::Other("attempt_count not implemented".into()))
    }

    fn save_revision(
        &mut self,
        _rev: &sddk_domain::ExecutionGraphRevision,
    ) -> Result<(), StorageError> {
        Err(StorageError::Other("save_revision not implemented".into()))
    }

    fn load_revision(
        &self,
        _run_id: &RunId,
        _rev_id: &sddk_domain::RevisionId,
    ) -> Result<Option<sddk_domain::ExecutionGraphRevision>, StorageError> {
        Err(StorageError::Other("load_revision not implemented".into()))
    }

    fn latest_revision(
        &self,
        _run_id: &RunId,
    ) -> Result<Option<sddk_domain::ExecutionGraphRevision>, StorageError> {
        Err(StorageError::Other(
            "latest_revision not implemented".into(),
        ))
    }

    // The 7 new methods below use default implementations from the trait.
}
