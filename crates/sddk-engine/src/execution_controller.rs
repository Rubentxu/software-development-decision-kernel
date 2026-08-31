//! Bounded-execution controller for workflow runtime.
//!
//! Guards [`WorkflowRuntime::execute()`][crate::WorkflowRuntime::execute] and
//! [`WorkflowRuntime::tick()`][crate::WorkflowRuntime::tick] with two independent
//! termination triggers:
//!
//! 1. **Wall-budget exhaustion** — checked at the top of every tick via
//!    [`pre_tick`](ExecutionController::pre_tick).  When the configured
//!    `max_wall_ms` has elapsed, the controller returns
//!    [`RuntimeError::BudgetExceeded`][crate::RuntimeError::BudgetExceeded].
//!
//! 2. **No-progress starvation** — checked after every tick's
//!    `apply_outcomes_to_state()` via
//!    [`observe`](ExecutionController::observe).  When the node snapshot has not
//!    changed for `no_progress_threshold` consecutive ticks, the controller
//!    returns [`RuntimeError::NoProgressDetected`][crate::RuntimeError::NoProgressDetected].
//!
//! Recovery from both errors happens at the next visible boundary (top of `tick()`);
//! there is no pre-emptive kill within a running operator.
//!
//! # Snapshot model
//!
//! A [`ProgressSnapshot`] captures observable state using a serde-json hash
//! over each node's `(state, attempt_count, outputs_hash, terminal)` tuple.
//! Only observable mutations count as progress — internal activity such as
//! `tick_seq` increments are excluded.

use std::collections::BTreeMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::time::Instant;

use serde_json;

use crate::RuntimeError;
use sddk_domain::workflow_run::NodeRun;
use sddk_domain::{NodeRunState, OperatorId};

/// Captures observable state of all nodes at one point in time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProgressSnapshot(BTreeMap<OperatorId, NodeSnapshot>);

/// Captures observable state of a single node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NodeSnapshot {
    /// Current execution state.
    state: NodeRunState,
    /// Number of attempts made so far.
    attempt_count: usize,
    /// Stable hash of the node's current outputs.
    outputs_hash: u64,
    /// Whether the node is in a terminal state.
    terminal: bool,
}

impl NodeSnapshot {
    /// Captures observable state from a [`NodeRun`].
    fn from_node(node: &NodeRun) -> Self {
        let outputs_hash = hash_node_outputs(node);
        Self {
            state: node.state.clone(),
            attempt_count: node.attempts.len(),
            outputs_hash,
            terminal: node.is_terminal(),
        }
    }
}

/// Compute a stable hash over the node's observable progress state.
///
/// Hashes: `state`, `attempts.len()` (attempt sequence), and the
/// `outcome` of the last attempt.  These are the observable fields that
/// constitute progress per REQ-WF-RT-017 S3.
///
/// Uses `serde_json` serialization for cross-platform determinism.
fn hash_node_outputs(node: &NodeRun) -> u64 {
    let mut hasher = DefaultHasher::new();
    // State is observable — serialize to JSON for deterministic hashing
    if let Ok(json) = serde_json::to_string(&node.state) {
        json.hash(&mut hasher);
    }
    // Attempt sequence is observable
    node.attempts.len().hash(&mut hasher);
    // Outcome of last attempt (if any) is observable
    if let Some(last) = node.attempts.last()
        && let Some(outcome) = &last.outcome
        && let Ok(json) = serde_json::to_string(outcome)
    {
        json.hash(&mut hasher);
    }
    hasher.finish()
}

/// Bounded-execution controller for one workflow run.
#[derive(Debug)]
pub(crate) struct ExecutionController {
    /// When the run started.
    start: Instant,
    /// Maximum allowed wall-clock time in milliseconds.
    wall_budget_ms: u64,
    /// Maximum consecutive ticks without observable progress.
    no_progress_threshold: u32,
    /// Consecutive ticks seen without progress.
    consecutive_no_progress: u32,
    /// Snapshot from the previous tick.
    prev_snapshot: Option<ProgressSnapshot>,
}

impl ExecutionController {
    /// Constructs a controller from a [`sddk_domain::Budgets`] budget.
    ///
    /// # Panics
    ///
    /// Panics if `max_wall_ms` does not fit in `u64`.
    pub(crate) fn from_budgets(b: &sddk_domain::Budgets) -> Self {
        Self {
            start: Instant::now(),
            wall_budget_ms: b.max_wall_ms,
            no_progress_threshold: b.no_progress_threshold,
            consecutive_no_progress: 0,
            prev_snapshot: None,
        }
    }

