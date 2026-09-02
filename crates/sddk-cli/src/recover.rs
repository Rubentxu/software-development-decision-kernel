//! Shadow router for the `recover` facade verb (D2).
//!
//! Routes to `cycle rebuild`.

use std::path::PathBuf;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat,
    cycle::{self, CycleRebuildArgs, RuntimeArgs},
};

/// Run the `recover` facade command — delegates to `cycle rebuild`.
pub(crate) fn run_recover(
    cycle: String,
    dry_run: bool,
    format: OutputFormat,
    environment: &CliEnvironment,
) -> CommandOutput {
    let args = CycleRebuildArgs {
        runtime: RuntimeArgs {
            root: Some(PathBuf::from(".")),
            scope: Some(".".to_string()),
            remote: None,
            fallback_seed: None,
            no_infer: false,
        },
        cycle: Some(cycle),
        dry_run,
        lease_owner: None,
        fencing_token: None,
        timestamp: None,
        actor: None,
        format,
    };
    cycle::run_cycle(cycle::CycleCommand::Rebuild(args), environment)
}
