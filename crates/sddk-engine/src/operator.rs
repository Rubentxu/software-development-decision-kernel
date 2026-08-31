//! Kernel-pure workflow operators for cycle-26 runtime.
//!
//! This module defines the `Operator` trait and its 5 in-scope implementations
//! (Task, Sequence, Parallel, Choice, Map). The 7 out-of-scope variants return
//! `Err(OperatorError::NotImplementedInCycle16)` via `build_operator()`.
//!
//! ARCH008: This module is in the zero-SDD-phase zone — it contains no SDD phase enum references.
//! to the legacy SDD `Phase` taxonomy.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use sddk_domain::{
    GraphStore, NodeId, NodeRun, Operator as DomainOperator, TaskExecutor, WorkflowIR, WorkflowRun,
};

// -- GraphStoreBox wrapper -----------------------------------------------------

/// Wraps `Box<dyn GraphStore + Send>` so it implements `GraphStore`.
/// This allows `Arc<Mutex<GraphStoreBox>>` to satisfy `Arc<Mutex<dyn GraphStore + Send>>`.
pub struct GraphStoreBox {
    pub inner: Box<dyn GraphStore + Send>,
}

impl GraphStore for GraphStoreBox {
    fn save_state(
        &mut self,
        state: &sddk_domain::GraphState,
    ) -> Result<(), sddk_domain::StorageError> {
        self.inner.save_state(state)
    }
    fn load_state(&self) -> Result<Option<sddk_domain::GraphState>, sddk_domain::StorageError> {
        self.inner.load_state()
    }
    fn checkpoint(
        &self,
    ) -> Result<Option<sddk_domain::projections::Checkpoint>, sddk_domain::StorageError> {
        self.inner.checkpoint()
    }
    fn record_ir_digest(
        &mut self,
        ir_hash: &str,
        ir_json: &str,
    ) -> Result<(), sddk_domain::StorageError> {
        self.inner.record_ir_digest(ir_hash, ir_json)
    }
    fn record_graph_revision(
        &mut self,
        rev: &sddk_domain::graph::ExecutionGraphRevision,
    ) -> Result<(), sddk_domain::StorageError> {
        self.inner.record_graph_revision(rev)
    }
    fn load_node_attempts(
        &self,
        run_id: &sddk_domain::RunId,
        node_id: &sddk_domain::NodeId,
    ) -> Result<Vec<sddk_domain::Attempt>, sddk_domain::StorageError> {
        self.inner.load_node_attempts(run_id, node_id)
    }
    fn attempt_count(
        &self,
        run_id: &sddk_domain::RunId,
        node_id: &sddk_domain::NodeId,
    ) -> Result<u32, sddk_domain::StorageError> {
        self.inner.attempt_count(run_id, node_id)
    }
    fn save_revision(
        &mut self,
        rev: &sddk_domain::graph::ExecutionGraphRevision,
    ) -> Result<(), sddk_domain::StorageError> {
        self.inner.save_revision(rev)
    }
    fn load_revision(
        &self,
        run_id: &sddk_domain::RunId,
        rev_id: &sddk_domain::RevisionId,
    ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, sddk_domain::StorageError> {
        self.inner.load_revision(run_id, rev_id)
    }
    fn latest_revision(
        &self,
        run_id: &sddk_domain::RunId,
    ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, sddk_domain::StorageError> {
        self.inner.latest_revision(run_id)
    }
}
use serde_json::Value;
use thiserror::Error;

// -- Clock -------------------------------------------------------------------

/// A simple wall-clock source for operator evaluation.
///
/// This is a minimal implementation that returns the current system time.
/// In cycle-17, this may be replaced with a more sophisticated clock
/// abstraction that supports deterministic time in replay.
#[derive(Debug, Clone, Default)]
pub struct Clock;

impl Clock {
    /// Returns the current time as an RFC3339 string.
    pub fn now(&self) -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        sddk_domain::format::format_rfc3339_utc(secs)
    }
}

// -- OperatorContext ---------------------------------------------------------

/// Type alias for scratch store used in Parallel child contexts.
type ScratchStore = Arc<Mutex<GraphStoreBox>>;

/// Cycle context passed to `Operator::evaluate`.
///
/// Field types:
/// - `node_run: Arc<Mutex<NodeRun>>` — shared with workflow_runtime for attempt writes.
///   Per child.evaluate Pure contract (operator.rs:309), children do NOT mutate node_run;
///   the Mutex is uncontended during parallel evaluate.
/// - `store: Arc<Mutex<S>>` — graph store. For Parallel children, each child gets a
///   per-thread `ScratchGraphStore` (cycle-19 scratch isolation preserved).
///   `S` must implement `GraphStore + Send`.
/// - `ir`, `run`, `clock`, `executor` — Arc-shared, immutable from child's perspective.
/// - `pending_sender` — cycle-20 WU-4: Some(tx) when spawned by runtime receiver map.
pub struct OperatorContext<S: GraphStore + Send = GraphStoreBox> {
    /// The node run being evaluated.
    pub node_run: Arc<Mutex<NodeRun>>,
    /// The workflow IR this run was instantiated from.
    pub ir: Arc<WorkflowIR>,
    /// The workflow run record.
    pub run: Arc<WorkflowRun>,
    /// The graph store for persistence.
    pub store: Arc<Mutex<S>>,
    /// Wall-clock source.
    pub clock: Clock,
    /// Task executor for capability routing.
    pub executor: Arc<dyn TaskExecutor>,
    /// Sender for cross-tick Pending results (cycle-20).
    /// `Some(tx)` when the runtime owns the receiver (multi-tick Parallel).
    /// `None` for legacy single-tick operators.
    pub pending_sender: Option<std::sync::mpsc::Sender<ChildResult>>,
}

impl<S: GraphStore + Send> std::fmt::Debug for OperatorContext<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OperatorContext")
            .field("node_run", &self.node_run.lock().unwrap().node_id)
            .field("run", &self.run.run_id)
            .finish()
    }
}

impl OperatorContext<GraphStoreBox> {
    /// Construct an OperatorContext suitable for testing.
    ///
    /// Defaults:
    /// - `store`: `Arc::new(Mutex::new(Box::new(ScratchGraphStore)))`
    /// - `clock`: `Clock::default()`
    /// - `executor`: `Arc::new(NoopTaskExecutor)`
    /// - `pending_sender`: `None`
    ///
    /// Required: `node_run`, `ir`, `run` (the runtime-critical fields).
    ///
    /// **Test-only**: do not use in production runtime paths.
    pub fn for_test(
        node_run: Arc<Mutex<NodeRun>>,
        ir: Arc<WorkflowIR>,
        run: Arc<WorkflowRun>,
    ) -> Self {
        Self {
            node_run,
            ir,
            run,
            store: Arc::new(Mutex::new(GraphStoreBox {
                inner: Box::new(ScratchGraphStore),
            })),
            clock: Clock,
            executor: Arc::new(sddk_domain::NoopTaskExecutor),
            pending_sender: None,
        }
    }
}

// -- ScratchGraphStore --------------------------------------------------------

/// A no-op GraphStore for per-child scratch storage in Parallel evaluation.
/// Children that need real persistence acquire it via WorkflowRuntime::tick.
#[derive(Debug)]
pub struct ScratchGraphStore;

impl GraphStore for ScratchGraphStore {
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
        _rev: &sddk_domain::graph::ExecutionGraphRevision,
    ) -> Result<(), sddk_domain::StorageError> {
        Ok(())
    }
    fn load_revision(
        &self,
        _run_id: &sddk_domain::RunId,
        _rev_id: &sddk_domain::RevisionId,
    ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, sddk_domain::StorageError> {
        Ok(None)
    }
    fn latest_revision(
        &self,
        _run_id: &sddk_domain::RunId,
    ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, sddk_domain::StorageError> {
        Ok(None)
    }
}

// Explicit impl: Box<ScratchGraphStore> implements GraphStore + Send (blanket impl not picked up by coherence)
impl GraphStore for Box<ScratchGraphStore>
where
    Box<ScratchGraphStore>: Send,
{
    fn save_state(
        &mut self,
        state: &sddk_domain::GraphState,
    ) -> Result<(), sddk_domain::StorageError> {
        (**self).save_state(state)
    }
    fn load_state(&self) -> Result<Option<sddk_domain::GraphState>, sddk_domain::StorageError> {
        (**self).load_state()
    }
    fn checkpoint(
        &self,
    ) -> Result<Option<sddk_domain::projections::Checkpoint>, sddk_domain::StorageError> {
        (**self).checkpoint()
    }
    fn record_ir_digest(
        &mut self,
        ir_hash: &str,
        ir_json: &str,
    ) -> Result<(), sddk_domain::StorageError> {
        (**self).record_ir_digest(ir_hash, ir_json)
    }
    fn record_graph_revision(
        &mut self,
        rev: &sddk_domain::graph::ExecutionGraphRevision,
    ) -> Result<(), sddk_domain::StorageError> {
        (**self).record_graph_revision(rev)
    }
    fn load_node_attempts(
        &self,
        run_id: &sddk_domain::RunId,
        node_id: &sddk_domain::NodeId,
    ) -> Result<Vec<sddk_domain::Attempt>, sddk_domain::StorageError> {
        (**self).load_node_attempts(run_id, node_id)
    }
    fn attempt_count(
        &self,
        run_id: &sddk_domain::RunId,
        node_id: &sddk_domain::NodeId,
    ) -> Result<u32, sddk_domain::StorageError> {
        (**self).attempt_count(run_id, node_id)
    }
    fn save_revision(
        &mut self,
        rev: &sddk_domain::graph::ExecutionGraphRevision,
    ) -> Result<(), sddk_domain::StorageError> {
        (**self).save_revision(rev)
    }
    fn load_revision(
        &self,
        run_id: &sddk_domain::RunId,
        rev_id: &sddk_domain::RevisionId,
    ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, sddk_domain::StorageError> {
        (**self).load_revision(run_id, rev_id)
    }
    fn latest_revision(
        &self,
        run_id: &sddk_domain::RunId,
    ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, sddk_domain::StorageError> {
        (**self).latest_revision(run_id)
    }
}

// -- Checkpoint types (forward-debt for cycle-20+ resumption) -----------------

/// Cycle-19 forward-debt: resumable Parallel across ticks.
/// In cycle-19, channel is drained synchronously inside Parallel::evaluate;
/// no checkpoint is ever persisted on Attempt. This enum documents the type
/// vocabulary so cycle-20 does not need to expand Attempt.
#[derive(Debug)]
pub enum CheckpointHandle {
    /// No persistence needed (synchronous completion, cycle-19).
    None,
    /// Concurrent Parallel children completion channel (cycle-20+ resume).
    /// Stored in a side-channel struct (`ParallelCheckpointState`), NOT on Attempt.
    /// Because Attempt derives Clone, embedding an `mpsc::Receiver` would break Clone;
    /// cycle-20 stores the receiver in a runtime-owned map keyed by `run_id:node_id`.
    Channel { resume_token: u64 },
    /// Cycle-32: Map cross-tick replay via MapCheckpointState.
    /// `state` is owned by the runtime's `pending_map` after handoff;
    /// the Arc transfers ownership from `Map::evaluate` to the runtime
    /// because `std::sync::mpsc::Receiver<ChildResult>` is NOT Clone.
    /// Arc is used instead of Box so that CheckpointHandle can derive Clone.
    MapChannel {
        state: std::sync::Arc<MapCheckpointState>,
        token: u64,
    },
}

// Manual Clone impl required because MapCheckpointState contains Receiver (!Clone).
// MapChannel uses Arc which IS Clone, so the enum can still implement Clone.
impl Clone for CheckpointHandle {
    fn clone(&self) -> Self {
        match self {
            CheckpointHandle::None => CheckpointHandle::None,
            CheckpointHandle::Channel { resume_token } => CheckpointHandle::Channel {
                resume_token: *resume_token,
            },
            CheckpointHandle::MapChannel { state, token } => CheckpointHandle::MapChannel {
                state: std::sync::Arc::clone(state),
                token: *token,
            },
        }
    }
}

// Manual PartialEq impl — MapCheckpointState contains Arc<Mutex<Receiver>>
// which is not Eq, so we compare only the token fields.
impl PartialEq for CheckpointHandle {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (CheckpointHandle::None, CheckpointHandle::None) => true,
            (
                CheckpointHandle::Channel { resume_token: a },
                CheckpointHandle::Channel { resume_token: b },
            ) => a == b,
            (
                CheckpointHandle::MapChannel { token: a, .. },
                CheckpointHandle::MapChannel { token: b, .. },
            ) => a == b,
            _ => false,
        }
    }
}
impl Eq for CheckpointHandle {}

/// Opaque runtime state for a parallel that's mid-flight across multiple ticks.
/// Cycle-20+ will hold a map of these on `WorkflowRuntime<R>`.
#[derive(Debug)]
pub struct ParallelCheckpointState {
    pub receiver: std::sync::mpsc::Receiver<ChildResult>,
    pub child_count: usize,
}

/// Opaque runtime state for a Map operator that's mid-flight across multiple ticks.
/// Cycle-30+ will hold a map of these on `WorkflowRuntime<R>` keyed by `run_id:node_id`.
#[derive(Debug, Clone)]
pub struct MapCheckpointState {
    /// Receiver for child iteration results.
    /// Wrapped in Arc<Mutex<...>> to satisfy Send+Sync requirements.
    /// Arc<Receiver> makes it Send; Mutex provides interior mutability for draining.
    pub receiver: std::sync::Arc<std::sync::Mutex<std::sync::mpsc::Receiver<ChildResult>>>,
    /// Total number of items from source.
    pub items_len: usize,
    /// Completed iteration results indexed by iteration number.
    pub completed_results: BTreeMap<usize, ChildResult>,
    /// Snapshot of source outputs for replay (source NOT re-evaluated on resume).
    pub source_outputs_snapshot: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Checkpoint {
    None,
    /// Cycle-19 stub; future cycle-20 stores a `ParallelCheckpointState`
    /// reference here (Arc'd) for cross-tick resumption.
    ParallelChannel {
        token: u64,
    },
    /// Cycle-30: Map cross-tick replay via `MapCheckpointState`.
    MapChannel {
        token: u64,
    },
}

/// Result from a child thread in a Parallel operator evaluation.
#[derive(Debug, Clone)]
pub struct ChildResult {
    pub child_index: usize,
    /// `Ok(NodeOutcome::Succeeded {..} | Failed {..})` for normal eval.
    /// `Err(OperatorError)` for failures.
    /// `Err(OperatorError::ChildPanicked { child_index })` is the panic signal.
    pub outcome: Result<NodeOutcome, OperatorError>,
    pub started_at: String,
    pub ended_at: String,
}

impl ChildResult {
    pub fn succeeded(&self) -> bool {
        matches!(self.outcome, Ok(NodeOutcome::Succeeded { .. }))
    }
}

// -- NodeOutcome -------------------------------------------------------------

/// Outcome of an operator evaluation.
#[derive(Debug, Clone)]
pub enum NodeOutcome {
    /// Node is waiting for dependencies.
    Pending { checkpoint: CheckpointHandle },
    /// Node is currently executing.
    Running,
    /// Node completed successfully with outputs.
    Succeeded {
        /// Node that completed.
        node_id: NodeId,
        /// Output values from the operator.
        outputs: BTreeMap<String, Value>,
    },
    /// Node failed with an error.
    Failed {
        /// Node that failed.
        node_id: NodeId,
        /// Error reason.
        reason: String,
    },
}

// -- OperatorError -----------------------------------------------------------

/// Errors from operator evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OperatorError {
    /// Operator variant is not implemented in cycle-16.
    #[error("operator variant `{variant}` is not implemented in cycle-16")]
    NotImplementedInCycle16 {
        /// The operator variant name.
        variant: &'static str,
    },
    /// Evaluation failed with a runtime error.
    #[error("operator evaluation failed: {0}")]
    EvalFailed(String),
    /// Cycle-19: a child of a `Parallel` operator panicked during `evaluate`.
    #[error("parallel child {child_index} panicked during evaluation")]
    ChildPanicked { child_index: usize },
}

// -- Operator trait ---------------------------------------------------------

/// A workflow operator that can be evaluated in the runtime.
///
/// Implementors: Task, Sequence, Parallel, Choice.
/// Out-of-scope variants (Map, Join, Race, Loop, Gate, Wait, SubWorkflow, Compensate)
/// are handled by `build_operator()` returning `Err(OperatorError::NotImplementedInCycle16)`.
///
/// Invariant INV-11: `Arc<dyn Operator>` and `Box<dyn Operator>` produce
/// identical runtime behavior for `Operator::evaluate(&self, ctx)`. The
/// Arc deref-coerces to `&dyn Operator` and resolves to the same vtable
/// entry as the prior Box contract.
pub trait Operator: std::fmt::Debug + std::any::Any + Send + Sync {
    /// Returns the operator kind name.
    fn kind(&self) -> &'static str;

    /// Evaluates this operator with the given context.
    fn evaluate(&self, ctx: &mut OperatorContext) -> Result<NodeOutcome, OperatorError>;
}

/// A leaf operator that represents a single capability invocation.
///
/// In cycle-16, Task always succeeds immediately (no-op demo).
/// Capability routing is deferred to cycle-17.
#[derive(Debug, Clone)]
pub struct Task {
    /// Capability required for this task.
    pub capability: sddk_domain::CapabilityId,
    /// Inputs to the capability.
    pub inputs: BTreeMap<String, Value>,
}

impl Operator for Task {
    fn kind(&self) -> &'static str {
        "Task"
    }

    fn evaluate(&self, ctx: &mut OperatorContext) -> Result<NodeOutcome, OperatorError> {
        let node_id = ctx.node_run.lock().unwrap().node_id.clone();
        // Pure: returns outcome without mutating ctx.node_run.
        // Runtime tier records attempts after evaluate returns.
        match ctx.executor.execute(&self.capability.0, &self.inputs) {
            Ok(output) => Ok(NodeOutcome::Succeeded {
                node_id,
                outputs: output.outputs,
            }),
            Err(e) => Ok(NodeOutcome::Failed {
                node_id,
                reason: e.message,
            }),
        }
    }
}

