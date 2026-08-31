//! Pack registry commands: validate, list, inspect, install, verify, enable, disable.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use sddk_domain::{PackDiagnostic, load_pack_manifest, validate_pack_manifest};
use sddk_domain::{resolve_project_identity, stable_workspace_id};
use sddk_engine::pack_registry::{PackRegistry, RegistryEntry, VerifyReport};
use serde::Serialize;

use crate::{CliEnvironment, CommandOutput, OutputFormat, render_result};

#[derive(Debug, Subcommand)]
pub(crate) enum PackCommand {
    /// Validate a pack manifest against the pack model.
    Validate(PackValidateArgs),
    /// List discovered packs in the project registry.
    List(PackListArgs),
    /// Inspect a single pack manifest.
    Inspect(PackInspectArgs),
    /// Install a pack from a local source directory.
    Install(PackInstallArgs),
    /// Verify a pack manifest and its dependency satisfaction.
    Verify(PackVerifyArgs),
    /// Enable a pack (idempotent).
    Enable(PackEnableArgs),
    /// Disable a pack (idempotent).
    Disable(PackDisableArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct PackValidateArgs {
    /// Manifest path.
    #[arg(long, default_value = "manifest.toml")]
    pub(crate) manifest: PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct PackListArgs {
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct PackInspectArgs {
    /// Pack identifier.
    #[arg(long)]
    pub(crate) id: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct PackInstallArgs {
    /// Source directory containing a valid manifest.toml.
    #[arg(long)]
    pub(crate) source: PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct PackVerifyArgs {
    /// Pack identifier.
    #[arg(long)]
    pub(crate) id: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct PackEnableArgs {
    /// Pack identifier.
    #[arg(long)]
    pub(crate) id: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct PackDisableArgs {
    /// Pack identifier.
    #[arg(long)]
    pub(crate) id: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
struct PackToggleArgs {
    #[arg(long)]
    id: String,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

pub(crate) fn run_pack(command: PackCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        PackCommand::Validate(args) => run_pack_validate(args),
        PackCommand::List(args) => run_pack_list(args, environment),
        PackCommand::Inspect(args) => run_pack_inspect(args, environment),
        PackCommand::Install(args) => run_pack_install(args, environment),
        PackCommand::Verify(args) => run_pack_verify(args, environment),
        PackCommand::Enable(args) => run_pack_enable_disable(
            PackToggleArgs {
                id: args.id,
                format: args.format,
            },
            environment,
            true,
        ),
        PackCommand::Disable(args) => run_pack_enable_disable(
            PackToggleArgs {
                id: args.id,
                format: args.format,
            },
            environment,
            false,
        ),
    }
}

/// Resolves the project pack registry from the current root and environment.
fn resolve_registry(environment: &CliEnvironment) -> Result<PackRegistry, String> {
    let root = std::env::current_dir().map_err(|error| error.to_string())?;
    let remote = crate::resolve_remote(&root, None).map_err(|error| error.to_string())?;
    let identity = resolve_project_identity(remote.as_deref(), ".", None)
        .map_err(|error| error.to_string())?;
    let workspace_id = stable_workspace_id(&identity.project_id, &root.to_string_lossy());
    let paths = sddk_engine::resolve_xdg_paths(
        &environment.xdg(),
        identity.project_id.as_str(),
        &workspace_id,
    )
    .map_err(|error| error.to_string())?;
    let state_path = paths.project_data.join("pack-registry.json");
    Ok(PackRegistry::new(&root, state_path))
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct PackValidationOutput {
    id: String,
    version: String,
    schema_version: i32,
    valid: bool,
    diagnostics: Vec<PackDiagnostic>,
}

fn run_pack_validate(args: PackValidateArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<PackValidationOutput> {
        let manifest = load_pack_manifest(&args.manifest)?;
        let diagnostics = validate_pack_manifest(&manifest);
        Ok(PackValidationOutput {
            id: manifest.pack.id.clone(),
            version: manifest.pack.version.clone(),
            schema_version: manifest.pack.schema_version,
            valid: diagnostics.is_empty(),
            diagnostics,
        })
    })();
    match result {
        Ok(output) => {
            let mut command = render_result(Ok(output.clone()), format, pack_validation_text);
            if !output.valid {
                command.status = 1;
            }
            command
        }
        Err(error) => crate::failure(error.to_string()),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct PackListOutput {
    packs: Vec<RegistryEntry>,
}

fn run_pack_list(args: PackListArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> Result<PackListOutput, String> {
        let registry = resolve_registry(environment)?;
        let packs = registry.discover().map_err(|error| error.to_string())?;
        Ok(PackListOutput { packs })
    })();
    match result {
        Ok(output) => render_result(Ok(output), format, pack_list_text),
        Err(error) => crate::failure(error),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct PackInspectOutput {
    entry: RegistryEntry,
}

fn run_pack_inspect(args: PackInspectArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> Result<PackInspectOutput, String> {
        let registry = resolve_registry(environment)?;
        let entry = registry.find(&args.id).map_err(|error| error.to_string())?;
        Ok(PackInspectOutput { entry })
    })();
    match result {
        Ok(output) => render_result(Ok(output), format, pack_inspect_text),
        Err(error) => crate::failure(error),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct PackInstallOutput {
    entry: RegistryEntry,
}

fn run_pack_install(args: PackInstallArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> Result<PackInstallOutput, String> {
        let registry = resolve_registry(environment)?;
        let entry = registry
            .install(&args.source)
            .map_err(|error| error.to_string())?;
        Ok(PackInstallOutput { entry })
    })();
    match result {
        Ok(output) => render_result(Ok(output), format, pack_install_text),
        Err(error) => crate::failure(error),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct PackVerifyOutput {
    report: VerifyReport,
}

fn run_pack_verify(args: PackVerifyArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> Result<PackVerifyOutput, String> {
        let registry = resolve_registry(environment)?;
        let report = registry
            .verify(&args.id)
            .map_err(|error| error.to_string())?;
        Ok(PackVerifyOutput { report })
    })();
    match result {
        Ok(output) => {
            let mut command = render_result(Ok(output.clone()), format, pack_verify_text);
            if !output.report.valid {
                command.status = 1;
            }
            command
        }
        Err(error) => crate::failure(error),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct PackToggleOutput {
    id: String,
    enabled: bool,
}

fn run_pack_enable_disable(
    args: PackToggleArgs,
    environment: &CliEnvironment,
    enable: bool,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> Result<PackToggleOutput, String> {
        let registry = resolve_registry(environment)?;
        if enable {
            registry
                .enable(&args.id)
                .map_err(|error| error.to_string())?;
        } else {
            registry
                .disable(&args.id)
                .map_err(|error| error.to_string())?;
        }
        Ok(PackToggleOutput {
            id: args.id.clone(),
            enabled: enable,
        })
    })();
    match result {
        Ok(output) => render_result(Ok(output), format, pack_toggle_text),
        Err(error) => crate::failure(error),
    }
}

fn pack_validation_text(output: &PackValidationOutput) -> String {
    let mut text = format!(
        "id: {}\nversion: {}\nschema_version: {}\nvalid: {}\n",
        output.id, output.version, output.schema_version, output.valid
    );
    for diagnostic in &output.diagnostics {
        text.push_str(&format!(
            "error[{}]: {}\n  help: {}\n",
            diagnostic.code, diagnostic.message, diagnostic.hint
        ));
    }
    text
}

fn pack_list_text(output: &PackListOutput) -> String {
    if output.packs.is_empty() {
        return "no packs found under packs/\n".to_string();
    }
    let mut text = String::from("id\tversion\tcategory\tenabled\n");
    for pack in &output.packs {
        text.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            pack.id, pack.version, pack.category, pack.enabled
        ));
    }
    text
}

fn pack_inspect_text(output: &PackInspectOutput) -> String {
    let entry = &output.entry;
    format!(
        "id: {}\nversion: {}\ncategory: {}\nenabled: {}\nmanifest: {}\n",
        entry.id,
        entry.version,
        entry.category,
        entry.enabled,
        entry.manifest_path.display()
    )
}

fn pack_install_text(output: &PackInstallOutput) -> String {
    let entry = &output.entry;
    format!(
        "installed: {}\nversion: {}\ncategory: {}\nmanifest: {}\n",
        entry.id,
        entry.version,
        entry.category,
        entry.manifest_path.display()
    )
}

fn pack_verify_text(output: &PackVerifyOutput) -> String {
    let report = &output.report;
    let mut text = format!("id: {}\nvalid: {}\n", report.id, report.valid);
    for diagnostic in &report.diagnostics {
        text.push_str(&format!(
            "error[{}]: {}\n  help: {}\n",
            diagnostic.code, diagnostic.message, diagnostic.hint
        ));
    }
    for requirement in &report.unsatisfied {
        text.push_str(&format!("unsatisfied: {}\n", requirement));
    }
    text
}

fn pack_toggle_text(output: &PackToggleOutput) -> String {
    format!("id: {}\nenabled: {}\n", output.id, output.enabled)
}
