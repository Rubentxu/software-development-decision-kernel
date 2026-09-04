//! Human approval command surface for `sddk approval list|grant|deny`.
//!
//! `list` rebuilds `ApprovalProjection` from the ledger and shows pending
//! approvals (where decision is `None`). `grant` and `deny` call
//! `emit_approval_decision` to append the decision event.

use clap::{Args, Subcommand};
use sddk_domain::projections::ApprovalProjection;
use sddk_domain::{ActorKind, ApprovalDecision, EventEnvelopeV1, EventStore, Projection, SddkErrorCode};
use sddk_engine::{
    authority::infer_actor_kind,
    event_bus::{self, ApprovalDecisionInput},
};
use sddk_storage::SqliteEventStore;
use serde::Serialize;
use time::format_description::well_known::Rfc3339;

use crate::{CliEnvironment, CommandOutput, OutputFormat, RuntimeArgs, RuntimeContext};

/// Approval subcommands.
#[derive(Debug, Subcommand)]
pub(crate) enum ApprovalCommand {
    /// List pending approval requests for a cycle.
    List(ApprovalListArgs),
    /// Grant a pending approval request.
    Grant(ApprovalDecisionArgs),
    /// Deny a pending approval request.
    Deny(ApprovalDecisionArgs),
}

/// Arguments for `sddk approval list`.
#[derive(Debug, Clone, Args)]
pub(crate) struct ApprovalListArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle to list pending approvals for.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Arguments for `sddk approval grant` and `sddk approval deny`.
#[derive(Debug, Clone, Args)]
pub(crate) struct ApprovalDecisionArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier where the approval was requested.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Capability identifier that requires approval.
    #[arg(long)]
    pub(crate) capability: String,
    /// Human actor id (default: $USER).
    #[arg(long)]
    pub(crate) actor: Option<String>,
    /// Mandatory justification for the decision.
    #[arg(long)]
    pub(crate) reason: String,
    /// Explicit RFC 3339 timestamp (for deterministic tests).
    #[arg(long)]
    pub(crate) timestamp: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Runs an approval subcommand.
pub(crate) fn run_approval(
    command: ApprovalCommand,
    environment: &CliEnvironment,
) -> CommandOutput {
    match command {
        ApprovalCommand::List(args) => run_approval_list(args, environment),
        ApprovalCommand::Grant(args) => {
            run_approval_decision(args, environment, ApprovalDecision::Granted)
        }
        ApprovalCommand::Deny(args) => {
            run_approval_decision(args, environment, ApprovalDecision::Denied)
        }
    }
}

// ── List ────────────────────────────────────────────────────────────────────────

fn run_approval_list(args: ApprovalListArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<Vec<PendingApprovalOutput>> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let pending = list_pending_approvals(&context, &args.cycle)?;
        Ok(pending)
    })();
    render_result(result, format, approval_list_text)
}

