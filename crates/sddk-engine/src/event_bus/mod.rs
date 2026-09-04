//! Bridges cycle phase transitions to the `events_v1` ledger substrate.

pub mod correlation;
pub mod emit;
pub mod envelopes;
pub mod storage_path;

// Re-export all public items for external consumers.
// External consumers (e.g. crates/sddk-cli/src/approval.rs) use:
//   sddk_engine::event_bus::{emit_approval_requested, emit_approval_decision, ...}
pub use correlation::{
    trace_causation_chain, with_causation, with_correlation_from_context, with_correlation_id,
};
pub use emit::{
    ApprovalDecisionInput, ApprovalRequestedInput, OutcomeEventInput, PhaseEventInput,
    PlanningDependencyEventInput, PlanningWorkItemEventInput, WorkflowNodeEventInput,
    WorkflowRunEventInput, emit_approval_decision, emit_approval_requested, emit_outcome_event,
    emit_phase_event, emit_planning_work_item_activated, emit_planning_work_item_completed,
    emit_planning_work_item_drafted, emit_planning_work_item_paused,
    emit_planning_work_item_resumed, emit_planning_work_item_superseded,
    emit_workflow_node_completed, emit_workflow_node_failed, emit_workflow_node_running,
    emit_workflow_run_completed, emit_workflow_run_started,
};
pub use envelopes::{build_event_envelope, build_outcome_envelope};
pub use storage_path::project_storage_dir;
