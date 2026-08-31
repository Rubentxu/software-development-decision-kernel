//! `dev models` — manage agent-models.yaml: list, set, validate, tui-path.
//! Exit codes: 0 success · 2 invalid config/arguments · 3 unresolvable
//! target or bundle (relayed by the TUI as its own exit 3).

use crate::dev::agent_models::{AgentModelsConfig, IdeKey, ModelTier};
use crate::dev::common::atomic_write;
use crate::dev::paths::resolve_active_framework_root;
use crate::{CliEnvironment, CommandOutput, OutputFormat};
use clap::{Args, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Args)]
pub(crate) struct ModelsArgs {
    #[command(subcommand)]
    pub(super) command: ModelsCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub(crate) enum ModelsCommand {
    /// List bundle agents with their current tier and per-IDE overrides.
    List(ModelsListArgs),
    /// Edit tier/override for one agent, validate, and write atomically.
    Set(ModelsSetArgs),
    /// Validate the target agent-models.yaml (exit 0 valid / 2 invalid).
    Validate(ModelsValidateArgs),
    /// Print the path of the bundled agent-models TUI script.
    TuiPath,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ModelsListArgs {
    /// Target agent-models.yaml (default: active bundle assets/agent-models.yaml).
    #[arg(long)]
    pub(super) file: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ModelsSetArgs {
    /// Target agent-models.yaml (default: active bundle assets/agent-models.yaml).
    #[arg(long)]
    pub(super) file: Option<PathBuf>,
    /// Agent to edit.
    #[arg(long)]
    pub(super) agent: String,
    /// Set the agent's tier.
    #[arg(long, value_enum)]
    pub(super) tier: Option<ModelTier>,
    /// Set a per-IDE override (repeatable, format IDE=MODEL).
    #[arg(long, value_name = "IDE=MODEL")]
    pub(super) r#override: Vec<String>,
    /// Clear a per-IDE override (repeatable).
    #[arg(long, value_enum)]
    pub(super) clear_override: Vec<IdeKey>,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ModelsValidateArgs {
    /// Target agent-models.yaml (default: active bundle assets/agent-models.yaml).
    #[arg(long)]
    pub(super) file: Option<PathBuf>,
}

// ── Shared helpers ────────────────────────────────────────────────────────────

fn bundle_root(environment: &CliEnvironment) -> Result<PathBuf, String> {
    resolve_active_framework_root(environment)
        .map_err(|error| format!("framework bundle unresolvable: {error}"))
}

fn resolve_target(file: Option<&Path>, environment: &CliEnvironment) -> Result<PathBuf, String> {
    if let Some(path) = file {
        return Ok(path.to_path_buf());
    }
    bundle_root(environment).map(|root| root.join("assets").join("agent-models.yaml"))
}

fn failure(status: i32, message: String) -> CommandOutput {
    CommandOutput {
        status,
        stdout: String::new(),
        stderr: format!("error: {message}\n"),
    }
}

/// Bundle agent names: every `agents/*.md` stem of the framework root.
fn bundle_agent_names(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root.join("agents"))
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("md"))
                .filter_map(|entry| {
                    entry
                        .path()
                        .file_stem()
                        .map(|stem| stem.to_string_lossy().into_owned())
                })
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

fn parse_override(value: &str) -> Result<(IdeKey, String), String> {
    let Some((ide_name, model)) = value.split_once('=') else {
        return Err(format!(
            "invalid override `{value}` (expected IDE=MODEL, e.g. opencode=deepseek/deepseek-chat)"
        ));
    };
    let Some(ide) = IdeKey::parse(ide_name.trim()) else {
        return Err(format!(
            "unknown IDE `{ide_name}` (expected opencode|zcode|claude|codex)"
        ));
    };
    if model.trim().is_empty() {
        return Err("empty model id in override".to_owned());
    }
    Ok((ide, model.trim().to_owned()))
}

// ── list ──────────────────────────────────────────────────────────────────────

fn list_json(target: &Path, config: &AgentModelsConfig, names: &[String]) -> String {
    let tiers = serde_json::Map::from_iter(config.tiers().iter().map(|(tier, table)| {
        let table = serde_json::Map::from_iter(table.iter().map(|(ide, model)| {
            (
                ide.as_str().to_owned(),
                serde_json::Value::String(model.clone()),
            )
        }));
        (tier.as_str().to_owned(), serde_json::Value::Object(table))
    }));
    let agents: Vec<serde_json::Value> = names
        .iter()
        .map(|name| {
            let mut entry = serde_json::Map::new();
            entry.insert("name".to_owned(), serde_json::Value::String(name.clone()));
            entry.insert(
                "tier".to_owned(),
                config
                    .tier_of(name)
                    .map(|tier| serde_json::Value::String(tier.as_str().to_owned()))
                    .unwrap_or(serde_json::Value::Null),
            );
            let overrides = serde_json::Map::from_iter(
                config
                    .overrides_of(name)
                    .into_iter()
                    .flat_map(|map| map.iter())
                    .map(|(ide, model)| {
                        (
                            ide.as_str().to_owned(),
                            serde_json::Value::String(model.clone()),
                        )
                    }),
            );
            entry.insert("overrides".to_owned(), serde_json::Value::Object(overrides));
            serde_json::Value::Object(entry)
        })
        .collect();
    let doc = serde_json::json!({
        "target": target,
        "tiers": tiers,
        "agents": agents,
    });
    format!(
        "{}\n",
        serde_json::to_string_pretty(&doc).unwrap_or_default()
    )
}

fn list_text(target: &Path, config: &AgentModelsConfig, names: &[String]) -> String {
    let mut text = format!("target: {}\ntiers:\n", target.display());
    for tier in [ModelTier::Premium, ModelTier::Fast] {
        let Some(table) = config.tiers().get(&tier) else {
            continue;
        };
        let entries: Vec<String> = table
            .iter()
            .map(|(ide, model)| format!("{}={model}", ide.as_str()))
            .collect();
        text.push_str(&format!("  {}: {}\n", tier.as_str(), entries.join(", ")));
    }
    text.push_str("agents:\n");
    for name in names {
        match config.tier_of(name) {
            Some(tier) => {
                let overrides: Vec<String> = config
                    .overrides_of(name)
                    .into_iter()
                    .flat_map(|map| map.iter())
                    .map(|(ide, model)| format!("{}: {model}", ide.as_str()))
                    .collect();
                if overrides.is_empty() {
                    text.push_str(&format!("  {name}: tier={}\n", tier.as_str()));
                } else {
                    text.push_str(&format!(
                        "  {name}: tier={} overrides={{{}}}\n",
                        tier.as_str(),
                        overrides.join(", ")
                    ));
                }
            }
            None => text.push_str(&format!("  {name}: tier=none\n")),
        }
    }
    text
}

fn run_list(args: ModelsListArgs, environment: &CliEnvironment) -> CommandOutput {
    let target = match resolve_target(args.file.as_deref(), environment) {
        Ok(target) => target,
        Err(message) => return failure(3, message),
    };
    let config = match AgentModelsConfig::from_file(&target) {
        Ok(Some(config)) => config,
        Ok(None) => AgentModelsConfig::default(),
        Err(error) => return failure(2, error.to_string()),
    };
    let names = match bundle_root(environment) {
        Ok(root) => bundle_agent_names(&root),
        Err(_) => config.agents().keys().cloned().collect(),
    };
    let text = list_text(&target, &config, &names);
    match args.format {
        OutputFormat::Text => CommandOutput {
            status: 0,
            stdout: text,
            stderr: String::new(),
        },
        OutputFormat::Json => CommandOutput {
            status: 0,
            stdout: list_json(&target, &config, &names),
            stderr: String::new(),
        },
    }
}

// ── set ───────────────────────────────────────────────────────────────────────

fn run_set(args: ModelsSetArgs, environment: &CliEnvironment) -> CommandOutput {
    let target = match resolve_target(args.file.as_deref(), environment) {
        Ok(target) => target,
        Err(message) => return failure(3, message),
    };
    // Load (absent → init empty config at the target path), mutate, validate,
    // serialize, and write atomically. Invalid input writes nothing.
    let mut config = match AgentModelsConfig::from_file(&target) {
        Ok(Some(config)) => config,
        Ok(None) => AgentModelsConfig::default(),
        Err(error) => return failure(2, error.to_string()),
    };
    if let Some(tier) = args.tier {
        config.set_tier(&args.agent, tier);
    }
    for override_arg in &args.r#override {
        let (ide, model) = match parse_override(override_arg) {
            Ok(parsed) => parsed,
            Err(message) => return failure(2, message),
        };
        if let Err(error) = config.set_override(&args.agent, ide, model) {
            return failure(2, error.to_string());
        }
    }
    for ide in &args.clear_override {
        config.clear_override(&args.agent, *ide);
    }
    if args.tier.is_none() && args.r#override.is_empty() && args.clear_override.is_empty() {
        return failure(
            2,
            "nothing to set: pass --tier, --override, or --clear-override".to_owned(),
        );
    }
    let yaml = match config.to_yaml() {
        Ok(yaml) => yaml,
        Err(error) => return failure(2, error.to_string()),
    };
    // Validate what we are about to write (round-trip through the loader).
    if let Err(error) = AgentModelsConfig::from_yaml(&yaml) {
        return failure(2, error.to_string());
    }
    if let Err(error) = atomic_write(&target, yaml.as_bytes(), None) {
        return failure(2, error.to_string());
    }
    CommandOutput {
        status: 0,
        stdout: format!("agent `{}` updated in {}\n", args.agent, target.display()),
        stderr: String::new(),
    }
}

// ── validate ──────────────────────────────────────────────────────────────────

fn run_validate(args: ModelsValidateArgs, environment: &CliEnvironment) -> CommandOutput {
    let target = match resolve_target(args.file.as_deref(), environment) {
        Ok(target) => target,
        Err(message) => return failure(3, message),
    };
    match AgentModelsConfig::from_file(&target) {
        Ok(Some(config)) => CommandOutput {
            status: 0,
            stdout: format!(
                "agent-models.yaml valid: {} agents, {} tier tables\n",
                config.agents().len(),
                config.tiers().len()
            ),
            stderr: String::new(),
        },
        Ok(None) => CommandOutput {
            status: 0,
            stdout: format!(
                "agent-models.yaml absent at {} (empty config is valid)\n",
                target.display()
            ),
            stderr: String::new(),
        },
        Err(error) => failure(2, error.to_string()),
    }
}

// ── tui-path ──────────────────────────────────────────────────────────────────

fn run_tui_path(environment: &CliEnvironment) -> CommandOutput {
    let root = match bundle_root(environment) {
        Ok(root) => root,
        Err(message) => return failure(3, message),
    };
    let script = root.join("assets").join("agent-models").join("tui.sh");
    if !script.is_file() {
        return failure(
            3,
            format!("TUI script missing from bundle: {}", script.display()),
        );
    }
    CommandOutput {
        status: 0,
        stdout: format!("{}\n", script.display()),
        stderr: String::new(),
    }
}

// ── Public subcommand ──────────────────────────────────────────────────────────

pub(super) fn run_dev_models(args: ModelsArgs, environment: &CliEnvironment) -> CommandOutput {
    match args.command {
        ModelsCommand::List(list_args) => run_list(list_args, environment),
        ModelsCommand::Set(set_args) => run_set(set_args, environment),
        ModelsCommand::Validate(validate_args) => run_validate(validate_args, environment),
        ModelsCommand::TuiPath => run_tui_path(environment),
    }
}

#[cfg(test)]
#[path = "tests/models_cmd_tests.rs"]
mod models_cmd_tests;
