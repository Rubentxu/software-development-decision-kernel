//! Agent result validation and legacy conversion commands.

use std::fs;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use sddk_domain::{AgentResult, Phase, convert_legacy_map, convert_legacy_text};
use serde::Serialize;
use serde_json::Value;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat, RuntimeArgs, RuntimeContext, render_result,
};

const AGENT_RESULT_SCHEMA: &str = "schemas/agent-result.schema.json";

fn load_schema(root: &std::path::Path, name: &str) -> anyhow::Result<Value> {
    let mut loader = |reference: &str| -> Result<Value, sddk_domain::SchemaError> {
        let path = root.join("schemas").join(reference);
        serde_json::from_str(&fs::read_to_string(&path).map_err(|_| {
            sddk_domain::SchemaError::UnresolvedRef {
                reference: reference.to_owned(),
            }
        })?)
        .map_err(|error| sddk_domain::SchemaError::Compile(error.to_string()))
    };
    let schema: Value = serde_json::from_str(&fs::read_to_string(root.join(name))?)?;
    Ok(sddk_domain::dereference_local_refs(&schema, &mut loader)?)
}

#[derive(Debug, Subcommand)]
pub(crate) enum ValidateCommand {
    /// Validate a JSON document against one canonical schema.
    Schema(ValidateSchemaArgs),
}

/// Canonical schema selectable by `sddk validate schema`.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum SchemaName {
    /// Agent result schema.
    AgentResult,
    /// Cycle manifest schema.
    Cycle,
    /// Phase result schema.
    PhaseResult,
    /// Adoption receipt schema.
    Adoption,
    /// Artifact reference schema.
    ArtifactRef,
}

impl SchemaName {
    pub(crate) fn file(self) -> &'static str {
        match self {
            SchemaName::AgentResult => "schemas/agent-result.schema.json",
            SchemaName::Cycle => "schemas/cycle.schema.json",
            SchemaName::PhaseResult => "schemas/phase-result.schema.json",
            SchemaName::Adoption => "schemas/adoption.schema.json",
            SchemaName::ArtifactRef => "schemas/artifact-ref.schema.json",
        }
    }
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ValidateSchemaArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Schema to validate against.
    #[arg(long, value_enum)]
    pub(crate) schema: SchemaName,
    /// JSON file to validate.
    #[arg(long)]
    pub(crate) file: PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Subcommand)]
pub(crate) enum AgentResultCommand {
    /// Convert legacy JSON or text output into a structured agent result.
    Convert(AgentResultConvertArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct AgentResultConvertArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Legacy JSON file to convert.
    #[arg(long)]
    pub(crate) file: Option<PathBuf>,
    /// Legacy text blob to convert.
    #[arg(long)]
    pub(crate) text: Option<String>,
    /// Agent identifier.
    #[arg(long)]
    pub(crate) agent: String,
    /// Cycle identifier.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Workflow phase of the result.
    #[arg(long)]
    pub(crate) phase: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

pub(crate) fn run_validate(
    command: ValidateCommand,
    environment: &CliEnvironment,
) -> CommandOutput {
    match command {
        ValidateCommand::Schema(args) => run_validate_schema(args, environment),
    }
}

pub(crate) fn run_agent_result(
    command: AgentResultCommand,
    environment: &CliEnvironment,
) -> CommandOutput {
    match command {
        AgentResultCommand::Convert(args) => run_agent_result_convert(args, environment),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct ValidationOutput {
    valid: bool,
    errors: Vec<String>,
}

fn run_validate_schema(args: ValidateSchemaArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<ValidationOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let instance: Value = serde_json::from_str(&fs::read_to_string(&args.file)?)?;
        let schema = load_schema(&context.root, args.schema.file())?;
        let errors = sddk_domain::validate_against_schema(&instance, &schema)?;
        Ok(ValidationOutput {
            valid: errors.is_empty(),
            errors,
        })
    })();
    match result {
        Ok(output) => {
            let mut command = render_result(Ok(output.clone()), format, validation_text);
            if !output.valid {
                command.status = 1;
            }
            command
        }
        Err(error) => crate::failure_envelope(&error),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ConversionOutput {
    result: AgentResult,
    warnings: Vec<String>,
    schema_errors: Vec<String>,
}

fn run_agent_result_convert(
    args: AgentResultConvertArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<ConversionOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let phase: Phase = serde_json::from_value(Value::String(args.phase.clone()))?;
        let (converted, is_text) = match (&args.file, &args.text) {
            (Some(file), None) => {
                let value: Value = serde_json::from_str(&fs::read_to_string(file)?)?;
                (
                    convert_legacy_map(&args.agent, &args.cycle, &phase, &value)?,
                    false,
                )
            }
            (None, Some(text)) => (
                convert_legacy_text(&args.agent, &args.cycle, &phase, text),
                true,
            ),
            _ => anyhow::bail!("exactly one of --file or --text is required"),
        };
        let schema = load_schema(&context.root, AGENT_RESULT_SCHEMA)?;
        let instance = serde_json::to_value(&converted.result)?;
        let schema_errors = sddk_domain::validate_against_schema(&instance, &schema)?;
        let mut warnings = converted.warnings;
        if is_text {
            warnings.push("output converted from unverified text".to_owned());
        }
        Ok(ConversionOutput {
            result: converted.result,
            warnings,
            schema_errors,
        })
    })();
    render_result(result, format, conversion_text)
}

fn validation_text(output: &ValidationOutput) -> String {
    if output.valid {
        return "valid: true\n".to_owned();
    }
    let mut text = format!("valid: false\n{} errors:\n", output.errors.len());
    for error in &output.errors {
        text.push_str(&format!("- {error}\n"));
    }
    text
}

fn conversion_text(output: &ConversionOutput) -> String {
    let mut text = format!(
        "agent: {}\nphase: {}\nverdict: {}\nsummary: {}\nwarnings: {}\nschema_valid: {}\n",
        output.result.agent,
        serde_json::to_value(output.result.phase)
            .unwrap()
            .as_str()
            .unwrap_or(""),
        serde_json::to_value(&output.result.verdict)
            .unwrap()
            .as_str()
            .unwrap_or(""),
        output.result.summary,
        output.warnings.len(),
        output.schema_errors.is_empty()
    );
    for warning in &output.warnings {
        text.push_str(&format!("- warning: {warning}\n"));
    }
    for error in &output.schema_errors {
        text.push_str(&format!("- schema: {error}\n"));
    }
    text
}
