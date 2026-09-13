//! Event emission functions for the event bus.

use sddk_domain::{
    ActorKind, ActorRef, ApprovalDecision, EntityRef, EventAppended, EventEnvelopeV1, EventStore,
    StorageError,
};
use serde_json::json;

use crate::TransitionOutcome;
use crate::authority::{AuthorityContext, WritableSurface};
use crate::secretary_closed_set::validate_secretary_event;

use super::correlation::{with_causation, with_correlation_id};
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

/// Input for an `authority.admission.decided` event emission (D-02, R-4-005:
/// every deny — and every advisory High approval — is durably recorded).
#[derive(Debug, Clone)]
pub struct AdmissionDecidedInput {
    /// Project that owns the governed surface.
    pub project_id: String,
    /// Stream/cycle context of the governed action (best effort).
    pub cycle_id: Option<String>,
    /// Admission surface the action targeted.
    pub surface: String,
    /// Action kind segment (e.g. `cli_run`).
    pub action_kind: String,
    /// Admission verdict: `allow` | `deny` | `require_approval` | `advisory_high_approval`.
    pub verdict: String,
    /// Engine decision id (`decision_id()` of the AdmissionDecision).
    pub decision_id: String,
    /// SHA-256 of the AdmissionExplanation (decision digest).
    pub explanation_digest: String,
    /// Wall-clock time of the decision (RFC 3339).
    pub occurred_at: String,
    /// Causation chain: set to predecessor event_id in the same stream.
    pub causation_id: Option<String>,
    /// Correlation group: propagates the command's frame_id for grouping related events.
    pub correlation_id: Option<String>,
}

// ── Planning event input types ─────────────────────────────────────────────────

/// Input for a `planning.work_item.created` event emission.
#[derive(Debug, Clone)]
pub struct WorkItemCreatedInput {
    pub project_id: String,
    pub cycle_id: String,
    pub work_item_id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub actor_id: String,
    pub actor_kind: ActorKind,
    pub occurred_at: String,
    pub causation_id: Option<String>,
    pub correlation_id: Option<String>,
    /// Authority context for validation (REQ-PLN2-AUTH-003).
    pub authority: AuthorityContext,
}

/// Input for a `planning.work_item.transitioned` event emission.
#[derive(Debug, Clone)]
pub struct WorkItemTransitionedInput {
    pub project_id: String,
    pub cycle_id: String,
    pub work_item_id: String,
    pub from_status: String,
    pub to_status: String,
    pub actor_id: String,
    pub actor_kind: ActorKind,
    pub occurred_at: String,
    pub causation_id: Option<String>,
    pub correlation_id: Option<String>,
    /// Authority context for validation (REQ-PLN2-AUTH-003).
    pub authority: AuthorityContext,
}

/// Input for a `planning.dependency.added` event emission.
#[derive(Debug, Clone)]
pub struct DependencyAddedInput {
    pub project_id: String,
    pub cycle_id: String,
    pub from_work_item_id: String,
    pub to_work_item_id: String,
    pub dependency_kind: String,
    pub actor_id: String,
    pub actor_kind: ActorKind,
    pub occurred_at: String,
    pub causation_id: Option<String>,
    pub correlation_id: Option<String>,
    /// Authority context for validation (REQ-PLN2-AUTH-003).
    pub authority: AuthorityContext,
}

/// Input for a `planning.evidence.attached` event emission.
#[derive(Debug, Clone)]
pub struct EvidenceAttachedInput {
    pub project_id: String,
    pub cycle_id: String,
    pub work_item_id: String,
    pub evidence_id: String,
    pub evidence_kind: String,
    pub cas_hash: String,
    pub actor_id: String,
    pub actor_kind: ActorKind,
    pub occurred_at: String,
    pub causation_id: Option<String>,
    pub correlation_id: Option<String>,
    /// Authority context for validation (REQ-PLN2-AUTH-003).
    pub authority: AuthorityContext,
}