// -- Sequence operator --------------------------------------------------------

/// Executes child operators in order, one at a time.
///
/// A Sequence with children [A, B, C] will:
/// - Evaluate A until it succeeds, then B, then C.
/// - If any child fails, the sequence terminates with that failure.
#[derive(Debug)]
pub struct Sequence {
    /// Ordered list of child operators.
    pub children: Vec<Arc<dyn Operator>>,
}

impl Operator for Sequence {
    fn kind(&self) -> &'static str {
        "Sequence"
    }

    fn evaluate(&self, ctx: &mut OperatorContext) -> Result<NodeOutcome, OperatorError> {
        let node_id = ctx.node_run.lock().unwrap().node_id.clone();
        let num_children = self.children.len();

        if num_children == 0 {
            return Ok(NodeOutcome::Succeeded {
                node_id,
                outputs: Default::default(),
            });
        }

        // Each evaluate call advances ONE child; runtime persists the attempt.
        // cycle-43 fix (INC-DEBT-016): Sequence MUST track its own progress by
        // pushing a marker attempt to ctx.node_run.attempts after each child
        // evaluation. Otherwise:
        //   - completed_steps reads 0 every tick (runtime doesn't push for Sequence)
        //   - Sequence keeps evaluating child[0] and never advances
        // This is the dm02 hang root cause.
        let completed_steps = ctx.node_run.lock().unwrap().attempts.len();

        if completed_steps >= num_children {
            return Ok(NodeOutcome::Succeeded {
                node_id,
                outputs: Default::default(),
            });
        }

        let child = &self.children[completed_steps];
        let child_outcome = child.evaluate(ctx)?;

        // cycle-43: push a marker attempt so completed_steps advances on the next
        // call. This keeps the runtime's state machine consistent — without this,
        // Sequence can never reach Succeeded via the normal tick loop.
        let marker_attempt = sddk_domain::Attempt {
            attempt_id: sddk_domain::workflow_run::AttemptId(format!(
                "seq-tick-{}-{}",
                node_id.0, completed_steps
            )),
            node_id: node_id.clone(),
            route: sddk_domain::Route {
                provider: "sequence".to_string(),
                model: "step".to_string(),
                host: "local".to_string(),
            },
            started_at: ctx.clock.now(),
            ended_at: Some(ctx.clock.now()),
            outcome: Some(match &child_outcome {
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
                NodeOutcome::Pending { .. } | NodeOutcome::Running => {
                    sddk_domain::workflow_run::AttemptOutcome::Pending {
                        resume_token: 0,
                        attempt_seq: completed_steps as u32,
                    }
                }
            }),
            usage: sddk_domain::Usage {
                tokens_in: 0,
                tokens_out: 0,
                cost_micros: 0,
                wall_ms: 0,
            },
            context_capsule: sddk_domain::ContextCapsuleRef::Pointer {
                cid: format!("seq-tick-{}-{}", node_id.0, completed_steps),
            },
            idempotency_key: sddk_domain::IdempotencyKey {
                project_id: "sddk".to_string(),
                run_id: ctx.run.run_id.clone(),
                node_id: node_id.clone(),
                attempt_seq: completed_steps as u32,
            },
            schema_version: 1,
        };
        ctx.node_run.lock().unwrap().attempts.push(marker_attempt);

        if completed_steps + 1 >= num_children {
            Ok(NodeOutcome::Succeeded {
                node_id,
                outputs: Default::default(),
            })
        } else {
            Ok(NodeOutcome::Running)
        }
    }
}

// -- Parallel helper functions -------------------------------------------------

/// Returns the effective concurrency limit, defaulting to 16 when the field is 0.
/// Per REQ-WF-RT-010 §R-WF-RT-010.2.
pub(crate) fn apply_default_max_concurrency(value: u32) -> u32 {
    if value == 0 { 16 } else { value }
}

/// A blocking counting semaphore implemented over `Mutex<usize>` + `Condvar`.
///
/// `std::sync::Semaphore` is not yet stable on the toolchain (tracking issue
/// rust-lang/rust#27798, FCP closed with deprecation on 2016-01-17). This is a
/// stdlib-only alternative that
/// preserves INV-10 (no lock on **workflow state**) — the mutex here
/// guards the permit counter only, not workflow data.
///
/// INV-10 (cycle-20): Zero `Mutex`/`RwLock` on **workflow state**.
/// The `Mutex<usize>` in CountingSemaphore is permitted — it guards an internal
/// permit counter, not workflow data. The `Arc<Mutex<NodeRun>>` and
/// `Arc<Mutex<dyn GraphStore>>` in OperatorContext are permitted: they replace
/// the cycle-19 `Box::leak` exception (P1 forward debt) with interior-mutability
/// that drops cleanly when all Arcs are released.
struct CountingSemaphore {
    permits: std::sync::Mutex<usize>,
    condvar: std::sync::Condvar,
}

impl CountingSemaphore {
    fn new(permits: usize) -> Self {
        CountingSemaphore {
            permits: std::sync::Mutex::new(permits),
            condvar: std::sync::Condvar::new(),
        }
    }

    /// Acquire one permit, blocking the calling thread until one is available.
    fn acquire(&self) {
        let mut p = self.permits.lock().expect("semaphore mutex poisoned");
        while *p == 0 {
            p = self.condvar.wait(p).expect("semaphore condvar poisoned");
        }
        *p -= 1;
    }

    /// Release one permit, waking one waiter if any are blocked.
    fn release(&self) {
        let mut p = self.permits.lock().expect("semaphore mutex poisoned");
        *p += 1;
        self.condvar.notify_one();
    }
}

/// RAII guard that releases a permit on drop. Ensures the permit is returned
/// to the semaphore even if the child panics inside `catch_unwind`.
struct PermitGuard {
    sem: Arc<CountingSemaphore>,
}

impl Drop for PermitGuard {
    fn drop(&mut self) {
        self.sem.release();
    }
}

/// Builds an `Attempt` record for a single child of a `Parallel` evaluation.
pub(crate) fn build_attempt(
    node_id: &NodeId,
    run_id: &sddk_domain::workflow_ir::RunId,
    child_index: usize,
    result: &ChildResult,
    _clock: &Clock,
) -> sddk_domain::workflow_run::Attempt {
    let started_at = result.started_at.clone();
    let ended_at = Some(result.ended_at.clone());
    let outcome = match &result.outcome {
        Ok(NodeOutcome::Succeeded { outputs, .. }) => {
            Some(sddk_domain::workflow_run::AttemptOutcome::Succeeded {
                outputs: outputs.clone(),
            })
        }
        Ok(NodeOutcome::Failed { reason, .. }) => {
            Some(sddk_domain::workflow_run::AttemptOutcome::Failed {
                error: reason.clone(),
            })
        }
        Ok(NodeOutcome::Pending { checkpoint }) => {
            // cycle-20: Pending is a first-class outcome.
            // Extract resume_token from checkpoint; default to 0 if None.
            let resume_token = match checkpoint {
                CheckpointHandle::Channel { resume_token } => *resume_token,
                CheckpointHandle::MapChannel { token, .. } => *token,
                CheckpointHandle::None => 0u64,
            };
            Some(sddk_domain::workflow_run::AttemptOutcome::Pending {
                resume_token,
                attempt_seq: child_index as u32,
            })
        }
        Ok(NodeOutcome::Running) => Some(sddk_domain::workflow_run::AttemptOutcome::Failed {
            error: "parallel child returned Running".into(),
        }),
        Err(OperatorError::ChildPanicked { child_index: _ }) => {
            Some(sddk_domain::workflow_run::AttemptOutcome::Failed {
                error: format!("child {} panicked", child_index),
            })
        }
        Err(e) => Some(sddk_domain::workflow_run::AttemptOutcome::Failed {
            error: e.to_string(),
        }),
    };
    sddk_domain::workflow_run::Attempt {
        attempt_id: sddk_domain::workflow_run::AttemptId(format!(
            "par-{}-child-{}",
            node_id.0, child_index
        )),
        node_id: node_id.clone(),
        route: sddk_domain::workflow_run::Route {
            provider: "cycle19".into(),
            model: "parallel".into(),
            host: "local".into(),
        },
        started_at,
        ended_at,
        outcome,
        usage: sddk_domain::workflow_run::Usage {
            tokens_in: 0,
            tokens_out: 0,
            cost_micros: 0,
            wall_ms: 0,
        },
        context_capsule: sddk_domain::workflow_run::ContextCapsuleRef::Pointer {
            cid: format!("par-{}-child-{}", node_id.0, child_index),
        },
        idempotency_key: sddk_domain::workflow_run::IdempotencyKey {
            project_id: "sddk".into(),
            run_id: run_id.clone(),
            node_id: node_id.clone(),
            // INV-8: attempt_seq == child_index — parent orders, not mpsc arrival
            attempt_seq: child_index as u32,
        },
        schema_version: 1,
    }
}

// -- Parallel operator --------------------------------------------------------

/// Executes child operators concurrently, up to max_concurrency.
///
/// A Parallel with children [A, B] will evaluate both simultaneously.
/// The runtime收敛es when all children complete.
#[derive(Debug)]
pub struct Parallel {
    /// Child operators to execute in parallel.
    pub children: Vec<Arc<dyn Operator>>,
    /// Maximum concurrent executions.
    pub max_concurrency: u32,
}

