//! Content-addressed artifact commands.

use std::fs;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use sddk_gateway::ArtifactStore;
use serde::Serialize;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat, RuntimeArgs, RuntimeContext, render_result,
};

#[derive(Debug, Subcommand)]
pub(crate) enum ArtifactCommand {
    /// Store a file by its content digest and record metadata.
    Store(ArtifactStoreArgs),
    /// Verify and read stored bytes by digest.
    Get(ArtifactGetArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ArtifactStoreArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// File whose bytes are stored.
    #[arg(long)]
    pub(crate) file: PathBuf,
    /// Artifact kind from the workflow contract.
    #[arg(long)]
    pub(crate) kind: String,
    /// Related cycle identifier.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
    /// Producer identifier.
    #[arg(long)]
    pub(crate) producer: Option<String>,
    /// Explicit RFC 3339 timestamp for deterministic execution.
    #[arg(long)]
    pub(crate) timestamp: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ArtifactGetArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Content digest to verify and read.
    #[arg(long)]
    pub(crate) digest: String,
    /// Destination file for the verified bytes.
    #[arg(long)]
    pub(crate) output: PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

pub(crate) fn run_artifact(
    command: ArtifactCommand,
    environment: &CliEnvironment,
) -> CommandOutput {
    match command {
        ArtifactCommand::Store(args) => run_artifact_store(args, environment),
        ArtifactCommand::Get(args) => run_artifact_get(args, environment),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct StoredArtifactOutput {
    artifact_id: String,
    sha256: String,
    kind: String,
    path: String,
    size: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ArtifactGetOutput {
    digest: String,
    bytes: usize,
    output: String,
}

fn run_artifact_store(args: ArtifactStoreArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<StoredArtifactOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let bytes = fs::read(&args.file)?;
        let store = ArtifactStore::new(context.storage, context.artifacts_path.clone());
        let timestamp = args
            .timestamp
            .clone()
            .unwrap_or_else(crate::git_cmd::default_timestamp);
        let producer = args.producer.clone().unwrap_or_else(|| "sddk-cli".into());
        let record = store.store(
            &bytes,
            &sddk_gateway::ArtifactMeta {
                project_id: context.identity.project_id.to_string(),
                cycle_id: args.cycle.clone(),
                kind: args.kind.clone(),
                path: args.file.to_string_lossy().into_owned(),
                producer,
                created_at: timestamp,
            },
        )?;
        Ok(StoredArtifactOutput {
            artifact_id: record.artifact_id,
            sha256: record.sha256.unwrap_or_default(),
            kind: record.kind,
            path: record.path,
            size: bytes.len(),
        })
    })();
    render_result(result, format, stored_artifact_text)
}

fn run_artifact_get(args: ArtifactGetArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<ArtifactGetOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let store = ArtifactStore::new(context.storage, context.artifacts_path.clone());
        let bytes = store.get(&args.digest)?;
        fs::write(&args.output, &bytes)?;
        Ok(ArtifactGetOutput {
            digest: args.digest.clone(),
            bytes: bytes.len(),
            output: args.output.to_string_lossy().into_owned(),
        })
    })();
    render_result(result, format, artifact_get_text)
}

fn stored_artifact_text(output: &StoredArtifactOutput) -> String {
    format!(
        "artifact_id: {}\nsha256: {}\nkind: {}\npath: {}\nsize: {}\n",
        output.artifact_id, output.sha256, output.kind, output.path, output.size
    )
}

fn artifact_get_text(output: &ArtifactGetOutput) -> String {
    format!(
        "digest: {}\nbytes: {}\noutput: {}\n",
        output.digest, output.bytes, output.output
    )
}
