//! Agent permission check command.

use clap::{Args, Subcommand};
use sddk_gateway::PermissionPolicy;
use serde::Serialize;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat, RuntimeArgs, RuntimeContext, render_result,
};

#[derive(Debug, Subcommand)]
pub(crate) enum PermissionCommand {
    /// Evaluate agent/phase/capability under the default-deny registry.
    Check(PermissionCheckArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct PermissionCheckArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Agent identifier.
    #[arg(long)]
    pub(crate) agent: String,
    /// Workflow phase.
    #[arg(long)]
    pub(crate) phase: String,
    /// Requested capability.
    #[arg(long)]
    pub(crate) capability: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

pub(crate) fn run_permission(
    command: PermissionCommand,
    environment: &CliEnvironment,
) -> CommandOutput {
    match command {
        PermissionCommand::Check(args) => run_permission_check(args, environment),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct PermissionOutput {
    agent: String,
    phase: String,
    capability: String,
    allowed: bool,
    reason: String,
}

fn run_permission_check(args: PermissionCheckArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<PermissionOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let policy = PermissionPolicy::from_file(context.root.join("permissions.yaml"))?;
        let decision = policy.authorize(&args.agent, &args.phase, &args.capability);
        Ok(PermissionOutput {
            agent: args.agent.clone(),
            phase: args.phase.clone(),
            capability: args.capability.clone(),
            allowed: decision.allowed,
            reason: decision.reason,
        })
    })();
    render_result(result, format, permission_text)
}

fn permission_text(output: &PermissionOutput) -> String {
    format!(
        "agent: {}\nphase: {}\ncapability: {}\nallowed: {}\nreason: {}\n",
        output.agent, output.phase, output.capability, output.allowed, output.reason
    )
}