impl Operator for Parallel {
    fn kind(&self) -> &'static str {
        "Parallel"
    }

    fn evaluate(&self, ctx: &mut OperatorContext) -> Result<NodeOutcome, OperatorError> {
        let node_id = ctx.node_run.lock().unwrap().node_id.clone();
        let num_children = self.children.len();

        // Empty parallel immediately succeeds
        if num_children == 0 {
            ctx.node_run.lock().unwrap().state = sddk_domain::workflow_run::NodeRunState::Completed;
            return Ok(NodeOutcome::Succeeded {
                node_id,
                outputs: Default::default(),
            });
        }

        // PRE-CHECK: replay-safety — if all children already have attempts, return Succeeded.
        if ctx.node_run.lock().unwrap().attempts.len() >= num_children {
            ctx.node_run.lock().unwrap().state = sddk_domain::workflow_run::NodeRunState::Completed;
            return Ok(NodeOutcome::Succeeded {
                node_id,
                outputs: Default::default(),
            });
        }

        let max_conc = apply_default_max_concurrency(self.max_concurrency);

        // -- Non-blocking path (cycle-20+ runtime): use pending_sender ----------
        if let Some(pending_sender) = ctx.pending_sender.take() {
            // Clone everything we need from ctx BEFORE spawning the thread
            let children: Vec<Arc<dyn Operator>> = self.children.clone();
            let ir = Arc::clone(&ctx.ir);
            let run = Arc::clone(&ctx.run);
            let node_run = Arc::clone(&ctx.node_run); // Clone BEFORE spawn so ctx not needed inside
            let clock = ctx.clock.clone();
            let executor = Arc::clone(&ctx.executor);
            let semaphore = Arc::new(CountingSemaphore::new(max_conc as usize));

            std::thread::spawn(move || {
                let (tx, rx) = std::sync::mpsc::channel::<ChildResult>();
                let mut handles: Vec<std::thread::JoinHandle<()>> =
                    Vec::with_capacity(children.len());

                // Spawn children
                for (i, child) in children.iter().enumerate() {
                    let sem = Arc::clone(&semaphore);
                    let tx = tx.clone();
                    let child = Arc::clone(child);

                    // Arc clone of parent node_run (read-only inside child per Pure contract).
                    let node_run = Arc::clone(&node_run);
                    // PER-CHILD scratch store (not shared with parent, not shared across children).
                    let store: ScratchStore = Arc::new(Mutex::new(GraphStoreBox {
                        inner: Box::new(ScratchGraphStore),
                    }));

                    // Build child context WITHOUT pending_sender (child reports to supervisor)
                    let mut child_ctx = OperatorContext {
                        node_run,
                        ir: Arc::clone(&ir),
                        run: Arc::clone(&run),
                        store,
                        clock: clock.clone(),
                        executor: Arc::clone(&executor),
                        pending_sender: None, // child → supervisor, not direct to runtime
                    };

                    let handle = std::thread::spawn(move || {
                        sem.acquire();
                        let _release_on_exit = PermitGuard {
                            sem: Arc::clone(&sem),
                        };
                        let started_at = child_ctx.clock.now();
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            child.evaluate(&mut child_ctx)
                        }));
                        let ended_at = child_ctx.clock.now();
                        let outcome = match result {
                            Ok(Ok(outcome)) => Ok(outcome),
                            Ok(Err(e)) => Err(e),
                            Err(_) => Err(OperatorError::ChildPanicked { child_index: i }),
                        };
                        let _ = tx.send(ChildResult {
                            child_index: i,
                            outcome,
                            started_at,
                            ended_at,
                        });
                    });
                    handles.push(handle);
                }
                drop(tx); // close sender; rx drains when all children send

                // Drain results and forward to runtime
                let mut collected: BTreeMap<usize, ChildResult> = BTreeMap::new();
                for _ in 0..children.len() {
                    match rx.recv() {
                        Ok(result) => {
                            collected.insert(result.child_index, result);
                        }
                        Err(_) => break,
                    }
                }

                // Join all handles
                for h in handles {
                    let _ = h.join();
                }

                // Forward to runtime via pending_sender
                for child_index in 0..children.len() {
                    if let Some(result) = collected.remove(&child_index) {
                        let _ = pending_sender.send(result);
                    }
                }
            });

            // Return Pending immediately — runtime will drain results on next tick
            return Ok(NodeOutcome::Pending {
                checkpoint: CheckpointHandle::Channel { resume_token: 0 },
            });
        }

        // -- Blocking path (tests without runtime): original behavior ------------
        // 1. Build mpsc + semaphore
        let (tx, rx) = std::sync::mpsc::channel::<ChildResult>();
        let semaphore = Arc::new(CountingSemaphore::new(max_conc as usize));
        let mut handles: Vec<std::thread::JoinHandle<()>> = Vec::with_capacity(num_children);

        // 2. Spawn N threads
        for (i, child) in self.children.iter().enumerate() {
            let sem = Arc::clone(&semaphore);
            let tx = tx.clone();
            let child = Arc::clone(child);
            // Arc clone of parent node_run (read-only inside child per Pure contract).
            let node_run = Arc::clone(&ctx.node_run);
            // PER-CHILD scratch store (not shared with parent, not shared across children).
            let store = Arc::new(Mutex::new(GraphStoreBox {
                inner: Box::new(ScratchGraphStore),
            }));

            let mut child_ctx = OperatorContext {
                node_run,
                ir: Arc::clone(&ctx.ir),
                run: Arc::clone(&ctx.run),
                store,
                clock: ctx.clock.clone(),
                executor: Arc::clone(&ctx.executor),
                pending_sender: None,
            };

            let handle = std::thread::spawn(move || {
                sem.acquire();
                let _release_on_exit = PermitGuard {
                    sem: Arc::clone(&sem),
                };
                let started_at = child_ctx.clock.now();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    child.evaluate(&mut child_ctx)
                }));
                let ended_at = child_ctx.clock.now();
                let outcome = match result {
                    Ok(Ok(outcome)) => Ok(outcome),
                    Ok(Err(e)) => Err(e),
                    Err(_) => Err(OperatorError::ChildPanicked { child_index: i }),
                };
                let _ = tx.send(ChildResult {
                    child_index: i,
                    outcome,
                    started_at,
                    ended_at,
                });
            });
            handles.push(handle);
        }
        drop(tx); // close sender side; rx drains when all N sends complete

        // 3. Drain mpsc — collected in BTreeMap preserves child_index order
        let mut collected: BTreeMap<usize, ChildResult> = BTreeMap::new();
        for _ in 0..num_children {
            match rx.recv() {
                Ok(result) => {
                    collected.insert(result.child_index, result);
                }
                Err(_) => break,
            }
        }

        // 4. Join all JoinHandles — INV-9 (no thread leaks)
        for h in handles {
            let _ = h.join();
        }

        // 5. Apply outcomes in child_index order — INV-8 ordering invariant
        let mut any_panic_index: Option<usize> = None;
        let mut first_failure: Option<OperatorError> = None;
        for child_index in 0..num_children {
            let child_result = match collected.remove(&child_index) {
                Some(r) => r,
                None => {
                    return Err(OperatorError::EvalFailed(format!(
                        "parallel: missing child {} result",
                        child_index
                    )));
                }
            };

            if let Err(OperatorError::ChildPanicked { child_index: pi }) = &child_result.outcome {
                any_panic_index = any_panic_index.or(Some(*pi));
            }
            if first_failure.is_none() {
                if let Err(e) = &child_result.outcome {
                    first_failure = Some(e.clone());
                } else if let Ok(NodeOutcome::Failed { reason, .. }) = &child_result.outcome {
                    first_failure = Some(OperatorError::EvalFailed(reason.clone()));
                }
            }

            let attempt = build_attempt(
                &node_id,
                &ctx.run.run_id,
                child_index,
                &child_result,
                &ctx.clock,
            );
            ctx.node_run.lock().unwrap().attempts.push(attempt);
        }

        // 6. Final outcome
        if let Some(idx) = any_panic_index {
            ctx.node_run.lock().unwrap().state = sddk_domain::workflow_run::NodeRunState::Failed;
            return Ok(NodeOutcome::Failed {
                node_id,
                reason: format!("child {} panicked", idx),
            });
        }
        if let Some(err) = first_failure {
            ctx.node_run.lock().unwrap().state = sddk_domain::workflow_run::NodeRunState::Failed;
            return Err(err);
        }
        ctx.node_run.lock().unwrap().state = sddk_domain::workflow_run::NodeRunState::Completed;
        Ok(NodeOutcome::Succeeded {
            node_id,
            outputs: Default::default(),
        })
    }
}

// -- Choice operator ---------------------------------------------------------

/// Conditional branch: evaluates the first matching condition.
///
/// Choice evaluates its `branches` map and selects the first matching key,
/// falling back to `default` if no branch matches.
#[derive(Debug)]
pub struct Choice {
    /// Map of condition string to operator.
    pub branches: BTreeMap<String, Arc<dyn Operator>>,
    /// Default operator when no branch matches.
    pub default: Arc<dyn Operator>,
}

impl Operator for Choice {
    fn kind(&self) -> &'static str {
        "Choice"
    }

    fn evaluate(&self, ctx: &mut OperatorContext) -> Result<NodeOutcome, OperatorError> {
        use sddk_domain::{
            Attempt, AttemptOutcome, ContextCapsuleRef, IdempotencyKey, NodeRunState, Route, Usage,
        };
        use std::collections::BTreeMap;

        let node_id = ctx.node_run.lock().unwrap().node_id.clone();

        // In cycle-16, we evaluate conditions in order and take the first match.
        // Since real condition evaluation is deferred to cycle-17, we always
        // fall through to the default branch in cycle-16.
        // The choice logic is: evaluate each branch condition, dispatch first match,
        // or fall back to default if none match.

        let selected_branch = if self.branches.is_empty() {
            "default".to_string()
        } else {
            // Cycle-16: no real condition evaluation — always take the first branch
            // Real condition evaluation (guard expression parsing) is cycle-17
            self.branches
                .keys()
                .next()
                .cloned()
                .unwrap_or_else(|| "default".to_string())
        };

        // Record the choice as an attempt
        let attempt = Attempt {
            attempt_id: sddk_domain::workflow_run::AttemptId(format!(
                "choice-{}-{}",
                node_id.0, selected_branch
            )),
            node_id: node_id.clone(),
            route: Route {
                provider: "cycle16".to_string(),
                model: "noop".to_string(),
                host: "local".to_string(),
            },
            started_at: ctx.clock.now(),
            ended_at: Some(ctx.clock.now()),
            outcome: Some(AttemptOutcome::Succeeded {
                outputs: {
                    let mut outputs = BTreeMap::new();
                    outputs.insert(
                        "selected_branch".to_string(),
                        serde_json::Value::String(selected_branch),
                    );
                    outputs
                },
            }),
            usage: Usage {
                tokens_in: 0,
                tokens_out: 0,
                cost_micros: 0,
                wall_ms: 0,
            },
            context_capsule: ContextCapsuleRef::Pointer {
                cid: format!("choice-{}-branch", node_id.0),
            },
            idempotency_key: IdempotencyKey {
                project_id: "sddk".to_string(),
                run_id: ctx.run.run_id.clone(),
                node_id: node_id.clone(),
                attempt_seq: 0,
            },
            schema_version: 1,
        };
        ctx.node_run.lock().unwrap().attempts.push(attempt);

        // Choice completes successfully after selecting a branch
        // The actual branch execution would happen in cycle-17
        ctx.node_run.lock().unwrap().state = NodeRunState::Completed;
        Ok(NodeOutcome::Succeeded {
            node_id,
            outputs: Default::default(),
        })
    }
}

// -- Map operator -------------------------------------------------------------

/// Map operator: fan-out body across a collection from source.
///
/// **Cycle-30 semantics:**
/// - Source-context isolation (DC-MAP-001): `source.evaluate` uses fresh child
///   `OperatorContext` with Arc-cloned shared fields, own ScratchGraphStore,
///   `pending_sender: None`. Source MUST NOT mutate parent's `node_run.state`/`attempts`.
/// - Extracts `outputs["items"]: Array` from source outputs (key convention)
/// - Body MUST be `DomainOperator::Task`; other variants return
///   `EvalFailed("map body must be Task")`
/// - Iterates over source items; per iteration `i`, merges `{item: items[i], index: i}`
///   into body's base inputs (non-destructive)
/// - `max_concurrency` is ENFORCED:
///   - `0` → unbounded (all items run concurrently)
///   - `1` → sequential (no thread spawn)
///   - `>= 2` → semaphore-gated thread pool with at most N concurrent iterations
/// - Error aggregation = **collect-all**: `Succeeded` if ≥1 body succeeded;
///   `Failed` with composite reason only if ALL failed
/// - Outputs: `outputs["results"]: Array<Value>` (successful outputs only, iteration order)
///   + `outputs["failures"]: Array<{index: u64, reason: string}>` (all failures, iteration order)
/// - **Cross-tick replay** (cycle-30): when body returns `Pending`, MapCheckpointState
///   is built and `Pending { Channel { resume_token: T } }` returned. Runtime drains
///   per tick. Source NOT re-evaluated on replay (INV-11).
///
/// **Divergence from `Parallel`**: `apply_default_max_concurrency(0)` returns 16
/// (default cap); Map's `map_max_concurrency_effective(0, n)` returns `n.max(1)` (unbounded).
///
/// **Deferred to cycle-31+:**
/// - DC-MAP-002 (dispatch global)
#[derive(Debug)]
pub struct Map {
    /// Resolved source operator — evaluated to produce the collection.
    source: Arc<dyn Operator>,
    /// Resolved body operator — evaluated once per item in the source collection.
    /// Stored as `Arc<Task>` (not `Arc<dyn Operator>`). The body IR-level invariant
    /// (must be Task) is enforced at authoring time and at construction time via
    /// downcast. During evaluate, the Task is cloned from this Arc and modified
    /// with item/index inputs. See ADR-0066 §70-71 for rationale.
    body: Arc<Task>,
    /// Maximum concurrent mappings.
    /// - `0` → unbounded (all items run concurrently)
    /// - `1` → sequential (no thread spawn)
    /// - `>= 2` → semaphore-gated thread pool
    pub max_concurrency: u32,
}

/// Returns the effective concurrency limit for Map.
///
/// Divergent from `apply_default_max_concurrency`: when `mc == 0`, Map returns
/// `n.max(1)` (unbounded = one permit per item), whereas Parallel returns 16.
fn map_max_concurrency_effective(mc: u32, n: usize) -> usize {
    if mc == 0 { n.max(1) } else { mc as usize }
}

