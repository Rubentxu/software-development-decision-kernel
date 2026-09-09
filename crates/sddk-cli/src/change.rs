//! Shadow router for the `change` facade verb (M6.1).
//!
//! Routes to `plan workitem create` for spec coverage and traceability.
//!
//! `pub` for integration tests under `tests/`; `missing_docs` allowed at module scope.
//!
#![allow(missing_docs)]

use std::path::PathBuf;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat,
    cycle::RuntimeArgs,
    plan::{self, PlanCommand, WorkItemCommand, WorkItemCreateArgs},
};

/// Run the `change` facade command — delegates to `plan workitem create`.
pub(crate) fn run_change(
    cycle_id: String,
    title: String,
    description: String,
    actor: Option<String>,
    format: OutputFormat,
    environment: &CliEnvironment,
) -> CommandOutput {
    let runtime = RuntimeArgs {
        root: Some(PathBuf::from(".")),
        scope: Some(".".to_string()),
        remote: None,
        fallback_seed: None,
        no_infer: false,
    };
    let args = WorkItemCreateArgs {
        cycle_id,
        title,
        description,
        actor_id: actor.unwrap_or_else(|| "agent:cli".into()),
        format,
    };
    plan::run_plan(
        PlanCommand::WorkItem {
            command: WorkItemCommand::Create(args),
        },
        &runtime,
        environment,
    )
}
