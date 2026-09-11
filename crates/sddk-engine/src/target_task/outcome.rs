//! `outcome` — typed execution results for `DagExecutor` (M6.3).
//!
//! Each task emits a `TaskOutcome`; the executor aggregates them into
//! an `ExecutionReport`. The shapes are serializable so the agent
//! surface can consume them as JSON envelopes.

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};

use super::TargetTaskError;

/// Per-task outcome produced by `DagExecutor`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskOutcome {
    /// Task id within the parent target.
    pub task_id: String,
    /// Outcome status.
    pub status: TaskStatus,
    /// Human-readable note (e.g. admission explanation, body message).
    pub note: String,
}

/// Status of a single task in a DAG walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task was admitted and its body ran to completion.
    Executed,
    /// Task was skipped (e.g. dry-run, or upstream Denied).
    Skipped,
    /// Authority Engine denied the task; executor halts.
    Denied,
    /// Task body returned an error.
    Failed,
    /// Task is a declaration stub with no wired body (SP-07): reported
    /// honestly instead of a fabricated `executed`.
    NotImplemented,
}

/// Top-level aggregate status of a walk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    /// All admitted tasks executed; none failed or were denied.
    Succeeded,
    /// Walk halted early due to a `Denied` admission.
    Denied,
    /// Walk halted early due to a task body returning an error.
    Failed,
    /// No execution took place; only admission was evaluated.
    DryRun,
    /// Walk completed but at least one task was an unimplemented stub
    /// (SP-07): the target did NOT deliver its intent.
    Degraded,
}

/// Aggregate report produced by `DagExecutor`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionReport {
    /// Target name.
    pub target: String,
    /// Top-level aggregate status.
    pub status: ExecutionStatus,
    /// Whether the walk ran in dry-run mode.
    pub dry_run: bool,
    /// Per-task outcomes in walk order.
    pub tasks: Vec<TaskOutcome>,
    /// Validation message from the registry (mirrors `TargetResolution.validation`).
    pub validation: String,
}

impl ExecutionReport {
    /// Construct an empty report for a target that did not resolve.
    pub fn unresolved(target: &str, validation: impl Into<String>) -> Self {
        Self {
            target: target.to_string(),
            status: ExecutionStatus::Failed,
            dry_run: false,
            tasks: vec![],
            validation: validation.into(),
        }
    }

    /// Number of tasks that were actually executed.
    pub fn executed_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Executed)
            .count()
    }

    /// Number of tasks that were skipped (dry-run or upstream denial).
    pub fn skipped_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Skipped)
            .count()
    }

    /// Whether any task was denied.
    pub fn had_denial(&self) -> bool {
        self.tasks.iter().any(|t| t.status == TaskStatus::Denied)
    }

    /// Whether any task halted the walk (denied or required approval).
    pub fn had_halt(&self) -> bool {
        self.had_denial()
            || self
                .tasks
                .iter()
                .any(|t| t.note.contains("awaiting approval"))
    }
}

// Convenience constructors used by the executor.
impl TaskOutcome {
    pub(crate) fn executed(task_id: impl Into<String>, note: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            status: TaskStatus::Executed,
            note: note.into(),
        }
    }

    pub(crate) fn skipped(task_id: impl Into<String>, note: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            status: TaskStatus::Skipped,
            note: note.into(),
        }
    }

    pub(crate) fn denied(task_id: impl Into<String>, note: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            status: TaskStatus::Denied,
            note: note.into(),
        }
    }

    pub(crate) fn not_implemented(task_id: impl Into<String>, note: impl Into<String>) -> Self {
        Self {
            task_id: task_id.into(),
            status: TaskStatus::NotImplemented,
            note: note.into(),
        }
    }
}

// Re-export TargetTaskError so callers do not need to import the
// crate root symbol.
pub type ExecutionError = TargetTaskError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_outcome_constructors_set_status_correctly() {
        assert_eq!(
            TaskOutcome::executed("a", "ok").status,
            TaskStatus::Executed
        );
        assert_eq!(
            TaskOutcome::skipped("a", "dry-run").status,
            TaskStatus::Skipped
        );
        assert_eq!(
            TaskOutcome::denied("a", "no capability").status,
            TaskStatus::Denied
        );
    }

    #[test]
    fn execution_report_unresolved_carries_validation() {
        let r = ExecutionReport::unresolved("ghost", "unknown target `ghost`");
        assert_eq!(r.status, ExecutionStatus::Failed);
        assert_eq!(r.tasks.len(), 0);
        assert!(r.validation.contains("unknown target"));
    }

    #[test]
    fn execution_report_counters_are_accurate() {
        let r = ExecutionReport {
            target: "run".into(),
            status: ExecutionStatus::Succeeded,
            dry_run: false,
            tasks: vec![
                TaskOutcome::executed("a", "ok"),
                TaskOutcome::executed("b", "ok"),
                TaskOutcome::skipped("c", "dry-run"),
            ],
            validation: "ok".into(),
        };
        assert_eq!(r.executed_count(), 2);
        assert_eq!(r.skipped_count(), 1);
        assert!(!r.had_denial());
    }

    #[test]
    fn json_roundtrip_preserves_status_enum() {
        let r = ExecutionReport {
            target: "t".into(),
            status: ExecutionStatus::DryRun,
            dry_run: true,
            tasks: vec![TaskOutcome::skipped("a", "dry")],
            validation: "ok".into(),
        };
        let json = serde_json::to_string(&r).expect("serialize");
        let parsed: ExecutionReport = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, r);
    }
}