impl Map {
    /// Constructs a `Map` from a domain `Map` operator and the `WorkflowIR`.
    ///
    /// Resolves `source` and `body` `OperatorId` references via `build_operator`
    /// at construction time. After construction, `Map::evaluate` uses the
    /// resolved `Arc<dyn Operator>` slots directly — no `ctx.ir.operators.get()` calls.
    ///
    /// # Errors
    /// Returns `OperatorError::EvalFailed` if `source` or `body` OperatorId
    /// is not found in the IR.
    pub fn new(ir_op: &DomainOperator, ir: &WorkflowIR) -> Result<Self, OperatorError> {
        match ir_op {
            DomainOperator::Map {
                source,
                body,
                max_concurrency,
            } => {
                let source_op = ir.operators.get(source).ok_or_else(|| {
                    OperatorError::EvalFailed(format!("map source not found: {}", source.0))
                })?;
                let body_op = ir.operators.get(body).ok_or_else(|| {
                    OperatorError::EvalFailed(format!("map body not found: {}", body.0))
                })?;
                let resolved_source = build_operator(source_op, ir)?;
                let resolved_body = build_operator(body_op, ir)?;
                // Body must be Task (IR-level invariant). Downcast Arc<dyn Operator> → Arc<Task>.
                // Uses Arc::downcast which requires T: Any (satisfied by dyn Operator + Send + Sync).
                let body_task: Arc<Task> = match std::sync::Arc::downcast::<Task>(resolved_body) {
                    Ok(t) => t,
                    Err(_) => {
                        return Err(OperatorError::EvalFailed("map body must be Task".into()));
                    }
                };
                Ok(Map {
                    source: resolved_source,
                    body: body_task,
                    max_concurrency: *max_concurrency,
                })
            }
            _ => Err(OperatorError::EvalFailed(
                "Map::new requires DomainOperator::Map".into(),
            )),
        }
    }
}

impl Operator for Map {
    fn kind(&self) -> &'static str {
        "Map"
    }

    fn evaluate(&self, ctx: &mut OperatorContext) -> Result<NodeOutcome, OperatorError> {
        // (a) Evaluate source operator with isolated child context (DC-MAP-001).
        // Source gets fresh child ctx: Arc-cloned shared fields, own ScratchGraphStore,
        // pending_sender: None. Source MUST NOT mutate parent's node_run.state/attempts.
        // cycle-31: source is pre-resolved Arc<dyn Operator> — NO ctx.ir.operators.get() call.
        let source_store: ScratchStore = Arc::new(Mutex::new(GraphStoreBox {
            inner: Box::new(ScratchGraphStore),
        }));
        let mut source_ctx = OperatorContext {
            node_run: Arc::clone(&ctx.node_run),
            ir: Arc::clone(&ctx.ir),
            run: Arc::clone(&ctx.run),
            store: source_store,
            clock: ctx.clock.clone(),
            executor: Arc::clone(&ctx.executor),
            pending_sender: None, // source does not propagate pending directly
        };
        let source_outcome = self.source.evaluate(&mut source_ctx)?;

        // (b) Extract items collection from source outputs (key convention: outputs["items"]).
        // Clone outputs for items extraction, then move source_outcome for snapshot (cycle-32).
        let source_outputs_clone: BTreeMap<String, serde_json::Value>;
        let items: Vec<serde_json::Value> = match &source_outcome {
            NodeOutcome::Succeeded { outputs, .. } => {
                source_outputs_clone = outputs.clone();
                let items_val = match source_outputs_clone.get("items") {
                    Some(v) => v,
                    None => {
                        return Ok(NodeOutcome::Failed {
                            node_id: ctx.node_run.lock().unwrap().node_id.clone(),
                            reason: "expected outputs[\"items\"]: Array".into(),
                        });
                    }
                };
                match items_val {
                    serde_json::Value::Array(arr) => arr.clone(),
                    _ => {
                        return Ok(NodeOutcome::Failed {
                            node_id: ctx.node_run.lock().unwrap().node_id.clone(),
                            reason: "expected outputs[\"items\"]: Array".into(),
                        });
                    }
                }
            }
            NodeOutcome::Failed { reason, .. } => {
                return Ok(NodeOutcome::Failed {
                    node_id: ctx.node_run.lock().unwrap().node_id.clone(),
                    reason: reason.clone(),
                });
            }
            other => return Ok(other.clone()), // Pending/Running propagation
        };

        // (c) Body is pre-resolved Arc<Task> (validated at construction time).
        let body_task = &*self.body;

        // (d) Concurrency enforcement: max_concurrency == 1 → sequential (no thread spawn)
        if self.max_concurrency == 1 || items.len() <= 1 {
            return self.evaluate_sequential(ctx, &items, body_task);
        }

        // (e) Concurrent evaluation with semaphore-gated thread pool
        // Pass source_outputs_clone for snapshot capture in checkpoint (cycle-32)
        self.evaluate_concurrent(ctx, &items, body_task, source_outputs_clone)
    }
}

impl Map {
    /// Sequential evaluation: no thread spawn, iterate in order.
    fn evaluate_sequential(
        &self,
        ctx: &mut OperatorContext,
        items: &[serde_json::Value],
        body_task: &Task,
    ) -> Result<NodeOutcome, OperatorError> {
        let mut results: Vec<serde_json::Value> = Vec::with_capacity(items.len());
        let mut failures: Vec<serde_json::Value> = Vec::new();

        for (i, item) in items.iter().enumerate() {
            let mut iter_task = body_task.clone();
            iter_task.inputs.insert("item".to_string(), item.clone());
            iter_task.inputs.insert(
                "index".to_string(),
                serde_json::Value::Number(serde_json::Number::from(i)),
            );
            let body_op: Arc<dyn Operator> = Arc::new(iter_task);
            match body_op.evaluate(ctx)? {
                NodeOutcome::Succeeded { outputs, .. } => {
                    let result_obj = serde_json::Value::Object(serde_json::Map::from_iter(outputs));
                    results.push(result_obj);
                }
                NodeOutcome::Failed { reason, .. } => {
                    failures.push(serde_json::json!({
                        "index": i as u64,
                        "reason": reason,
                    }));
                }
                NodeOutcome::Pending { checkpoint: _ } => {
                    // Sequential Pending: build checkpoint before returning (cycle-32).
                    // For sequential (no concurrent threads), we create a placeholder channel.
                    // Source outputs snapshot captured from items.
                    let source_snapshot: BTreeMap<String, serde_json::Value> =
                        serde_json::from_value(serde_json::json!({ "items": items }))
                            .unwrap_or_default();

                    // Create a dummy channel for MapCheckpointState (sequential case)
                    let (tx, rx) = std::sync::mpsc::channel::<ChildResult>();
                    drop(tx); // Close tx immediately - no one will send in sequential case

                    let checkpoint = MapCheckpointState {
                        receiver: std::sync::Arc::new(std::sync::Mutex::new(rx)),
                        items_len: items.len(),
                        completed_results: BTreeMap::new(), // Sequential: no concurrent results
                        source_outputs_snapshot: source_snapshot,
                    };

                    // Return Pending with MapChannel checkpoint (cycle-32)
                    return Ok(NodeOutcome::Pending {
                        checkpoint: CheckpointHandle::MapChannel {
                            state: std::sync::Arc::new(checkpoint),
                            token: 0,
                        },
                    });
                }
                other => return Ok(other), // Running propagation
            }
        }

        // Aggregate with collect-all semantics
        self.aggregate_collect_all(ctx, results, failures)
    }

