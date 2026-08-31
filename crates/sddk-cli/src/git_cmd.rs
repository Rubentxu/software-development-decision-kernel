//! Typed local Git commands with policy checks and verified receipts.

use std::collections::BTreeMap;

use clap::{Args, Subcommand};
use sddk_gateway::{CapabilityGateway, CapabilityPlanInput, CapabilityPolicy, GitExecutor};
use serde::Serialize;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{
    CliEnvironment, CommandOutput, OutputFormat, RuntimeArgs, RuntimeContext, render_result,
};

#[derive(Debug, Subcommand)]
pub(crate) enum GitCommand {
    /// Read repository head, branch, and dirty state.
    Inspect(GitInspectArgs),
    /// Create a branch after a policy check and record a receipt.
    CreateBranch(GitBranchArgs),
    /// Create an empty commit and verify HEAD after a policy check.
    Commit(GitCommitArgs),
    /// Create a tag and verify it after a policy check.
    Tag(GitTagArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct GitInspectArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct GitBranchArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Branch name to create.
    #[arg(long)]
    pub(crate) name: String,
    /// Explicit approval for policies that require it.
    #[arg(long)]
    pub(crate) approve: bool,
    /// Explicit RFC 3339 timestamp for deterministic execution.
    #[arg(long)]
    pub(crate) timestamp: Option<String>,
    /// Explicit actor for deterministic execution.
    #[arg(long)]
    pub(crate) actor: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct GitCommitArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Commit message.
    #[arg(long)]
    pub(crate) message: String,
    /// Explicit approval for policies that require it.
    #[arg(long)]
    pub(crate) approve: bool,
    /// Explicit RFC 3339 timestamp for deterministic execution.
    #[arg(long)]
    pub(crate) timestamp: Option<String>,
    /// Explicit actor for deterministic execution.
    #[arg(long)]
    pub(crate) actor: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct GitTagArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Tag name to create.
    #[arg(long)]
    pub(crate) name: String,
    /// Explicit approval for policies that require it.
    #[arg(long)]
    pub(crate) approve: bool,
    /// Explicit RFC 3339 timestamp for deterministic execution.
    #[arg(long)]
    pub(crate) timestamp: Option<String>,
    /// Explicit actor for deterministic execution.
    #[arg(long)]
    pub(crate) actor: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

pub(crate) fn run_git(command: GitCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        GitCommand::Inspect(args) => run_git_inspect(args, environment),
        GitCommand::CreateBranch(args) => run_git_branch(args, environment),
        GitCommand::Commit(args) => run_git_commit(args, environment),
        GitCommand::Tag(args) => run_git_tag(args, environment),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct GitReceiptOutput {
    capability: String,
    status: String,
    result: serde_json::Value,
}

fn run_git_inspect(args: GitInspectArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<sddk_gateway::GitInspect> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let git = GitExecutor::new(context.root.clone());
        Ok(git.inspect()?)
    })();
    render_result(result, format, inspect_text)
}

fn run_git_branch(args: GitBranchArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<GitReceiptOutput> {
        let (mut gateway, git, input) = effect_setup(
            &args.runtime,
            &args.timestamp,
            &args.actor,
            environment,
            "git.create_branch",
            &args.name,
            &args.approve,
        )?;
        let begin = gateway.begin_effect(&input)?;
        if begin.status != sddk_domain::CapabilityStatus::Started {
            return Ok(receipt_output(&begin));
        }
        let branch = git.create_branch(&args.name)?;
        let receipt = gateway.finish_effect(
            &begin.receipt_id,
            sddk_domain::CapabilityStatus::Succeeded,
            serde_json::to_value(branch)?,
            &input.timestamp,
        )?;
        Ok(receipt_output(&receipt))
    })();
    render_result(result, format, git_receipt_text)
}