fn list_pending_approvals(
    context: &RuntimeContext,
    cycle_id: &str,
) -> anyhow::Result<Vec<PendingApprovalOutput>> {
    // Open SqliteEventStore at the ledger's parent directory.
    // SqliteEventStore::open(dir) appends "ledger.sqlite" to dir,
    // while context.paths.ledger is the full file path.
    let event_store = SqliteEventStore::open(context.paths.ledger.parent().unwrap())
        .map_err(|e| anyhow::anyhow!("SqliteEventStore::open: {e}"))?;

    // Load all events for this cycle via EventStore trait.
    let events: Vec<EventEnvelopeV1> = event_store
        .load_stream(cycle_id, None, u32::MAX)
        .map_err(|e| anyhow::anyhow!("load_stream: {e}"))?;

    let mut proj = ApprovalProjection::new(cycle_id);
    for event in &events {
        let _ = proj.apply(event);
    }

    let mut pending = Vec::new();
    for ((cid, cap), state) in proj.states() {
        if state.decision.is_none() {
            pending.push(PendingApprovalOutput {
                cycle_id: cid.clone(),
                capability: cap.clone(),
                request_hash: state.request_hash.clone(),
                requested_at: state.last_event_at.clone(),
                actor: state.actor.clone().unwrap_or_default(),
                reason: state.reason.clone().unwrap_or_default(),
            });
        }
    }

    pending.sort_by(|a, b| {
        a.cycle_id
            .cmp(&b.cycle_id)
            .then(a.capability.cmp(&b.capability))
    });
    Ok(pending)
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct PendingApprovalOutput {
    cycle_id: String,
    capability: String,
    request_hash: String,
    requested_at: String,
    actor: String,
    reason: String,
}

fn approval_list_text(approvals: &Vec<PendingApprovalOutput>) -> String {
    if approvals.is_empty() {
        return "no pending approvals\n".to_owned();
    }
    let mut out = String::new();
    for a in approvals {
        out.push_str(&format!(
            "cycle_id: {}\ncapability: {}\nrequest_hash: {}\nrequested_at: {}\nactor: {}\nreason: {}\n---\n",
            a.cycle_id,
            a.capability,
            a.request_hash,
            a.requested_at,
            a.actor,
            a.reason
        ));
    }
    out
}

// ── Grant / Deny ───────────────────────────────────────────────────────────────

fn run_approval_decision(
    args: ApprovalDecisionArgs,
    environment: &CliEnvironment,
    decision: ApprovalDecision,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<ApprovalDecisionOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;

        // Validate reason is non-empty.
        if args.reason.trim().is_empty() {
            anyhow::bail!("reason cannot be empty");
        }

        let timestamp = args.timestamp.clone().unwrap_or_else(default_timestamp);

        let actor = args
            .actor
            .clone()
            .or_else(|| environment.user.clone())
            .unwrap_or_else(|| "sddk-cli".into());

        // Open SqliteEventStore at the ledger's parent directory (same fix as above).
        let mut event_store = SqliteEventStore::open(context.paths.ledger.parent().unwrap())
            .map_err(|e| anyhow::anyhow!("SqliteEventStore::open: {e}"))?;

        // Load the cycle's events and build the projection to find request_hash.
        let events: Vec<EventEnvelopeV1> = event_store
            .load_stream(&args.cycle, None, u32::MAX)
            .map_err(|e| anyhow::anyhow!("load_stream: {e}"))?;

        let mut proj = ApprovalProjection::new(&args.cycle);
        for e in &events {
            let _ = proj.apply(e);
        }

        let key = (args.cycle.clone(), args.capability.clone());
        let state = proj.states().get(&key).ok_or_else(|| {
            anyhow::anyhow!(
                "no pending approval found for cycle={}, capability={}",
                args.cycle,
                args.capability
            )
        })?;

        if state.decision.is_some() {
            anyhow::bail!(
                "approval already resolved: cycle={}, capability={}",
                args.cycle,
                args.capability
            );
        }

        let request_hash = state.request_hash.clone();
        let actor_kind = infer_actor_kind(&actor);

        let input = ApprovalDecisionInput {
            project_id: context.identity.project_id.to_string(),
            cycle_id: args.cycle.clone(),
            capability: args.capability.clone(),
            request_hash,
            decision,
            actor_id: actor,
            actor_kind,
            reason: args.reason.clone(),
            occurred_at: timestamp,
        };

        let appended = event_bus::emit_approval_decision(&mut event_store, &input)
            .map_err(|e| anyhow::anyhow!("emit_approval_decision failed: {}", e))?;

        Ok(ApprovalDecisionOutput {
            event_id: appended.event_id,
            decision: format!("{:?}", input.decision).to_lowercase(),
            cycle_id: args.cycle,
            capability: args.capability,
        })
    })();
    render_result(result, format, approval_decision_text)
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ApprovalDecisionOutput {
    event_id: String,
    decision: String,
    cycle_id: String,
    capability: String,
}

fn approval_decision_text(output: &ApprovalDecisionOutput) -> String {
    format!(
        "event_id: {}\ndecision: {}\ncycle_id: {}\ncapability: {}\n",
        output.event_id, output.decision, output.cycle_id, output.capability
    )
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn default_timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("RFC 3339 formatting cannot fail")
}