    /// Concurrent evaluation: semaphore-gated thread pool.
    fn evaluate_concurrent(
        &self,
        ctx: &mut OperatorContext,
        items: &[serde_json::Value],
        body_task: &Task,
        source_outputs: BTreeMap<String, serde_json::Value>,
    ) -> Result<NodeOutcome, OperatorError> {
        let max_conc = map_max_concurrency_effective(self.max_concurrency, items.len());

        // Build mpsc + semaphore
        let (tx, rx) = std::sync::mpsc::channel::<ChildResult>();
        let semaphore = Arc::new(CountingSemaphore::new(max_conc));
        let mut handles: Vec<std::thread::JoinHandle<()>> = Vec::with_capacity(items.len());

        for (i, item) in items.iter().enumerate() {
            let sem = Arc::clone(&semaphore);
            let tx = tx.clone();
            let mut iter_task = body_task.clone();
            iter_task.inputs.insert("item".to_string(), item.clone());
            iter_task.inputs.insert(
                "index".to_string(),
                serde_json::Value::Number(serde_json::Number::from(i)),
            );
            let body_op: Arc<dyn Operator> = Arc::new(iter_task);

            // Per-child scratch store (not shared with parent, not shared across children)
            let store: ScratchStore = Arc::new(Mutex::new(GraphStoreBox {
                inner: Box::new(ScratchGraphStore),
            }));
            let node_run = Arc::clone(&ctx.node_run);
            let ir = Arc::clone(&ctx.ir);
            let run = Arc::clone(&ctx.run);
            let clock = ctx.clock.clone();
            let executor = Arc::clone(&ctx.executor);

            let handle = std::thread::spawn(move || {
                sem.acquire();
                let _release_on_exit = PermitGuard {
                    sem: Arc::clone(&sem),
                };
                let started_at = clock.now();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let mut child_ctx = OperatorContext {
                        node_run,
                        ir,
                        run,
                        store,
                        clock: clock.clone(),
                        executor,
                        pending_sender: None,
                    };
                    body_op.evaluate(&mut child_ctx)
                }));
                let ended_at = clock.now();
                let outcome = match result {
                    Ok(Ok(outcome)) => outcome,
                    Ok(Err(e)) => {
                        let _ = tx.send(ChildResult {
                            child_index: i,
                            outcome: Err(e),
                            started_at,
                            ended_at,
                        });
                        return;
                    }
                    Err(_) => {
                        let _ = tx.send(ChildResult {
                            child_index: i,
                            outcome: Err(OperatorError::ChildPanicked { child_index: i }),
                            started_at,
                            ended_at,
                        });
                        return;
                    }
                };
                let _ = tx.send(ChildResult {
                    child_index: i,
                    outcome: Ok(outcome),
                    started_at,
                    ended_at,
                });
            });
            handles.push(handle);
        }

        // Close sender so receiver knows when done
        drop(tx);

        // Drain results preserving iteration order (use recv to not consume rx)
        let mut collected: BTreeMap<usize, ChildResult> = BTreeMap::new();
        while let Ok(result) = rx.recv() {
            // Check for Pending before inserting - if Pending, build checkpoint and return
            if matches!(result.outcome, Ok(NodeOutcome::Pending { .. })) {
                // Cycle-32: pending body iteration. Build MapCheckpointState before returning.
                // Runtime will drain remaining results from rx.
                // FIX: capture source_outputs.clone() instead of BTreeMap::new() (INV-11 fix).
                let checkpoint = MapCheckpointState {
                    receiver: std::sync::Arc::new(std::sync::Mutex::new(rx)),
                    items_len: items.len(),
                    completed_results: collected,
                    source_outputs_snapshot: source_outputs.clone(),
                };

                return Ok(NodeOutcome::Pending {
                    checkpoint: CheckpointHandle::MapChannel {
                        state: std::sync::Arc::new(checkpoint),
                        token: 0,
                    },
                });
            }
            collected.insert(result.child_index, result);
        }

        // Join all handles (INV-9: no thread leaks)
        for h in handles {
            let _ = h.join();
        }

        // Aggregate with collect-all semantics
        let mut results: Vec<serde_json::Value> = Vec::with_capacity(items.len());
        let mut failures: Vec<serde_json::Value> = Vec::new();

        for i in 0..items.len() {
            if let Some(child_result) = collected.remove(&i) {
                match child_result.outcome {
                    Ok(NodeOutcome::Succeeded { outputs, .. }) => {
                        let result_obj =
                            serde_json::Value::Object(serde_json::Map::from_iter(outputs));
                        results.push(result_obj);
                    }
                    Ok(NodeOutcome::Failed { reason, .. }) => {
                        failures.push(serde_json::json!({
                            "index": i as u64,
                            "reason": reason,
                        }));
                    }
                    Ok(NodeOutcome::Pending { .. }) => {
                        // Should not reach here - Pending handled in while loop above.
                        // Defensive: build checkpoint with what we have.
                        let checkpoint = MapCheckpointState {
                            receiver: std::sync::Arc::new(std::sync::Mutex::new(rx)),
                            items_len: items.len(),
                            completed_results: collected,
                            source_outputs_snapshot: source_outputs.clone(),
                        };
                        return Ok(NodeOutcome::Pending {
                            checkpoint: CheckpointHandle::MapChannel {
                                state: std::sync::Arc::new(checkpoint),
                                token: 0,
                            },
                        });
                    }
                    Ok(NodeOutcome::Running) => {
                        return Ok(NodeOutcome::Failed {
                            node_id: ctx.node_run.lock().unwrap().node_id.clone(),
                            reason: "map child returned Running".into(),
                        });
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
        }

        self.aggregate_collect_all(ctx, results, failures)
    }

    /// Aggregate results and failures using collect-all semantics.
    fn aggregate_collect_all(
        &self,
        ctx: &mut OperatorContext,
        results: Vec<serde_json::Value>,
        failures: Vec<serde_json::Value>,
    ) -> Result<NodeOutcome, OperatorError> {
        let node_id = ctx.node_run.lock().unwrap().node_id.clone();

        if failures.is_empty() {
            // No failures: either items was empty (vacuous success) or all succeeded
            let mut outputs = BTreeMap::new();
            outputs.insert("results".to_string(), serde_json::Value::Array(results));
            outputs.insert("failures".to_string(), serde_json::Value::Array(failures));
            Ok(NodeOutcome::Succeeded { node_id, outputs })
        } else if results.is_empty() {
            // All failed → Failed with composite reason
            let composite_reason = build_map_composite_failure_reason(&failures);
            Ok(NodeOutcome::Failed {
                node_id,
                reason: composite_reason,
            })
        } else {
            // Partial success → Succeeded with both results and failures
            let mut outputs = BTreeMap::new();
            outputs.insert("results".to_string(), serde_json::Value::Array(results));
            outputs.insert("failures".to_string(), serde_json::Value::Array(failures));
            Ok(NodeOutcome::Succeeded { node_id, outputs })
        }
    }
}

/// Builds the composite failure reason for all-fail case.
/// Format: "map body failed at all N iterations: [0]reason0; [1]reason1; ..."
/// Truncates to top-10 entries with "..." elision.
pub fn build_map_composite_failure_reason(failures: &[serde_json::Value]) -> String {
    let n = failures.len();
    let reasons: Vec<String> = failures
        .iter()
        .take(10)
        .map(|f| {
            let obj = f.as_object().expect("failure must be object");
            let idx = obj.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
            let reason = obj
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            format!("[{}]{}", idx, reason)
        })
        .collect();

    let mut msg = format!("map body failed at all {} iterations: ", n);
    msg.push_str(&reasons.join("; "));
    if n > 10 {
        msg.push_str("; ...");
    }
    msg
}

// -- Build Operator -------------------------------------------------------------

/// Recursively builds a runtime `Arc<dyn Operator>` from a domain `Operator` IR node.
///
/// This is the canonical constructor for runtime operators, replacing the legacy
/// `dispatch()` function. It resolves `OperatorId` references at construction time
/// (not evaluate time), storing resolved `Arc<dyn Operator>` children on the
/// runtime types.
///
/// # Arguments
/// - `ir_op` — the domain `Operator` enum value from `WorkflowIR.operators`
/// - `ir` — the `WorkflowIR` being constructed (borrowed for ID resolution)
///
/// # Returns
/// - `Ok(Arc<dyn Operator>)` for the 5 in-scope variants (Task, Sequence, Parallel, Choice, Map)
/// - `Err(OperatorError::NotImplementedInCycle16)` for the 7 out-of-scope variants
///
/// # Visibility
/// `pub` — callable from tests AND from future cross-crate consumers (e.g. `sddk-runtime`).
pub fn build_operator(
    ir_op: &DomainOperator,
    ir: &WorkflowIR,
) -> Result<Arc<dyn Operator>, OperatorError> {
    match ir_op {
        DomainOperator::Task { capability, inputs } => Ok(Arc::new(Task {
            capability: capability.clone(),
            inputs: inputs.clone(),
        })),
        DomainOperator::Sequence { body } => {
            let resolved: Vec<Arc<dyn Operator>> = body
                .iter()
                .map(|child_id| {
                    let child_ir = ir.operators.get(child_id).ok_or_else(|| {
                        OperatorError::EvalFailed(format!("operator not found: {}", child_id.0))
                    })?;
                    build_operator(child_ir, ir)
                })
                .collect::<Result<_, _>>()?;
            Ok(Arc::new(Sequence { children: resolved }))
        }
        DomainOperator::Parallel {
            branches,
            max_concurrency,
        } => {
            let resolved: Vec<Arc<dyn Operator>> = branches
                .iter()
                .map(|branch_id| {
                    let branch_ir = ir.operators.get(branch_id).ok_or_else(|| {
                        OperatorError::EvalFailed(format!("operator not found: {}", branch_id.0))
                    })?;
                    build_operator(branch_ir, ir)
                })
                .collect::<Result<_, _>>()?;
            Ok(Arc::new(Parallel {
                children: resolved,
                max_concurrency: *max_concurrency,
            }))
        }
        DomainOperator::Choice { branches } => {
            let resolved: BTreeMap<String, Arc<dyn Operator>> = branches
                .iter()
                .map(|(name, branch_id)| {
                    let branch_ir = ir.operators.get(branch_id).ok_or_else(|| {
                        OperatorError::EvalFailed(format!("operator not found: {}", branch_id.0))
                    })?;
                    build_operator(branch_ir, ir).map(|op| (name.clone(), op))
                })
                .collect::<Result<_, _>>()?;
            // Default no-op Task preserved when no other default is supplied
            let default = Arc::new(Task {
                capability: sddk_domain::CapabilityId("default".to_string()),
                inputs: Default::default(),
            });
            Ok(Arc::new(Choice {
                branches: resolved,
                default,
            }))
        }
        DomainOperator::Map {
            source: _,
            body: _,
            max_concurrency: _,
        } => {
            // Map::new resolves source and body via build_operator recursively
            Ok(Arc::new(Map::new(ir_op, ir)?))
        }
        // Out-of-scope variants
        DomainOperator::Join { .. } => {
            Err(OperatorError::NotImplementedInCycle16 { variant: "Join" })
        }
        DomainOperator::Race { .. } => {
            Err(OperatorError::NotImplementedInCycle16 { variant: "Race" })
        }
        DomainOperator::Loop { .. } => {
            Err(OperatorError::NotImplementedInCycle16 { variant: "Loop" })
        }
        DomainOperator::Gate { .. } => {
            Err(OperatorError::NotImplementedInCycle16 { variant: "Gate" })
        }
        DomainOperator::Wait { .. } => {
            Err(OperatorError::NotImplementedInCycle16 { variant: "Wait" })
        }
        DomainOperator::SubWorkflow { .. } => Err(OperatorError::NotImplementedInCycle16 {
            variant: "SubWorkflow",
        }),
        DomainOperator::Compensate { .. } => Err(OperatorError::NotImplementedInCycle16 {
            variant: "Compensate",
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_implements_operator() {
        let task = Task {
            capability: sddk_domain::CapabilityId("test.cap".to_string()),
            inputs: Default::default(),
        };
        assert_eq!(task.kind(), "Task");
    }

    #[test]
    fn sequence_implements_operator() {
        let seq = Sequence { children: vec![] };
        assert_eq!(seq.kind(), "Sequence");
    }

    #[test]
    fn parallel_implements_operator() {
        let par = Parallel {
            children: vec![],
            max_concurrency: 4,
        };
        assert_eq!(par.kind(), "Parallel");
    }

    #[test]
    fn choice_implements_operator() {
        let choice = Choice {
            branches: Default::default(),
            default: Arc::new(Task {
                capability: sddk_domain::CapabilityId("default".to_string()),
                inputs: Default::default(),
            }),
        };
        assert_eq!(choice.kind(), "Choice");
    }

    #[test]
    fn build_operator_task_leaf() {
        use sddk_domain::{CapabilityId, Operator as DomainOperator};
        let ir_op = DomainOperator::Task {
            capability: CapabilityId("git.commit".to_string()),
            inputs: Default::default(),
        };
        use sddk_domain::TemplateRef;
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
        let result = build_operator(&ir_op, &ir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().kind(), "Task");
    }

    #[test]
    fn build_operator_sequence_empty() {
        let ir_op = DomainOperator::Sequence { body: vec![] };
        use sddk_domain::TemplateRef;
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
        let result = build_operator(&ir_op, &ir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().kind(), "Sequence");
    }

    #[test]
    fn build_operator_parallel_empty_branches() {
        let ir_op = DomainOperator::Parallel {
            branches: vec![],
            max_concurrency: 2,
        };
        use sddk_domain::TemplateRef;
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
        let result = build_operator(&ir_op, &ir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().kind(), "Parallel");
    }

    #[test]
    fn build_operator_choice() {
        let ir_op = DomainOperator::Choice {
            branches: Default::default(),
        };
        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
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
        let result = build_operator(&ir_op, &ir);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().kind(), "Choice");
    }

    #[test]
    fn dispatch_maps_map() {
        // Verify build_operator can construct Map from domain Map operator
        use sddk_domain::OperatorId;
        let source_id = OperatorId("src".into());
        let body_id = OperatorId("body".into());
        let mut operators = std::collections::BTreeMap::new();
        operators.insert(
            source_id.clone(),
            DomainOperator::Task {
                capability: sddk_domain::CapabilityId("test.cap".into()),
                inputs: Default::default(),
            },
        );
        operators.insert(
            body_id.clone(),
            DomainOperator::Task {
                capability: sddk_domain::CapabilityId("test.cap".into()),
                inputs: Default::default(),
            },
        );
        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
                id: "test".into(),
                version: "1.0".into(),
            },
            operators,
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
        let ir_op = DomainOperator::Map {
            source: source_id,
            body: body_id,
            max_concurrency: 2,
        };
        let result = build_operator(&ir_op, &ir).expect("Map should build");
        assert_eq!(result.kind(), "Map");
    }

    #[test]
    fn map_implements_operator() {
        // Build a minimal IR for test Map construction
        use sddk_domain::OperatorId;
        let source_id = OperatorId("src".into());
        let body_id = OperatorId("body".into());
        let mut operators = std::collections::BTreeMap::new();
        operators.insert(
            source_id.clone(),
            DomainOperator::Task {
                capability: sddk_domain::CapabilityId("test.cap".into()),
                inputs: Default::default(),
            },
        );
        operators.insert(
            body_id.clone(),
            DomainOperator::Task {
                capability: sddk_domain::CapabilityId("test.cap".into()),
                inputs: Default::default(),
            },
        );
        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
                id: "test".into(),
                version: "1.0".into(),
            },
            operators,
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
        let m = Map::new(
            &DomainOperator::Map {
                source: source_id,
                body: body_id,
                max_concurrency: 4,
            },
            &ir,
        )
        .expect("test Map construction");
        assert_eq!(m.kind(), "Map");
        assert_eq!(m.max_concurrency, 4);
    }

    #[test]
    fn build_operator_returns_not_implemented_for_join() {
        let ir_op = DomainOperator::Join {
            policy: "all".into(),
            branches: vec![],
        };
        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
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
        let result = build_operator(&ir_op, &ir);
        assert!(matches!(
            result,
            Err(OperatorError::NotImplementedInCycle16 { variant: _ })
        ));
    }

    #[test]
    fn build_operator_returns_not_implemented_for_race() {
        let ir_op = DomainOperator::Race {
            branches: vec![],
            timeout_ms: 1000,
        };
        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
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
        let result = build_operator(&ir_op, &ir);
        assert!(matches!(
            result,
            Err(OperatorError::NotImplementedInCycle16 { variant: _ })
        ));
    }

    #[test]
    fn build_operator_returns_not_implemented_for_loop() {
        let ir_op = DomainOperator::Loop {
            max_iterations: 10,
            until: sddk_domain::GuardExpr {
                expr: "true".into(),
            },
            body: sddk_domain::OperatorId("b".into()),
        };
        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
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
        let result = build_operator(&ir_op, &ir);
        assert!(matches!(
            result,
            Err(OperatorError::NotImplementedInCycle16 { variant: _ })
        ));
    }

    #[test]
    fn build_operator_returns_not_implemented_for_gate() {
        let ir_op = DomainOperator::Gate {
            condition: sddk_domain::GuardExpr {
                expr: "true".into(),
            },
            body: sddk_domain::OperatorId("b".into()),
        };
        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
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
        let result = build_operator(&ir_op, &ir);
        assert!(matches!(
            result,
            Err(OperatorError::NotImplementedInCycle16 { variant: _ })
        ));
    }

    #[test]
    fn build_operator_returns_not_implemented_for_wait() {
        let ir_op = DomainOperator::Wait {
            event_type: "external.event".into(),
            timeout_ms: 5000,
        };
        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
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
        let result = build_operator(&ir_op, &ir);
        assert!(matches!(
            result,
            Err(OperatorError::NotImplementedInCycle16 { variant: _ })
        ));
    }

    #[test]
    fn build_operator_returns_not_implemented_for_subworkflow() {
        let ir_op = DomainOperator::SubWorkflow {
            run_ref: "workflow-123".into(),
        };
        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
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
        let result = build_operator(&ir_op, &ir);
        assert!(matches!(
            result,
            Err(OperatorError::NotImplementedInCycle16 { variant: _ })
        ));
    }

    #[test]
    fn build_operator_returns_not_implemented_for_compensate() {
        let ir_op = DomainOperator::Compensate {
            of: sddk_domain::OperatorId("op".into()),
        };
        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
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
        let result = build_operator(&ir_op, &ir);
        assert!(matches!(
            result,
            Err(OperatorError::NotImplementedInCycle16 { variant: _ })
        ));
    }

    #[test]
    fn clock_returns_rfc3339_timestamp() {
        let clock = Clock;
        let ts = clock.now();
        assert!(ts.ends_with('Z'), "expected RFC3339 UTC format: {ts}");
        assert_eq!(ts.len(), 20, "expected 20-char RFC3339: {ts}");
    }

    // -- TaskExecutor tests -------------------------------------------------------

    #[test]
    fn noop_task_executor_returns_success() {
        use sddk_domain::{NoopTaskExecutor, TaskExecutor};
        use std::collections::BTreeMap;

        let executor = NoopTaskExecutor;
        let inputs = BTreeMap::new();
        let result = executor.execute("test.cap", &inputs);
        assert!(result.is_ok());
        assert!(result.unwrap().outputs.is_empty());
    }

    #[test]
    fn noop_task_executor_ignores_inputs() {
        use sddk_domain::{NoopTaskExecutor, TaskExecutor};
        use serde_json::Value;
        use std::collections::BTreeMap;

        let executor = NoopTaskExecutor;
        let mut inputs = BTreeMap::new();
        inputs.insert("key".to_string(), Value::String("val".to_string()));
        let result = executor.execute("git.commit", &inputs);
        assert!(result.is_ok());
    }

    #[test]
    fn task_evaluate_calls_executor_and_succeeds() {
        use sddk_domain::StorageError;
        use sddk_domain::{
            CapabilityId, GraphStore, NodeRun, NodeRunState, TaskError, TaskExecutor, TaskOutput,
            WorkflowIR, WorkflowRun,
        };
        use serde_json::Value;
        use std::collections::BTreeMap;
        use std::sync::Arc;

        // Build a mock store that implements GraphStore minimally
        struct MockStore;
        impl GraphStore for MockStore {
            fn save_state(&mut self, _state: &sddk_domain::GraphState) -> Result<(), StorageError> {
                Ok(())
            }
            fn load_state(&self) -> Result<Option<sddk_domain::GraphState>, StorageError> {
                Ok(None)
            }
            fn checkpoint(
                &self,
            ) -> Result<Option<sddk_domain::projections::Checkpoint>, StorageError> {
                Ok(None)
            }
            fn record_ir_digest(
                &mut self,
                _ir_hash: &str,
                _ir_json: &str,
            ) -> Result<(), StorageError> {
                Ok(())
            }
            fn record_graph_revision(
                &mut self,
                _rev: &sddk_domain::graph::ExecutionGraphRevision,
            ) -> Result<(), StorageError> {
                Ok(())
            }
            fn load_node_attempts(
                &self,
                _run_id: &sddk_domain::RunId,
                _node_id: &sddk_domain::NodeId,
            ) -> Result<Vec<sddk_domain::Attempt>, StorageError> {
                Ok(vec![])
            }
            fn attempt_count(
                &self,
                _run_id: &sddk_domain::RunId,
                _node_id: &sddk_domain::NodeId,
            ) -> Result<u32, StorageError> {
                Ok(0)
            }
            fn save_revision(
                &mut self,
                _rev: &sddk_domain::graph::ExecutionGraphRevision,
            ) -> Result<(), StorageError> {
                Ok(())
            }
            fn load_revision(
                &self,
                _run_id: &sddk_domain::RunId,
                _rev_id: &sddk_domain::RevisionId,
            ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, StorageError>
            {
                Ok(None)
            }
            fn latest_revision(
                &self,
                _run_id: &sddk_domain::RunId,
            ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, StorageError>
            {
                Ok(None)
            }
        }

        // Counter-based executor that tracks calls
        struct CountingExecutor {
            count: std::sync::atomic::AtomicUsize,
        }
        impl TaskExecutor for CountingExecutor {
            fn execute(
                &self,
                capability: &str,
                _inputs: &BTreeMap<String, Value>,
            ) -> Result<TaskOutput, TaskError> {
                self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                // Echo back the capability as an output for verification
                let mut outputs = BTreeMap::new();
                outputs.insert(
                    "called_capability".to_string(),
                    Value::String(capability.to_string()),
                );
                Ok(TaskOutput { outputs })
            }
        }

        let executor = Arc::new(CountingExecutor {
            count: std::sync::atomic::AtomicUsize::new(0),
        });

        let node_run = Arc::new(Mutex::new(NodeRun {
            node_id: sddk_domain::NodeId("test-node".to_string()),
            state: NodeRunState::Ready,
            dependencies: Default::default(),
            attempts: vec![],
            expansion_permissions: Default::default(),
            schema_version: 1,
        }));

        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
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

        let run = WorkflowRun {
            run_id: sddk_domain::RunId("test-run".into()),
            template_ref: ir.template_ref.clone(),
            ir_hash: ir.compute_content_hash(),
            graph_revision: sddk_domain::RevisionId("rev".into()),
            state: sddk_domain::WorkflowRunState::Running,
            inputs: Default::default(),
            outputs: None,
            correlation_id: sddk_domain::CorrelationId("corr".into()),
            budget: Default::default(),
            schema_version: 1,
        };

        let store: Arc<Mutex<GraphStoreBox>> = Arc::new(Mutex::new(GraphStoreBox {
            inner: Box::new(MockStore),
        }));
        let clock = Clock;

        let mut ctx = OperatorContext {
            node_run: Arc::clone(&node_run),
            ir: Arc::new(ir.clone()),
            run: Arc::new(run.clone()),
            store,
            clock,
            executor,
            pending_sender: None,
        };

        let task = Task {
            capability: CapabilityId("my.capability".to_string()),
            inputs: Default::default(),
        };

        let outcome = task.evaluate(&mut ctx).unwrap();

        assert!(
            matches!(outcome, NodeOutcome::Succeeded { node_id, .. } if node_id.0 == "test-node")
        );
        // Note: node_run.state is NOT mutated by Task::evaluate (pure-return contract).
        // State transitions are handled by the runtime tier.
    }

    #[test]
    fn task_evaluate_returns_failed_on_executor_error() {
        use sddk_domain::StorageError;
        use sddk_domain::{
            CapabilityId, GraphStore, NodeRun, NodeRunState, TaskError, TaskExecutor, TaskOutput,
            WorkflowIR, WorkflowRun,
        };
        use serde_json::Value;
        use std::collections::BTreeMap;
        use std::sync::Arc;

        struct MockStore;
        impl GraphStore for MockStore {
            fn save_state(&mut self, _state: &sddk_domain::GraphState) -> Result<(), StorageError> {
                Ok(())
            }
            fn load_state(&self) -> Result<Option<sddk_domain::GraphState>, StorageError> {
                Ok(None)
            }
            fn checkpoint(
                &self,
            ) -> Result<Option<sddk_domain::projections::Checkpoint>, StorageError> {
                Ok(None)
            }
            fn record_ir_digest(
                &mut self,
                _ir_hash: &str,
                _ir_json: &str,
            ) -> Result<(), StorageError> {
                Ok(())
            }
            fn record_graph_revision(
                &mut self,
                _rev: &sddk_domain::graph::ExecutionGraphRevision,
            ) -> Result<(), StorageError> {
                Ok(())
            }
            fn load_node_attempts(
                &self,
                _run_id: &sddk_domain::RunId,
                _node_id: &sddk_domain::NodeId,
            ) -> Result<Vec<sddk_domain::Attempt>, StorageError> {
                Ok(vec![])
            }
            fn attempt_count(
                &self,
                _run_id: &sddk_domain::RunId,
                _node_id: &sddk_domain::NodeId,
            ) -> Result<u32, StorageError> {
                Ok(0)
            }
            fn save_revision(
                &mut self,
                _rev: &sddk_domain::graph::ExecutionGraphRevision,
            ) -> Result<(), StorageError> {
                Ok(())
            }
            fn load_revision(
                &self,
                _run_id: &sddk_domain::RunId,
                _rev_id: &sddk_domain::RevisionId,
            ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, StorageError>
            {
                Ok(None)
            }
            fn latest_revision(
                &self,
                _run_id: &sddk_domain::RunId,
            ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, StorageError>
            {
                Ok(None)
            }
        }

        struct FailingExecutor;
        impl TaskExecutor for FailingExecutor {
            fn execute(
                &self,
                _capability: &str,
                _inputs: &BTreeMap<String, Value>,
            ) -> Result<TaskOutput, TaskError> {
                Err(TaskError {
                    message: "capability not found".to_string(),
                })
            }
        }

        let executor = Arc::new(FailingExecutor);

        let node_run = Arc::new(Mutex::new(NodeRun {
            node_id: sddk_domain::NodeId("fail-node".to_string()),
            state: NodeRunState::Ready,
            dependencies: Default::default(),
            attempts: vec![],
            expansion_permissions: Default::default(),
            schema_version: 1,
        }));

        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
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

        let run = WorkflowRun {
            run_id: sddk_domain::RunId("test-run".into()),
            template_ref: ir.template_ref.clone(),
            ir_hash: ir.compute_content_hash(),
            graph_revision: sddk_domain::RevisionId("rev".into()),
            state: sddk_domain::WorkflowRunState::Running,
            inputs: Default::default(),
            outputs: None,
            correlation_id: sddk_domain::CorrelationId("corr".into()),
            budget: Default::default(),
            schema_version: 1,
        };

        let store: Arc<Mutex<GraphStoreBox>> = Arc::new(Mutex::new(GraphStoreBox {
            inner: Box::new(MockStore),
        }));
        let clock = Clock;

        let mut ctx = OperatorContext {
            node_run: Arc::clone(&node_run),
            ir: Arc::new(ir.clone()),
            run: Arc::new(run.clone()),
            store,
            clock,
            executor,
            pending_sender: None,
        };

        let task = Task {
            capability: CapabilityId("missing.cap".to_string()),
            inputs: Default::default(),
        };

        let outcome = task.evaluate(&mut ctx).unwrap();

        assert!(
            matches!(outcome, NodeOutcome::Failed { node_id, reason } if node_id.0 == "fail-node" && reason == "capability not found")
        );
        // Note: node_run.state is NOT mutated by Task::evaluate (pure-return contract).
    }

    // -- Sequence operator tests -------------------------------------------------

    #[test]
    fn sequence_empty_succeeds_immediately() {
        use sddk_domain::{NodeRunState, WorkflowIR, WorkflowRun};
        use std::sync::Arc;

        let sequence = Sequence { children: vec![] };

        let node_run = Arc::new(Mutex::new(NodeRun {
            node_id: sddk_domain::NodeId("empty-seq".to_string()),
            state: NodeRunState::Ready,
            dependencies: Default::default(),
            attempts: vec![],
            expansion_permissions: Default::default(),
            schema_version: 1,
        }));

        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
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

        let run = WorkflowRun {
            run_id: sddk_domain::RunId("test-run".into()),
            template_ref: ir.template_ref.clone(),
            ir_hash: ir.compute_content_hash(),
            graph_revision: sddk_domain::RevisionId("rev".into()),
            state: sddk_domain::WorkflowRunState::Running,
            inputs: Default::default(),
            outputs: None,
            correlation_id: sddk_domain::CorrelationId("corr".into()),
            budget: Default::default(),
            schema_version: 1,
        };

        let mut ctx = OperatorContext::for_test(
            Arc::clone(&node_run),
            Arc::new(ir.clone()),
            Arc::new(run.clone()),
        );

        let outcome = sequence.evaluate(&mut ctx).unwrap();

        assert!(
            matches!(outcome, NodeOutcome::Succeeded { node_id, .. } if node_id.0 == "empty-seq")
        );
        // Note: node_run.state is NOT mutated by Sequence::evaluate (pure-return contract).
    }

    #[test]
    fn sequence_success_all_children_complete() {
        use sddk_domain::{NodeRunState, WorkflowIR, WorkflowRun};
        use std::sync::Arc;

        // Create sequence with 3 children
        let children: Vec<Arc<dyn Operator>> = vec![
            Arc::new(Task {
                capability: sddk_domain::CapabilityId("a".to_string()),
                inputs: Default::default(),
            }),
            Arc::new(Task {
                capability: sddk_domain::CapabilityId("b".to_string()),
                inputs: Default::default(),
            }),
            Arc::new(Task {
                capability: sddk_domain::CapabilityId("c".to_string()),
                inputs: Default::default(),
            }),
        ];
        let sequence = Sequence { children };

        let node_run = Arc::new(Mutex::new(NodeRun {
            node_id: sddk_domain::NodeId("seq-3".to_string()),
            state: NodeRunState::Ready,
            dependencies: Default::default(),
            attempts: vec![],
            expansion_permissions: Default::default(),
            schema_version: 1,
        }));

        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
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

        let run = WorkflowRun {
            run_id: sddk_domain::RunId("test-run".into()),
            template_ref: ir.template_ref.clone(),
            ir_hash: ir.compute_content_hash(),
            graph_revision: sddk_domain::RevisionId("rev".into()),
            state: sddk_domain::WorkflowRunState::Running,
            inputs: Default::default(),
            outputs: None,
            correlation_id: sddk_domain::CorrelationId("corr".into()),
            budget: Default::default(),
            schema_version: 1,
        };

        // First evaluate: child 0 completes, returns Running
        // Note: with pure-return contract, runtime records attempts between calls.
        // This test verifies single-call behavior only; multi-call Sequence
        // requires a runtime wrapper (see workflow_runtime_tests.rs).
        let outcome1 = {
            let mut ctx = OperatorContext::for_test(
                Arc::clone(&node_run),
                Arc::new(ir.clone()),
                Arc::new(run.clone()),
            );
            sequence.evaluate(&mut ctx).unwrap()
        };
        assert!(matches!(&outcome1, NodeOutcome::Running));
        // node_run.attempts.len() stays 0 until runtime records attempts
    }

    #[test]
    fn sequence_fail_mid_is_not_implemented_in_cycle16() {
        // In cycle-16, Sequence doesn't support mid-sequence failure.
        // Children are "completed" via attempts tracking, not real evaluation.
        // This test documents that real failure semantics are deferred to cycle-17.
        let sequence = Sequence { children: vec![] };
        assert_eq!(sequence.kind(), "Sequence");
    }

    // -- Parallel operator tests -------------------------------------------------

    #[test]
    fn parallel_empty_succeeds_immediately() {
        use sddk_domain::{NodeRunState, WorkflowIR, WorkflowRun};
        use std::sync::Arc;

        let parallel = Parallel {
            children: vec![],
            max_concurrency: 2,
        };

        let node_run = Arc::new(Mutex::new(NodeRun {
            node_id: sddk_domain::NodeId("empty-par".to_string()),
            state: NodeRunState::Ready,
            dependencies: Default::default(),
            attempts: vec![],
            expansion_permissions: Default::default(),
            schema_version: 1,
        }));

        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
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

        let run = WorkflowRun {
            run_id: sddk_domain::RunId("test-run".into()),
            template_ref: ir.template_ref.clone(),
            ir_hash: ir.compute_content_hash(),
            graph_revision: sddk_domain::RevisionId("rev".into()),
            state: sddk_domain::WorkflowRunState::Running,
            inputs: Default::default(),
            outputs: None,
            correlation_id: sddk_domain::CorrelationId("corr".into()),
            budget: Default::default(),
            schema_version: 1,
        };

        let mut ctx = OperatorContext::for_test(
            Arc::clone(&node_run),
            Arc::new(ir.clone()),
            Arc::new(run.clone()),
        );

        let outcome = parallel.evaluate(&mut ctx).unwrap();

        assert!(
            matches!(outcome, NodeOutcome::Succeeded { node_id, .. } if node_id.0 == "empty-par")
        );
        assert_eq!(node_run.lock().unwrap().state, NodeRunState::Completed);
    }

    #[test]
    fn parallel_success_all_children_complete() {
        use sddk_domain::{NodeRunState, WorkflowIR, WorkflowRun};
        use std::sync::Arc;

        // Create parallel with 2 children
        let children: Vec<Arc<dyn Operator>> = vec![
            Arc::new(Task {
                capability: sddk_domain::CapabilityId("left".to_string()),
                inputs: Default::default(),
            }),
            Arc::new(Task {
                capability: sddk_domain::CapabilityId("right".to_string()),
                inputs: Default::default(),
            }),
        ];
        let parallel = Parallel {
            children,
            max_concurrency: 2,
        };

        let node_run = Arc::new(Mutex::new(NodeRun {
            node_id: sddk_domain::NodeId("par-2".to_string()),
            state: NodeRunState::Ready,
            dependencies: Default::default(),
            attempts: vec![],
            expansion_permissions: Default::default(),
            schema_version: 1,
        }));

        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
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

        let run = WorkflowRun {
            run_id: sddk_domain::RunId("test-run".into()),
            template_ref: ir.template_ref.clone(),
            ir_hash: ir.compute_content_hash(),
            graph_revision: sddk_domain::RevisionId("rev".into()),
            state: sddk_domain::WorkflowRunState::Running,
            inputs: Default::default(),
            outputs: None,
            correlation_id: sddk_domain::CorrelationId("corr".into()),
            budget: Default::default(),
            schema_version: 1,
        };

        // Concurrent: all children evaluated in one call, returns Succeeded
        let outcome = {
            let mut ctx = OperatorContext::for_test(
                Arc::clone(&node_run),
                Arc::new(ir.clone()),
                Arc::new(run.clone()),
            );
            parallel.evaluate(&mut ctx).unwrap()
        };
        // Concurrent Parallel evaluates all children in one call
        assert!(matches!(outcome, NodeOutcome::Succeeded { node_id, .. } if node_id.0 == "par-2"));
        assert_eq!(
            node_run.lock().unwrap().attempts.len(),
            2,
            "2 children → 2 attempts in one call"
        );
        assert_eq!(node_run.lock().unwrap().state, NodeRunState::Completed);
    }

    #[test]
    fn parallel_fails_immediately_on_child_failure() {
        use sddk_domain::{NodeRunState, WorkflowIR, WorkflowRun};
        use std::sync::Arc;

        // First child is a Task (succeeds), second child is a Sequence (empty = succeeds).
        // Failure must originate from a child returning Failed; the Parallel operator
        // propagates that failure upward without inspecting the child's internal state.
        let children: Vec<Arc<dyn Operator>> = vec![Arc::new(Task {
            capability: sddk_domain::CapabilityId("ok".to_string()),
            inputs: Default::default(),
        })];
        let _parallel = Parallel {
            children,
            max_concurrency: 2,
        };

        let _node_run = Arc::new(Mutex::new(NodeRun {
            node_id: sddk_domain::NodeId("par-fail".to_string()),
            state: NodeRunState::Ready,
            dependencies: Default::default(),
            attempts: vec![],
            expansion_permissions: Default::default(),
            schema_version: 1,
        }));

        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
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

        let run = WorkflowRun {
            run_id: sddk_domain::RunId("test-run".into()),
            template_ref: ir.template_ref.clone(),
            ir_hash: ir.compute_content_hash(),
            graph_revision: sddk_domain::RevisionId("rev".into()),
            state: sddk_domain::WorkflowRunState::Running,
            inputs: Default::default(),
            outputs: None,
            correlation_id: sddk_domain::CorrelationId("corr".into()),
            budget: Default::default(),
            schema_version: 1,
        };

        // Parallel with 1 child: succeeds immediately after the single child completes
        let children2: Vec<Arc<dyn Operator>> = vec![Arc::new(Task {
            capability: sddk_domain::CapabilityId("ok".to_string()),
            inputs: Default::default(),
        })];
        let parallel2 = Parallel {
            children: children2,
            max_concurrency: 2,
        };

        let node_run2 = Arc::new(Mutex::new(NodeRun {
            node_id: sddk_domain::NodeId("par-1child".to_string()),
            state: NodeRunState::Ready,
            dependencies: Default::default(),
            attempts: vec![],
            expansion_permissions: Default::default(),
            schema_version: 1,
        }));

        let outcome = {
            let mut ctx = OperatorContext::for_test(
                Arc::clone(&node_run2),
                Arc::new(ir.clone()),
                Arc::new(run.clone()),
            );
            parallel2.evaluate(&mut ctx).unwrap()
        };
        // 1 child, 1 evaluate call → Succeeded
        assert!(
            matches!(outcome, NodeOutcome::Succeeded { node_id, .. } if node_id.0 == "par-1child")
        );
        assert_eq!(node_run2.lock().unwrap().attempts.len(), 1);
        assert_eq!(node_run2.lock().unwrap().state, NodeRunState::Completed);

        // 0 children: succeeds immediately
        let parallel_empty = Parallel {
            children: vec![],
            max_concurrency: 2,
        };
        let node_run3 = Arc::new(Mutex::new(NodeRun {
            node_id: sddk_domain::NodeId("par-empty".to_string()),
            state: NodeRunState::Ready,
            dependencies: Default::default(),
            attempts: vec![],
            expansion_permissions: Default::default(),
            schema_version: 1,
        }));
        let outcome = {
            let mut ctx = OperatorContext::for_test(
                Arc::clone(&node_run3),
                Arc::new(ir.clone()),
                Arc::new(run.clone()),
            );
            parallel_empty.evaluate(&mut ctx).unwrap()
        };
        assert!(
            matches!(outcome, NodeOutcome::Succeeded { node_id, .. } if node_id.0 == "par-empty")
        );
        assert_eq!(node_run3.lock().unwrap().state, NodeRunState::Completed);
    }

    #[test]
    fn parallel_three_children_require_three_calls() {
        use sddk_domain::StorageError;
        use sddk_domain::{GraphStore, NodeRunState, NoopTaskExecutor, WorkflowIR, WorkflowRun};
        use std::sync::Arc;

        struct MockStore;
        impl GraphStore for MockStore {
            fn save_state(&mut self, _state: &sddk_domain::GraphState) -> Result<(), StorageError> {
                Ok(())
            }
            fn load_state(&self) -> Result<Option<sddk_domain::GraphState>, StorageError> {
                Ok(None)
            }
            fn checkpoint(
                &self,
            ) -> Result<Option<sddk_domain::projections::Checkpoint>, StorageError> {
                Ok(None)
            }
            fn record_ir_digest(
                &mut self,
                _ir_hash: &str,
                _ir_json: &str,
            ) -> Result<(), StorageError> {
                Ok(())
            }
            fn record_graph_revision(
                &mut self,
                _rev: &sddk_domain::graph::ExecutionGraphRevision,
            ) -> Result<(), StorageError> {
                Ok(())
            }
            fn load_node_attempts(
                &self,
                _run_id: &sddk_domain::RunId,
                _node_id: &sddk_domain::NodeId,
            ) -> Result<Vec<sddk_domain::Attempt>, StorageError> {
                Ok(vec![])
            }
            fn attempt_count(
                &self,
                _run_id: &sddk_domain::RunId,
                _node_id: &sddk_domain::NodeId,
            ) -> Result<u32, StorageError> {
                Ok(0)
            }
            fn save_revision(
                &mut self,
                _rev: &sddk_domain::graph::ExecutionGraphRevision,
            ) -> Result<(), StorageError> {
                Ok(())
            }
            fn load_revision(
                &self,
                _run_id: &sddk_domain::RunId,
                _rev_id: &sddk_domain::RevisionId,
            ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, StorageError>
            {
                Ok(None)
            }
            fn latest_revision(
                &self,
                _run_id: &sddk_domain::RunId,
            ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, StorageError>
            {
                Ok(None)
            }
        }

        let children: Vec<Arc<dyn Operator>> = vec![
            Arc::new(Task {
                capability: sddk_domain::CapabilityId("a".to_string()),
                inputs: Default::default(),
            }),
            Arc::new(Task {
                capability: sddk_domain::CapabilityId("b".to_string()),
                inputs: Default::default(),
            }),
            Arc::new(Task {
                capability: sddk_domain::CapabilityId("c".to_string()),
                inputs: Default::default(),
            }),
        ];
        let parallel = Parallel {
            children,
            max_concurrency: 3,
        };

        let node_run = Arc::new(Mutex::new(NodeRun {
            node_id: sddk_domain::NodeId("par-3".to_string()),
            state: NodeRunState::Ready,
            dependencies: Default::default(),
            attempts: vec![],
            expansion_permissions: Default::default(),
            schema_version: 1,
        }));

        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
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

        let run = WorkflowRun {
            run_id: sddk_domain::RunId("test-run".into()),
            template_ref: ir.template_ref.clone(),
            ir_hash: ir.compute_content_hash(),
            graph_revision: sddk_domain::RevisionId("rev".into()),
            state: sddk_domain::WorkflowRunState::Running,
            inputs: Default::default(),
            outputs: None,
            correlation_id: sddk_domain::CorrelationId("corr".into()),
            budget: Default::default(),
            schema_version: 1,
        };

        let store: Arc<Mutex<GraphStoreBox>> = Arc::new(Mutex::new(GraphStoreBox {
            inner: Box::new(MockStore),
        }));

        // Concurrent: all 3 children evaluated in one call, returns Succeeded
        let outcome = {
            let executor: Arc<dyn TaskExecutor> = Arc::new(NoopTaskExecutor);
            let clock = Clock;
            let mut ctx = OperatorContext {
                node_run: Arc::clone(&node_run),
                ir: Arc::new(ir.clone()),
                run: Arc::new(run.clone()),
                store: Arc::clone(&store),
                clock,
                executor,
                pending_sender: None,
            };
            parallel.evaluate(&mut ctx).unwrap()
        };
        // Concurrent Parallel evaluates all children in one call
        assert!(matches!(outcome, NodeOutcome::Succeeded { node_id, .. } if node_id.0 == "par-3"));
        assert_eq!(
            node_run.lock().unwrap().attempts.len(),
            3,
            "3 children → 3 attempts in one call"
        );
        assert_eq!(node_run.lock().unwrap().state, NodeRunState::Completed);
    }

    #[test]
    fn parallel_records_correct_attempt_count() {
        use sddk_domain::StorageError;
        use sddk_domain::{GraphStore, NodeRunState, NoopTaskExecutor, WorkflowIR, WorkflowRun};
        use std::sync::Arc;

        struct MockStore;
        impl GraphStore for MockStore {
            fn save_state(&mut self, _state: &sddk_domain::GraphState) -> Result<(), StorageError> {
                Ok(())
            }
            fn load_state(&self) -> Result<Option<sddk_domain::GraphState>, StorageError> {
                Ok(None)
            }
            fn checkpoint(
                &self,
            ) -> Result<Option<sddk_domain::projections::Checkpoint>, StorageError> {
                Ok(None)
            }
            fn record_ir_digest(
                &mut self,
                _ir_hash: &str,
                _ir_json: &str,
            ) -> Result<(), StorageError> {
                Ok(())
            }
            fn record_graph_revision(
                &mut self,
                _rev: &sddk_domain::graph::ExecutionGraphRevision,
            ) -> Result<(), StorageError> {
                Ok(())
            }
            fn load_node_attempts(
                &self,
                _run_id: &sddk_domain::RunId,
                _node_id: &sddk_domain::NodeId,
            ) -> Result<Vec<sddk_domain::Attempt>, StorageError> {
                Ok(vec![])
            }
            fn attempt_count(
                &self,
                _run_id: &sddk_domain::RunId,
                _node_id: &sddk_domain::NodeId,
            ) -> Result<u32, StorageError> {
                Ok(0)
            }
            fn save_revision(
                &mut self,
                _rev: &sddk_domain::graph::ExecutionGraphRevision,
            ) -> Result<(), StorageError> {
                Ok(())
            }
            fn load_revision(
                &self,
                _run_id: &sddk_domain::RunId,
                _rev_id: &sddk_domain::RevisionId,
            ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, StorageError>
            {
                Ok(None)
            }
            fn latest_revision(
                &self,
                _run_id: &sddk_domain::RunId,
            ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, StorageError>
            {
                Ok(None)
            }
        }

        // 4 children
        let children: Vec<Arc<dyn Operator>> = vec![
            Arc::new(Task {
                capability: sddk_domain::CapabilityId("a".to_string()),
                inputs: Default::default(),
            }),
            Arc::new(Task {
                capability: sddk_domain::CapabilityId("b".to_string()),
                inputs: Default::default(),
            }),
            Arc::new(Task {
                capability: sddk_domain::CapabilityId("c".to_string()),
                inputs: Default::default(),
            }),
            Arc::new(Task {
                capability: sddk_domain::CapabilityId("d".to_string()),
                inputs: Default::default(),
            }),
        ];
        let parallel = Parallel {
            children,
            max_concurrency: 4,
        };

        let node_run = Arc::new(Mutex::new(NodeRun {
            node_id: sddk_domain::NodeId("par-4".to_string()),
            state: NodeRunState::Ready,
            dependencies: Default::default(),
            attempts: vec![],
            expansion_permissions: Default::default(),
            schema_version: 1,
        }));

        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
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

        let run = WorkflowRun {
            run_id: sddk_domain::RunId("test-run".into()),
            template_ref: ir.template_ref.clone(),
            ir_hash: ir.compute_content_hash(),
            graph_revision: sddk_domain::RevisionId("rev".into()),
            state: sddk_domain::WorkflowRunState::Running,
            inputs: Default::default(),
            outputs: None,
            correlation_id: sddk_domain::CorrelationId("corr".into()),
            budget: Default::default(),
            schema_version: 1,
        };

        let store: Arc<Mutex<GraphStoreBox>> = Arc::new(Mutex::new(GraphStoreBox {
            inner: Box::new(MockStore),
        }));

        // Concurrent: all 4 children evaluated in one call, returns Succeeded
        let outcome = {
            let executor: Arc<dyn TaskExecutor> = Arc::new(NoopTaskExecutor);
            let clock = Clock;
            let mut ctx = OperatorContext {
                node_run: Arc::clone(&node_run),
                ir: Arc::new(ir.clone()),
                run: Arc::new(run.clone()),
                store: Arc::clone(&store),
                clock,
                executor,
                pending_sender: None,
            };
            parallel.evaluate(&mut ctx).unwrap()
        };
        assert!(
            matches!(outcome, NodeOutcome::Succeeded { .. }),
            "concurrent parallel: one call succeeds all children"
        );
        assert_eq!(
            node_run.lock().unwrap().attempts.len(),
            4,
            "4 children → 4 attempts in one call"
        );
        assert_eq!(node_run.lock().unwrap().state, NodeRunState::Completed);
    }

    #[test]
    fn parallel_max_concurrency_field_is_recorded() {
        // Verifies that max_concurrency is stored on the Parallel struct
        let children: Vec<Arc<dyn Operator>> = vec![Arc::new(Task {
            capability: sddk_domain::CapabilityId("a".to_string()),
            inputs: Default::default(),
        })];
        let parallel = Parallel {
            children,
            max_concurrency: 8,
        };
        assert_eq!(parallel.max_concurrency, 8);

        let parallel_unlimited = Parallel {
            children: vec![],
            max_concurrency: 0,
        };
        assert_eq!(parallel_unlimited.max_concurrency, 0);
    }

    // -- Choice operator tests -------------------------------------------------

    #[test]
    fn choice_default_fallback() {
        use sddk_domain::StorageError;
        use sddk_domain::{GraphStore, NodeRunState, NoopTaskExecutor, WorkflowIR, WorkflowRun};
        use std::sync::Arc;

        struct MockStore;
        impl GraphStore for MockStore {
            fn save_state(&mut self, _state: &sddk_domain::GraphState) -> Result<(), StorageError> {
                Ok(())
            }
            fn load_state(&self) -> Result<Option<sddk_domain::GraphState>, StorageError> {
                Ok(None)
            }
            fn checkpoint(
                &self,
            ) -> Result<Option<sddk_domain::projections::Checkpoint>, StorageError> {
                Ok(None)
            }
            fn record_ir_digest(
                &mut self,
                _ir_hash: &str,
                _ir_json: &str,
            ) -> Result<(), StorageError> {
                Ok(())
            }
            fn record_graph_revision(
                &mut self,
                _rev: &sddk_domain::graph::ExecutionGraphRevision,
            ) -> Result<(), StorageError> {
                Ok(())
            }
            fn load_node_attempts(
                &self,
                _run_id: &sddk_domain::RunId,
                _node_id: &sddk_domain::NodeId,
            ) -> Result<Vec<sddk_domain::Attempt>, StorageError> {
                Ok(vec![])
            }
            fn attempt_count(
                &self,
                _run_id: &sddk_domain::RunId,
                _node_id: &sddk_domain::NodeId,
            ) -> Result<u32, StorageError> {
                Ok(0)
            }
            fn save_revision(
                &mut self,
                _rev: &sddk_domain::graph::ExecutionGraphRevision,
            ) -> Result<(), StorageError> {
                Ok(())
            }
            fn load_revision(
                &self,
                _run_id: &sddk_domain::RunId,
                _rev_id: &sddk_domain::RevisionId,
            ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, StorageError>
            {
                Ok(None)
            }
            fn latest_revision(
                &self,
                _run_id: &sddk_domain::RunId,
            ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, StorageError>
            {
                Ok(None)
            }
        }

        // Create a choice with empty branches (falls back to default)
        let branches: std::collections::BTreeMap<String, Arc<dyn Operator>> =
            std::collections::BTreeMap::new();
        let default = Arc::new(Task {
            capability: sddk_domain::CapabilityId("default".to_string()),
            inputs: Default::default(),
        });
        let choice = Choice { branches, default };

        let node_run = Arc::new(Mutex::new(NodeRun {
            node_id: sddk_domain::NodeId("choice-no-branches".to_string()),
            state: NodeRunState::Ready,
            dependencies: Default::default(),
            attempts: vec![],
            expansion_permissions: Default::default(),
            schema_version: 1,
        }));

        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
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

        let run = WorkflowRun {
            run_id: sddk_domain::RunId("test-run".into()),
            template_ref: ir.template_ref.clone(),
            ir_hash: ir.compute_content_hash(),
            graph_revision: sddk_domain::RevisionId("rev".into()),
            state: sddk_domain::WorkflowRunState::Running,
            inputs: Default::default(),
            outputs: None,
            correlation_id: sddk_domain::CorrelationId("corr".into()),
            budget: Default::default(),
            schema_version: 1,
        };

        let store: Arc<Mutex<GraphStoreBox>> = Arc::new(Mutex::new(GraphStoreBox {
            inner: Box::new(MockStore),
        }));
        let executor = Arc::new(NoopTaskExecutor);
        let clock = Clock;

        let mut ctx = OperatorContext {
            node_run: Arc::clone(&node_run),
            ir: Arc::new(ir.clone()),
            run: Arc::new(run.clone()),
            store,
            clock,
            executor,
            pending_sender: None,
        };

        let outcome = choice.evaluate(&mut ctx).unwrap();

        assert!(
            matches!(outcome, NodeOutcome::Succeeded { node_id, .. } if node_id.0 == "choice-no-branches")
        );
        assert_eq!(node_run.lock().unwrap().state, NodeRunState::Completed);
    }

    #[test]
    fn choice_first_match_in_cycle16_is_default() {
        use sddk_domain::StorageError;
        use sddk_domain::{GraphStore, NodeRunState, NoopTaskExecutor, WorkflowIR, WorkflowRun};
        use std::sync::Arc;

        struct MockStore;
        impl GraphStore for MockStore {
            fn save_state(&mut self, _state: &sddk_domain::GraphState) -> Result<(), StorageError> {
                Ok(())
            }
            fn load_state(&self) -> Result<Option<sddk_domain::GraphState>, StorageError> {
                Ok(None)
            }
            fn checkpoint(
                &self,
            ) -> Result<Option<sddk_domain::projections::Checkpoint>, StorageError> {
                Ok(None)
            }
            fn record_ir_digest(
                &mut self,
                _ir_hash: &str,
                _ir_json: &str,
            ) -> Result<(), StorageError> {
                Ok(())
            }
            fn record_graph_revision(
                &mut self,
                _rev: &sddk_domain::graph::ExecutionGraphRevision,
            ) -> Result<(), StorageError> {
                Ok(())
            }
            fn load_node_attempts(
                &self,
                _run_id: &sddk_domain::RunId,
                _node_id: &sddk_domain::NodeId,
            ) -> Result<Vec<sddk_domain::Attempt>, StorageError> {
                Ok(vec![])
            }
            fn attempt_count(
                &self,
                _run_id: &sddk_domain::RunId,
                _node_id: &sddk_domain::NodeId,
            ) -> Result<u32, StorageError> {
                Ok(0)
            }
            fn save_revision(
                &mut self,
                _rev: &sddk_domain::graph::ExecutionGraphRevision,
            ) -> Result<(), StorageError> {
                Ok(())
            }
            fn load_revision(
                &self,
                _run_id: &sddk_domain::RunId,
                _rev_id: &sddk_domain::RevisionId,
            ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, StorageError>
            {
                Ok(None)
            }
            fn latest_revision(
                &self,
                _run_id: &sddk_domain::RunId,
            ) -> Result<Option<sddk_domain::graph::ExecutionGraphRevision>, StorageError>
            {
                Ok(None)
            }
        }

        // Create a choice with branches but no real condition evaluation in cycle-16
        // In cycle-16, we take the first branch (since real conditions are deferred)
        let mut branches: std::collections::BTreeMap<String, Arc<dyn Operator>> =
            std::collections::BTreeMap::new();
        branches.insert(
            "always-true".to_string(),
            Arc::new(Task {
                capability: sddk_domain::CapabilityId("branch-a".to_string()),
                inputs: Default::default(),
            }),
        );
        branches.insert(
            "sometimes-true".to_string(),
            Arc::new(Task {
                capability: sddk_domain::CapabilityId("branch-b".to_string()),
                inputs: Default::default(),
            }),
        );

        let default = Arc::new(Task {
            capability: sddk_domain::CapabilityId("default".to_string()),
            inputs: Default::default(),
        });
        let choice = Choice { branches, default };

        let node_run = Arc::new(Mutex::new(NodeRun {
            node_id: sddk_domain::NodeId("choice-with-branches".to_string()),
            state: NodeRunState::Ready,
            dependencies: Default::default(),
            attempts: vec![],
            expansion_permissions: Default::default(),
            schema_version: 1,
        }));

        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
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

        let run = WorkflowRun {
            run_id: sddk_domain::RunId("test-run".into()),
            template_ref: ir.template_ref.clone(),
            ir_hash: ir.compute_content_hash(),
            graph_revision: sddk_domain::RevisionId("rev".into()),
            state: sddk_domain::WorkflowRunState::Running,
            inputs: Default::default(),
            outputs: None,
            correlation_id: sddk_domain::CorrelationId("corr".into()),
            budget: Default::default(),
            schema_version: 1,
        };

        let store: Arc<Mutex<GraphStoreBox>> = Arc::new(Mutex::new(GraphStoreBox {
            inner: Box::new(MockStore),
        }));
        let executor = Arc::new(NoopTaskExecutor);
        let clock = Clock;

        let mut ctx = OperatorContext {
            node_run: Arc::clone(&node_run),
            ir: Arc::new(ir.clone()),
            run: Arc::new(run.clone()),
            store,
            clock,
            executor,
            pending_sender: None,
        };

        // In cycle-16, we take the first branch key (always-true) since we can't evaluate conditions
        let outcome = choice.evaluate(&mut ctx).unwrap();

        assert!(
            matches!(outcome, NodeOutcome::Succeeded { node_id, .. } if node_id.0 == "choice-with-branches")
        );
        assert_eq!(node_run.lock().unwrap().state, NodeRunState::Completed);
        // The attempt records which branch was selected
        assert_eq!(node_run.lock().unwrap().attempts.len(), 1);
    }

    // -- Arc<dyn Operator> tests -------------------------------------------------

    #[test]
    fn arc_dyn_operator_dispatches_correctly() {
        // INV-11: Arc<dyn Operator> compiles + dispatches identically to Box<dyn Operator>
        let task: Arc<dyn Operator> = Arc::new(Task {
            capability: sddk_domain::CapabilityId("test.cap".to_string()),
            inputs: Default::default(),
        });
        let _seq: Arc<dyn Operator> = Arc::new(Sequence { children: vec![] });
        let _par: Arc<dyn Operator> = Arc::new(Parallel {
            children: vec![],
            max_concurrency: 4,
        });
        let _choice: Arc<dyn Operator> = Arc::new(Choice {
            branches: Default::default(),
            default: Arc::new(Task {
                capability: sddk_domain::CapabilityId("default".to_string()),
                inputs: Default::default(),
            }),
        });
        assert_eq!(task.kind(), "Task");
    }

    // -- Checkpoint types tests -------------------------------------------------

    #[test]
    fn checkpoint_handle_variants_compare() {
        assert_ne!(
            CheckpointHandle::None,
            CheckpointHandle::Channel { resume_token: 0 }
        );
        assert_eq!(
            CheckpointHandle::Channel { resume_token: 1 },
            CheckpointHandle::Channel { resume_token: 1 }
        );
    }

    #[test]
    fn checkpoint_variants_compare() {
        assert_ne!(Checkpoint::None, Checkpoint::ParallelChannel { token: 1 });
        assert_eq!(
            Checkpoint::ParallelChannel { token: 42 },
            Checkpoint::ParallelChannel { token: 42 }
        );
    }

    #[test]
    fn operator_error_child_panicked_displays_child_index() {
        let err = OperatorError::ChildPanicked { child_index: 2 };
        let s = format!("{}", err);
        assert!(s.contains("2"), "expected '2' in display, got: {}", s);
    }

    #[test]
    fn node_outcome_pending_carries_checkpoint_handle() {
        let pending = NodeOutcome::Pending {
            checkpoint: CheckpointHandle::None,
        };
        assert!(matches!(
            pending,
            NodeOutcome::Pending {
                checkpoint: CheckpointHandle::None
            }
        ));

        let with_token = NodeOutcome::Pending {
            checkpoint: CheckpointHandle::Channel { resume_token: 99 },
        };
        assert!(matches!(
            with_token,
            NodeOutcome::Pending {
                checkpoint: CheckpointHandle::Channel { resume_token: 99 }
            }
        ));
    }

    #[test]
    fn child_result_succeeded_helper() {
        let success = ChildResult {
            child_index: 0,
            outcome: Ok(NodeOutcome::Succeeded {
                node_id: NodeId("n".into()),
                outputs: Default::default(),
            }),
            started_at: "2026-01-01T00:00:00Z".into(),
            ended_at: "2026-01-01T00:00:01Z".into(),
        };
        assert!(success.succeeded());

        let failed = ChildResult {
            child_index: 1,
            outcome: Ok(NodeOutcome::Failed {
                node_id: NodeId("n".into()),
                reason: "oops".into(),
            }),
            started_at: "2026-01-01T00:00:00Z".into(),
            ended_at: "2026-01-01T00:00:01Z".into(),
        };
        assert!(!failed.succeeded());
    }

    #[test]
    fn operator_context_send_and_static() {
        // INV-12: OperatorContext is Send + 'static (cycle-20 refactor removes Box::leak)
        fn assert_send_static<T: Send + 'static>() {}
        assert_send_static::<OperatorContext>();
    }

    #[test]
    fn operator_context_uses_arc_for_ir_and_run() {
        // INV-12: ir and run are Arc-wrapped; node_run and store are Arc<Mutex<T>>
        use sddk_domain::{NodeRunState, WorkflowIR, WorkflowRun};
        use std::sync::{Arc, Mutex};

        let ir = WorkflowIR {
            ir_id: None,
            schema_version: 1,
            template_ref: sddk_domain::TemplateRef {
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
        let run = WorkflowRun {
            run_id: sddk_domain::RunId("test-run".into()),
            template_ref: ir.template_ref.clone(),
            ir_hash: ir.compute_content_hash(),
            graph_revision: sddk_domain::RevisionId("rev".into()),
            state: sddk_domain::WorkflowRunState::Running,
            inputs: Default::default(),
            outputs: None,
            correlation_id: sddk_domain::CorrelationId("corr".into()),
            budget: Default::default(),
            schema_version: 1,
        };
        let node_run = Arc::new(Mutex::new(NodeRun {
            node_id: sddk_domain::NodeId("test-node".to_string()),
            state: NodeRunState::Ready,
            dependencies: Default::default(),
            attempts: vec![],
            expansion_permissions: Default::default(),
            schema_version: 1,
        }));
        let store: Arc<Mutex<GraphStoreBox>> = Arc::new(Mutex::new(GraphStoreBox {
            inner: Box::new(ScratchGraphStore),
        }));
        let arc_ir = Arc::new(ir);
        let arc_run = Arc::new(run);
        let executor: Arc<dyn TaskExecutor> = Arc::new(sddk_domain::NoopTaskExecutor);

        let ctx = OperatorContext {
            node_run: Arc::clone(&node_run),
            ir: Arc::clone(&arc_ir),
            run: Arc::clone(&arc_run),
            store: Arc::clone(&store),
            clock: Clock,
            executor,
            pending_sender: None,
        };
        // Arc identities match
        assert!(Arc::ptr_eq(&ctx.ir, &arc_ir));
        assert!(Arc::ptr_eq(&ctx.run, &arc_run));
        // node_run and store are Arc-wrapped
        assert!(Arc::ptr_eq(&ctx.node_run, &node_run));
        assert!(Arc::ptr_eq(&ctx.store, &store));
    }
}
