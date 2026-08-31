//! Kernel-pure workflow runtime for cycle-16.
//!
//! `WorkflowRuntime<R>` consumes a `WorkflowIR` and drives a `WorkflowRun`
//! end-to-end. It uses the `GraphStore` port for persistence and emits
//! canonical events via the event bus.
//!
//! ARCH008: This module is in the zero-SDD-phase zone — it contains no SDD phase enum references.
//! to the legacy SDD `Phase` taxonomy.

use std::collections::{BTreeMap, HashMap};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use sddk_domain::{
    ActorKind, EventStore, GraphStore, NodeId, NodeRun, NodeRunState, Operator, OperatorId,
    TaskExecutor, WorkflowIR, WorkflowRun, WorkflowRunState,
};
use serde_json::Value;
use thiserror::Error;

use crate::event_bus::{
    WorkflowNodeEventInput, WorkflowRunEventInput, emit_workflow_node_completed,
    emit_workflow_node_failed, emit_workflow_node_running, emit_workflow_run_completed,
    emit_workflow_run_started,
};
use crate::execution_controller::ExecutionController;
use crate::operator::{
    ChildResult, Clock, GraphStoreBox, NodeOutcome, OperatorContext, OperatorError,
    ScratchGraphStore, build_operator,
};

/// Result type for runtime operations.
pub type Result<T> = std::result::Result<T, RuntimeError>;

/// Runtime errors.
#[derive(Debug, Clone, Error)]
pub enum RuntimeError {
    /// Workflow is in an invalid state for the requested operation.
    #[error("invalid state: {reason}")]
    InvalidState {
        /// Human-readable reason for the invalid state.
        reason: String,
    },

