//! Tests for WorkflowRuntime lifecycle — start → tick → complete transitions.
//!
//! These tests verify the WorkflowRuntime state machine:
//! - start() transitions from Pending to Running
//! - tick() evaluates ready nodes
//! - complete() transitions to Completed
//! - fail() transitions to Failed
//! - pause() / resume() / cancel() work correctly

use sddk_domain::{GraphStore, NoopTaskExecutor, WorkflowIR, WorkflowRun, WorkflowRunState};
use sddk_engine::operator::Clock;
use sddk_engine::workflow_runtime::WorkflowRuntime;
use std::collections::BTreeMap;
use std::sync::Arc;

// Minimal mock GraphStore for testing
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

    fn checkpoint(&self) -> Result<Option<sddk_domain::Checkpoint>, sddk_domain::StorageError> {
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
        _rev: &sddk_domain::ExecutionGraphRevision,
    ) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }

    fn load_node_attempts(
        &self,
        _run_id: &sddk_domain::RunId,
        _node_id: &sddk_domain::NodeId,
    ) -> Result<Vec<sddk_domain::Attempt>, sddk_domain::StorageError> {
        Ok(vec![])
    }

    fn attempt_count(
        &self,
        _run_id: &sddk_domain::RunId,
        _node_id: &sddk_domain::NodeId,
    ) -> Result<u32, sddk_domain::StorageError> {
        Ok(0)
    }

    fn save_revision(
        &mut self,
        _rev: &sddk_domain::ExecutionGraphRevision,
    ) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }

    fn load_revision(
        &self,
        _run_id: &sddk_domain::RunId,
        _rev_id: &sddk_domain::RevisionId,
    ) -> Result<Option<sddk_domain::ExecutionGraphRevision>, sddk_domain::StorageError> {
        Ok(None)
    }

    fn latest_revision(
        &self,
        _run_id: &sddk_domain::RunId,
    ) -> Result<Option<sddk_domain::ExecutionGraphRevision>, sddk_domain::StorageError> {
        Ok(None)
    }
}

/// Verifies that start() transitions WorkflowRun from Pending to Running.
#[test]
fn start_then_tick_then_complete_transitions_state() {
    // This test requires WorkflowRuntime to exist and have a working start/tick/complete
    // Currently this will fail to compile because WorkflowRuntime doesn't exist yet.
    // Once T-4 is implemented, this test should pass.
    let store = MockStore;
    let clock = Clock;

    // Create a minimal WorkflowIR
    let ir = WorkflowIR {
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
    };

    // Create a minimal WorkflowRun
    let _run = WorkflowRun {
        run_id: sddk_domain::RunId("run-1".into()),
        template_ref: ir.template_ref.clone(),
        ir_hash: "sha256:test".into(),
        graph_revision: sddk_domain::RevisionId("rev-1".into()),
        state: WorkflowRunState::Pending,
        inputs: Default::default(),
        outputs: None,
        correlation_id: sddk_domain::CorrelationId("corr-1".into()),
        budget: Default::default(),
        schema_version: 1,
    };

    let mut runtime = WorkflowRuntime::new(ir, store, clock, Arc::new(NoopTaskExecutor));
    // start() should transition from Pending to Running
    let result = runtime.start();
    assert!(result.is_ok(), "start() should succeed, got: {:?}", result);

    // After start, the run should be Running
    assert_eq!(runtime.run().state, WorkflowRunState::Running);
}

/// Verifies that complete() transitions WorkflowRun to Completed.
#[test]
fn complete_transitions_to_completed() {
    let store = MockStore;
    let clock = Clock;

    let ir = WorkflowIR {
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
    };

    let _run = WorkflowRun {
        run_id: sddk_domain::RunId("run-1".into()),
        template_ref: ir.template_ref.clone(),
        ir_hash: "sha256:test".into(),
        graph_revision: sddk_domain::RevisionId("rev-1".into()),
        state: WorkflowRunState::Pending,
        inputs: Default::default(),
        outputs: None,
        correlation_id: sddk_domain::CorrelationId("corr-1".into()),
        budget: Default::default(),
        schema_version: 1,
    };

    let mut runtime = WorkflowRuntime::new(ir, store, clock, Arc::new(NoopTaskExecutor));
    runtime.start().unwrap();

    let outputs: BTreeMap<String, serde_json::Value> = Default::default();
    let result = runtime.complete(outputs);
    assert!(
        result.is_ok(),
        "complete() should succeed, got: {:?}",
        result
    );
    assert_eq!(runtime.run().state, WorkflowRunState::Completed);
}

