//! Shadow router for the `run` facade verb (D2).
//!
//! Routes to `capability apply`.

use std::path::PathBuf;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat,
    capability::{self, CapabilityArgs, CapabilityCommand},
    cycle::RuntimeArgs,
};

/// Run the `run` facade command — delegates to `capability apply`.
pub(crate) fn run_run(
    name: String,
    format: OutputFormat,
    environment: &CliEnvironment,
) -> CommandOutput {
    // The capability name is used as both the capability identifier and the
    // program name in the facade (the capability registry resolves the actual
    // executable).
    let args = CapabilityArgs {
        runtime: RuntimeArgs {
            root: PathBuf::from("."),
            scope: ".".to_string(),
            remote: None,
            fallback_seed: None,
        },
        cycle: None,
        capability: name.clone(),
        reason: "facade: sddk run".into(),
        program: name,
        arg: vec![],
        env: vec![],
        timeout_ms: 30_000,
        output_max_bytes: 1_048_576,
        approve: false,
        agent: None,
        phase: None,
        timestamp: None,
        actor: None,
        format,
    };
    capability::run_capability(CapabilityCommand::Apply(args), environment)
}
