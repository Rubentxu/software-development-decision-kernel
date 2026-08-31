//! Knowledge vault path, profile, and configuration commands.
//!
//! The canonical knowledge vault is keyed by stable project identity under
//! `~/.sddk-knowledge/`. A persisted profile at
//! `$XDG_DATA_HOME/sddk/projects/{project_id}/knowledge-profile.json`
//! records the vault path and Engram preference so that the vault path
//! remains stable across renames of the checkout directory.

use std::fs;
use std::path::{Path, PathBuf};

use clap::{Args, Subcommand, ValueEnum};
use sddk_domain::KnowledgeProfile as PersistedKnowledgeProfile;
use serde::{Deserialize, Serialize};

use crate::{CliEnvironment, CommandOutput, OutputFormat};

/// Knowledge vault command surface.
#[derive(Debug, Subcommand)]
pub(crate) enum KnowledgeCommand {
    /// Print the canonical knowledge vault path for this project.
    Path(KnowledgePathArgs),
    /// Show the knowledge profile (vault path, engram status).
    Status(KnowledgeStatusArgs),
    /// Configure the knowledge profile (e.g., --engram enabled).
    Configure(KnowledgeConfigureArgs),
    /// Detect and classify unregistered repository knowledge into an import plan.
    Scan(KnowledgeScanArgs),
    /// Import one reviewed knowledge plan into the managed vault.
    Import(KnowledgeImportArgs),
    /// Compare registered knowledge provenance with the current repository.
    Verify(KnowledgeVerifyArgs),
}

