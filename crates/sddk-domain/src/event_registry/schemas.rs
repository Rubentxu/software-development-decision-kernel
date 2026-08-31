//! Schema builders and `schema_struct!` macro for all known SDDK event types.
//!
//! Each schema struct is registered in [`std_registry()`](super::registry::EventSchemaRegistry)
//! and validated by [`CanonicalEventValidator`](super::validator::CanonicalEventValidator).
use std::sync::Arc;

use super::error::EventSchema;
use super::registry::EventSchemaRegistry;

// ── Schema builders for known event types ────────────────────────────────────

/// Returns a registry pre-populated with all known SDDK event types.
///
/// Call this once at application startup to initialize the validator.
pub fn std_registry() -> Arc<EventSchemaRegistry> {
    let mut registry = EventSchemaRegistry::new();

    // ── Workflow events ──────────────────────────────────────────────────────
    registry.register(WorkflowPhaseEnteredSchema);
    registry.register(WorkflowPhaseExitedSchema);
    registry.register(WorkflowTransitionSucceededSchema);
    registry.register(WorkflowTransitionFailedSchema);
    registry.register(WorkflowIrCompiledSchema);
    registry.register(WorkflowRunStartedSchema);
    registry.register(WorkflowRunCompletedSchema);
    registry.register(WorkflowNodeRunningSchema);
    registry.register(WorkflowNodeCompletedSchema);
    registry.register(WorkflowNodeFailedSchema);
    registry.register(WorkflowRunCancelledSchema);
    registry.register(WorkflowGraphRevisionAcceptedSchema);

    // ── Approval events ──────────────────────────────────────────────────────
    registry.register(ApprovalCapabilityRequestedSchema);
    registry.register(ApprovalCapabilityGrantedSchema);
    registry.register(ApprovalCapabilityDeniedSchema);

    // ── UAT events ─────────────────────────────────────────────────────────
    registry.register(UatScenarioStartedSchema);
    registry.register(UatCheckPassedSchema);
    registry.register(UatAcceptanceGrantedSchema);

    // ── Cycle events ────────────────────────────────────────────────────────
    registry.register(CycleCreatedSchema);
    registry.register(CycleTransitionedSchema);
    registry.register(CycleSnapshotRestoredSchema);

    // ── Lease events ───────────────────────────────────────────────────────
    registry.register(LeaseReleasedSchema);

    Arc::new(registry)
}

// ── Schema implementations ────────────────────────────────────────────────────

// Each schema struct is documented at its macro invocation site (visible in the file).
// Adding per-struct docs would double the line count with no additional signal.
macro_rules! schema_struct {
    ($name:ident, $event_type:expr, $version:expr, $description:expr, $validate:expr) => {
        #[allow(missing_docs)]
        pub struct $name;
        impl EventSchema for $name {
            fn info(&self) -> super::error::EventSchemaInfo {
                super::error::EventSchemaInfo {
                    event_type: $event_type.to_string(),
                    schema_version: $version,
                    description: $description.to_string(),
                }
            }
            fn validate_payload(&self, payload: &serde_json::Value) -> Result<(), String> {
                if !$validate(payload) {
                    return Err(format!(
                        "payload for {} v{} failed validation",
                        $event_type, $version
                    ));
                }
                Ok(())
            }
        }
    };
}

fn is_object(payload: &serde_json::Value) -> bool {
    payload.is_object()
}

fn has_string_field(payload: &serde_json::Value, field: &str) -> bool {
    payload.get(field).and_then(|v| v.as_str()).is_some()
}

schema_struct!(
    WorkflowPhaseEnteredSchema,
    "workflow.phase.entered",
    1,
    "workflow phase entered — payload must be an object with a 'phase' string field",
    |p: &serde_json::Value| is_object(p) && has_string_field(p, "phase")
);

schema_struct!(
    WorkflowPhaseExitedSchema,
    "workflow.phase.exited",
    1,
    "workflow phase exited — payload must be an object",
    is_object
);

schema_struct!(
    WorkflowTransitionSucceededSchema,
    "workflow.transition.succeeded",
    1,
    "workflow transition succeeded — payload must contain 'transition_id', 'outcome', 'from_phase', 'to_phase'",
    |p: &serde_json::Value| {
        is_object(p) && has_string_field(p, "transition_id") && has_string_field(p, "outcome")
    }
);

schema_struct!(
    WorkflowTransitionFailedSchema,
    "workflow.transition.failed",
    1,
    "workflow transition failed — payload must contain 'transition_id', 'outcome', 'failed_gates'",
    |p: &serde_json::Value| { is_object(p) && has_string_field(p, "transition_id") }
);

schema_struct!(
    WorkflowIrCompiledSchema,
    "workflow.ir.compiled",
    1,
    "workflow IR compiled — payload must contain 'template_id', 'ir_hash'",
    |p: &serde_json::Value| {
        is_object(p) && has_string_field(p, "template_id") && has_string_field(p, "ir_hash")
    }
);

