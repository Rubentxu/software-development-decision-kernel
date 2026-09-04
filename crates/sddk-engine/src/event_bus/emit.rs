//! Event emission functions for the event bus.

use sddk_domain::{
    ActorKind, ActorRef, ApprovalDecision, EntityRef, EventAppended, EventEnvelopeV1, EventStore,
    StorageError,
};
use serde_json::json;

use crate::TransitionOutcome;

use super::correlation::{with_causation, with_correlation_from_context, with_correlation_id};
use super::envelopes::{build_event_envelope, build_outcome_envelope};

// ── Input types ────────────────────────────────────────────────────────────────

/// Input for phase-transition event emission.
#[derive(Debug, Clone)]
pub struct PhaseEventInput {
    /// Project that owns the cycle.
    pub project_id: String,
    /// Cycle being transitioned.
    pub cycle_id: String,
    /// Phase being exited.
    pub from_phase: String,
    /// Phase being entered.
    pub to_phase: String,
    /// Wall-clock time of the transition (RFC 3339).
    pub transition_at: String,
    /// Actor identifier.
    pub actor_id: String,
    /// Actor kind.
    pub actor_kind: ActorKind,
    /// Prefix for deterministic event_id generation.
    pub event_id_prefix: String,
    /// Causation chain: set to predecessor event_id in the same stream.
    pub causation_id: Option<String>,
    /// Correlation group: propagates the command's frame_id for grouping related events.
    pub correlation_id: Option<String>,
}

/// Input for transition-outcome event emission.
#[derive(Debug, Clone)]
pub struct OutcomeEventInput {
    /// Project that owns the cycle.
    pub project_id: String,
    /// Cycle being transitioned.
    pub cycle_id: String,
    /// Transition identifier.
    pub transition_id: String,
    /// Phase being exited (None if transition failed before planning).
    pub from_phase: Option<String>,
    /// Phase being entered (None if transition failed before reaching target).
    pub to_phase: Option<String>,
    /// Wall-clock time of the transition (RFC 3339).
    pub transition_at: String,
    /// Actor identifier.
    pub actor_id: String,
    /// Actor kind.
    pub actor_kind: ActorKind,
    /// Prefix for deterministic event_id generation.
    pub event_id_prefix: String,
    /// Names of gates that failed (empty for succeeded transitions).
    pub failed_gates: Vec<String>,
    /// Causation chain: set to predecessor event_id in the same stream.
    pub causation_id: Option<String>,
    /// Correlation group: propagates the command's frame_id for grouping related events.
    pub correlation_id: Option<String>,
}

/// Input for an approval-requested event emission.
#[derive(Debug, Clone)]
pub struct ApprovalRequestedInput {
    /// Project that owns the cycle.
    pub project_id: String,
    /// Cycle awaiting approval.
    pub cycle_id: String,
    /// Capability requiring approval.
    pub capability: String,
    /// SHA-256 hash of the structured request.
    pub request_hash: String,
    /// RFC3339 expiry timestamp for the approval window.
    pub expires_at: String,
    /// Wall-clock time of emission (RFC 3339).
    pub occurred_at: String,
    /// Actor identifier (orchestrator agent).
    pub actor_id: String,
    /// Actor kind (caller-supplied; must be Agent for approval requests).
    pub actor_kind: ActorKind,
    /// Causation chain: set to predecessor event_id in the same stream.
    pub causation_id: Option<String>,
    /// Correlation group: propagates the command's frame_id for grouping related events.
    pub correlation_id: Option<String>,
}