    /// Checks whether the wall-clock budget has been exhausted.
    ///
    /// Called at the top of every tick.  Returns `Ok(())` when the budget is
    /// still valid; returns `Err(RuntimeError::BudgetExceeded)` when the
    /// configured wall time has been consumed.
    pub(crate) fn pre_tick(&mut self) -> Result<(), RuntimeError> {
        let elapsed_ms = self.start.elapsed().as_millis() as u64;
        if elapsed_ms >= self.wall_budget_ms {
            return Err(RuntimeError::BudgetExceeded {
                elapsed_ms,
                max_wall_ms: self.wall_budget_ms,
            });
        }
        Ok(())
    }

    /// Observes the current node states and updates the no-progress counter.
    ///
    /// Called after `apply_outcomes_to_state()` in every tick.  If the
    /// snapshot has not changed since the previous tick, the consecutive
    /// no-progress counter is incremented.  When the counter reaches
    /// `no_progress_threshold`, returns `Err(RuntimeError::NoProgressDetected)`.
    pub(crate) fn observe(
        &mut self,
        nodes: &BTreeMap<OperatorId, NodeRun>,
    ) -> Result<(), RuntimeError> {
        let snapshot = self.snapshot(nodes);
        let progress = self.diff(&snapshot);
        if !progress {
            self.consecutive_no_progress += 1;
        } else {
            self.consecutive_no_progress = 0;
        }
        self.prev_snapshot = Some(snapshot);

        if self.no_progress_threshold != u32::MAX
            && self.consecutive_no_progress >= self.no_progress_threshold
        {
            return Err(RuntimeError::NoProgressDetected {
                consecutive: self.consecutive_no_progress,
                threshold: self.no_progress_threshold,
            });
        }
        Ok(())
    }

    /// Returns the current [`ProgressSnapshot`] for all given nodes.
    fn snapshot(&self, nodes: &BTreeMap<OperatorId, NodeRun>) -> ProgressSnapshot {
        ProgressSnapshot(
            nodes
                .iter()
                .map(|(id, node)| (id.clone(), NodeSnapshot::from_node(node)))
                .collect(),
        )
    }