    /// Storage error from the graph store.
    #[error("storage error: {0}")]
    Storage(#[from] sddk_domain::StorageError),

    /// Operator evaluation error.
    #[error("operator error: {0}")]
    Operator(#[from] crate::operator::OperatorError),

    /// Workflow is already in a terminal state.
    #[error("workflow already terminal: {state:?}")]
    AlreadyTerminal {
        /// The current terminal state.
        state: WorkflowRunState,
    },

    /// Wall-clock budget was exhausted at an observable recovery boundary.
    #[error("budget exceeded: {elapsed_ms}ms elapsed (max {max_wall_ms}ms)")]
    BudgetExceeded {
        /// Milliseconds actually elapsed.
        elapsed_ms: u64,
        /// Configured maximum wall time in milliseconds.
        max_wall_ms: u64,
    },

    /// No observable progress for the configured consecutive-tick threshold.
    #[error("no progress detected: {consecutive} consecutive ticks (threshold {threshold})")]
    NoProgressDetected {
        /// Consecutive ticks observed without progress.
        consecutive: u32,
        /// Configured threshold.
        threshold: u32,
    },
}

/// Marker trait for stores that can be used with WorkflowRuntime.
///
/// Currently just a constraint to group GraphStore with Send + Sync.
pub trait RunStore: GraphStore + Send + Sync {}

impl<T: GraphStore + Send + Sync> RunStore for T {}

/// Tick outcome — what happened during one tick.
#[derive(Debug, Clone)]
pub enum TickOutcome {
    /// All nodes are waiting — workflow is blocked.
    Waiting,
    /// Some nodes are still running.
    Running,
    /// All nodes completed — workflow can finish.
    AllComplete,
    /// A node failed.
    Failed,
}

/// Aggregated outcome from a single tick phase (DRAIN or SPAWN).
struct TickPhaseOutcome {
    outcomes: Vec<(OperatorId, NodeId, NodeOutcome)>,
    all_done: bool,
    any_failed: bool,
}

impl TickPhaseOutcome {
    fn empty() -> Self {
        Self {
            outcomes: Vec::new(),
            all_done: true,
            any_failed: false,
        }
    }

    fn merge(&mut self, other: TickPhaseOutcome) {
        self.outcomes.extend(other.outcomes);
        self.all_done = self.all_done && other.all_done;
        self.any_failed = self.any_failed || other.any_failed;
    }
}

/// Composite key for receiver-map entries: a unique Parallel slot per workflow run.
/// `OperatorId` defined as newtype `pub struct OperatorId(pub String)` in domain layer.
pub type ParallelKey = (sddk_domain::RunId, sddk_domain::OperatorId);

/// Composite key for Map checkpoint entries: same shape as ParallelKey.
/// Cycle-32: Map checkpoints are stored in pending_map keyed by (RunId, OperatorId).
pub type MapKey = ParallelKey;

// ── WorkflowRuntime ──────────────────────────────────────────────────────────

/// Kernel-pure workflow runtime.
///
/// Drives a `WorkflowRun` from `Pending` through to a terminal state
/// (`Completed`, `Failed`, or `Cancelled`) by evaluating operators.
pub struct WorkflowRuntime<R: RunStore> {
    // ── Core state ───────────────────────────────────────────────────────────
    /// The workflow IR this run was instantiated from.
    ir: WorkflowIR,
    /// The workflow run record.
    run: WorkflowRun,
    /// Node runs keyed by operator_id.
    nodes: BTreeMap<OperatorId, NodeRun>,
    /// The graph store for persistence.
    #[allow(dead_code)]
    store: R,
    /// Wall-clock source.
    #[allow(dead_code)]
    clock: Clock,
    /// Task executor for operator evaluation.
    executor: Arc<dyn TaskExecutor>,
    /// Event store for emitting workflow events (optional).
    event_store: Option<Arc<Mutex<dyn EventStore>>>,
    /// Receiver map for in-flight `Parallel` children that returned `NodeOutcome::Pending`
    /// on a prior tick. INV-4: state lives on runtime, NOT on `Attempt`.
    /// INV-10 (cycle-20): `Mutex<Receiver<ChildResult>>` guards a side-channel (the channel
    /// endpoint), NOT workflow state — explicitly permitted per cycle-20 INV-10 revision.
    pending_parallel: HashMap<ParallelKey, Arc<Mutex<mpsc::Receiver<ChildResult>>>>,
    /// Receiver map for in-flight `Map` operators that returned `NodeOutcome::Pending`
    /// on a prior tick. Cycle-32: stores `MapCheckpointState` (receiver + items_len +
    /// completed_results + source_outputs_snapshot) keyed by (RunId, OperatorId).
    /// The outer Arc<Mutex<>> allows mutable access to MapCheckpointState during drain.
    pending_map: HashMap<MapKey, Arc<std::sync::Mutex<crate::operator::MapCheckpointState>>>,
    /// Bounded-execution controller — created lazily at the start of `execute()`.
    controller: Option<ExecutionController>,
}

impl<R: RunStore> WorkflowRuntime<R> {
    /// Constructs a new runtime from an IR, store, and task executor.
    pub fn new(ir: WorkflowIR, store: R, clock: Clock, executor: Arc<dyn TaskExecutor>) -> Self {
        let run_id = sddk_domain::RunId(format!("runtime-{}", uuid::Uuid::new_v4()));
        let run = WorkflowRun {
            run_id,
            template_ref: ir.template_ref.clone(),
            ir_hash: ir.compute_content_hash(),
            graph_revision: sddk_domain::RevisionId(format!("rev-{}", uuid::Uuid::new_v4())),
            state: WorkflowRunState::Pending,
            inputs: Default::default(),
            outputs: None,
            correlation_id: sddk_domain::CorrelationId(format!("corr-{}", uuid::Uuid::new_v4())),
            budget: ir.budgets.clone(),
            schema_version: 1,
        };

        // Initialize node runs from the IR operators
        let mut nodes = BTreeMap::new();
        for op_id in ir.operators.keys() {
            // For cycle-16, we use OperatorId as the node_id value
            let node_id = NodeId(op_id.0.clone());
            let node_run = NodeRun {
                node_id,
                state: NodeRunState::Pending,
                dependencies: Default::default(),
                attempts: vec![],
                expansion_permissions: ir.expansion_permissions.clone(),
                schema_version: 1,
            };
            nodes.insert(op_id.clone(), node_run);
        }

        Self {
            ir,
            run,
            nodes,
            store,
            clock,
            executor,
            event_store: None,
            pending_parallel: HashMap::new(),
            pending_map: HashMap::new(),
            controller: None,
        }
    }

    /// Constructs a new runtime from an IR, store, event store, and task executor.
    pub fn new_with_event_store(
        ir: WorkflowIR,
        store: R,
        clock: Clock,
        event_store: Arc<Mutex<dyn EventStore>>,
        executor: Arc<dyn TaskExecutor>,
    ) -> Self {
        let run_id = sddk_domain::RunId(format!("runtime-{}", uuid::Uuid::new_v4()));
        let run = WorkflowRun {
            run_id,
            template_ref: ir.template_ref.clone(),
            ir_hash: ir.compute_content_hash(),
            graph_revision: sddk_domain::RevisionId(format!("rev-{}", uuid::Uuid::new_v4())),
            state: WorkflowRunState::Pending,
            inputs: Default::default(),
            outputs: None,
            correlation_id: sddk_domain::CorrelationId(format!("corr-{}", uuid::Uuid::new_v4())),
            budget: ir.budgets.clone(),
            schema_version: 1,
        };

        // Initialize node runs from the IR operators
        let mut nodes = BTreeMap::new();
        for op_id in ir.operators.keys() {
            // For cycle-16, we use OperatorId as the node_id value
            let node_id = NodeId(op_id.0.clone());
            let node_run = NodeRun {
                node_id,
                state: NodeRunState::Pending,
                dependencies: Default::default(),
                attempts: vec![],
                expansion_permissions: ir.expansion_permissions.clone(),
                schema_version: 1,
            };
            nodes.insert(op_id.clone(), node_run);
        }

        Self {
            ir,
            run,
            nodes,
            store,
            clock,
            executor,
            event_store: Some(event_store),
            pending_parallel: HashMap::new(),
            pending_map: HashMap::new(),
            controller: None,
        }
    }

    /// Constructs a new runtime directly from a `WorkflowIR`, consuming the IR.
    ///
    /// Convenience constructor that wraps `new()` but takes only the IR,
    /// using a no-op executor and wall-clock.
    pub fn run_ir(ir: WorkflowIR, store: R) -> Self {
        let clock = Clock;
        let executor: Arc<dyn TaskExecutor> = Arc::new(sddk_domain::NoopTaskExecutor);
        Self::new(ir, store, clock, executor)
    }

    /// Returns the current workflow run state.
    pub fn state(&self) -> &WorkflowRunState {
        &self.run.state
    }

    /// Starts the workflow — transitions from Pending to Running.
    pub fn start(&mut self) -> Result<()> {
        if self.run.state != WorkflowRunState::Pending {
            return Err(RuntimeError::InvalidState {
                reason: format!("start() requires Pending, got {:?}", self.run.state),
            });
        }
        self.run.state = WorkflowRunState::Running;
        Ok(())
    }

    /// Executes the workflow end-to-end.
    ///
    /// This is the main entry point that runs the workflow from `Pending`
    /// to a terminal state, emitting canonical events at each lifecycle point.
    ///
    /// Emits:
    /// - `workflow.run.started` at entry
    /// - `workflow.node.running` / `workflow.node.completed` / `workflow.node.failed` per node
    /// - `workflow.run.completed` when the run reaches a terminal state
    pub fn execute(&mut self) -> Result<()> {
        // Try to load an existing run for replay/resumption
        // In cycle-16, this is a no-op since the default implementation returns NotImplemented
        match self.store.load_run(&self.run.run_id) {
            Ok(Some(_loaded_run)) => {
                // Run exists — in cycle-17 this would resume from the loaded state
                // For cycle-16, we just proceed (run is already in the state machine)
            }
            Ok(None) => {
                // No existing run — first time execution
            }
            Err(_) => {
                // Storage error — proceed anyway (cycle-16 default is not implemented)
            }
        }

        // Emit workflow.run.started
        self.emit_run_started()?;

        // Transition to Running
        self.start()?;

        // Lazily create the bounded-execution controller, capturing Instant::now()
        // at the start of execute() — the wall-budget timer starts here.
        if self.controller.is_none() {
            self.controller = Some(ExecutionController::from_budgets(&self.run.budget));
        }

        // Run tick loop until terminal state
        loop {
            match self.tick()? {
                TickOutcome::AllComplete => {
                    self.complete(Default::default())?;
                    break;
                }
                TickOutcome::Failed => {
                    self.fail("node failed".into())?;
                    break;
                }
                TickOutcome::Waiting | TickOutcome::Running => {
                    // Continue ticking
                }
            }
        }

        // Emit workflow.run.completed
        self.emit_run_completed()?;

        Ok(())
    }

    /// Emits `workflow.run.started` event if an event store is configured.
    fn emit_run_started(&mut self) -> Result<()> {
        if let Some(ref mut store) = self.event_store {
            let input = WorkflowRunEventInput {
                project_id: "sddk".into(),
                run_id: self.run.run_id.0.clone(),
                occurred_at: self.clock.now(),
                actor_id: "workflow-runtime".into(),
                actor_kind: ActorKind::System,
            };
            emit_workflow_run_started(store, &input)?;
        }
        Ok(())
    }

    /// Emits `workflow.run.completed` event if an event store is configured.
    fn emit_run_completed(&mut self) -> Result<()> {
        if let Some(ref mut store) = self.event_store {
            let input = WorkflowRunEventInput {
                project_id: "sddk".into(),
                run_id: self.run.run_id.0.clone(),
                occurred_at: self.clock.now(),
                actor_id: "workflow-runtime".into(),
                actor_kind: ActorKind::System,
            };
            emit_workflow_run_completed(store, &input)?;
        }
        Ok(())
    }

    /// Emits `workflow.node.running` event for a node if an event store is configured.
    fn emit_node_running(&mut self, node_id: &NodeId) -> Result<()> {
        if let Some(ref mut store) = self.event_store {
            let input = WorkflowNodeEventInput {
                project_id: "sddk".into(),
                run_id: self.run.run_id.0.clone(),
                node_id: node_id.0.clone(),
                occurred_at: self.clock.now(),
                actor_id: "workflow-runtime".into(),
                actor_kind: ActorKind::System,
                reason: None,
            };
            emit_workflow_node_running(store, &input)?;
        }
        Ok(())
    }

    /// Emits `workflow.node.completed` event for a node if an event store is configured.
    fn emit_node_completed(&mut self, node_id: &NodeId) -> Result<()> {
        if let Some(ref mut store) = self.event_store {
            let input = WorkflowNodeEventInput {
                project_id: "sddk".into(),
                run_id: self.run.run_id.0.clone(),
                node_id: node_id.0.clone(),
                occurred_at: self.clock.now(),
                actor_id: "workflow-runtime".into(),
                actor_kind: ActorKind::System,
                reason: None,
            };
            emit_workflow_node_completed(store, &input)?;
        }
        Ok(())
    }

    /// Emits `workflow.node.failed` event for a node if an event store is configured.
    fn emit_node_failed(&mut self, node_id: &NodeId, reason: &str) -> Result<()> {
        if let Some(ref mut store) = self.event_store {
            let input = WorkflowNodeEventInput {
                project_id: "sddk".into(),
                run_id: self.run.run_id.0.clone(),
                node_id: node_id.0.clone(),
                occurred_at: self.clock.now(),
                actor_id: "workflow-runtime".into(),
                actor_kind: ActorKind::System,
                reason: Some(reason.into()),
            };
            emit_workflow_node_failed(store, &input)?;
        }
        Ok(())
    }

    // ── tick() orchestrator ─────────────────────────────────────────────────────

    pub fn tick(&mut self) -> Result<TickOutcome> {
        // AlreadyTerminal wins — check this first before any bounded-execution check.
        if self.run.state.is_terminal() {
            return Err(RuntimeError::AlreadyTerminal {
                state: self.run.state.clone(),
            });
        }

        // Strict Running precondition: tick() requires the workflow to be in Running state.
        // Pending and Paused are not valid states for tick — the caller must transition
        // to Running first via start() or resume().
        if self.run.state != WorkflowRunState::Running {
            return Err(RuntimeError::InvalidState {
                reason: format!("tick() requires Running, got {:?}", self.run.state),
            });
        }

        // Wall-budget check at the top of every tick (D3).
        if let Some(ref mut ctrl) = self.controller {
            ctrl.pre_tick()?;
        }

        // DRAIN phase: drain both pending_parallel and pending_map
        let drain_parallel = self.drain_pending_parallel();
        let drain_map = self.drain_pending_map();
        let mut spawn = self.spawn_pending_and_ready(&drain_parallel);

        // Merge all drain outcomes into spawn
        spawn.merge(drain_parallel);
        spawn.merge(drain_map);

        self.apply_outcomes_to_state(&spawn.outcomes);

        // No-progress check after observable state changes (D3).
        if let Some(ref mut ctrl) = self.controller {
            ctrl.observe(&self.nodes)?;
        }

        if spawn.any_failed {
            Ok(TickOutcome::Failed)
        } else if spawn.all_done {
            Ok(TickOutcome::AllComplete)
        } else {
            Ok(TickOutcome::Running)
        }
    }

    // ── tick helper methods ─────────────────────────────────────────────────────

    /// DRAIN phase: iterate pending_parallel entries and collect child results.
    fn drain_pending_parallel(&mut self) -> TickPhaseOutcome {
        use std::collections::BTreeMap;

        let mut outcome = TickPhaseOutcome::empty();
        let mut reinsert_entries: Vec<(ParallelKey, Arc<Mutex<mpsc::Receiver<ChildResult>>>)> =
            Vec::new();

        for (key, rx_arc) in self.pending_parallel.drain() {
            let rx_lock = rx_arc.lock().expect("receiver mutex poisoned");
            let mut collected: BTreeMap<usize, ChildResult> = BTreeMap::new();
            let mut disconnected = false;

            loop {
                match rx_lock.try_recv() {
                    Ok(result) => {
                        collected.insert(result.child_index, result);
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        break;
                    }
                }
            }
            drop(rx_lock);

            let (op_id, _node_id) = {
                let op_id = key.1.clone();
                let node_id = NodeId(op_id.0.clone());
                (op_id, node_id)
            };

            let child_count = match self.ir.operators.get(&op_id) {
                Some(op) => {
                    if let Operator::Parallel { branches, .. } = op {
                        branches.len()
                    } else {
                        continue;
                    }
                }
                None => continue,
            };

            if collected.len() == child_count || disconnected {
                let node_run = match self.nodes.get_mut(&op_id) {
                    Some(nr) => nr,
                    None => continue,
                };

                let node_id = NodeId(op_id.0.clone());
                let run_id = &self.run.run_id;

                for child_index in 0..child_count {
                    let result = match collected.remove(&child_index) {
                        Some(r) => r,
                        None => ChildResult {
                            child_index,
                            outcome: Err(crate::operator::OperatorError::EvalFailed(format!(
                                "child {} did not report",
                                child_index
                            ))),
                            started_at: self.clock.now(),
                            ended_at: self.clock.now(),
                        },
                    };

                    let attempt = crate::operator::build_attempt(
                        &node_id,
                        run_id,
                        result.child_index,
                        &result,
                        &self.clock,
                    );
                    node_run.attempts.push(attempt);
                }

                let any_failed_child = node_run.attempts.iter().any(|a| {
                    matches!(
                        a.outcome,
                        Some(sddk_domain::workflow_run::AttemptOutcome::Failed { .. })
                    )
                });

                if any_failed_child {
                    outcome.outcomes.push((
                        op_id.clone(),
                        node_id.clone(),
                        NodeOutcome::Failed {
                            node_id: node_id.clone(),
                            reason: "child failed".into(),
                        },
                    ));
                } else {
                    outcome.outcomes.push((
                        op_id.clone(),
                        node_id.clone(),
                        NodeOutcome::Succeeded {
                            node_id: node_id.clone(),
                            outputs: Default::default(),
                        },
                    ));
                }
            } else {
                reinsert_entries.push((key, rx_arc));
            }
        }

        for (key, rx_arc) in reinsert_entries {
            self.pending_parallel.insert(key, rx_arc);
        }

        outcome
    }

    /// DRAIN phase for Map: iterate pending_map entries and collect child results.
    /// Mirrors drain_pending_parallel but works with MapCheckpointState.
    fn drain_pending_map(&mut self) -> TickPhaseOutcome {
        use crate::operator::NodeOutcome;
        use std::collections::BTreeMap;

        let mut outcome = TickPhaseOutcome::empty();
        let mut reinsert_entries: Vec<(
            MapKey,
            Arc<std::sync::Mutex<crate::operator::MapCheckpointState>>,
        )> = Vec::new();

        for (key, state_arc) in self.pending_map.drain() {
            // Lock the outer mutex to access the state
            let mut state = state_arc.lock().unwrap();

            // Clone the Arc<Mutex<Receiver>> from the state
            let receiver_arc = std::sync::Arc::clone(&state.receiver);

            // Lock the inner receiver and drain
            let mut disconnected = false;
            loop {
                let receiver_guard = receiver_arc.lock().unwrap();
                match receiver_guard.try_recv() {
                    Ok(child_result) => {
                        drop(receiver_guard);
                        // Insert into completed_results (already holding state lock)
                        state
                            .completed_results
                            .insert(child_result.child_index, child_result);
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        break;
                    }
                }
            }

            let (op_id, _node_id) = {
                let op_id = key.1.clone();
                let node_id = sddk_domain::NodeId(op_id.0.clone());
                (op_id, node_id)
            };

            // Termination condition: completed all items OR receiver disconnected
            let complete = state.completed_results.len() == state.items_len || disconnected;

            if complete {
                // Build results and failures arrays from completed_results
                let mut results: Vec<serde_json::Value> = Vec::with_capacity(state.items_len);
                let mut failures: Vec<serde_json::Value> = Vec::new();

                for i in 0..state.items_len {
                    match state.completed_results.remove(&i) {
                        Some(child_result) => {
                            match child_result.outcome {
                                Ok(NodeOutcome::Succeeded { outputs, .. }) => {
                                    let result_obj = serde_json::Value::Object(
                                        serde_json::Map::from_iter(outputs),
                                    );
                                    results.push(result_obj);
                                }
                                Ok(NodeOutcome::Failed { reason, .. }) => {
                                    failures.push(serde_json::json!({
                                        "index": i as u64,
                                        "reason": reason,
                                    }));
                                }
                                Ok(NodeOutcome::Pending { .. }) => {
                                    // Should not reach here - complete means no pending
                                    failures.push(serde_json::json!({
                                        "index": i as u64,
                                        "reason": "pending not expected at drain",
                                    }));
                                }
                                Ok(NodeOutcome::Running) => {
                                    failures.push(serde_json::json!({
                                        "index": i as u64,
                                        "reason": "unexpected running state",
                                    }));
                                }
                                Err(OperatorError::ChildPanicked { child_index }) => {
                                    failures.push(serde_json::json!({
                                        "index": child_index as u64,
                                        "reason": format!("child {} panicked", child_index),
                                    }));
                                }
                                Err(e) => {
                                    failures.push(serde_json::json!({
                                        "index": i as u64,
                                        "reason": e.to_string(),
                                    }));
                                }
                            }
                        }
                        None => {
                            // Missing result - child did not report
                            failures.push(serde_json::json!({
                                "index": i as u64,
                                "reason": format!("child {} did not report", i),
                            }));
                        }
                    }
                }

                // Aggregate using collect-all semantics
                let node_id_owned = sddk_domain::NodeId(op_id.0.clone());
                if failures.is_empty() {
                    // All succeeded
                    let mut outputs = BTreeMap::new();
                    outputs.insert("results".to_string(), serde_json::Value::Array(results));
                    outputs.insert("failures".to_string(), serde_json::Value::Array(failures));
                    outcome.outcomes.push((
                        op_id.clone(),
                        node_id_owned.clone(),
                        NodeOutcome::Succeeded {
                            node_id: node_id_owned,
                            outputs,
                        },
                    ));
                } else if results.is_empty() {
                    // All failed
                    let composite_reason =
                        crate::operator::build_map_composite_failure_reason(&failures);
                    outcome.outcomes.push((
                        op_id.clone(),
                        node_id_owned.clone(),
                        NodeOutcome::Failed {
                            node_id: node_id_owned,
                            reason: composite_reason,
                        },
                    ));
                } else {
                    // Partial success
                    let mut outputs = BTreeMap::new();
                    outputs.insert("results".to_string(), serde_json::Value::Array(results));
                    outputs.insert("failures".to_string(), serde_json::Value::Array(failures));
                    outcome.outcomes.push((
                        op_id.clone(),
                        node_id_owned.clone(),
                        NodeOutcome::Succeeded {
                            node_id: node_id_owned,
                            outputs,
                        },
                    ));
                }
            } else {
                // Partial - re-insert for next tick
                drop(state); // Release lock before moving state_arc
                reinsert_entries.push((key, state_arc));
            }
        }

        // Re-insert partial entries
        for (key, state_arc) in reinsert_entries {
            self.pending_map.insert(key, state_arc);
        }

        outcome
    }

    /// SPAWN phase: evaluate Pending|Ready nodes NOT in pending_parallel.
    fn spawn_pending_and_ready(&mut self, prior: &TickPhaseOutcome) -> TickPhaseOutcome {
        let mut outcome = TickPhaseOutcome::empty();

        for (op_id, node_run) in &mut self.nodes {
            match node_run.state {
                NodeRunState::Pending | NodeRunState::Ready | NodeRunState::Running => {
                    let key = (self.run.run_id.clone(), op_id.clone());
                    // Skip if already has a pending parallel checkpoint
                    if self.pending_parallel.contains_key(&key) {
                        outcome.all_done = false;
                        continue;
                    }
                    // Skip if already has a pending map checkpoint (cycle-32)
                    if self.pending_map.contains_key(&key) {
                        outcome.all_done = false;
                        continue;
                    }

                    if prior.outcomes.iter().any(|(o, _, _)| o == op_id) {
                        continue;
                    }

                    let ir_op = match self.ir.operators.get(op_id) {
                        Some(op) => op,
                        None => {
                            outcome.outcomes.push((
                                op_id.clone(),
                                NodeId(op_id.0.clone()),
                                NodeOutcome::Failed {
                                    node_id: NodeId(op_id.0.clone()),
                                    reason: "operator not found".into(),
                                },
                            ));
                            continue;
                        }
                    };

                    let runtime_op = match build_operator(ir_op, &self.ir) {
                        Ok(op) => op,
                        Err(e) => {
                            outcome.outcomes.push((
                                op_id.clone(),
                                NodeId(op_id.0.clone()),
                                NodeOutcome::Failed {
                                    node_id: NodeId(op_id.0.clone()),
                                    reason: format!("operator error: {}", e),
                                },
                            ));
                            continue;
                        }
                    };

                    let is_parallel = matches!(ir_op, Operator::Parallel { .. });

                    let pending_sender = if is_parallel {
                        let (tx, rx) = mpsc::channel::<ChildResult>();
                        let rx_arc = Arc::new(Mutex::new(rx));
                        self.pending_parallel.insert(key, rx_arc);
                        Some(tx)
                    } else {
                        None
                    };

                    let node_run_owned = node_run.clone();
                    let node_run_arc = Arc::new(Mutex::new(node_run_owned));

                    let executor = Arc::clone(&self.executor);
                    let store = Arc::new(Mutex::new(GraphStoreBox {
                        inner: Box::new(ScratchGraphStore),
                    }));
                    let mut ctx: OperatorContext = OperatorContext {
                        node_run: Arc::clone(&node_run_arc),
                        ir: Arc::new(self.ir.clone()),
                        run: Arc::new(self.run.clone()),
                        store,
                        clock: self.clock.clone(),
                        executor,
                        pending_sender,
                    };

                    match runtime_op.evaluate(&mut ctx) {
                        Ok(outcome_val) => {
                            // cycle-22: defensive sync with WARN log preserved
                            match Arc::try_unwrap(node_run_arc) {
                                Ok(mutex) => {
                                    *node_run = mutex
                                        .into_inner()
                                        .expect("Mutex<NodeRun> poisoned at sync point");
                                }
                                Err(arc) => {
                                    let count = Arc::strong_count(&arc);
                                    *node_run = arc
                                        .lock()
                                        .expect("Mutex<NodeRun> poisoned at sync point")
                                        .clone();
                                    eprintln!(
                                        "WARN: Arc<Mutex<NodeRun>> sync via lock fallback ({} refs at sync point) \
                                         — INV-9 audit: investigate thread leak source",
                                        count
                                    );
                                }
                            }

                            let attempt_outcome = match &outcome_val {
                                NodeOutcome::Succeeded { .. } => {
                                    sddk_domain::workflow_run::AttemptOutcome::Succeeded {
                                        outputs: Default::default(),
                                    }
                                }
                                NodeOutcome::Failed { reason, .. } => {
                                    sddk_domain::workflow_run::AttemptOutcome::Failed {
                                        error: reason.clone(),
                                    }
                                }
                                NodeOutcome::Pending { checkpoint } => {
                                    // cycle-32: Check if this is a MapChannel checkpoint
                                    if let crate::operator::CheckpointHandle::MapChannel {
                                        state,
                                        token: _,
                                    } = checkpoint
                                    {
                                        let map_key = (self.run.run_id.clone(), op_id.clone());
                                        // Wrap in Mutex for mutable access during drain
                                        self.pending_map.insert(
                                            map_key,
                                            Arc::new(std::sync::Mutex::new((**state).clone())),
                                        );
                                    }
                                    sddk_domain::workflow_run::AttemptOutcome::Pending {
                                        resume_token: 0,
                                        attempt_seq: 0,
                                    }
                                }
                                NodeOutcome::Running => {
                                    sddk_domain::workflow_run::AttemptOutcome::Failed {
                                        error: "unexpected Running outcome".into(),
                                    }
                                }
                            };

                            let _ = self.store.record_attempt(&sddk_domain::Attempt {
                                attempt_id: sddk_domain::workflow_run::AttemptId(format!(
                                    "tick-attempt-{}-{}",
                                    self.run.run_id.0, op_id.0
                                )),
                                node_id: NodeId(op_id.0.clone()),
                                route: sddk_domain::Route {
                                    provider: "runtime".to_string(),
                                    model: "cycle16".to_string(),
                                    host: "local".to_string(),
                                },
                                started_at: self.clock.now(),
                                ended_at: Some(self.clock.now()),
                                outcome: Some(attempt_outcome),
                                usage: sddk_domain::Usage {
                                    tokens_in: 0,
                                    tokens_out: 0,
                                    cost_micros: 0,
                                    wall_ms: 0,
                                },
                                context_capsule: sddk_domain::ContextCapsuleRef::Pointer {
                                    cid: format!("ctx-{}-{}", self.run.run_id.0, op_id.0),
                                },
                                idempotency_key: sddk_domain::IdempotencyKey {
                                    project_id: "sddk".to_string(),
                                    run_id: self.run.run_id.clone(),
                                    node_id: NodeId(op_id.0.clone()),
                                    attempt_seq: 0,
                                },
                                schema_version: 1,
                            });
                            outcome.outcomes.push((
                                op_id.clone(),
                                NodeId(op_id.0.clone()),
                                outcome_val,
                            ));
                        }
                        Err(e) => {
                            // cycle-22: defensive sync with WARN log preserved
                            match Arc::try_unwrap(node_run_arc) {
                                Ok(mutex) => {
                                    *node_run = mutex
                                        .into_inner()
                                        .expect("Mutex<NodeRun> poisoned at sync point");
                                }
                                Err(arc) => {
                                    let count = Arc::strong_count(&arc);
                                    *node_run = arc
                                        .lock()
                                        .expect("Mutex<NodeRun> poisoned at sync point")
                                        .clone();
                                    eprintln!(
                                        "WARN: Arc<Mutex<NodeRun>> sync via lock fallback ({} refs at sync point) \
                                         — INV-9 audit: investigate thread leak source",
                                        count
                                    );
                                }
                            }
                            let _ = self.store.record_attempt(&sddk_domain::Attempt {
                                attempt_id: sddk_domain::workflow_run::AttemptId(format!(
                                    "tick-attempt-{}-{}",
                                    self.run.run_id.0, op_id.0
                                )),
                                node_id: NodeId(op_id.0.clone()),
                                route: sddk_domain::Route {
                                    provider: "runtime".to_string(),
                                    model: "cycle16".to_string(),
                                    host: "local".to_string(),
                                },
                                started_at: self.clock.now(),
                                ended_at: Some(self.clock.now()),
                                outcome: Some(sddk_domain::workflow_run::AttemptOutcome::Failed {
                                    error: format!("evaluation error: {}", e),
                                }),
                                usage: sddk_domain::Usage {
                                    tokens_in: 0,
                                    tokens_out: 0,
                                    cost_micros: 0,
                                    wall_ms: 0,
                                },
                                context_capsule: sddk_domain::ContextCapsuleRef::Pointer {
                                    cid: format!("ctx-{}-{}", self.run.run_id.0, op_id.0),
                                },
                                idempotency_key: sddk_domain::IdempotencyKey {
                                    project_id: "sddk".to_string(),
                                    run_id: self.run.run_id.clone(),
                                    node_id: NodeId(op_id.0.clone()),
                                    attempt_seq: 0,
                                },
                                schema_version: 1,
                            });
                            outcome.outcomes.push((
                                op_id.clone(),
                                NodeId(op_id.0.clone()),
                                NodeOutcome::Failed {
                                    node_id: NodeId(op_id.0.clone()),
                                    reason: format!("evaluation error: {}", e),
                                },
                            ));
                        }
                    }
                }
                // cycle-43: NodeRunState::Running is now matched in the FIRST match
                // arm (spawn_pending_and_ready ~line 728) so this later arm is unreachable.
                NodeRunState::Completed | NodeRunState::Failed | NodeRunState::Skipped => {}
            }
        }

        // Emit events and compute all_done/any_failed from outcomes
        for (_op_id, node_id, out) in &outcome.outcomes {
            match out {
                NodeOutcome::Succeeded { .. } => {
                    let _ = self.emit_node_running(node_id);
                    let _ = self.emit_node_completed(node_id);
                }
                NodeOutcome::Running => {
                    let _ = self.emit_node_running(node_id);
                    outcome.all_done = false;
                }
                NodeOutcome::Pending { .. } => {
                    outcome.all_done = false;
                }
                NodeOutcome::Failed { reason, .. } => {
                    let _ = self.emit_node_running(node_id);
                    let _ = self.emit_node_failed(node_id, reason);
                    outcome.any_failed = true;
                }
            }
        }

        outcome
    }

    /// Apply state transitions from outcomes and persist to store.
    fn apply_outcomes_to_state(&mut self, outcomes: &[(OperatorId, NodeId, NodeOutcome)]) {
        for (op_id, node_run) in &mut self.nodes {
            let node_id = NodeId(op_id.0.clone());
            for (_op_id, _node_id, outcome) in outcomes {
                if node_id == *_node_id {
                    match outcome {
                        NodeOutcome::Pending { .. } => {
                            node_run.state = NodeRunState::Pending;
                        }
                        NodeOutcome::Running => {
                            node_run.state = NodeRunState::Running;
                        }
                        NodeOutcome::Succeeded { .. } => {
                            node_run.state = NodeRunState::Completed;
                        }
                        NodeOutcome::Failed { .. } => {
                            node_run.state = NodeRunState::Failed;
                        }
                    }
                    let _ = self.store.record_node_run(node_run);
                }
            }
        }
    }

    /// Completes the workflow with outputs.
    pub fn complete(&mut self, outputs: BTreeMap<String, Value>) -> Result<()> {
        if self.run.state.is_terminal() {
            return Err(RuntimeError::AlreadyTerminal {
                state: self.run.state.clone(),
            });
        }
        self.run.state = WorkflowRunState::Completed;
        self.run.outputs = Some(outputs);
        Ok(())
    }

    /// Fails the workflow with an error.
    pub fn fail(&mut self, reason: String) -> Result<()> {
        if self.run.state.is_terminal() {
            return Err(RuntimeError::AlreadyTerminal {
                state: self.run.state.clone(),
            });
        }
        self.run.state = WorkflowRunState::Failed;
        let mut outputs = BTreeMap::new();
        outputs.insert("error".into(), Value::String(reason));
        self.run.outputs = Some(outputs);
        Ok(())
    }

    /// Pauses the workflow.
    pub fn pause(&mut self) -> Result<()> {
        if self.run.state != WorkflowRunState::Running {
            return Err(RuntimeError::InvalidState {
                reason: format!("pause() requires Running, got {:?}", self.run.state),
            });
        }
        self.run.state = WorkflowRunState::Paused;
        Ok(())
    }

    /// Resumes a paused workflow.
    pub fn resume(&mut self) -> Result<()> {
        if self.run.state != WorkflowRunState::Paused {
            return Err(RuntimeError::InvalidState {
                reason: format!("resume() requires Paused, got {:?}", self.run.state),
            });
        }
        self.run.state = WorkflowRunState::Running;
        Ok(())
    }

    /// Cancels the workflow.
    pub fn cancel(&mut self) -> Result<()> {
        if self.run.state.is_terminal() {
            return Err(RuntimeError::AlreadyTerminal {
                state: self.run.state.clone(),
            });
        }
        self.run.state = WorkflowRunState::Cancelled;
        Ok(())
    }

    /// Returns a reference to the workflow run.
    pub fn run(&self) -> &WorkflowRun {
        &self.run
    }

    /// Returns a mutable reference to the workflow run.
    pub fn run_mut(&mut self) -> &mut WorkflowRun {
        &mut self.run
    }

    /// Returns the node runs.
    pub fn nodes(&self) -> &BTreeMap<OperatorId, NodeRun> {
        &self.nodes
    }

    /// Returns the IR.
    pub fn ir(&self) -> &WorkflowIR {
        &self.ir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::{GraphStore, NodeId, NoopTaskExecutor, WorkflowRunState};
    use std::result::Result as StdResult;

    // Minimal mock store for testing
    struct MockStore;

    impl GraphStore for MockStore {
        fn save_state(
            &mut self,
            _state: &sddk_domain::GraphState,
        ) -> StdResult<(), sddk_domain::StorageError> {
            Ok(())
        }

        fn load_state(
            &self,
        ) -> StdResult<Option<sddk_domain::GraphState>, sddk_domain::StorageError> {
            Ok(None)
        }

        fn checkpoint(
            &self,
        ) -> StdResult<Option<sddk_domain::Checkpoint>, sddk_domain::StorageError> {
            Ok(None)
        }

        fn record_ir_digest(
            &mut self,
            _ir_hash: &str,
            _ir_json: &str,
        ) -> StdResult<(), sddk_domain::StorageError> {
            Ok(())
        }

        fn record_graph_revision(
            &mut self,
            _rev: &sddk_domain::ExecutionGraphRevision,
        ) -> StdResult<(), sddk_domain::StorageError> {
            Ok(())
        }

        fn load_node_attempts(
            &self,
            _run_id: &sddk_domain::RunId,
            _node_id: &NodeId,
        ) -> StdResult<Vec<sddk_domain::Attempt>, sddk_domain::StorageError> {
            Ok(vec![])
        }

        fn attempt_count(
            &self,
            _run_id: &sddk_domain::RunId,
            _node_id: &NodeId,
        ) -> StdResult<u32, sddk_domain::StorageError> {
            Ok(0)
        }

        fn save_revision(
            &mut self,
            _rev: &sddk_domain::ExecutionGraphRevision,
        ) -> StdResult<(), sddk_domain::StorageError> {
            Ok(())
        }

        fn load_revision(
            &self,
            _run_id: &sddk_domain::RunId,
            _rev_id: &sddk_domain::RevisionId,
        ) -> StdResult<Option<sddk_domain::ExecutionGraphRevision>, sddk_domain::StorageError>
        {
            Ok(None)
        }

        fn latest_revision(
            &self,
            _run_id: &sddk_domain::RunId,
        ) -> StdResult<Option<sddk_domain::ExecutionGraphRevision>, sddk_domain::StorageError>
        {
            Ok(None)
        }
    }

    fn make_ir() -> WorkflowIR {
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

    #[test]
    fn start_transitions_to_running() {
        let store = MockStore;
        let clock = Clock;
        let ir = make_ir();
        let executor = Arc::new(NoopTaskExecutor);
        let mut runtime = WorkflowRuntime::new(ir, store, clock, executor);
        assert_eq!(runtime.run.state, WorkflowRunState::Pending);
        runtime.start().unwrap();
        assert_eq!(runtime.run.state, WorkflowRunState::Running);
    }

    #[test]
    fn start_on_non_pending_fails() {
        let store = MockStore;
        let clock = Clock;
        let ir = make_ir();
        let executor = Arc::new(NoopTaskExecutor);
        let mut runtime = WorkflowRuntime::new(ir, store, clock, executor);
        runtime.start().unwrap();
        let result = runtime.start();
        assert!(result.is_err());
    }

    #[test]
    fn complete_transitions_to_completed() {
        let store = MockStore;
        let clock = Clock;
        let ir = make_ir();
        let executor = Arc::new(NoopTaskExecutor);
        let mut runtime = WorkflowRuntime::new(ir, store, clock, executor);
        runtime.start().unwrap();
        runtime.complete(Default::default()).unwrap();
        assert_eq!(runtime.run.state, WorkflowRunState::Completed);
    }

    #[test]
    fn complete_on_terminal_fails() {
        let store = MockStore;
        let clock = Clock;
        let ir = make_ir();
        let executor = Arc::new(NoopTaskExecutor);
        let mut runtime = WorkflowRuntime::new(ir, store, clock, executor);
        runtime.start().unwrap();
        runtime.complete(Default::default()).unwrap();
        let result = runtime.complete(Default::default());
        assert!(result.is_err());
    }

    #[test]
    fn fail_transitions_to_failed() {
        let store = MockStore;
        let clock = Clock;
        let ir = make_ir();
        let executor = Arc::new(NoopTaskExecutor);
        let mut runtime = WorkflowRuntime::new(ir, store, clock, executor);
        runtime.start().unwrap();
        runtime.fail("test error".into()).unwrap();
        assert_eq!(runtime.run.state, WorkflowRunState::Failed);
    }

    #[test]
    fn pause_resume_cycle() {
        let store = MockStore;
        let clock = Clock;
        let ir = make_ir();
        let executor = Arc::new(NoopTaskExecutor);
        let mut runtime = WorkflowRuntime::new(ir, store, clock, executor);
        runtime.start().unwrap();
        runtime.pause().unwrap();
        assert_eq!(runtime.run.state, WorkflowRunState::Paused);
        runtime.resume().unwrap();
        assert_eq!(runtime.run.state, WorkflowRunState::Running);
    }

    #[test]
    fn cancel_transitions_to_cancelled() {
        let store = MockStore;
        let clock = Clock;
        let ir = make_ir();
        let executor = Arc::new(NoopTaskExecutor);
        let mut runtime = WorkflowRuntime::new(ir, store, clock, executor);
        runtime.start().unwrap();
        runtime.cancel().unwrap();
        assert_eq!(runtime.run.state, WorkflowRunState::Cancelled);
    }

    // ── Runtime state invariant: tick requires Running ──────────────────────────────

    #[test]
    fn tick_on_pending_returns_invalid_state() {
        // RED: tick() should reject being called when workflow is Pending.
        // The precondition is Running — Pending is not a valid tick state.
        let store = MockStore;
        let clock = Clock;
        let ir = make_ir();
        let executor = Arc::new(NoopTaskExecutor);
        let mut runtime = WorkflowRuntime::new(ir, store, clock, executor);
        // Explicitly leave runtime in Pending state (do NOT call start())
        assert_eq!(runtime.run.state, WorkflowRunState::Pending);
        let result = runtime.tick();
        assert!(
            result.is_err(),
            "tick() on Pending should return Err(InvalidState), got: {:?}",
            result
        );
        let err = result.unwrap_err();
        assert!(
            matches!(err, RuntimeError::InvalidState { .. }),
            "expected InvalidState error, got: {:?}",
            err
        );
    }

    #[test]
    fn tick_on_paused_returns_invalid_state() {
        // RED: tick() should reject being called when workflow is Paused.
        // The precondition is Running — Paused is not a valid tick state.
        let store = MockStore;
        let clock = Clock;
        let ir = make_ir();
        let executor = Arc::new(NoopTaskExecutor);
        let mut runtime = WorkflowRuntime::new(ir, store, clock, executor);
        runtime.start().unwrap();
        runtime.pause().unwrap();
        assert_eq!(runtime.run.state, WorkflowRunState::Paused);
        let result = runtime.tick();
        assert!(
            result.is_err(),
            "tick() on Paused should return Err(InvalidState), got: {:?}",
            result
        );
        let err = result.unwrap_err();
        assert!(
            matches!(err, RuntimeError::InvalidState { .. }),
            "expected InvalidState error, got: {:?}",
            err
        );
    }

    #[test]
    fn tick_already_terminal_wins_over_running_check() {
        // AlreadyTerminal should win over the Running precondition check.
        // This preserves the terminal AlreadyTerminal behavior.
        let store = MockStore;
        let clock = Clock;
        let ir = make_ir();
        let executor = Arc::new(NoopTaskExecutor);
        let mut runtime = WorkflowRuntime::new(ir, store, clock, executor);
        runtime.start().unwrap();
        runtime.complete(Default::default()).unwrap();
        assert!(runtime.run.state.is_terminal());
        let result = runtime.tick();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, RuntimeError::AlreadyTerminal { .. }),
            "AlreadyTerminal should win over InvalidState, got: {:?}",
            err
        );
    }

    #[test]
    fn tick_returns_outcome() {
        let store = MockStore;
        let clock = Clock;
        let ir = make_ir();
        let executor = Arc::new(NoopTaskExecutor);
        let mut runtime = WorkflowRuntime::new(ir, store, clock, executor);
        runtime.start().unwrap();
        let outcome = runtime.tick().unwrap();
        assert!(matches!(outcome, TickOutcome::AllComplete));
    }
}