/// Verifies that fail() transitions WorkflowRun to Failed.
#[test]
fn fail_transitions_to_failed() {
    let store = MockStore;
    let clock = Clock;

    let ir = WorkflowIR {
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
    };

    let _run = WorkflowRun {
        run_id: sddk_domain::RunId("run-1".into()),
        template_ref: ir.template_ref.clone(),
        ir_hash: "sha256:test".into(),
        graph_revision: sddk_domain::RevisionId("rev-1".into()),
        state: WorkflowRunState::Pending,
        inputs: Default::default(),
        outputs: None,
        correlation_id: sddk_domain::CorrelationId("corr-1".into()),
        budget: Default::default(),
        schema_version: 1,
    };

    let mut runtime = WorkflowRuntime::new(ir, store, clock, Arc::new(NoopTaskExecutor));
    runtime.start().unwrap();

    let result = runtime.fail("test error".into());
    assert!(result.is_ok(), "fail() should succeed, got: {:?}", result);
    assert_eq!(runtime.run().state, WorkflowRunState::Failed);
}

/// Verifies that pause() / resume() work on Running state.
#[test]
fn pause_resume_transitions() {
    let store = MockStore;
    let clock = Clock;

    let ir = WorkflowIR {
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
    };

    let _run = WorkflowRun {
        run_id: sddk_domain::RunId("run-1".into()),
        template_ref: ir.template_ref.clone(),
        ir_hash: "sha256:test".into(),
        graph_revision: sddk_domain::RevisionId("rev-1".into()),
        state: WorkflowRunState::Pending,
        inputs: Default::default(),
        outputs: None,
        correlation_id: sddk_domain::CorrelationId("corr-1".into()),
        budget: Default::default(),
        schema_version: 1,
    };

    let mut runtime = WorkflowRuntime::new(ir, store, clock, Arc::new(NoopTaskExecutor));
    runtime.start().unwrap();

    let result = runtime.pause();
    assert!(result.is_ok(), "pause() should succeed");
    assert_eq!(runtime.run().state, WorkflowRunState::Paused);

    let result = runtime.resume();
    assert!(result.is_ok(), "resume() should succeed");
    assert_eq!(runtime.run().state, WorkflowRunState::Running);
}

/// Verifies that cancel() transitions to Cancelled.
#[test]
fn cancel_transitions_to_cancelled() {
    let store = MockStore;
    let clock = Clock;

    let ir = WorkflowIR {
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
    };

    let _run = WorkflowRun {
        run_id: sddk_domain::RunId("run-1".into()),
        template_ref: ir.template_ref.clone(),
        ir_hash: "sha256:test".into(),
        graph_revision: sddk_domain::RevisionId("rev-1".into()),
        state: WorkflowRunState::Pending,
        inputs: Default::default(),
        outputs: None,
        correlation_id: sddk_domain::CorrelationId("corr-1".into()),
        budget: Default::default(),
        schema_version: 1,
    };

    let mut runtime = WorkflowRuntime::new(ir, store, clock, Arc::new(NoopTaskExecutor));
    runtime.start().unwrap();

    let result = runtime.cancel();
    assert!(result.is_ok(), "cancel() should succeed");
    assert_eq!(runtime.run().state, WorkflowRunState::Cancelled);
}

/// Verifies that complete() on a terminal state returns AlreadyTerminal.
#[test]
fn complete_on_terminal_is_idempotent() {
    let store = MockStore;
    let clock = Clock;

    let ir = WorkflowIR {
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
    };

    let _run = WorkflowRun {
        run_id: sddk_domain::RunId("run-1".into()),
        template_ref: ir.template_ref.clone(),
        ir_hash: "sha256:test".into(),
        graph_revision: sddk_domain::RevisionId("rev-1".into()),
        state: WorkflowRunState::Pending,
        inputs: Default::default(),
        outputs: None,
        correlation_id: sddk_domain::CorrelationId("corr-1".into()),
        budget: Default::default(),
        schema_version: 1,
    };

    let mut runtime = WorkflowRuntime::new(ir, store, clock, Arc::new(NoopTaskExecutor));
    runtime.start().unwrap();
    runtime.complete(Default::default()).unwrap();

    // Second complete should return error (already terminal)
    let result = runtime.complete(Default::default());
    assert!(result.is_err(), "complete() on terminal should fail");
}

/// Verifies that start() on non-Pending state returns error.
#[test]
fn start_on_non_pending_returns_error() {
    let store = MockStore;
    let clock = Clock;

    let ir = WorkflowIR {
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
    };

    // Note: WorkflowRuntime::new() always creates a Pending run.
    // To test start() on non-Pending, we first start() then try start() again.
    let mut runtime = WorkflowRuntime::new(ir, store, clock, Arc::new(NoopTaskExecutor));
    runtime.start().unwrap(); // Now Running
    let result = runtime.start(); // Should fail - already Running
    assert!(result.is_err(), "start() on non-Pending should fail");
}

