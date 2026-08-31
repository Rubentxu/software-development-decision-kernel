//! Capability gateway command surface.

use std::collections::BTreeMap;

use clap::{Args, Subcommand};
use sddk_gateway::{CapabilityGateway, CapabilityPlanInput, CapabilityPolicy};
use serde::Serialize;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::{
    CliEnvironment, CommandOutput, OutputFormat, RuntimeArgs, RuntimeContext, render_result,
};

#[derive(Debug, Subcommand)]
pub(crate) enum CapabilityCommand {
    /// Evaluate policy and produce an executable plan.
    Plan(CapabilityArgs),
    /// Plan, run, and persist a capability receipt.
    Apply(CapabilityArgs),
    /// List persisted capability receipts for the project.
    Status(CapabilityStatusArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CapabilityArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Related cycle identifier.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
    /// Declared capability identifier.
    #[arg(long)]
    pub(crate) capability: String,
    /// Human-readable justification.
    #[arg(long, default_value = "requested by actor")]
    pub(crate) reason: String,
    /// Executable invoked by the typed runner.
    #[arg(long)]
    pub(crate) program: String,
    /// Positional argument passed without a shell.
    #[arg(long, action = clap::ArgAction::Append)]
    pub(crate) arg: Vec<String>,
    /// Environment allowlist entry as `key=value`.
    #[arg(long, action = clap::ArgAction::Append)]
    pub(crate) env: Vec<String>,
    /// Runner timeout in milliseconds.
    #[arg(long, default_value_t = 30_000)]
    pub(crate) timeout_ms: u64,
    /// Runner output limit in bytes per stream.
    #[arg(long, default_value_t = 1_048_576)]
    pub(crate) output_max_bytes: usize,
    /// Explicit human approval for R3/R4 capabilities.
    #[arg(long)]
    pub(crate) approve: bool,
    /// Agent identifier; together with --phase enables the permission gate.
    #[arg(long)]
    pub(crate) agent: Option<String>,
    /// Workflow phase; together with --agent enables the permission gate.
    #[arg(long)]
    pub(crate) phase: Option<String>,
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
pub(crate) struct CapabilityStatusArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

pub(crate) fn run_capability(
    command: CapabilityCommand,
    environment: &CliEnvironment,
) -> CommandOutput {
    match command {
        CapabilityCommand::Plan(args) => run_capability_plan(args, environment, false),
        CapabilityCommand::Apply(args) => run_capability_plan(args, environment, true),
        CapabilityCommand::Status(args) => run_capability_status(args, environment),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct PlanOutput {
    capability: String,
    allowed: bool,
    requires_approval: bool,
    program: String,
    args: Vec<String>,
    receipt_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ReceiptOutput {
    receipt_id: String,
    capability: String,
    status: String,
    result: Option<serde_json::Value>,
}

fn run_capability_plan(
    args: CapabilityArgs,
    environment: &CliEnvironment,
    apply: bool,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<serde_json::Value> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        match (&args.agent, &args.phase) {
            (Some(agent), Some(phase)) => {
                let permissions = sddk_gateway::PermissionPolicy::from_file(
                    context.root.join("permissions.yaml"),
                )?;
                let decision = permissions.authorize(agent, phase, &args.capability);
                if !decision.allowed {
                    anyhow::bail!("{}", decision.reason);
                }
            }
            (None, None) => {}
            _ => anyhow::bail!("--agent and --phase must be supplied together"),
        }
        let workflow = context.engine.workflow().clone();
        let policy = CapabilityPolicy::from_workflow(&workflow);
        let env = parse_env(&args.env)?;
        let timestamp = args.timestamp.clone().unwrap_or_else(default_timestamp);
        let actor = args
            .actor
            .clone()
            .or_else(|| environment.sddk_actor.clone())
            .or_else(|| environment.user.clone())
            .unwrap_or_else(|| "sddk-cli".into());
        let input = CapabilityPlanInput {
            project_id: context.identity.project_id.to_string(),
            cycle_id: args.cycle.clone(),
            capability: args.capability.clone(),
            reason: args.reason.clone(),
            program: args.program.clone(),
            args: args.arg.clone(),
            env,
            timeout_ms: args.timeout_ms,
            output_max_bytes: args.output_max_bytes,
            approve: args.approve,
            timestamp: timestamp.clone(),
            actor: actor.clone(),
        };
        let mut gateway = CapabilityGateway::new(policy, workflow, context.storage);
        if apply {
            let plan = gateway.plan(input)?;
            let receipt = gateway.apply(&plan)?;
            Ok(serde_json::to_value(ReceiptOutput {
                receipt_id: receipt.receipt_id,
                capability: receipt.capability,
                status: serde_json::to_value(receipt.status)?
                    .as_str()
                    .unwrap()
                    .to_owned(),
                result: receipt.result,
            })?)
        } else {
            let plan = gateway.plan(input)?;
            Ok(serde_json::to_value(PlanOutput {
                capability: plan.decision.capability,
                allowed: plan.decision.allowed,
                requires_approval: plan.decision.requires_approval,
                program: plan.run_spec.program,
                args: plan.run_spec.args,
                receipt_id: plan.receipt_id,
            })?)
        }
    })();
    render_result(result, format, plan_receipt_text)
}

fn run_capability_status(
    args: CapabilityStatusArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<Vec<ReceiptOutput>> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let workflow = context.engine.workflow().clone();
        let policy = CapabilityPolicy::from_workflow(&workflow);
        let gateway = CapabilityGateway::new(policy, workflow, context.storage);
        Ok(gateway
            .receipts(context.identity.project_id.as_str())?
            .into_iter()
            .map(|receipt| ReceiptOutput {
                receipt_id: receipt.receipt_id,
                capability: receipt.capability,
                status: serde_json::to_value(receipt.status)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned(),
                result: receipt.result,
            })
            .collect())
    })();
    render_result(result, format, receipts_text)
}

fn plan_receipt_text(value: &serde_json::Value) -> String {
    match value.get("receipt_id") {
        Some(_) => format!(
            "receipt_id: {}\ncapability: {}\nstatus: {}\nresult: {}\n",
            value["receipt_id"].as_str().unwrap_or(""),
            value["capability"].as_str().unwrap_or(""),
            value["status"].as_str().unwrap_or(""),
            value["result"]
        ),
        None => format!(
            "capability: {}\nallowed: {}\nrequires_approval: {}\nprogram: {}\nargs: {}\nreceipt_id: {}\n",
            value["capability"].as_str().unwrap_or(""),
            value["allowed"].as_bool().unwrap_or(false),
            value["requires_approval"].as_bool().unwrap_or(false),
            value["program"].as_str().unwrap_or(""),
            value["args"].as_array().map_or_else(String::new, |args| {
                args.iter()
                    .filter_map(|arg| arg.as_str())
                    .collect::<Vec<_>>()
                    .join(" ")
            }),
            value["receipt_id"].as_str().unwrap_or("")
        ),
    }
}

fn receipts_text(receipts: &Vec<ReceiptOutput>) -> String {
    if receipts.is_empty() {
        return "no receipts\n".to_owned();
    }
    let mut output = String::new();
    for receipt in receipts {
        output.push_str(&format!(
            "{} {} {}\n",
            receipt.receipt_id, receipt.capability, receipt.status
        ));
    }
    output
}

fn parse_env(values: &[String]) -> anyhow::Result<BTreeMap<String, String>> {
    let mut env = BTreeMap::new();
    for value in values {
        let (key, value) = value
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("environment entry must use key=value: {value}"))?;
        if key.is_empty() {
            anyhow::bail!("environment key cannot be empty: {value}");
        }
        env.insert(key.to_owned(), value.to_owned());
    }
    Ok(env)
}

fn default_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("RFC 3339 formatting cannot fail")
}
