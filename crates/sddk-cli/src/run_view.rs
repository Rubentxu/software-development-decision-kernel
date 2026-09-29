//! `sddk run-view` — materialize `RunStateView` and `ActionSurfaceView`
//! for a persisted run.
//!
//! Spec: ~/.sddk-knowledge/sddk-framework/specs/engine/REQ-CurrentRunView-Boundary.md
//! ADR:  ~/.sddk-knowledge/sddk-framework/adrs/ADR-075-CURRENT-RUN-VIEW-SHAPE.md

use std::process::ExitCode;

use serde::Serialize;

use sddk_engine::{
    ActionKind, ActionSurfaceView, FrontierProjection, PolicySnapshot, RunOrigin, RunStateView,
    ViewError, build_action_surface_view_with_frontier, empty_projection,
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

/// Load a `RunStateView` for `run_id` from a real source.
///
/// This is the **single seam** where a ledger-backed read will land
/// (option (a) of INC-DEBT-039). While it has no source, it must refuse
/// rather than synthesize: the scaffold it replaces guessed `origin` from
/// the `run_id` prefix and passed constant empty vectors for `frontier`,
/// `blockers` and `pending_decisions`, so the emitted view was
/// indistinguishable from a real one. `ActionSurfaceView` derives its
/// available actions from that view, so the fabrication propagated into
/// policy evaluation.
///
/// The error is typed and names the missing capability, so an operator
/// sees "there is no run-state source yet" instead of a plausible view.
/// `Ok` is unreachable today; when a real `RunStateViewInputs` lands, it
/// becomes reachable here and nowhere else.
fn load_run_state_view(
    run_id: &str,
    _as_of: Option<u64>,
    _environment: &CliEnvironment,
) -> Result<RunStateView, String> {
    Err(format!(
        "{{\"error\":\"RUN_STATE_SOURCE_UNAVAILABLE\",\
         \"message\":\"no run_state source is wired for `{run_id}\"; \
         frontier, blockers and pending_decisions cannot be reported without one\",\
         \"run_id\":\"{run_id}\",\
         \"debt\":\"INC-DEBT-039\"}}"
    ))
}

/// Run the `sddk run-view` command.
///
/// INC-DEBT-039: the v0 scaffold built a `RunStateView` without reading
/// the ledger — `frontier`, `blockers` and `pending_decisions` were
/// constant `vec![]` and `origin` was guessed from the `run_id` prefix.
/// `REQ-CurrentRunView-Shape.md:43` defines `frontier` as empty *"iff the
/// run is terminal or no node is ready"*, so a constant empty vector
/// cannot distinguish "nothing is ready" from "nothing was consulted".
/// The command emitted an authoritative-looking view over data it had
/// not read, and `ActionSurfaceView` derived its actions from that.
///
/// Until a ledger-backed `RunStateViewInputs` exists (option (a) of the
/// INC), this command **fails closed with a typed error** (option (b))
/// rather than fabricating a view. A view that says "I have no source" is
/// visible; a view that says "nothing is ready" when it never looked is
/// used in silence.
pub fn run_run_view(
    run_id: String,
    as_of: Option<u64>,
    policy: String,
    format: OutputFormat,
    _environment: &CliEnvironment,
) -> CommandOutput {
    // Source availability is decided FIRST. The scaffold resolved the
    // policy first, so an unknown policy name on a source-less run
    // reported `POLICY_NOT_FOUND` — pointing the operator at the wrong
    // defect.
    //
    // `load_run_state_view` is the single place that will grow a real
    // ledger-backed read. Until it returns a sourced view, it must
    // refuse. See INC-DEBT-039 for the model decision (what is
    // "frontier" when `node_runs_v1` is empty) that must precede it.
    let state = match load_run_state_view(&run_id, as_of, _environment) {
        Ok(s) => s,
        Err(source_error) => {
            return CommandOutput {
                status: 4,
                stdout: String::new(),
                stderr: source_error,
            };
        }
    };

    // Resolve policy second, only once we have a real view to apply it to.
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

    // INC-DEBT-039: the v0 scaffold read no storage here. It resolved
    // `origin` from the `run_id` prefix and passed constant empty
    // vectors as `frontier`, `blockers` and `pending_decisions`. That
    // produced a view indistinguishable from a real one, and
    // `ActionSurfaceView` derived its available actions from it. The
    // state is now loaded by `load_run_state_view` above, which refuses
    // until a ledger-backed source exists.
    //
    // `as_of` is forwarded to the loader and is no longer read here.

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