#[test]
fn runtime_run_ir_constructor() {
    // WorkflowRuntime::run_ir() creates a runtime from an IR + store
    use sddk_domain::TemplateRef;

    struct TestStore;
    impl GraphStore for TestStore {
        fn save_state(
            &mut self,
            _: &sddk_domain::GraphState,
        ) -> Result<(), sddk_domain::StorageError> {
            Ok(())
        }
        fn load_state(&self) -> Result<Option<sddk_domain::GraphState>, sddk_domain::StorageError> {
            Ok(None)
        }
        fn checkpoint(&self) -> Result<Option<sddk_domain::Checkpoint>, sddk_domain::StorageError> {
            Ok(None)
        }
        fn record_ir_digest(&mut self, _: &str, _: &str) -> Result<(), sddk_domain::StorageError> {
            Ok(())
        }
        fn record_graph_revision(
            &mut self,
            _: &sddk_domain::graph::ExecutionGraphRevision,
        ) -> Result<(), sddk_domain::StorageError> {
            Ok(())
        }
        fn load_node_attempts(
            &self,
            _: &sddk_domain::RunId,
            _: &sddk_domain::NodeId,
        ) -> Result<Vec<sddk_domain::Attempt>, sddk_domain::StorageError> {
            Ok(vec![])
        }
        fn attempt_count(
            &self,
            _: &sddk_domain::RunId,
            _: &sddk_domain::NodeId,
        ) -> Result<u32, sddk_domain::StorageError> {
            Ok(0)
        }
        fn save_revision(
            &mut self,
            _: &sddk_domain::graph::ExecutionGraphRevision,
        ) -> Result<(), sddk_domain::StorageError> {
            Ok(())
        }
        fn load_revision(
            &self,
            _: &sddk_domain::RunId,
            _: &sddk_domain::RevisionId,
        ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, sddk_domain::StorageError>
        {
            Ok(None)
        }
        fn latest_revision(
            &self,
            _: &sddk_domain::RunId,
        ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, sddk_domain::StorageError>
        {
            Ok(None)
        }
    }

    let ir = WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: TemplateRef {
            id: "test".into(),
            version: "1.0".into(),
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
    };

    let store = TestStore;
    let runtime = WorkflowRuntime::run_ir(ir, store);

    // run_ir() creates a Pending runtime
    assert_eq!(runtime.state(), &WorkflowRunState::Pending);
    assert!(runtime.run().run_id.0.starts_with("runtime-"));
}

#[test]
fn runtime_state_accessor() {
    use sddk_domain::TemplateRef;

    struct TestStore;
    impl GraphStore for TestStore {
        fn save_state(
            &mut self,
            _: &sddk_domain::GraphState,
        ) -> Result<(), sddk_domain::StorageError> {
            Ok(())
        }
        fn load_state(&self) -> Result<Option<sddk_domain::GraphState>, sddk_domain::StorageError> {
            Ok(None)
        }
        fn checkpoint(&self) -> Result<Option<sddk_domain::Checkpoint>, sddk_domain::StorageError> {
            Ok(None)
        }
        fn record_ir_digest(&mut self, _: &str, _: &str) -> Result<(), sddk_domain::StorageError> {
            Ok(())
        }
        fn record_graph_revision(
            &mut self,
            _: &sddk_domain::graph::ExecutionGraphRevision,
        ) -> Result<(), sddk_domain::StorageError> {
            Ok(())
        }
        fn load_node_attempts(
            &self,
            _: &sddk_domain::RunId,
            _: &sddk_domain::NodeId,
        ) -> Result<Vec<sddk_domain::Attempt>, sddk_domain::StorageError> {
            Ok(vec![])
        }
        fn attempt_count(
            &self,
            _: &sddk_domain::RunId,
            _: &sddk_domain::NodeId,
        ) -> Result<u32, sddk_domain::StorageError> {
            Ok(0)
        }
        fn save_revision(
            &mut self,
            _: &sddk_domain::graph::ExecutionGraphRevision,
        ) -> Result<(), sddk_domain::StorageError> {
            Ok(())
        }
        fn load_revision(
            &self,
            _: &sddk_domain::RunId,
            _: &sddk_domain::RevisionId,
        ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, sddk_domain::StorageError>
        {
            Ok(None)
        }
        fn latest_revision(
            &self,
            _: &sddk_domain::RunId,
        ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, sddk_domain::StorageError>
        {
            Ok(None)
        }
    }

    let ir = WorkflowIR {
        ir_id: None,
        schema_version: 1,
        template_ref: TemplateRef {
            id: "test".into(),
            version: "1.0".into(),
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
    };

    let mut runtime = WorkflowRuntime::run_ir(ir, TestStore);

    // state() returns Pending initially
    assert_eq!(runtime.state(), &WorkflowRunState::Pending);

    // After start(), state() returns Running
    runtime.start().unwrap();
    assert_eq!(runtime.state(), &WorkflowRunState::Running);

    // After complete(), state() returns Completed
    runtime.complete(Default::default()).unwrap();
    assert_eq!(runtime.state(), &WorkflowRunState::Completed);
}
