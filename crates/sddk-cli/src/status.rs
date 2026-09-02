//! Shadow router for the `status` facade verb (D2).
//!
//! Routes to `cycle status`.

use std::path::PathBuf;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat,
    cycle::{self, CycleStatusArgs, RuntimeArgs},
};

/// Run the `status` facade command — delegates to `cycle status`.
pub(crate) fn run_status(
    cycle: String,
    format: OutputFormat,
    environment: &CliEnvironment,
) -> CommandOutput {
    let args = CycleStatusArgs {
        runtime: RuntimeArgs {
            root: Some(PathBuf::from(".")),
            scope: Some(".".to_string()),
            remote: None,
            fallback_seed: None,
            no_infer: false,
        },
        cycle: Some(cycle),
        format,
    };
    cycle::run_cycle(cycle::CycleCommand::Status(args), environment)
}