fn render_result<T: serde::Serialize>(
    result: anyhow::Result<T>,
    format: OutputFormat,
    text: fn(&T) -> String,
) -> CommandOutput {
    match result {
        Ok(value) => match format {
            OutputFormat::Json => CommandOutput {
                stdout: format!("{}\n", serde_json::to_string_pretty(&value).unwrap()),
                ..CommandOutput::default()
            },
            OutputFormat::Text => CommandOutput {
                stdout: text(&value),
                ..CommandOutput::default()
            },
        },
        Err(error) => {
            let code = error
                .downcast_ref::<sddk_gateway::GatewayError>()
                .map(|e| e.code());
            match format {
                OutputFormat::Json => CommandOutput {
                    status: 1,
                    stdout: format!(
                        "{{\"error\": \"{}\", \"code\": \"{}\"}}\n",
                        error,
                        code.unwrap_or("INTERNAL_ERROR")
                    ),
                    stderr: String::new(),
                },
                OutputFormat::Text => {
                    if let Some(code) = code {
                        CommandOutput {
                            status: 1,
                            stdout: String::new(),
                            stderr: format!("error[{}]: {}\n", code, error),
                        }
                    } else {
                        CommandOutput {
                            status: 1,
                            stdout: String::new(),
                            stderr: format!("error: {}\n", error),
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::event_envelope::{ActorKind, ActorRef};
    use serde_json::json;

    fn make_event(
        stream_id: &str,
        event_type: &str,
        sequence: u64,
        payload: serde_json::Value,
    ) -> EventEnvelopeV1 {
        let mut env = EventEnvelopeV1 {
            event_id: format!("e-{stream_id}-{sequence}"),
            event_type: event_type.into(),
            schema_version: 1,
            stream_id: stream_id.into(),
            sequence,
            project_id: "p-1".into(),
            occurred_at: "2026-08-17T10:00:00Z".into(),
            recorded_at: "2026-08-17T10:00:01Z".into(),
            actor: ActorRef {
                kind: ActorKind::System,
                id: "sddk-cli".into(),
                definition_hash: None,
                policy_hash: None,
                model: None,
            },
            subjects: vec![],
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
        env.content_hash = env.compute_content_hash();
        env
    }

    #[test]
    fn approval_list_text_empty() {
        let pending: Vec<PendingApprovalOutput> = vec![];
        assert_eq!(approval_list_text(&pending), "no pending approvals\n");
    }

    #[test]
    fn approval_list_text_one_pending() {
        let pending = vec![PendingApprovalOutput {
            cycle_id: "c-1".into(),
            capability: "git.delete_branch".into(),
            request_hash: "sha256:abc1234".into(),
            requested_at: "2026-08-17T10:00:00Z".into(),
            actor: String::new(),
            reason: String::new(),
        }];
        let text = approval_list_text(&pending);
        assert!(text.contains("c-1"));
        assert!(text.contains("git.delete_branch"));
        assert!(text.contains("sha256:abc1234"));
    }

    #[test]
    fn approval_decision_text_format() {
        let output = ApprovalDecisionOutput {
            event_id: "approval-cap-git-delete_branch-abc1234-granted".into(),
            decision: "granted".into(),
            cycle_id: "c-1".into(),
            capability: "git.delete_branch".into(),
        };
        let text = approval_decision_text(&output);
        assert!(text.contains("approval-cap-git-delete_branch-abc1234-granted"));
        assert!(text.contains("granted"));
        assert!(text.contains("c-1"));
        assert!(text.contains("git.delete_branch"));
    }

    #[test]
    fn approval_projection_list_integration() {
        // Simulate: requested -> grant flow visible in list.
        let mut proj = ApprovalProjection::new("c-1");
        proj.apply(&make_event(
            "c-1",
            "approval.capability.requested",
            1,
            json!({
                "cycle_id": "c-1",
                "capability": "git.delete_branch",
                "request_hash": "sha256:abc1234",
                "expires_at": "2026-08-18T18:00:00Z"
            }),
        ))
        .unwrap();

        // List should show 1 pending.
        let mut pending_count = 0;
        for ((_cid, _cap), state) in proj.states() {
            if state.decision.is_none() {
                pending_count += 1;
            }
        }
        assert_eq!(pending_count, 1);

        // Apply grant.
        proj.apply(&make_event(
            "c-1",
            "approval.capability.granted",
            2,
            json!({
                "cycle_id": "c-1",
                "capability": "git.delete_branch",
                "request_hash": "sha256:abc1234",
                "actor": "alice",
                "reason": "ok, reversible via reflog"
            }),
        ))
        .unwrap();

        // List should show 0 pending.
        let mut pending_count = 0;
        for ((_cid, _cap), state) in proj.states() {
            if state.decision.is_none() {
                pending_count += 1;
            }
        }
        assert_eq!(pending_count, 0);
    }
}