// ---------------------------------------------------------------------------
// Argument structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Args)]
pub(crate) struct KnowledgePathArgs {
    /// Checkout or worktree root.
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    /// Required monorepo scope, using `.` for the repository root.
    #[arg(long, default_value = ".")]
    pub(crate) scope: String,
    /// Explicit remote URL instead of read-only Git discovery.
    #[arg(long)]
    pub(crate) remote: Option<String>,
    /// Stable UUID for a repository without a remote.
    #[arg(long)]
    pub(crate) fallback_seed: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct KnowledgeStatusArgs {
    /// Checkout or worktree root.
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    /// Required monorepo scope, using `.` for the repository root.
    #[arg(long, default_value = ".")]
    pub(crate) scope: String,
    /// Explicit remote URL instead of read-only Git discovery.
    #[arg(long)]
    pub(crate) remote: Option<String>,
    /// Stable UUID for a repository without a remote.
    #[arg(long)]
    pub(crate) fallback_seed: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct KnowledgeConfigureArgs {
    /// Checkout or worktree root.
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    /// Required monorepo scope, using `.` for the repository root.
    #[arg(long, default_value = ".")]
    pub(crate) scope: String,
    /// Explicit remote URL instead of read-only Git discovery.
    #[arg(long)]
    pub(crate) remote: Option<String>,
    /// Stable UUID for a repository without a remote.
    #[arg(long)]
    pub(crate) fallback_seed: Option<String>,
    /// Enable or disable Engram memory integration.
    #[arg(long, value_enum)]
    pub(crate) engram: Option<EngramSetting>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct KnowledgeImportArgs {
    /// Checkout or worktree root.
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    /// Required monorepo scope, using `.` for the repository root.
    #[arg(long, default_value = ".")]
    pub(crate) scope: String,
    /// Explicit remote URL instead of read-only Git discovery.
    #[arg(long)]
    pub(crate) remote: Option<String>,
    /// Stable UUID for a repository without a remote.
    #[arg(long)]
    pub(crate) fallback_seed: Option<String>,
    /// Identifier emitted by `sddk knowledge scan`.
    #[arg(long)]
    pub(crate) plan: String,
    /// Reviewed existing entries whose changed version is compatible and may be trusted.
    #[arg(long, value_delimiter = ',')]
    pub(crate) approve: Vec<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct KnowledgeScanArgs {
    /// Checkout or worktree root.
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    /// Required monorepo scope, using `.` for the repository root.
    #[arg(long, default_value = ".")]
    pub(crate) scope: String,
    /// Explicit remote URL instead of read-only Git discovery.
    #[arg(long)]
    pub(crate) remote: Option<String>,
    /// Stable UUID for a repository without a remote.
    #[arg(long)]
    pub(crate) fallback_seed: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct KnowledgeVerifyArgs {
    /// Checkout or worktree root.
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    /// Required monorepo scope, using `.` for the repository root.
    #[arg(long, default_value = ".")]
    pub(crate) scope: String,
    /// Explicit remote URL instead of read-only Git discovery.
    #[arg(long)]
    pub(crate) remote: Option<String>,
    /// Stable UUID for a repository without a remote.
    #[arg(long)]
    pub(crate) fallback_seed: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum EngramSetting {
    Enabled,
    Disabled,
}

// ---------------------------------------------------------------------------
// Profile types
// ---------------------------------------------------------------------------

/// Resolved knowledge status returned by the CLI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct KnowledgeStatusOutput {
    /// Stable project identifier derived from remote URL or fallback seed.
    pub project_id: String,
    /// Human-readable project name (basename of the adopted checkout root).
    /// Used only for constructing the default vault path.
    pub project_name: String,
    /// Canonical knowledge vault path selected at adoption time.
    pub vault_path: PathBuf,
    /// Whether Engram memory integration is enabled.
    pub engram_enabled: bool,
    /// Whether a knowledge vault directory exists at `vault_path`.
    pub vault_present: bool,
    /// Whether an XDG knowledge profile has been persisted.
    pub profile_present: bool,
}

/// Input to profile resolution shared by path / status / configure commands.
#[derive(Debug, Clone)]
struct ProfileContext {
    project_id: String,
    project_name: String,
    profile: Option<PersistedKnowledgeProfile>,
    xdg_profile_path: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct ManagedKnowledgeContext {
    pub(crate) project_id: String,
    pub(crate) root: PathBuf,
    pub(crate) vault_path: PathBuf,
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Runs a knowledge command.
pub fn run_knowledge(command: KnowledgeCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        KnowledgeCommand::Path(args) => run_knowledge_path(args, environment),
        KnowledgeCommand::Status(args) => run_knowledge_status(args, environment),
        KnowledgeCommand::Configure(args) => run_knowledge_configure(args, environment),
        KnowledgeCommand::Scan(args) => crate::knowledge_ingest::run_scan(args, environment),
        KnowledgeCommand::Import(args) => crate::knowledge_ingest::run_import(args, environment),
        KnowledgeCommand::Verify(args) => crate::knowledge_ingest::run_verify(args, environment),
    }
}

// ---------------------------------------------------------------------------
// Subcommand implementations
// ---------------------------------------------------------------------------

fn run_knowledge_path(args: KnowledgePathArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<KnowledgePathOutput> {
        let ctx = resolve_profile_context(&args.into(), environment)?;
        Ok(KnowledgePathOutput {
            vault_path: match ctx.profile.as_ref() {
                Some(profile) => profile.vault_path.clone(),
                None => {
                    compute_default_vault_path(&ctx.project_id, &ctx.project_name, environment)?
                }
            },
        })
    })();
    match result {
        Ok(output) => render_result(output, format, knowledge_path_text),
        Err(error) => crate::failure(error.to_string()),
    }
}

fn run_knowledge_status(args: KnowledgeStatusArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<KnowledgeStatusOutput> {
        let ctx = resolve_profile_context(&args.into(), environment)?;
        let vault_path = match ctx.profile.as_ref() {
            Some(profile) => profile.vault_path.clone(),
            None => compute_default_vault_path(&ctx.project_id, &ctx.project_name, environment)?,
        };
        let engram_enabled = ctx.profile.as_ref().is_some_and(|p| p.engram_enabled);
        let vault_present = vault_path.exists();
        Ok(KnowledgeStatusOutput {
            project_id: ctx.project_id,
            project_name: ctx.project_name,
            vault_path,
            engram_enabled,
            vault_present,
            profile_present: ctx.profile.is_some(),
        })
    })();
    match result {
        Ok(profile) => render_result(profile, format, knowledge_status_text),
        Err(error) => crate::failure(error.to_string()),
    }
}

fn run_knowledge_configure(
    args: KnowledgeConfigureArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<KnowledgeStatusOutput> {
        let requested_engram = args.engram;
        let ctx = resolve_profile_context(&args.into(), environment)?;
        // Build profile from existing or freshly computed base
        let existing = ctx.profile.clone();
        let vault_path = match existing.as_ref() {
            Some(profile) => profile.vault_path.clone(),
            None => compute_default_vault_path(&ctx.project_id, &ctx.project_name, environment)?,
        };
        let engram_enabled = requested_engram
            .map(|s| s == EngramSetting::Enabled)
            .unwrap_or_else(|| existing.as_ref().is_some_and(|p| p.engram_enabled));

        let profile = PersistedKnowledgeProfile {
            project_id: sddk_domain::ProjectId::new(ctx.project_id.clone())?,
            project_name: ctx.project_name.clone(),
            vault_path: vault_path.clone(),
            engram_enabled,
        };

        // Persist profile
        let profile_dir = ctx.xdg_profile_path.parent().unwrap();
        fs::create_dir_all(profile_dir)?;
        let json = serde_json::to_string_pretty(&profile)?;
        fs::write(&ctx.xdg_profile_path, json)?;

        Ok(KnowledgeStatusOutput {
            project_id: ctx.project_id,
            project_name: ctx.project_name,
            vault_present: vault_path.exists(),
            profile_present: true,
            vault_path,
            engram_enabled,
        })
    })();
    match result {
        Ok(profile) => render_result(profile, format, knowledge_status_text),
        Err(error) => crate::failure(error.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Core resolution logic
// ---------------------------------------------------------------------------

/// Resolves the shared `ProfileContext` for path / status / configure.
///
fn resolve_profile_context(
    args: &PathArgs,
    environment: &CliEnvironment,
) -> anyhow::Result<ProfileContext> {
    let root = crate::canonical_root(&args.root)?;
    let remote = crate::resolve_remote(&root, args.remote.clone())?;

    let mut fallback_seed = args.fallback_seed.clone();
    if remote.is_none() && fallback_seed.is_none() {
        fallback_seed = crate::find_persisted_fallback_seed(environment, &root, &args.scope)?;
    }
    if remote.is_none() && fallback_seed.is_none() {
        anyhow::bail!(
            "no remote URL and no adoption receipt found; \
             run `sddk adopt apply` first, or pass --fallback-seed"
        );
    }

    let identity = sddk_domain::resolve_project_identity(
        remote.as_deref(),
        &args.scope,
        fallback_seed.as_deref(),
    )?;

    let project_name = root
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| anyhow::anyhow!("root has no UTF-8 basename"))?
        .to_owned();

    // Compute the XDG profile path: $XDG_DATA_HOME/sddk/projects/{project_id}/knowledge-profile.json
    let xdg_profile_path = xdg_knowledge_profile_path(environment, identity.project_id.as_str())?;
    let profile = fs::read_to_string(&xdg_profile_path)
        .ok()
        .and_then(|json| serde_json::from_str::<PersistedKnowledgeProfile>(&json).ok());

    Ok(ProfileContext {
        project_id: identity.project_id.to_string(),
        project_name,
        profile,
        xdg_profile_path,
    })
}

pub(crate) fn resolve_managed_knowledge(
    root: &Path,
    scope: &str,
    remote: Option<String>,
    fallback_seed: Option<String>,
    environment: &CliEnvironment,
) -> anyhow::Result<ManagedKnowledgeContext> {
    let root = crate::canonical_root(root)?;
    let ctx = resolve_profile_context(
        &PathArgs {
            root: root.clone(),
            scope: scope.to_owned(),
            remote,
            fallback_seed,
        },
        environment,
    )?;
    let profile = ctx
        .profile
        .ok_or_else(|| anyhow::anyhow!("project is not adopted; run `sddk adopt apply` first"))?;
    Ok(ManagedKnowledgeContext {
        project_id: ctx.project_id,
        root,
        vault_path: profile.vault_path,
    })
}

/// Returns the stable default vault path, preserving an existing legacy
/// basename vault when one is already present.
fn compute_default_vault_path(
    project_id: &str,
    project_name: &str,
    environment: &CliEnvironment,
) -> anyhow::Result<PathBuf> {
    Ok(sddk_engine::knowledge_vault_path(
        &environment.xdg(),
        project_id,
        project_name,
    )?)
}

/// Returns the persisted knowledge profile path:
/// `$XDG_DATA_HOME/sddk/projects/{project_id}/knowledge-profile.json`.
fn xdg_knowledge_profile_path(
    environment: &CliEnvironment,
    project_id: &str,
) -> anyhow::Result<PathBuf> {
    Ok(sddk_engine::knowledge_profile_path(
        &environment.xdg(),
        project_id,
    )?)
}

// ---------------------------------------------------------------------------
// Conversions
// ---------------------------------------------------------------------------

/// Minimal path-only args used internally for resolution.
#[derive(Debug, Clone)]
struct PathArgs {
    root: PathBuf,
    scope: String,
    remote: Option<String>,
    fallback_seed: Option<String>,
}

impl From<KnowledgePathArgs> for PathArgs {
    fn from(args: KnowledgePathArgs) -> Self {
        Self {
            root: args.root,
            scope: args.scope,
            remote: args.remote,
            fallback_seed: args.fallback_seed,
        }
    }
}

impl From<KnowledgeStatusArgs> for PathArgs {
    fn from(args: KnowledgeStatusArgs) -> Self {
        Self {
            root: args.root,
            scope: args.scope,
            remote: args.remote,
            fallback_seed: args.fallback_seed,
        }
    }
}

impl From<KnowledgeConfigureArgs> for PathArgs {
    fn from(args: KnowledgeConfigureArgs) -> Self {
        Self {
            root: args.root,
            scope: args.scope,
            remote: args.remote,
            fallback_seed: args.fallback_seed,
        }
    }
}

impl From<KnowledgeImportArgs> for PathArgs {
    fn from(args: KnowledgeImportArgs) -> Self {
        Self {
            root: args.root,
            scope: args.scope,
            remote: args.remote,
            fallback_seed: args.fallback_seed,
        }
    }
}

impl From<KnowledgeScanArgs> for PathArgs {
    fn from(args: KnowledgeScanArgs) -> Self {
        Self {
            root: args.root,
            scope: args.scope,
            remote: args.remote,
            fallback_seed: args.fallback_seed,
        }
    }
}

impl From<KnowledgeVerifyArgs> for PathArgs {
    fn from(args: KnowledgeVerifyArgs) -> Self {
        Self {
            root: args.root,
            scope: args.scope,
            remote: args.remote,
            fallback_seed: args.fallback_seed,
        }
    }
}

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct KnowledgePathOutput {
    vault_path: PathBuf,
}

fn knowledge_path_text(output: &KnowledgePathOutput) -> String {
    format!("{}\n", output.vault_path.display())
}

fn knowledge_status_text(profile: &KnowledgeStatusOutput) -> String {
    format!(
        "project_id: {}\nproject_name: {}\nvault_path: {}\nvault_present: {}\nprofile_present: {}\nengram_enabled: {}\n",
        profile.project_id,
        profile.project_name,
        profile.vault_path.display(),
        profile.vault_present,
        profile.profile_present,
        profile.engram_enabled
    )
}

fn render_result<T: serde::Serialize>(
    value: T,
    format: OutputFormat,
    text: fn(&T) -> String,
) -> CommandOutput {
    match format {
        OutputFormat::Text => CommandOutput {
            stdout: text(&value),
            ..CommandOutput::default()
        },
        OutputFormat::Json => match serde_json::to_string_pretty(&value) {
            Ok(json) => CommandOutput {
                stdout: format!("{}\n", json),
                ..CommandOutput::default()
            },
            Err(error) => crate::failure(error.to_string()),
        },
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_env() -> CliEnvironment {
        CliEnvironment {
            home: Some(PathBuf::from("/home/tester")),
            data_home: Some(PathBuf::from("/home/tester/.local/share")),
            sddk_data_dir: None,
            state_home: None,
            cache_home: None,
            sddk_actor: None,
            user: Some("tester".into()),
        }
    }

    #[test]
    fn compute_default_vault_path_uses_home() {
        let env = test_env();
        let path = compute_default_vault_path("p-project", "my-project", &env).unwrap();
        assert_eq!(
            path,
            PathBuf::from("/home/tester/.sddk-knowledge/p-project")
        );
    }

    #[test]
    fn xdg_profile_path_includes_project_id() {
        let env = test_env();
        let path = xdg_knowledge_profile_path(&env, "proj-abc123").unwrap();
        assert!(
            path.to_str()
                .unwrap()
                .ends_with("sddk/projects/proj-abc123/knowledge-profile.json")
        );
    }

    #[test]
    fn knowledge_status_text_contains_all_fields() {
        let profile = KnowledgeStatusOutput {
            project_id: "proj-abc".into(),
            project_name: "my-project".into(),
            vault_path: PathBuf::from("/home/tester/.sddk-knowledge/my-project"),
            engram_enabled: true,
            vault_present: true,
            profile_present: true,
        };
        let text = knowledge_status_text(&profile);
        assert!(text.contains("proj-abc"));
        assert!(text.contains("my-project"));
        assert!(text.contains("/home/tester/.sddk-knowledge/my-project"));
        assert!(text.contains("vault_present: true"));
        assert!(text.contains("profile_present: true"));
        assert!(text.contains("engram_enabled: true"));
    }

    #[test]
    fn knowledge_path_text_contains_vault_path() {
        let output = KnowledgePathOutput {
            vault_path: PathBuf::from("/home/tester/.sddk-knowledge/my-project"),
        };
        let text = knowledge_path_text(&output);
        assert!(text.contains("/home/tester/.sddk-knowledge/my-project"));
    }

    #[test]
    fn engram_setting_is_explicit() {
        assert_ne!(EngramSetting::Enabled, EngramSetting::Disabled);
    }
}
