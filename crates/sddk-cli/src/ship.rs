//! Shadow router for the `ship` facade verb (D2).
//!
//! Routes to `release plan`.

use std::path::PathBuf;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat,
    cycle::RuntimeArgs,
    release_cmd::{ReleaseArgs, ReleaseCommand, ReleaseRoute},
};

/// Run the `ship` facade command — delegates to `release plan`.
pub(crate) fn run_ship(
    tag: String,
    cycle: Option<String>,
    format: OutputFormat,
    environment: &CliEnvironment,
) -> CommandOutput {
    let args = ReleaseArgs {
        runtime: RuntimeArgs {
            root: Some(PathBuf::from(".")),
            scope: Some(".".to_string()),
            remote: None,
            fallback_seed: None,
            no_infer: false,
        },
        route: Some(ReleaseRoute::Local),
        repo: None,
        branch: "main".into(),
        base: "main".into(),
        title: "SDDK release".into(),
        tag,
        cycle,
        previous_tag: None,
        release_type: None,
        notes: String::new(),
        approve: false,
        timestamp: None,
        actor: None,
        prefix: None,
        format,
    };
    crate::release_cmd::run_release(ReleaseCommand::Plan(args), environment)
}
