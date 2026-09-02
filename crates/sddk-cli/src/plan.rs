//! Shadow router for the `plan` facade verb (D2).
//!
//! Routes to `cycle start`.

use std::path::PathBuf;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat,
    cycle::{self, CyclePathArg, CycleStartArgs, RuntimeArgs},
};

/// Run the `plan` facade command — delegates to `cycle start`.
pub(crate) fn run_plan(
    name: String,
    path: Option<CyclePathArg>,
    branch: Option<String>,
    format: OutputFormat,
    environment: &CliEnvironment,
) -> CommandOutput {
    let args = CycleStartArgs {
        runtime: RuntimeArgs {
            root: Some(PathBuf::from(".")),
            scope: Some(".".to_string()),
            remote: None,
            fallback_seed: None,
            no_infer: false,
        },
        name,
        path,
        branch,
        base: None,
        timestamp: None,
        actor: None,
        lease_owner: None,
        lease_ms: 3_600_000,
        format,
    };
    cycle::run_cycle(cycle::CycleCommand::Start(args), environment)
}
