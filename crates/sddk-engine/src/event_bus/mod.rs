//! Bridges cycle phase transitions to the `events_v1` ledger substrate.

pub mod correlation;
pub mod emit;
pub mod envelopes;
pub mod storage_path;

// Re-export all public items for external consumers.
// External consumers (e.g. crates/sddk-cli/src/approval.rs) use:
//   sddk_engine::event_bus::{emit_approval_requested, emit_approval_decision, ...}
pub use correlation::{trace_causation_chain, with_causation, with_correlation_from_context};
pub use emit::{
    ApprovalDecisionInput, ApprovalRequestedInput, OutcomeEventInput, PhaseEventInput,
    WorkflowNodeEventInput, WorkflowRunEventInput, emit_approval_decision, emit_approval_requested,
    emit_outcome_event, emit_phase_event, emit_workflow_node_completed, emit_workflow_node_failed,
    emit_workflow_node_running, emit_workflow_run_completed, emit_workflow_run_started,
};
pub use envelopes::{build_event_envelope, build_outcome_envelope};
pub use storage_path::project_storage_dir;