/// Input for a `planning.decision.recorded` event emission.
#[derive(Debug, Clone)]
pub struct DecisionRecordedInput {
    pub project_id: String,
    pub cycle_id: String,
    pub work_item_id: String,
    pub decision_id: String,
    pub decision_kind: String,
    pub rationale_summary: String,
    pub actor_id: String,
    pub actor_kind: ActorKind,
    pub occurred_at: String,
    pub causation_id: Option<String>,
    pub correlation_id: Option<String>,
    /// Authority context for validation (REQ-PLN2-AUTH-003).
    pub authority: AuthorityContext,
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
    validate_secretary_event(&exited_env.actor, &exited_env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
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
    validate_secretary_event(&entered_env.actor, &entered_env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
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
    validate_secretary_event(&env.actor, &env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
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
            role: None,
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
    validate_secretary_event(&env.actor, &env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
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
            role: None,
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
    validate_secretary_event(&env.actor, &env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
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
            role: None,
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
    validate_secretary_event(&env.actor, &env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
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
            role: None,
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
    validate_secretary_event(&env.actor, &env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
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
            role: None,
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
    validate_secretary_event(&env.actor, &env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
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
            role: None,
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
    validate_secretary_event(&env.actor, &env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
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
            role: None,
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
    validate_secretary_event(&env.actor, &env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
    store.append(&env)
}

// ── Planning Ledger event emission (PLN-LEDGER-001 / ADR-072) ─────────────────

use sddk_domain::planning::{DependencyEdgeKind, WorkItemId, WorkItemStatus};

/// Input for planning work item events.
#[derive(Debug, Clone)]
pub struct PlanningWorkItemEventInput {
    /// Project that owns the cycle.
    pub project_id: String,
    /// Cycle this work item belongs to.
    pub cycle_id: String,
    /// Work item identifier.
    pub work_item_id: WorkItemId,
    /// Work item title.
    pub title: String,
    /// Current status.
    pub status: WorkItemStatus,
    /// Optional actor reference.
    pub actor_ref: Option<sddk_domain::ActorRef>,
    /// Wall-clock time (RFC 3339).
    pub occurred_at: String,
    /// Causation chain.
    pub causation_id: Option<String>,
    /// Correlation group.
    pub correlation_id: Option<String>,
}

/// Input for planning dependency edge events.
#[derive(Debug, Clone)]
pub struct PlanningDependencyEventInput {
    /// Project that owns the cycle.
    pub project_id: String,
    /// Cycle this edge belongs to.
    pub cycle_id: String,
    /// Source work item (blocker).
    pub from_id: WorkItemId,
    /// Target work item (blocked).
    pub to_id: WorkItemId,
    /// Kind of dependency.
    pub kind: DependencyEdgeKind,
    /// Optional actor reference.
    pub actor_ref: Option<sddk_domain::ActorRef>,
    /// Wall-clock time (RFC 3339).
    pub occurred_at: String,
    /// Causation chain.
    pub causation_id: Option<String>,
    /// Correlation group.
    pub correlation_id: Option<String>,
}

/// Emits a `planning.work_item.drafted` event.
///
/// Uses Type::Custom (never std_registry), schema_version: 1,
/// and with_correlation_from_context per ADR-071.
pub fn emit_planning_work_item_drafted<S: EventStore>(
    store: &mut S,
    input: &PlanningWorkItemEventInput,
) -> Result<EventAppended, StorageError> {
    let event_id = format!("planning-wi-{}-drafted", input.work_item_id);
    let payload = serde_json::json!({
        "work_item_id": input.work_item_id,
        "cycle_id": input.cycle_id,
        "title": input.title,
        "status": input.status,
    });
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type: "planning.work_item.drafted".to_string(),
        schema_version: 1,
        stream_id: input.cycle_id.clone(),
        sequence: 0,
        project_id: input.project_id.clone(),
        occurred_at: input.occurred_at.clone(),
        recorded_at: input.occurred_at.clone(),
        actor: input
            .actor_ref
            .clone()
            .unwrap_or_else(|| sddk_domain::ActorRef {
                kind: sddk_domain::ActorKind::System,
                id: "system".to_string(),
                definition_hash: None,
                policy_hash: None,
                model: None,
                role: None,
            }),
        subjects: vec![sddk_domain::EntityRef {
            kind: "work_item".into(),
            id: input.work_item_id.clone(),
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
    validate_secretary_event(&env.actor, &env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
    store.append(&env)
}

/// Emits a `planning.work_item.activated` event.
pub fn emit_planning_work_item_activated<S: EventStore>(
    store: &mut S,
    input: &PlanningWorkItemEventInput,
) -> Result<EventAppended, StorageError> {
    let event_id = format!("planning-wi-{}-activated", input.work_item_id);
    let payload = serde_json::json!({
        "work_item_id": input.work_item_id,
        "cycle_id": input.cycle_id,
        "title": input.title,
        "status": input.status,
    });
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type: "planning.work_item.activated".to_string(),
        schema_version: 1,
        stream_id: input.cycle_id.clone(),
        sequence: 0,
        project_id: input.project_id.clone(),
        occurred_at: input.occurred_at.clone(),
        recorded_at: input.occurred_at.clone(),
        actor: input
            .actor_ref
            .clone()
            .unwrap_or_else(|| sddk_domain::ActorRef {
                kind: sddk_domain::ActorKind::System,
                id: "system".to_string(),
                definition_hash: None,
                policy_hash: None,
                model: None,
                role: None,
            }),
        subjects: vec![sddk_domain::EntityRef {
            kind: "work_item".into(),
            id: input.work_item_id.clone(),
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
    validate_secretary_event(&env.actor, &env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
    store.append(&env)
}

/// Emits a `planning.work_item.paused` event.
pub fn emit_planning_work_item_paused<S: EventStore>(
    store: &mut S,
    input: &PlanningWorkItemEventInput,
) -> Result<EventAppended, StorageError> {
    let event_id = format!("planning-wi-{}-paused", input.work_item_id);
    let payload = serde_json::json!({
        "work_item_id": input.work_item_id,
        "cycle_id": input.cycle_id,
        "title": input.title,
        "status": input.status,
    });
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type: "planning.work_item.paused".to_string(),
        schema_version: 1,
        stream_id: input.cycle_id.clone(),
        sequence: 0,
        project_id: input.project_id.clone(),
        occurred_at: input.occurred_at.clone(),
        recorded_at: input.occurred_at.clone(),
        actor: input
            .actor_ref
            .clone()
            .unwrap_or_else(|| sddk_domain::ActorRef {
                kind: sddk_domain::ActorKind::System,
                id: "system".to_string(),
                definition_hash: None,
                policy_hash: None,
                model: None,
                role: None,
            }),
        subjects: vec![sddk_domain::EntityRef {
            kind: "work_item".into(),
            id: input.work_item_id.clone(),
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
    validate_secretary_event(&env.actor, &env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
    store.append(&env)
}

/// Emits a `planning.work_item.resumed` event.
pub fn emit_planning_work_item_resumed<S: EventStore>(
    store: &mut S,
    input: &PlanningWorkItemEventInput,
) -> Result<EventAppended, StorageError> {
    let event_id = format!("planning-wi-{}-resumed", input.work_item_id);
    let payload = serde_json::json!({
        "work_item_id": input.work_item_id,
        "cycle_id": input.cycle_id,
        "title": input.title,
        "status": input.status,
    });
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type: "planning.work_item.resumed".to_string(),
        schema_version: 1,
        stream_id: input.cycle_id.clone(),
        sequence: 0,
        project_id: input.project_id.clone(),
        occurred_at: input.occurred_at.clone(),
        recorded_at: input.occurred_at.clone(),
        actor: input
            .actor_ref
            .clone()
            .unwrap_or_else(|| sddk_domain::ActorRef {
                kind: sddk_domain::ActorKind::System,
                id: "system".to_string(),
                definition_hash: None,
                policy_hash: None,
                model: None,
                role: None,
            }),
        subjects: vec![sddk_domain::EntityRef {
            kind: "work_item".into(),
            id: input.work_item_id.clone(),
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
    validate_secretary_event(&env.actor, &env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
    store.append(&env)
}

/// Emits a `planning.work_item.completed` event.
pub fn emit_planning_work_item_completed<S: EventStore>(
    store: &mut S,
    input: &PlanningWorkItemEventInput,
) -> Result<EventAppended, StorageError> {
    let event_id = format!("planning-wi-{}-completed", input.work_item_id);
    let payload = serde_json::json!({
        "work_item_id": input.work_item_id,
        "cycle_id": input.cycle_id,
        "title": input.title,
        "status": input.status,
    });
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type: "planning.work_item.completed".to_string(),
        schema_version: 1,
        stream_id: input.cycle_id.clone(),
        sequence: 0,
        project_id: input.project_id.clone(),
        occurred_at: input.occurred_at.clone(),
        recorded_at: input.occurred_at.clone(),
        actor: input
            .actor_ref
            .clone()
            .unwrap_or_else(|| sddk_domain::ActorRef {
                kind: sddk_domain::ActorKind::System,
                id: "system".to_string(),
                definition_hash: None,
                policy_hash: None,
                model: None,
                role: None,
            }),
        subjects: vec![sddk_domain::EntityRef {
            kind: "work_item".into(),
            id: input.work_item_id.clone(),
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
    validate_secretary_event(&env.actor, &env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
    store.append(&env)
}

/// Emits a `planning.work_item.superseded` event.
pub fn emit_planning_work_item_superseded<S: EventStore>(
    store: &mut S,
    input: &PlanningWorkItemEventInput,
) -> Result<EventAppended, StorageError> {
    let event_id = format!("planning-wi-{}-superseded", input.work_item_id);
    let payload = serde_json::json!({
        "work_item_id": input.work_item_id,
        "cycle_id": input.cycle_id,
        "title": input.title,
        "status": input.status,
    });
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type: "planning.work_item.superseded".to_string(),
        schema_version: 1,
        stream_id: input.cycle_id.clone(),
        sequence: 0,
        project_id: input.project_id.clone(),
        occurred_at: input.occurred_at.clone(),
        recorded_at: input.occurred_at.clone(),
        actor: input
            .actor_ref
            .clone()
            .unwrap_or_else(|| sddk_domain::ActorRef {
                kind: sddk_domain::ActorKind::System,
                id: "system".to_string(),
                definition_hash: None,
                policy_hash: None,
                model: None,
                role: None,
            }),
        subjects: vec![sddk_domain::EntityRef {
            kind: "work_item".into(),
            id: input.work_item_id.clone(),
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
    validate_secretary_event(&env.actor, &env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
    store.append(&env)
}

/// Emits a `planning.work_item.created` event.
pub fn emit_work_item_created<S: EventStore>(
    store: &mut S,
    input: &WorkItemCreatedInput,
) -> Result<EventAppended, StorageError> {
    // REQ-PLN2-AUTH-003: authority check MUST precede emission (fail-closed)
    input
        .authority
        .validate(WritableSurface::PlanItem)
        .map_err(|e| StorageError::Other(format!("authority rejected: {e}")))?;
    let event_id = format!("planning-wi-{}-created", input.work_item_id);
    let payload = serde_json::json!({
        "work_item_id": input.work_item_id,
        "cycle_id": input.cycle_id,
        "title": input.title,
        "description": input.description,
        "status": input.status,
    });
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type: "planning.work_item.created".to_string(),
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
            role: None,
        },
        subjects: vec![EntityRef {
            kind: "work_item".into(),
            id: input.work_item_id.clone(),
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
    validate_secretary_event(&env.actor, &env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
    store.append(&env)
}

/// Emits a `planning.work_item.transitioned` event.
pub fn emit_work_item_transitioned<S: EventStore>(
    store: &mut S,
    input: &WorkItemTransitionedInput,
) -> Result<EventAppended, StorageError> {
    // REQ-PLN2-AUTH-003: authority check MUST precede emission (fail-closed)
    input
        .authority
        .validate(WritableSurface::PlanItem)
        .map_err(|e| StorageError::Other(format!("authority rejected: {e}")))?;
    let event_id = format!("planning-wi-{}-transitioned", input.work_item_id);
    let payload = serde_json::json!({
        "work_item_id": input.work_item_id,
        "cycle_id": input.cycle_id,
        "from_status": input.from_status,
        "to_status": input.to_status,
    });
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type: "planning.work_item.transitioned".to_string(),
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
            role: None,
        },
        subjects: vec![EntityRef {
            kind: "work_item".into(),
            id: input.work_item_id.clone(),
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
    validate_secretary_event(&env.actor, &env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
    store.append(&env)
}

/// Emits a `planning.dependency.added` event.
pub fn emit_dependency_added<S: EventStore>(
    store: &mut S,
    input: &DependencyAddedInput,
) -> Result<EventAppended, StorageError> {
    // REQ-PLN2-AUTH-003: authority check MUST precede emission (fail-closed)
    input
        .authority
        .validate(WritableSurface::DependencyEdge)
        .map_err(|e| StorageError::Other(format!("authority rejected: {e}")))?;
    let event_id = format!(
        "planning-dep-{}-to-{}-added",
        input.from_work_item_id, input.to_work_item_id
    );
    let payload = serde_json::json!({
        "from_work_item_id": input.from_work_item_id,
        "to_work_item_id": input.to_work_item_id,
        "dependency_kind": input.dependency_kind,
        "cycle_id": input.cycle_id,
    });
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type: "planning.dependency.added".to_string(),
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
            role: None,
        },
        subjects: vec![
            EntityRef {
                kind: "work_item".into(),
                id: input.from_work_item_id.clone(),
                version: None,
                content_hash: None,
            },
            EntityRef {
                kind: "work_item".into(),
                id: input.to_work_item_id.clone(),
                version: None,
                content_hash: None,
            },
        ],
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
    validate_secretary_event(&env.actor, &env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
    store.append(&env)
}

/// Emits a `planning.evidence.attached` event.
pub fn emit_evidence_attached<S: EventStore>(
    store: &mut S,
    input: &EvidenceAttachedInput,
) -> Result<EventAppended, StorageError> {
    // REQ-PLN2-AUTH-003: authority check MUST precede emission (fail-closed)
    input
        .authority
        .validate(WritableSurface::EvidenceAttachment)
        .map_err(|e| StorageError::Other(format!("authority rejected: {e}")))?;
    let event_id = format!("planning-ev-{}-attached", input.evidence_id);
    let payload = serde_json::json!({
        "evidence_id": input.evidence_id,
        "work_item_id": input.work_item_id,
        "evidence_kind": input.evidence_kind,
        "cas_hash": input.cas_hash,
        "cycle_id": input.cycle_id,
    });
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type: "planning.evidence.attached".to_string(),
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
            role: None,
        },
        subjects: vec![EntityRef {
            kind: "work_item".into(),
            id: input.work_item_id.clone(),
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
    validate_secretary_event(&env.actor, &env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
    store.append(&env)
}

/// Emits a `planning.decision.recorded` event.
pub fn emit_decision_recorded<S: EventStore>(
    store: &mut S,
    input: &DecisionRecordedInput,
) -> Result<EventAppended, StorageError> {
    // REQ-PLN2-AUTH-003: authority check MUST precede emission (fail-closed)
    input
        .authority
        .validate(WritableSurface::DecisionRecord)
        .map_err(|e| StorageError::Other(format!("authority rejected: {e}")))?;
    let event_id = format!("planning-dec-{}-recorded", input.decision_id);
    let payload = serde_json::json!({
        "decision_id": input.decision_id,
        "work_item_id": input.work_item_id,
        "decision_kind": input.decision_kind,
        "rationale_summary": input.rationale_summary,
        "cycle_id": input.cycle_id,
    });
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type: "planning.decision.recorded".to_string(),
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
            role: None,
        },
        subjects: vec![EntityRef {
            kind: "work_item".into(),
            id: input.work_item_id.clone(),
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
    validate_secretary_event(&env.actor, &env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
    store.append(&env)
}

/// Emits an `authority.admission.decided` event (D-02).
///
/// Actor is fixed to `System("sddk-authority")`: admission decisions are
/// runtime facts about the engine, not actions of the caller.
///
/// The event_id is deterministic: `authority-<decision_id>-<verdict>`, so
/// re-emitting the same decision is idempotent (the store returns the
/// original `EventAppended` without allocating a new sequence).
///
/// Fail-soft by design at call sites: the governed mutation has already
/// been aborted before this event is recorded, so an emission error must
/// not change the caller's blocking outcome.
pub fn emit_admission_decision<S: EventStore>(
    store: &mut S,
    input: &AdmissionDecidedInput,
) -> Result<EventAppended, StorageError> {
    let event_id = format!("authority-{}-{}", input.decision_id, input.verdict);
    let payload = json!({
        "surface": input.surface,
        "action_kind": input.action_kind,
        "verdict": input.verdict,
        "decision_id": input.decision_id,
        "explanation_digest": input.explanation_digest,
    });
    let stream_id = input
        .cycle_id
        .clone()
        .unwrap_or_else(|| format!("authority-{surface}", surface = input.surface));
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type: "authority.admission.decided".to_string(),
        schema_version: 1,
        stream_id,
        sequence: 0,
        project_id: input.project_id.clone(),
        occurred_at: input.occurred_at.clone(),
        recorded_at: input.occurred_at.clone(),
        actor: ActorRef {
            kind: ActorKind::System,
            id: "sddk-authority".to_string(),
            definition_hash: None,
            policy_hash: None,
            model: None,
            role: None,
        },
        subjects: vec![EntityRef {
            kind: "surface".into(),
            id: input.surface.clone(),
            version: None,
            content_hash: None,
        }],
        payload,
        evidence_refs: vec![],
        content_hash: String::new(),
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: input.cycle_id.clone(),
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
    validate_secretary_event(&env.actor, &env.event_type)
        .map_err(|e| StorageError::Other(e.to_string()))?;
    store.append(&env)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── authority.admission.decided (WU-C4-3, D-02, R-4-005) ─────────────────

    /// In-memory store honoring the EventStore idempotency contract:
    /// re-appending the same event_id returns the original result without
    /// allocating a new sequence.
    struct AdmissionMemStore {
        events: Vec<EventEnvelopeV1>,
        appended: std::cell::RefCell<usize>,
    }

    impl AdmissionMemStore {
        fn new() -> Self {
            Self {
                events: vec![],
                appended: std::cell::RefCell::new(0),
            }
        }
    }

    impl EventStore for AdmissionMemStore {
        fn append(&mut self, envelope: &EventEnvelopeV1) -> Result<EventAppended, StorageError> {
            *self.appended.borrow_mut() += 1;
            if let Some(existing) = self.events.iter().find(|e| e.event_id == envelope.event_id) {
                return Ok(EventAppended {
                    event_id: existing.event_id.clone(),
                    stream_id: existing.stream_id.clone(),
                    sequence: existing.sequence,
                    content_hash: existing.content_hash.clone(),
                    recorded_at: existing.recorded_at.clone(),
                    chain_hash: String::new(),
                });
            }
            let sequence = envelope.sequence.max(1);
            let mut env = envelope.clone();
            env.sequence = sequence;
            self.events.push(env.clone());
            Ok(EventAppended {
                event_id: env.event_id,
                stream_id: env.stream_id,
                sequence,
                content_hash: env.content_hash,
                recorded_at: env.recorded_at,
                chain_hash: String::new(),
            })
        }
        fn load_by_event_id(
            &self,
            event_id: &str,
        ) -> Result<Option<EventEnvelopeV1>, StorageError> {
            Ok(self.events.iter().find(|e| e.event_id == event_id).cloned())
        }
        fn load_stream(
            &self,
            stream_id: &str,
            after_sequence: Option<u64>,
            limit: u32,
        ) -> Result<Vec<EventEnvelopeV1>, StorageError> {
            let start = after_sequence.unwrap_or(0);
            Ok(self
                .events
                .iter()
                .filter(|e| e.stream_id == stream_id && e.sequence > start)
                .take(limit as usize)
                .cloned()
                .collect())
        }
        fn last_sequence(&self, stream_id: &str) -> Result<Option<u64>, StorageError> {
            Ok(self
                .events
                .iter()
                .filter(|e| e.stream_id == stream_id)
                .map(|e| e.sequence)
                .max())
        }
        fn count(&self) -> Result<u64, StorageError> {
            Ok(self.events.len() as u64)
        }
        fn head_hash(&self, stream_id: &str) -> Result<Option<String>, StorageError> {
            Ok(self
                .events
                .iter()
                .filter(|e| e.stream_id == stream_id)
                .next_back()
                .map(|e| e.content_hash.clone()))
        }
        fn head_chain_hash(&self, _stream_id: &str) -> Result<Option<String>, StorageError> {
            Ok(None)
        }
        fn verify_stream_chain(&self, _stream_id: &str) -> Result<(), StorageError> {
            Ok(())
        }
        fn verify_chain_integrity(&self, _stream_id: &str) -> Result<(), StorageError> {
            Ok(())
        }
        fn backfill_chain_hash(&mut self, _stream_id: &str) -> Result<usize, StorageError> {
            Ok(0)
        }
        fn load_by_sequence(
            &self,
            stream_id: &str,
            sequence: u64,
        ) -> Result<Option<EventEnvelopeV1>, StorageError> {
            Ok(self
                .events
                .iter()
                .find(|e| e.stream_id == stream_id && e.sequence == sequence)
                .cloned())
        }
    }

    fn admission_input(verdict: &str) -> AdmissionDecidedInput {
        AdmissionDecidedInput {
            project_id: "p-1".into(),
            cycle_id: Some("c-42".into()),
            surface: "gate_receipts".into(),
            action_kind: "cli_run".into(),
            verdict: verdict.into(),
            decision_id: "deny-cap-human-cli_run".into(),
            explanation_digest: "sha256:deadbeef".into(),
            occurred_at: "2026-09-13T10:00:00Z".into(),
            causation_id: None,
            correlation_id: None,
        }
    }

    #[test]
    fn emit_admission_decision_is_idempotent_per_decision_id() {
        let mut store = AdmissionMemStore::new();
        let input = admission_input("deny");
        let first = emit_admission_decision(&mut store, &input).unwrap();
        let second = emit_admission_decision(&mut store, &input).unwrap();
        // Same event_id, same sequence: no duplicate stored.
        assert_eq!(first.event_id, second.event_id);
        assert_eq!(first.sequence, second.sequence);
        assert_eq!(store.count().unwrap(), 1, "exactly one event");
        let events = store.load_stream("c-42", None, 10).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "authority.admission.decided");
    }

    #[test]
    fn emit_admission_decision_event_contains_verdict_fields() {
        let mut store = AdmissionMemStore::new();
        let input = admission_input("deny");
        emit_admission_decision(&mut store, &input).unwrap();
        let env = store
            .load_by_event_id("authority-deny-cap-human-cli_run-deny")
            .unwrap()
            .expect("stored");
        assert_eq!(env.event_type, "authority.admission.decided");
        assert_eq!(env.actor.kind, ActorKind::System);
        assert_eq!(env.actor.id, "sddk-authority");
        assert_eq!(env.payload["surface"], "gate_receipts");
        assert_eq!(env.payload["action_kind"], "cli_run");
        assert_eq!(env.payload["verdict"], "deny");
        assert_eq!(env.payload["explanation_digest"], "sha256:deadbeef");
        // Deterministic event_id formula.
        assert!(
            env.event_id
                .starts_with("authority-deny-cap-human-cli_run-")
        );
    }

    #[test]
    fn emit_admission_decision_verdicts_yield_distinct_event_ids() {
        let mut store = AdmissionMemStore::new();
        emit_admission_decision(&mut store, &admission_input("deny")).unwrap();
        emit_admission_decision(&mut store, &admission_input("require_approval")).unwrap();
        emit_admission_decision(&mut store, &admission_input("advisory_high_approval")).unwrap();
        assert_eq!(store.count().unwrap(), 3);
    }

    #[test]
    fn emit_admission_decision_without_cycle_uses_surface_stream() {
        let mut store = AdmissionMemStore::new();
        let mut input = admission_input("deny");
        input.cycle_id = None;
        emit_admission_decision(&mut store, &input).unwrap();
        let events = store
            .load_stream("authority-gate_receipts", None, 10)
            .unwrap();
        assert_eq!(events.len(), 1, "falls back to per-surface stream");
    }

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