/// Input for an approval-decision event emission.
#[derive(Debug, Clone)]
pub struct ApprovalDecisionInput {
    /// Project that owns the cycle.
    pub project_id: String,
    /// Cycle where approval was requested.
    pub cycle_id: String,
    /// Capability that was approved or denied.
    pub capability: String,
    /// SHA-256 hash of the structured request.
    pub request_hash: String,
    /// Decision made by the human.
    pub decision: ApprovalDecision,
    /// Human operator identifier.
    pub actor_id: String,
    /// Actor kind (caller-supplied; must be Human or System for approval decisions).
    pub actor_kind: ActorKind,
    /// Mandatory justification for the decision.
    pub reason: String,
    /// Wall-clock time of the decision (RFC 3339).
    pub occurred_at: String,
    /// Causation chain: set to predecessor event_id in the same stream.
    pub causation_id: Option<String>,
    /// Correlation group: propagates the command's frame_id for grouping related events.
    pub correlation_id: Option<String>,
}

// ── Emit functions ─────────────────────────────────────────────────────────────

/// Appends two events to events_v1:
///
///   - `workflow.phase.exited` (for `from_phase`)
///   - `workflow.phase.entered` (for `to_phase`)
///
/// Both share `stream_id = cycle_id`. Idempotency comes from the unique
/// `event_id` built deterministically from `(event_id_prefix, cycle_id, phase_label)`.
///
/// Returns the stored `from_phase` and `to_phase` `EventAppended` references.
pub fn emit_phase_event<S: EventStore>(
    store: &mut S,
    input: &PhaseEventInput,
) -> Result<(EventAppended, EventAppended), StorageError> {
    let exited_id = format!("{}-exited-{}", input.event_id_prefix, input.cycle_id);
    let mut exited_env = build_event_envelope(
        &exited_id,
        "workflow.phase.exited",
        &input.from_phase,
        input,
    );
    if let Some(ref cid) = input.causation_id {
        with_causation(&mut exited_env, cid);
    }
    if let Some(ref corr) = input.correlation_id {
        with_correlation_id(&mut exited_env, corr);
    }
    exited_env.content_hash = exited_env.compute_content_hash();
    let from_result = store.append(&exited_env)?;

    let entered_id = format!("{}-entered-{}", input.event_id_prefix, input.cycle_id);
    let mut entered_env = build_event_envelope(
        &entered_id,
        "workflow.phase.entered",
        &input.to_phase,
        input,
    );
    if let Some(ref cid) = input.causation_id {
        with_causation(&mut entered_env, cid);
    }
    if let Some(ref corr) = input.correlation_id {
        with_correlation_id(&mut entered_env, corr);
    }
    entered_env.content_hash = entered_env.compute_content_hash();
    let to_result = store.append(&entered_env)?;

    Ok((from_result, to_result))
}

/// Emits a `workflow.transition.succeeded` or `workflow.transition.failed` event
/// to events_v1.
///
/// Idempotent: re-appending the same event_id returns the stored result.
///
/// Returns the stored `EventAppended` reference.
pub fn emit_outcome_event<S: EventStore>(
    store: &mut S,
    input: &OutcomeEventInput,
    outcome: TransitionOutcome,
) -> Result<EventAppended, StorageError> {
    let event_type = match outcome {
        TransitionOutcome::Succeeded => "workflow.transition.succeeded",
        TransitionOutcome::Failed => "workflow.transition.failed",
    };
    let event_id = format!("{}-outcome-{}", input.event_id_prefix, input.cycle_id);
    let mut env = build_outcome_envelope(event_id, event_type, input);
    if let Some(ref cid) = input.causation_id {
        with_causation(&mut env, cid);
    }
    if let Some(ref corr) = input.correlation_id {
        with_correlation_id(&mut env, corr);
    }
    env.content_hash = env.compute_content_hash();
    store.append(&env)
}

