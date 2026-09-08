//! `sddk run-view` — materialize `RunStateView` and `ActionSurfaceView`
//! for a persisted run.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-CurrentRunView-Boundary.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-075-CURRENT-RUN-VIEW-SHAPE.md

use std::process::ExitCode;

use serde::Serialize;

use sddk_engine::{
    ActionKind, ActionSurfaceView, FrontierProjection, PolicySnapshot, RunOrigin, RunStateView,
    ViewError, build_action_surface_view_with_frontier, build_run_state_view, empty_projection,
};

use crate::{CliEnvironment, CommandOutput, OutputFormat};

#[derive(Debug, Serialize)]
struct RunViewEnvelope<'a> {
    run_state: &'a RunStateView,
    action_surface: &'a ActionSurfaceView,
}

/// Resolve a named policy. Currently only `default` is registered.
fn resolve_policy(name: &str) -> Result<PolicySnapshot, ViewError> {
    match name {
        "default" => Ok(PolicySnapshot::default()),
        other => Err(ViewError::PolicyNotFound(other.to_string())),
    }
}

/// Run the `sddk run-view` command.
pub(crate) fn run_run_view(
    run_id: String,
    as_of: Option<u64>,
    policy: String,
    format: OutputFormat,
    _environment: &CliEnvironment,
) -> CommandOutput {
    // Resolve policy first (cheap, deterministic).
    let policy_snapshot = match resolve_policy(&policy) {
        Ok(p) => p,
        Err(ViewError::PolicyNotFound(name)) => {
            return CommandOutput {
                status: 2,
                stdout: String::new(),
                stderr: format!(
                    "{{\"error\":\"POLICY_NOT_FOUND\",\"message\":\"policy `{name}` not found\",\"run_id\":\"{run_id}\"}}"
                ),
            };
        }
        Err(e) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("policy resolution failed: {e}"),
            };
        }
    };

    // For the v0 scaffold we read no storage; the run is "found" if a
    // view can be built. Real implementation will query the ledger.
    let as_of = as_of.unwrap_or(0);

    // Heuristic for the scaffold: declared vs generated based on run_id prefix.
    let origin = if run_id.starts_with("R-decl") {
        RunOrigin::Declared
    } else {
        RunOrigin::Generated
    };

    // Build a minimal RunStateView from the run_id. Real impl reads ledger.
    let state = match build_run_state_view(
        run_id.clone(),
        as_of,
        origin,
        vec![],
        vec![],
        vec![],
        as_of,
    ) {
        Ok(s) => s,
        Err(ViewError::Storage(msg)) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!(
                    "{{\"error\":\"STORAGE_ERROR\",\"message\":\"{msg}\",\"run_id\":\"{run_id}\"}}"
                ),
            };
        }
        Err(e) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("view build failed: {e}"),
            };
        }
    };

    // DEC-PLANE-002: build a frontier projection from the CLI runtime.
    // When a workflow manifest is present, the projection is authoritative.
    // When absent, an empty projection is used (heuristic fallback in the
    // engine layer emits only Abort, which is acceptable for ad-hoc views).
    let projection = build_frontier_projection_from_cli(&run_id);

    let surface =
        match build_action_surface_view_with_frontier(&state, &policy_snapshot, &projection) {
            Ok(s) => s,
            Err(e) => {
                return CommandOutput {
                    status: 1,
                    stdout: String::new(),
                    stderr: format!("action surface build failed: {e}"),
                };
            }
        };

    // Invariant: views must agree on origin/run_id/evaluated_at.
    if state.origin() != surface.origin()
        || state.run_id() != surface.run_id()
        || state.evaluated_at() != surface.evaluated_at()
    {
        return CommandOutput {
            status: 3,
            stdout: String::new(),
            stderr: format!(
                "{{\"error\":\"INVARIANT_VIOLATION\",\"message\":\"run_state and action_surface disagree\",\"run_id\":\"{run_id}\"}}"
            ),
        };
    }

    let body = match format {
        OutputFormat::Json => emit_json(&state, &surface),
        OutputFormat::Text => emit_text(&state, &surface),
    };
    CommandOutput {
        status: 0,
        stdout: body,
        stderr: String::new(),
    }
}

fn emit_json(state: &RunStateView, surface: &ActionSurfaceView) -> String {
    let env = RunViewEnvelope {
        run_state: state,
        action_surface: surface,
    };
    serde_json::to_string_pretty(&env).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
}

fn emit_text(state: &RunStateView, surface: &ActionSurfaceView) -> String {
    let mut out = String::new();
    out.push_str("# Run state\n");
    out.push_str(&format!("origin: {}\n", origin_str(state.origin())));
    out.push_str(&format!("run_id: {}\n", state.run_id()));
    out.push_str(&format!("evaluated_at: {}\n", state.evaluated_at()));
    out.push_str("frontier:\n");
    if state.frontier().is_empty() {
        out.push_str("  (none)\n");
    } else {
        for n in state.frontier() {
            out.push_str(&format!("  - {n}\n"));
        }
    }
    out.push_str("blockers:\n");
    if state.blockers().is_empty() {
        out.push_str("  (none)\n");
    } else {
        for b in state.blockers() {
            out.push_str(&format!("  - {b}\n"));
        }
    }
    out.push_str("pending_decisions:\n");
    if state.pending_decisions().is_empty() {
        out.push_str("  (none)\n");
    } else {
        for d in state.pending_decisions() {
            out.push_str(&format!("  - {d}\n"));
        }
    }

    out.push_str("\n# Action surface\n");
    out.push_str(&format!("origin: {}\n", origin_str(surface.origin())));
    out.push_str(&format!("run_id: {}\n", surface.run_id()));
    out.push_str(&format!("evaluated_at: {}\n", surface.evaluated_at()));
    out.push_str(&format!(
        "policy_digest: {:016x}\n",
        surface.policy_digest()
    ));
    out.push_str("available_actions:\n");
    if surface.available_actions().is_empty() {
        out.push_str("  (none)\n");
    } else {
        for a in surface.available_actions() {
            out.push_str(&format!("  - {}\n", action_kind_str(*a)));
        }
    }
    out
}

fn origin_str(origin: RunOrigin) -> &'static str {
    match origin {
        RunOrigin::Declared => "declared",
        RunOrigin::Generated => "generated",
    }
}

fn action_kind_str(a: ActionKind) -> &'static str {
    match a {
        ActionKind::Start => "start",
        ActionKind::Resume => "resume",
        ActionKind::Abort => "abort",
        ActionKind::Approve => "approve",
        ActionKind::Escalate => "escalate",
        ActionKind::Retry => "retry",
        ActionKind::Reconcile => "reconcile",
    }
}

// Silence unused warnings for helpers kept for future expansion.
#[allow(dead_code)]
fn _exit_code_marker() -> ExitCode {
    ExitCode::SUCCESS
}

/// Build a `FrontierProjection` from the CLI runtime context.
///
/// DEC-PLANE-002: this is the bridge between the CLI's access to the
/// workflow manifest + ledger and the engine's `*_with_frontier` builder.
/// The current scaffold has no manifest access wired up, so it returns
/// an empty projection. Future cycles will load the manifest via
/// `RuntimeContext` and build the projection from `frontier_for_state`.
fn build_frontier_projection_from_cli(_run_id: &str) -> FrontierProjection {
    empty_projection()
}