    /// Returns `true` if the current snapshot differs from the previous one.
    ///
    /// A difference means at least one node changed observable state,
    /// attempt count, outputs, or terminal status — i.e. real progress.
    fn diff(&self, current: &ProgressSnapshot) -> bool {
        match &self.prev_snapshot {
            None => true,
            Some(prev) => prev != current,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::workflow_run::NodeRun;
    use sddk_domain::{NodeId, NodeRunState, OperatorId};
    use std::collections::BTreeSet;

    fn make_node(state: NodeRunState, _attempts: usize) -> NodeRun {
        // We use empty attempts in tests because Attempt requires many
        // non-defaultable fields. The no-progress detection primarily relies on
        // state + outputs_hash; with no attempts the hash is 0, which is fine
        // for testing the consecutive counter logic.
        NodeRun {
            node_id: NodeId("n1".into()),
            state,
            dependencies: BTreeSet::new(),
            attempts: vec![],
            expansion_permissions: BTreeSet::new(),
            schema_version: 1,
        }
    }

    #[test]
    fn pre_tick_ok_when_within_budget() {
        let budgets = sddk_domain::Budgets {
            max_wall_ms: 1_000_000,
            no_progress_threshold: u32::MAX,
            ..Default::default()
        };
        let mut ctrl = ExecutionController::from_budgets(&budgets);
        // Instant::now() just started, so we should be within budget
        assert!(ctrl.pre_tick().is_ok());
    }

    #[test]
    fn pre_tick_err_when_budget_exhausted() {
        let budgets = sddk_domain::Budgets {
            max_wall_ms: 0, // exhausted immediately
            no_progress_threshold: u32::MAX,
            ..Default::default()
        };
        let mut ctrl = ExecutionController::from_budgets(&budgets);
        // With max_wall_ms=0 the very first tick should fail
        assert!(matches!(
            ctrl.pre_tick(),
            Err(RuntimeError::BudgetExceeded { .. })
        ));
    }

    #[test]
    fn observe_detects_progress() {
        let budgets = sddk_domain::Budgets {
            max_wall_ms: u64::MAX,
            no_progress_threshold: 3,
            ..Default::default()
        };
        let mut ctrl = ExecutionController::from_budgets(&budgets);
        let mut nodes: BTreeMap<OperatorId, NodeRun> = BTreeMap::new();
        nodes.insert(OperatorId("n1".into()), make_node(NodeRunState::Pending, 0));

        // First observation — no previous snapshot, so always progress
        assert!(ctrl.observe(&nodes).is_ok());
        assert_eq!(ctrl.consecutive_no_progress, 0);

        // Same snapshot — no progress
        assert!(ctrl.observe(&nodes).is_ok());
        assert_eq!(ctrl.consecutive_no_progress, 1);

        // Same snapshot again — still no progress
        assert!(ctrl.observe(&nodes).is_ok());
        assert_eq!(ctrl.consecutive_no_progress, 2);

        // Third time hits threshold of 3 — should error
        assert!(matches!(
            ctrl.observe(&nodes),
            Err(RuntimeError::NoProgressDetected {
                consecutive: 3,
                threshold: 3
            })
        ));
    }

    #[test]
    fn observe_resets_on_progress() {
        let budgets = sddk_domain::Budgets {
            max_wall_ms: u64::MAX,
            no_progress_threshold: 3,
            ..Default::default()
        };
        let mut ctrl = ExecutionController::from_budgets(&budgets);
        let mut nodes: BTreeMap<OperatorId, NodeRun> = BTreeMap::new();
        nodes.insert(OperatorId("n1".into()), make_node(NodeRunState::Pending, 0));

        assert!(ctrl.observe(&nodes).is_ok()); // tick 1: progress
        assert_eq!(ctrl.consecutive_no_progress, 0);

        nodes.insert(OperatorId("n1".into()), make_node(NodeRunState::Running, 1));
        assert!(ctrl.observe(&nodes).is_ok()); // tick 2: different state = progress
        assert_eq!(ctrl.consecutive_no_progress, 0);
    }

    #[test]
    fn no_progress_threshold_max_disables_check() {
        let budgets = sddk_domain::Budgets {
            max_wall_ms: u64::MAX,
            no_progress_threshold: u32::MAX, // effectively disabled
            ..Default::default()
        };
        let mut ctrl = ExecutionController::from_budgets(&budgets);
        let nodes: BTreeMap<OperatorId, NodeRun> = BTreeMap::new();
        // With u32::MAX as threshold, the check is disabled — never error
        for _ in 0..1000 {
            assert!(
                ctrl.observe(&nodes).is_ok(),
                "u32::MAX should disable check"
            );
        }
    }

    #[test]
    fn observe_err_when_no_progress_threshold_reached() {
        // RED: verify NoProgressDetected fires when consecutive snapshots don't change.
        // The threshold is 3 — after 3 consecutive identical snapshots, error fires.
        let budgets = sddk_domain::Budgets {
            max_wall_ms: u64::MAX,
            no_progress_threshold: 3,
            ..Default::default()
        };
        let mut ctrl = ExecutionController::from_budgets(&budgets);
        let mut nodes: BTreeMap<OperatorId, NodeRun> = BTreeMap::new();
        nodes.insert(OperatorId("n1".into()), make_node(NodeRunState::Pending, 0));

        // First observation — no previous snapshot, so always progress
        assert!(ctrl.observe(&nodes).is_ok());
        assert_eq!(ctrl.consecutive_no_progress, 0);

        // Same snapshot — no progress (count = 1)
        assert!(ctrl.observe(&nodes).is_ok());
        assert_eq!(ctrl.consecutive_no_progress, 1);

        // Same snapshot again — no progress (count = 2)
        assert!(ctrl.observe(&nodes).is_ok());
        assert_eq!(ctrl.consecutive_no_progress, 2);

        // Fourth time hits threshold of 3 — should fire NoProgressDetected
        let result = ctrl.observe(&nodes);
        assert!(
            result.is_err(),
            "expected NoProgressDetected after 3 ticks, got: {:?}",
            result
        );
        let err = result.unwrap_err();
        assert!(
            matches!(
                err,
                RuntimeError::NoProgressDetected {
                    consecutive: 3,
                    threshold: 3
                }
            ),
            "expected NoProgressDetected {{consecutive:3, threshold:3}}, got: {:?}",
            err
        );
    }
}