fn run_git_commit(args: GitCommitArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<GitReceiptOutput> {
        let (mut gateway, git, input) = effect_setup(
            &args.runtime,
            &args.timestamp,
            &args.actor,
            environment,
            "git.commit",
            &args.message,
            &args.approve,
        )?;
        let begin = gateway.begin_effect(&input)?;
        if begin.status != sddk_domain::CapabilityStatus::Started {
            return Ok(receipt_output(&begin));
        }
        let commit = git.commit(&args.message)?;
        let receipt = gateway.finish_effect(
            &begin.receipt_id,
            sddk_domain::CapabilityStatus::Succeeded,
            serde_json::to_value(commit)?,
            &input.timestamp,
        )?;
        Ok(receipt_output(&receipt))
    })();
    render_result(result, format, git_receipt_text)
}

fn run_git_tag(args: GitTagArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<GitReceiptOutput> {
        let (mut gateway, git, input) = effect_setup(
            &args.runtime,
            &args.timestamp,
            &args.actor,
            environment,
            "git.tag",
            &args.name,
            &args.approve,
        )?;
        let begin = gateway.begin_effect(&input)?;
        if begin.status != sddk_domain::CapabilityStatus::Started {
            return Ok(receipt_output(&begin));
        }
        let tag = git.tag(&args.name)?;
        let receipt = gateway.finish_effect(
            &begin.receipt_id,
            sddk_domain::CapabilityStatus::Succeeded,
            serde_json::to_value(tag)?,
            &input.timestamp,
        )?;
        Ok(receipt_output(&receipt))
    })();
    render_result(result, format, git_receipt_text)
}

type EffectContext = (CapabilityGateway, GitExecutor, CapabilityPlanInput);

fn effect_setup(
    runtime: &RuntimeArgs,
    timestamp: &Option<String>,
    actor: &Option<String>,
    environment: &CliEnvironment,
    capability: &str,
    argument: &str,
    approve: &bool,
) -> anyhow::Result<EffectContext> {
    let context = RuntimeContext::open(runtime, environment, false)?;
    let workflow = context.engine.workflow().clone();
    let policy = CapabilityPolicy::from_workflow(&workflow);
    let gateway = CapabilityGateway::new(policy, workflow, context.storage);
    let git = GitExecutor::new(context.root.clone());
    let timestamp = timestamp.clone().unwrap_or_else(default_timestamp);
    let actor = actor
        .clone()
        .or_else(|| environment.sddk_actor.clone())
        .or_else(|| environment.user.clone())
        .unwrap_or_else(|| "sddk-cli".into());
    let input = CapabilityPlanInput {
        project_id: context.identity.project_id.to_string(),
        cycle_id: None,
        capability: capability.to_owned(),
        reason: format!("{capability} {argument}"),
        program: "git".into(),
        args: vec![argument.to_owned()],
        env: BTreeMap::new(),
        timeout_ms: 30_000,
        output_max_bytes: 1_048_576,
        approve: *approve,
        timestamp: timestamp.clone(),
        actor,
    };
    Ok((gateway, git, input))
}

fn receipt_output(receipt: &sddk_domain::CapabilityReceipt) -> GitReceiptOutput {
    GitReceiptOutput {
        capability: receipt.capability.clone(),
        status: serde_json::to_value(receipt.status)
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned(),
        result: receipt.result.clone().unwrap_or(serde_json::Value::Null),
    }
}

fn inspect_text(inspect: &sddk_gateway::GitInspect) -> String {
    format!(
        "head: {}\nbranch: {}\ndirty: {}\n",
        inspect.head.as_deref().unwrap_or("null"),
        inspect.branch.as_deref().unwrap_or("null"),
        inspect.dirty
    )
}

fn git_receipt_text(output: &GitReceiptOutput) -> String {
    format!(
        "capability: {}\nstatus: {}\nresult: {}\n",
        output.capability, output.status, output.result
    )
}

pub(crate) fn default_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("RFC 3339 formatting cannot fail")
}