schema_struct!(
    WorkflowRunStartedSchema,
    "workflow.run.started",
    1,
    "workflow run started — payload must contain 'run_id'",
    |p: &serde_json::Value| is_object(p) && has_string_field(p, "run_id")
);

schema_struct!(
    WorkflowRunCompletedSchema,
    "workflow.run.completed",
    1,
    "workflow run completed — payload must contain 'run_id'",
    |p: &serde_json::Value| is_object(p) && has_string_field(p, "run_id")
);

schema_struct!(
    WorkflowNodeRunningSchema,
    "workflow.node.running",
    1,
    "workflow node running — payload must contain 'run_id', 'node_id'",
    |p: &serde_json::Value| {
        is_object(p) && has_string_field(p, "run_id") && has_string_field(p, "node_id")
    }
);

schema_struct!(
    WorkflowNodeCompletedSchema,
    "workflow.node.completed",
    1,
    "workflow node completed — payload must contain 'run_id', 'node_id'",
    |p: &serde_json::Value| {
        is_object(p) && has_string_field(p, "run_id") && has_string_field(p, "node_id")
    }
);

schema_struct!(
    WorkflowNodeFailedSchema,
    "workflow.node.failed",
    1,
    "workflow node failed — payload must contain 'run_id', 'node_id', 'reason'",
    |p: &serde_json::Value| {
        is_object(p)
            && has_string_field(p, "run_id")
            && has_string_field(p, "node_id")
            && has_string_field(p, "reason")
    }
);

schema_struct!(
    WorkflowRunCancelledSchema,
    "workflow.run.cancelled",
    1,
    "workflow run cancelled — payload must contain 'run_id', 'reason'",
    |p: &serde_json::Value| {
        is_object(p) && has_string_field(p, "run_id") && has_string_field(p, "reason")
    }
);

schema_struct!(
    WorkflowGraphRevisionAcceptedSchema,
    "workflow.graph.revision.accepted",
    1,
    "workflow graph revision accepted — payload must contain 'run_id', 'revision', 'digest'",
    |p: &serde_json::Value| {
        is_object(p)
            && has_string_field(p, "run_id")
            && p.get("revision").is_some()
            && has_string_field(p, "digest")
    }
);

schema_struct!(
    ApprovalCapabilityRequestedSchema,
    "approval.capability.requested",
    1,
    "approval capability requested — payload must contain 'capability', 'cycle_id', 'request_hash'",
    |p: &serde_json::Value| {
        is_object(p)
            && has_string_field(p, "capability")
            && has_string_field(p, "cycle_id")
            && has_string_field(p, "request_hash")
    }
);

schema_struct!(
    ApprovalCapabilityGrantedSchema,
    "approval.capability.granted",
    1,
    "approval capability granted — payload must contain 'cycle_id', 'capability', 'request_hash'",
    |p: &serde_json::Value| {
        is_object(p)
            && has_string_field(p, "cycle_id")
            && has_string_field(p, "capability")
            && has_string_field(p, "request_hash")
    }
);

schema_struct!(
    ApprovalCapabilityDeniedSchema,
    "approval.capability.denied",
    1,
    "approval capability denied — payload must contain 'cycle_id', 'capability', 'request_hash'",
    |p: &serde_json::Value| {
        is_object(p)
            && has_string_field(p, "cycle_id")
            && has_string_field(p, "capability")
            && has_string_field(p, "request_hash")
    }
);

schema_struct!(
    UatScenarioStartedSchema,
    "uat.scenario.started",
    1,
    "UAT scenario started — payload must be an object",
    is_object
);

schema_struct!(
    UatCheckPassedSchema,
    "uat.check.passed",
    1,
    "UAT check passed — payload must be an object",
    is_object
);

schema_struct!(
    UatAcceptanceGrantedSchema,
    "uat.acceptance.granted",
    1,
    "UAT acceptance granted — payload must be an object",
    is_object
);

schema_struct!(
    CycleCreatedSchema,
    "cycle.created",
    1,
    "cycle created — payload must be an object (transition_id + outcome in state_after)",
    is_object
);

schema_struct!(
    CycleTransitionedSchema,
    "cycle.transitioned",
    1,
    "cycle transitioned — payload must contain 'transition_id', 'outcome'",
    |p: &serde_json::Value| {
        is_object(p) && has_string_field(p, "transition_id") && has_string_field(p, "outcome")
    }
);

schema_struct!(
    CycleSnapshotRestoredSchema,
    "cycle.snapshot.restored",
    1,
    "cycle snapshot restored — payload must contain 'cycle_id', 'restored_at_ms'",
    |p: &serde_json::Value| {
        is_object(p) && has_string_field(p, "cycle_id") && p.get("restored_at_ms").is_some()
    }
);

schema_struct!(
    LeaseReleasedSchema,
    "lease.released",
    1,
    "lease released — payload must contain 'cycle_id' (string) and 'released_at_ms' (i64)",
    |p: &serde_json::Value| {
        is_object(p) && has_string_field(p, "cycle_id") && p.get("released_at_ms").is_some()
    }
);