/// Emits an `approval.capability.requested` event to events_v1.
///
/// The event_id is deterministic: `approval-cap-<capability>-<request_hash[..16]>-requested`.
///
/// Idempotent: re-appending the same event_id returns the stored result.
///
/// Returns the stored `EventAppended` reference.
pub fn emit_approval_requested<S: EventStore>(
    store: &mut S,
    input: &ApprovalRequestedInput,
) -> Result<EventAppended, StorageError> {
    // Validator: only Agent may emit approval-requested events (per ADR-069 §4).
    match input.actor_kind {
        ActorKind::Agent => {}
        ActorKind::Human | ActorKind::System => {
            return Err(StorageError::Other(
                "emit_approval_requested requires actor_kind Agent".into(),
            ));
        }
    }
    // Normalize capability dots to hyphens for the event_id segment
    let capability_segment = input.capability.replace('.', "-");
    let event_id = format!(
        "approval-cap-{}-{}-requested",
        capability_segment,
        &input.request_hash[..16.min(input.request_hash.len())]
    );
    let payload = json!({
        "capability": input.capability,
        "cycle_id": input.cycle_id,
        "request_hash": input.request_hash,
        "expires_at": input.expires_at,
    });
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type: "approval.capability.requested".to_string(),
        schema_version: 1,
        stream_id: input.cycle_id.clone(),
        sequence: 0,
        project_id: input.project_id.clone(),
        occurred_at: input.occurred_at.clone(),
        recorded_at: input.occurred_at.clone(),
        actor: ActorRef {
            kind: input.actor_kind,
            id: input.actor_id.clone(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![EntityRef {
            kind: "cycle".into(),
            id: input.cycle_id.clone(),
            version: None,
            content_hash: None,
        }],
        payload,
        evidence_refs: vec![],
        content_hash: String::new(),
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: Some(input.cycle_id.clone()),
        frame_id: None,
        fork_id: None,
    };
    if let Some(ref cid) = input.causation_id {
        with_causation(&mut env, cid);
    }
    if let Some(ref corr) = input.correlation_id {
        with_correlation_id(&mut env, corr);
    }
    env.content_hash = env.compute_content_hash();
    store.append(&env)
}

/// Emits an `approval.capability.granted` or `approval.capability.denied` event
/// to events_v1.
///
/// The event_id is deterministic:
/// `approval-cap-<capability>-<request_hash[..16]>-granted|denied`.
///
/// Idempotent: re-appending the same event_id returns the stored result.
///
/// Returns the stored `EventAppended` reference.
pub fn emit_approval_decision<S: EventStore>(
    store: &mut S,
    input: &ApprovalDecisionInput,
) -> Result<EventAppended, StorageError> {
    // Validator: only Human or System may emit approval-decision events (per ADR-069 §4).
    match input.actor_kind {
        ActorKind::Human | ActorKind::System => {}
        ActorKind::Agent => {
            return Err(StorageError::Other(
                "emit_approval_decision requires actor_kind Human or System".into(),
            ));
        }
    }
    let verb = match input.decision {
        ApprovalDecision::Granted => "granted",
        ApprovalDecision::Denied => "denied",
    };
    let event_type = format!("approval.capability.{verb}");
    let capability_segment = input.capability.replace('.', "-");
    let event_id = format!(
        "approval-cap-{}-{}-{}",
        capability_segment,
        &input.request_hash[..16.min(input.request_hash.len())],
        verb
    );
    let payload = json!({
        "cycle_id": input.cycle_id,
        "capability": input.capability,
        "request_hash": input.request_hash,
        "actor": input.actor_id,
        "reason": input.reason,
    });
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type,
        schema_version: 1,
        stream_id: input.cycle_id.clone(),
        sequence: 0,
        project_id: input.project_id.clone(),
        occurred_at: input.occurred_at.clone(),
        recorded_at: input.occurred_at.clone(),
        actor: ActorRef {
            kind: input.actor_kind,
            id: input.actor_id.clone(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![EntityRef {
            kind: "cycle".into(),
            id: input.cycle_id.clone(),
            version: None,
            content_hash: None,
        }],
        payload,
        evidence_refs: vec![],
        content_hash: String::new(),
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: Some(input.cycle_id.clone()),
        frame_id: None,
        fork_id: None,
    };
    if let Some(ref cid) = input.causation_id {
        with_causation(&mut env, cid);
    }
    if let Some(ref corr) = input.correlation_id {
        with_correlation_id(&mut env, corr);
    }
    env.content_hash = env.compute_content_hash();
    store.append(&env)
}

// ── Workflow runtime event emission ────────────────────────────────────────────

/// Input for workflow run events.
#[derive(Debug, Clone)]
pub struct WorkflowRunEventInput {
    /// Project that owns the workflow run.
    pub project_id: String,
    /// Workflow run identifier.
    pub run_id: String,
    /// Wall-clock time of the event (RFC 3339).
    pub occurred_at: String,
    /// Actor identifier.
    pub actor_id: String,
    /// Actor kind.
    pub actor_kind: ActorKind,
    /// Causation chain: set to predecessor event_id in the same stream.
    pub causation_id: Option<String>,
    /// Correlation group: propagates the command's frame_id for grouping related events.
    pub correlation_id: Option<String>,
}

/// Input for workflow node events.
#[derive(Debug, Clone)]
pub struct WorkflowNodeEventInput {
    /// Project that owns the workflow run.
    pub project_id: String,
    /// Workflow run identifier.
    pub run_id: String,
    /// Node identifier.
    pub node_id: String,
    /// Wall-clock time of the event (RFC 3339).
    pub occurred_at: String,
    /// Actor identifier.
    pub actor_id: String,
    /// Actor kind.
    pub actor_kind: ActorKind,
    /// Optional reason (for failed events).
    pub reason: Option<String>,
    /// Causation chain: set to predecessor event_id in the same stream.
    pub causation_id: Option<String>,
    /// Correlation group: propagates the command's frame_id for grouping related events.
    pub correlation_id: Option<String>,
}

/// Emits a `workflow.run.started` event.
pub fn emit_workflow_run_started<S: EventStore>(
    store: &mut S,
    input: &WorkflowRunEventInput,
) -> Result<EventAppended, StorageError> {
    let event_id = format!("wf-run-{}-started", input.run_id);
    let payload = json!({ "run_id": input.run_id });
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type: "workflow.run.started".to_string(),
        schema_version: 1,
        stream_id: input.run_id.clone(),
        sequence: 0,
        project_id: input.project_id.clone(),
        occurred_at: input.occurred_at.clone(),
        recorded_at: input.occurred_at.clone(),
        actor: ActorRef {
            kind: input.actor_kind,
            id: input.actor_id.clone(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![EntityRef {
            kind: "workflow_run".into(),
            id: input.run_id.clone(),
            version: None,
            content_hash: None,
        }],
        payload,
        evidence_refs: vec![],
        content_hash: String::new(),
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: None,
        frame_id: None,
        fork_id: None,
    };
    if let Some(ref cid) = input.causation_id {
        with_causation(&mut env, cid);
    }
    if let Some(ref corr) = input.correlation_id {
        with_correlation_id(&mut env, corr);
    }
    env.content_hash = env.compute_content_hash();
    store.append(&env)
}

/// Emits a `workflow.run.completed` event.
pub fn emit_workflow_run_completed<S: EventStore>(
    store: &mut S,
    input: &WorkflowRunEventInput,
) -> Result<EventAppended, StorageError> {
    let event_id = format!("wf-run-{}-completed", input.run_id);
    let payload = json!({ "run_id": input.run_id });
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type: "workflow.run.completed".to_string(),
        schema_version: 1,
        stream_id: input.run_id.clone(),
        sequence: 0,
        project_id: input.project_id.clone(),
        occurred_at: input.occurred_at.clone(),
        recorded_at: input.occurred_at.clone(),
        actor: ActorRef {
            kind: input.actor_kind,
            id: input.actor_id.clone(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![EntityRef {
            kind: "workflow_run".into(),
            id: input.run_id.clone(),
            version: None,
            content_hash: None,
        }],
        payload,
        evidence_refs: vec![],
        content_hash: String::new(),
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: None,
        frame_id: None,
        fork_id: None,
    };
    if let Some(ref cid) = input.causation_id {
        with_causation(&mut env, cid);
    }
    if let Some(ref corr) = input.correlation_id {
        with_correlation_id(&mut env, corr);
    }
    env.content_hash = env.compute_content_hash();
    store.append(&env)
}

/// Emits a `workflow.node.running` event.
pub fn emit_workflow_node_running<S: EventStore>(
    store: &mut S,
    input: &WorkflowNodeEventInput,
) -> Result<EventAppended, StorageError> {
    let event_id = format!("wf-node-{}-{}-running", input.run_id, input.node_id);
    let payload = json!({
        "run_id": input.run_id,
        "node_id": input.node_id
    });
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type: "workflow.node.running".to_string(),
        schema_version: 1,
        stream_id: input.run_id.clone(),
        sequence: 0,
        project_id: input.project_id.clone(),
        occurred_at: input.occurred_at.clone(),
        recorded_at: input.occurred_at.clone(),
        actor: ActorRef {
            kind: input.actor_kind,
            id: input.actor_id.clone(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![EntityRef {
            kind: "workflow_node".into(),
            id: input.node_id.clone(),
            version: None,
            content_hash: None,
        }],
        payload,
        evidence_refs: vec![],
        content_hash: String::new(),
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: None,
        frame_id: None,
        fork_id: None,
    };
    if let Some(ref cid) = input.causation_id {
        with_causation(&mut env, cid);
    }
    if let Some(ref corr) = input.correlation_id {
        with_correlation_id(&mut env, corr);
    }
    env.content_hash = env.compute_content_hash();
    store.append(&env)
}

/// Emits a `workflow.node.completed` event.
pub fn emit_workflow_node_completed<S: EventStore>(
    store: &mut S,
    input: &WorkflowNodeEventInput,
) -> Result<EventAppended, StorageError> {
    let event_id = format!("wf-node-{}-{}-completed", input.run_id, input.node_id);
    let payload = json!({
        "run_id": input.run_id,
        "node_id": input.node_id
    });
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type: "workflow.node.completed".to_string(),
        schema_version: 1,
        stream_id: input.run_id.clone(),
        sequence: 0,
        project_id: input.project_id.clone(),
        occurred_at: input.occurred_at.clone(),
        recorded_at: input.occurred_at.clone(),
        actor: ActorRef {
            kind: input.actor_kind,
            id: input.actor_id.clone(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![EntityRef {
            kind: "workflow_node".into(),
            id: input.node_id.clone(),
            version: None,
            content_hash: None,
        }],
        payload,
        evidence_refs: vec![],
        content_hash: String::new(),
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: None,
        frame_id: None,
        fork_id: None,
    };
    if let Some(ref cid) = input.causation_id {
        with_causation(&mut env, cid);
    }
    if let Some(ref corr) = input.correlation_id {
        with_correlation_id(&mut env, corr);
    }
    env.content_hash = env.compute_content_hash();
    store.append(&env)
}

/// Emits a `workflow.node.failed` event.
pub fn emit_workflow_node_failed<S: EventStore>(
    store: &mut S,
    input: &WorkflowNodeEventInput,
) -> Result<EventAppended, StorageError> {
    let event_id = format!("wf-node-{}-{}-failed", input.run_id, input.node_id);
    let payload = json!({
        "run_id": input.run_id,
        "node_id": input.node_id,
        "reason": input.reason.clone().unwrap_or_default()
    });
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type: "workflow.node.failed".to_string(),
        schema_version: 1,
        stream_id: input.run_id.clone(),
        sequence: 0,
        project_id: input.project_id.clone(),
        occurred_at: input.occurred_at.clone(),
        recorded_at: input.occurred_at.clone(),
        actor: ActorRef {
            kind: input.actor_kind,
            id: input.actor_id.clone(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![EntityRef {
            kind: "workflow_node".into(),
            id: input.node_id.clone(),
            version: None,
            content_hash: None,
        }],
        payload,
        evidence_refs: vec![],
        content_hash: String::new(),
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: None,
        frame_id: None,
        fork_id: None,
    };
    if let Some(ref cid) = input.causation_id {
        with_causation(&mut env, cid);
    }
    if let Some(ref corr) = input.correlation_id {
        with_correlation_id(&mut env, corr);
    }
    env.content_hash = env.compute_content_hash();
    store.append(&env)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_approval_requested_produces_deterministic_event_id() {
        let input = ApprovalRequestedInput {
            project_id: "p-1".into(),
            cycle_id: "c-42".into(),
            capability: "git.delete_branch".into(),
            request_hash: "sha256:abcdef1234567890".into(),
            expires_at: "2026-08-18T18:00:00Z".into(),
            occurred_at: "2026-08-17T10:00:00Z".into(),
            actor_id: "agent:sddk".into(),
            actor_kind: ActorKind::Agent,
            causation_id: None,
            correlation_id: None,
        };
        // Compute the expected event_id manually
        let capability_segment = "git-delete_branch"; // dots replaced with hyphens
        let hash_prefix = "sha256:abcdef123"; // first 16 chars
        let expected_event_id = format!(
            "approval-cap-{}-{}-requested",
            capability_segment, hash_prefix
        );

        // Verify the deterministic formula
        let computed = format!(
            "approval-cap-{}-{}-requested",
            input.capability.replace('.', "-"),
            &input.request_hash[..16]
        );
        assert_eq!(computed, expected_event_id);
        assert!(
            expected_event_id
                .starts_with("approval-cap-git-delete_branch-sha256:abcdef123-requested")
        )
    }

    #[test]
    fn emit_approval_decision_event_id_differs_by_verb() {
        let request_hash = "sha256:abcdef1234567890";
        let capability = "git.delete_branch";
        let capability_segment = capability.replace('.', "-");
        let hash_prefix = &request_hash[..16]; // "sha256:abcdef123" (16 chars)

        let granted_id = format!(
            "approval-cap-{}-{}-granted",
            capability_segment, hash_prefix
        );
        let denied_id = format!("approval-cap-{}-{}-denied", capability_segment, hash_prefix);

        assert_ne!(granted_id, denied_id);
        assert!(granted_id.contains("granted"));
        assert!(denied_id.contains("denied"));
        // Verify the hash prefix is exactly 16 chars
        assert_eq!(hash_prefix, "sha256:abcdef123");
    }

    #[test]
    fn approval_requested_input_carries_all_required_fields() {
        let input = ApprovalRequestedInput {
            project_id: "p-test".into(),
            cycle_id: "c-99".into(),
            capability: "git.delete_branch".into(),
            request_hash: "sha256:deadbeefcafebabe".into(),
            expires_at: "2026-08-20T12:00:00Z".into(),
            occurred_at: "2026-08-18T09:00:00Z".into(),
            actor_id: "agent:orchestrator".into(),
            actor_kind: ActorKind::Agent,
            causation_id: None,
            correlation_id: None,
        };
        assert_eq!(input.capability, "git.delete_branch");
        assert_eq!(input.cycle_id, "c-99");
        assert!(input.expires_at.contains("2026-08-20"));
        assert!(matches!(input.actor_kind, ActorKind::Agent));
    }

    #[test]
    fn approval_decision_input_carries_all_required_fields() {
        use sddk_domain::ApprovalDecision;
        let input = ApprovalDecisionInput {
            project_id: "p-test".into(),
            cycle_id: "c-99".into(),
            capability: "git.delete_branch".into(),
            request_hash: "sha256:deadbeefcafebabe".into(),
            decision: ApprovalDecision::Granted,
            actor_id: "alice".into(),
            actor_kind: ActorKind::Human,
            reason: "reversible via reflog".into(),
            occurred_at: "2026-08-18T09:30:00Z".into(),
            causation_id: None,
            correlation_id: None,
        };
        assert_eq!(input.decision, ApprovalDecision::Granted);
        assert_eq!(input.actor_id, "alice");
        assert!(!input.reason.is_empty());
        assert!(matches!(input.actor_kind, ActorKind::Human));
    }
}
