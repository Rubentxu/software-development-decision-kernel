//! Testable command surface for the SDDK CLI.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(missing_docs)]

mod analytics;
mod approval;
mod artifact;
mod capability;
mod cycle;
mod debt;
mod dev;

mod docs;
mod explore_cmd;
mod fork_cmd;
mod git_cmd;
mod graph_cmd;
mod inventory;
mod inventory_cycle;
mod knowledge_cmd;
mod knowledge_ingest;
mod ledger;
mod lint;
mod metrics;
mod pack_cmd;
mod permission;
mod plan;
mod recover;
mod release_cmd;
mod result_cmd;
mod rules_cmd;
mod run;
mod ship;
mod stale_cmd;
mod status;
mod telemetry;
mod uat;
mod uat_common;
mod uat_discover;
mod uat_enrich;
mod uat_generate;
mod uat_quality;
mod uat_serve;
mod vault_cmd;

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use analytics::AnalyticsCommand;
use approval::ApprovalCommand;
use artifact::ArtifactCommand;
use capability::CapabilityCommand;
use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum};
pub(crate) use cycle::{CycleCommand, RuntimeArgs, RuntimeContext};
use dev::DevCommand;
use git_cmd::GitCommand;
use knowledge_cmd::KnowledgeCommand;
use metrics::MetricsCommand;
use pack_cmd::PackCommand;
use permission::PermissionCommand;
use release_cmd::ReleaseCommand;
use result_cmd::{AgentResultCommand, ValidateCommand};
use rules_cmd::RulesCommand;
use sddk_domain::{
    IdentitySource, LedgerFactory, SddkErrorCode, normalize_scope, resolve_project_identity,
    stable_workspace_id,
};
use sddk_storage::{SqliteControlPlane, SqliteLedgerFactory};

use sddk_engine::{
    AdoptionPlan, AdoptionPlanInput, AdoptionStatus, AdoptionStatusKind, XdgEnvironment,
    adoption_status, apply_adoption, plan_adoption, read_adoption_receipt, repair_adoption,
};
/// Re-exports `Storage` so that `cycle.rs` can use it via `crate::Storage`
/// without a direct `use sddk_storage::Storage` import (ARCH003 edge elimination).
pub use sddk_storage::Storage;
use serde::Serialize;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;
use vault_cmd::VaultCommand;
use walkdir::WalkDir;

pub use docs::{GENERATED_WORKFLOW_DOC, GenerationStatus, generate_workflow_docs};
pub use inventory::{GENERATED_INVENTORY_DOC, generate_inventory};
pub use lint::{
    Diagnostic, LintReport, Severity, lint_repository, validate_classifications_registry,
};

/// Canonical workflow manifest path, relative to the repository root.
pub(crate) const WORKFLOW_MANIFEST: &str = "workflow/workflow.yaml";

/// Canonical workflow manifest embedded in this binary. `adopt apply` seeds it
/// into adopted repositories that lack one, and cycle commands fall back to it.
pub(crate) const CANONICAL_WORKFLOW: &str = include_str!("../../../workflow/workflow.yaml");

// ── Composition root ───────────────────────────────────────────────────────────

/// Resolves the XDG data root for the control-plane store.
fn control_plane_dir(env: &CliEnvironment) -> anyhow::Result<PathBuf> {
    let data_home = if let Some(dir) = &env.sddk_data_dir {
        dir.clone()
    } else {
        match (&env.data_home, &env.home) {
            (Some(data), _) => data.clone(),
            (None, Some(home)) => home.join(".local/share"),
            (None, None) => dirs::data_dir().ok_or_else(|| {
                anyhow::anyhow!("no data root: set HOME, XDG_DATA_HOME or SDDK_DATA_DIR")
            })?,
        }
    };
    if !data_home.is_absolute() {
        anyhow::bail!("data root must be absolute: {data_home:?}");
    }
    Ok(data_home.join("sddk/control-plane"))
}

/// Composition root: opens both the project ledger and the control-plane store.
///
/// Uses `SqliteLedgerFactory` (which implements `LedgerFactory`) to open the ledger,
/// making the factory the explicit entry point per ADR-0021 §2.
///
/// Returns `(Storage, SqliteControlPlane)` so that callers can pass
/// `&mut dyn ControlPlane` down to helpers without exposing concrete types.
pub(crate) fn compose(
    env: &CliEnvironment,
    ledger_path: &Path,
) -> anyhow::Result<(Storage, SqliteControlPlane)> {
    let factory = SqliteLedgerFactory;
    let storage = factory
        .open_ledger(ledger_path)
        .map_err(|e| anyhow::anyhow!("LedgerFactory: {e}"))?;
    let plane = SqliteControlPlane::open(&control_plane_dir(env)?)?;
    Ok((storage, plane))
}

/// Parsed SDDK command line.
#[derive(Debug, Parser)]
#[command(
    name = "sddk",
    version,
    about = "Deterministic SDDK workflow tooling — First-class commands: status, plan, run, ship, recover"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show the resolved framework version for the current directory.
    Version(VersionArgs),
    /// Resolve deterministic project and workspace identity.
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    /// Plan, apply, inspect, or repair project adoption.
    Adopt {
        #[command(subcommand)]
        command: AdoptCommand,
    },
    /// Validate repository contracts and generated workflow documentation.
    Lint {
        /// Repository root.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Diagnostic output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Generate deterministic repository documentation.
    Generate {
        #[command(subcommand)]
        command: GenerateCommand,
    },
    /// Plan and apply workflow cycles under the local authority.
    Cycle {
        #[command(subcommand)]
        command: CycleCommand,
    },
    /// Verify the causal ledger and list its events.
    Ledger {
        #[command(subcommand)]
        command: ledger::LedgerCommand,
    },
    /// Plan and execute typed capabilities under the default-deny policy.
    Capability {
        #[command(subcommand)]
        command: CapabilityCommand,
    },
    /// Run typed local Git operations with verified postconditions.
    Git {
        #[command(subcommand)]
        command: GitCommand,
    },
    /// Store and verify content-addressed artifacts.
    Artifact {
        #[command(subcommand)]
        command: ArtifactCommand,
    },
    /// Check agent phase and capability permissions.
    Permission {
        #[command(subcommand)]
        command: PermissionCommand,
    },
    /// Validate structured results against canonical schemas.
    Validate {
        #[command(subcommand)]
        command: ValidateCommand,
    },
    /// Convert legacy agent output into structured results.
    AgentResult {
        #[command(subcommand)]
        command: AgentResultCommand,
    },
    /// Plan and apply local Git releases or optional forge integrations.
    Release {
        #[command(subcommand)]
        command: ReleaseCommand,
    },
    /// Index, validate, search, and export knowledge vaults.
    Vault {
        #[command(subcommand)]
        command: VaultCommand,
    },
    /// Resolve the canonical knowledge vault path and profile.
    Knowledge {
        #[command(subcommand)]
        command: KnowledgeCommand,
    },
    /// Developer tooling: doctor, gates, and atomic install/verify.
    Dev {
        #[command(subcommand)]
        command: DevCommand,
    },
    /// Validate declarative pack manifests.
    Pack {
        #[command(subcommand)]
        command: PackCommand,
    },
    /// Query, inspect, and rebuild the reactive knowledge graph.
    Graph {
        #[command(subcommand)]
        command: graph_cmd::GraphCommand,
    },
    /// Record, aggregate, and tune cycle telemetry metrics.
    Metrics {
        #[command(subcommand)]
        command: MetricsCommand,
    },
    /// Report, trend, and bottleneck analytics from cycle metrics.
    Analytics {
        #[command(subcommand)]
        command: AnalyticsCommand,
    },
    /// Central telemetry control plane (cross-project ingest, aggregates, dashboard).
    Telemetry {
        #[command(subcommand)]
        command: telemetry::TelemetryCommand,
    },
    /// UAT (User Acceptance Testing): data-driven YAML plans rendered to
    /// self-contained HTML dashboards, with human-in-the-loop validation.
    Uat {
        #[command(subcommand)]
        command: uat::UatCommand,
    },
    /// Manage human approval decisions for governed capabilities.
    Approval {
        #[command(subcommand)]
        command: ApprovalCommand,
    },
    /// Architecture-rule registry: evaluate rules against baseline JSON (SDDK2-003).
    Rules {
        #[command(subcommand)]
        command: RulesCommand,
    },
    /// Staleness and impact queries over the reactive graph (SPEC-012).
    Stale {
        #[command(subcommand)]
        command: stale_cmd::StaleCommand,
    },
    /// Fork, replay, diff and promote controlled experiments (SPEC-009).
    Fork {
        #[command(subcommand)]
        command: fork_cmd::ForkCommand,
    },
    /// Render task-specific views over the reactive graph (SPEC-013).
    Explore {
        #[command(subcommand)]
        command: explore_cmd::ExploreCommand,
    },
    /// Generate or install shell completion scripts.
    Completion {
        #[command(subcommand)]
        command: CompletionCommand,
    },
    /// Debt management: report generation, INC listing, INC backfill, gate evaluation.
    Debt {
        #[command(flatten)]
        command: debt::DebtArgs,
    },
    // ── First-class facade commands (D2 shadow routing) ──────────────────────
    /// Show the current cycle snapshot and lease. Delegates to `cycle status`.
    Status {
        /// Cycle identifier.
        #[arg(long)]
        cycle: String,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Plan a new cycle from a goal. Delegates to `cycle start`.
    Plan {
        /// Display name used to derive the stable cycle identifier.
        #[arg(long)]
        name: String,
        /// Workflow path applied to the cycle.
        #[arg(long, value_enum)]
        path: Option<crate::cycle::CyclePathArg>,
        /// Git branch associated with the cycle.
        #[arg(long)]
        branch: Option<String>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Execute a capability. Delegates to `capability run`.
    Run {
        /// Capability name.
        #[arg(long)]
        name: String,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Plan a release (dry-run). Delegates to `release plan --dry-run`.
    Ship {
        /// Release tag.
        #[arg(long)]
        tag: String,
        /// Release cycle.
        #[arg(long)]
        cycle: Option<String>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Rebuild a cycle from ledger events (dry-run). Delegates to `cycle rebuild --dry-run`.
    Recover {
        /// Cycle identifier.
        #[arg(long)]
        cycle: String,
        /// Dry-run: validate the ledger state without persisting any changes.
        #[arg(long)]
        dry_run: bool,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
}

/// Completion subcommands; shell names are subcommands so
/// `sddk completion bash` keeps working while `sddk completion install`
/// adds installation.
#[derive(Debug, Subcommand)]
enum CompletionCommand {
    /// Print the bash completion script.
    Bash,
    /// Print the zsh completion script.
    Zsh,
    /// Print the fish completion script.
    Fish,
    /// Print the elvish completion script.
    Elvish,
    /// Print the powershell completion script.
    PowerShell,
    /// Install completions into the detected or requested shell.
    Install(CompletionInstallArgs),
}

#[derive(Debug, Clone, Args)]
struct CompletionInstallArgs {
    /// Target shell (default: detect from $SHELL).
    #[arg(long, value_enum)]
    shell: Option<CompletionShell>,
    /// Print target paths without writing any file.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Subcommand)]
enum ProjectCommand {
    /// Resolve identity without writing project or SDDK state.
    Resolve(ProjectResolveArgs),
}

#[derive(Debug, Subcommand)]
enum AdoptCommand {
    /// Preview identity, paths, and receipt without writing them.
    Plan(AdoptionArgs),
    /// Converge absent or matching partial adoption state.
    Apply(AdoptionArgs),
    /// Classify current receipt and SQLite registration state.
    Status(AdoptionArgs),
    /// Complete matching partial state without overwriting conflicts.
    Repair(AdoptionArgs),
    /// Converge runtime metadata without overwriting identity (sddk2-005).
    Refresh(AdoptionArgs),
}

#[derive(Debug, Clone, Args)]
struct ProjectResolveArgs {
    /// Checkout or worktree root.
    #[arg(long)]
    root: PathBuf,
    /// Required monorepo scope, using `.` for the repository root.
    #[arg(long)]
    scope: String,
    /// Explicit remote URL instead of read-only Git discovery.
    #[arg(long)]
    remote: Option<String>,
    /// Stable UUID for a repository without a remote.
    #[arg(long)]
    fallback_seed: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
struct AdoptionArgs {
    /// Checkout or worktree root.
    #[arg(long)]
    root: PathBuf,
    /// Required monorepo scope, using `.` for the repository root.
    #[arg(long)]
    scope: String,
    /// Explicit remote URL instead of read-only Git discovery.
    #[arg(long)]
    remote: Option<String>,
    /// Stable UUID for a repository without a remote.
    #[arg(long)]
    fallback_seed: Option<String>,
    /// Explicit RFC 3339 timestamp for deterministic execution.
    #[arg(long)]
    timestamp: Option<String>,
    /// Explicit actor for deterministic execution.
    #[arg(long)]
    actor: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, Subcommand)]
enum GenerateCommand {
    /// Render workflow metadata, tables, and Mermaid state diagram.
    Docs {
        /// Repository root.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Check generated output without writing it.
        #[arg(long)]
        check: bool,
        /// Write into the repo (docs/generated/) instead of XDG — dogfooding only.
        #[arg(long)]
        in_repo: bool,
    },
    /// Render a deterministic inventory of repository agents and skills.
    Inventory {
        /// Repository root.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Check generated output without writing it.
        #[arg(long)]
        check: bool,
        /// Write into the repo (docs/generated/) instead of XDG — dogfooding only.
        #[arg(long)]
        in_repo: bool,
    },
}

#[derive(Debug, Clone, Args)]
struct VersionArgs {
    /// Directory to resolve the version for (default: current dir).
    #[arg(long)]
    root: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

/// Shells supported by `sddk completion`.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    Elvish,
    PowerShell,
}

/// Captured process output and exit status.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CommandOutput {
    /// Process-style exit status.
    pub status: i32,
    /// Captured standard output.
    pub stdout: String,
    /// Captured standard error.
    pub stderr: String,
}

/// Process environment values used by CLI XDG and actor defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CliEnvironment {
    /// `HOME`, when set and non-empty.
    pub home: Option<PathBuf>,
    /// `XDG_DATA_HOME`, when set and non-empty.
    pub data_home: Option<PathBuf>,
    /// `SDDK_DATA_DIR`, when set and non-empty (takes precedence over data_home).
    pub sddk_data_dir: Option<PathBuf>,
    /// `XDG_STATE_HOME`, when set and non-empty.
    pub state_home: Option<PathBuf>,
    /// `XDG_CACHE_HOME`, when set and non-empty.
    pub cache_home: Option<PathBuf>,
    /// `SDDK_ACTOR`, when set and non-empty.
    pub sddk_actor: Option<String>,
    /// `USER`, when set and non-empty.
    pub user: Option<String>,
}

impl CliEnvironment {
    fn current() -> Self {
        Self {
            home: nonempty_env_path("HOME"),
            data_home: nonempty_env_path("XDG_DATA_HOME"),
            sddk_data_dir: nonempty_env_path("SDDK_DATA_DIR"),
            state_home: nonempty_env_path("XDG_STATE_HOME"),
            cache_home: nonempty_env_path("XDG_CACHE_HOME"),
            sddk_actor: nonempty_env_string("SDDK_ACTOR"),
            user: nonempty_env_string("USER"),
        }
    }

    fn xdg(&self) -> XdgEnvironment {
        XdgEnvironment {
            home: self.home.clone(),
            data_home: self.data_home.clone(),
            sddk_data_dir: self.sddk_data_dir.clone(),
            state_home: self.state_home.clone(),
            cache_home: self.cache_home.clone(),
        }
    }
}

/// Parses arguments and executes a command without terminating the process.
pub fn run_from<I, T>(args: I) -> CommandOutput
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match Cli::try_parse_from(args) {
        Ok(cli) => run_with_environment(cli, &CliEnvironment::current()),
        Err(error) => CommandOutput {
            status: error.exit_code(),
            stdout: String::new(),
            stderr: error.to_string(),
        },
    }
}

/// Executes an already parsed command and captures all output.
pub fn run(cli: Cli) -> CommandOutput {
    run_with_environment(cli, &CliEnvironment::current())
}

/// Executes an already parsed command with explicit process environment values.
pub fn run_with_environment(cli: Cli, environment: &CliEnvironment) -> CommandOutput {
    match cli.command {
        Command::Version(args) => run_version(args, environment),
        Command::Project {
            command: ProjectCommand::Resolve(args),
        } => run_project_resolve(args),
        Command::Adopt { command } => run_adopt(command, environment),
        Command::Lint { root, format } => match lint_repository(&root) {
            Ok(report) => {
                let status = i32::from(report.has_errors());
                let stdout = match format {
                    OutputFormat::Text => report.to_text(),
                    OutputFormat::Json => match serde_json::to_string_pretty(&report) {
                        Ok(json) => format!("{json}\n"),
                        Err(error) => {
                            return failure(format!("failed to serialize diagnostics: {error}"));
                        }
                    },
                };
                CommandOutput {
                    status,
                    stdout,
                    stderr: String::new(),
                }
            }
            Err(error) => failure(error.to_string()),
        },
        Command::Generate {
            command:
                GenerateCommand::Docs {
                    root,
                    check,
                    in_repo,
                },
        } => run_generation(
            generate_workflow_docs_dest(&root, environment, in_repo, check),
            GENERATED_WORKFLOW_DOC,
            "docs",
            &root,
        ),
        Command::Generate {
            command:
                GenerateCommand::Inventory {
                    root,
                    check,
                    in_repo,
                },
        } => run_generation(
            generate_inventory_dest(&root, environment, in_repo, check),
            GENERATED_INVENTORY_DOC,
            "inventory",
            &root,
        ),
        Command::Cycle { command } => cycle::run_cycle(command, environment),
        Command::Ledger { command } => ledger::run_ledger(command, environment),
        Command::Capability { command } => capability::run_capability(command, environment),
        Command::Git { command } => git_cmd::run_git(command, environment),
        Command::Artifact { command } => artifact::run_artifact(command, environment),
        Command::Permission { command } => permission::run_permission(command, environment),
        Command::Validate { command } => result_cmd::run_validate(command, environment),
        Command::AgentResult { command } => result_cmd::run_agent_result(command, environment),
        Command::Release { command } => release_cmd::run_release(command, environment),
        Command::Vault { command } => vault_cmd::run_vault(command, environment),
        Command::Knowledge { command } => knowledge_cmd::run_knowledge(command, environment),
        Command::Dev { command } => dev::run_dev(command, environment),
        Command::Pack { command } => pack_cmd::run_pack(command, environment),
        Command::Graph { command } => graph_cmd::run_graph(command, environment),
        Command::Metrics { command } => metrics::run_metrics(command, environment),
        Command::Analytics { command } => analytics::run_analytics(command, environment),
        Command::Telemetry { command } => telemetry::run_telemetry(command, environment),
        Command::Uat { command } => uat::run_uat(command, environment),
        Command::Approval { command } => approval::run_approval(command, environment),
        Command::Rules { command } => rules_cmd::run_rules(command, environment),
        Command::Stale { command } => stale_cmd::run_stale(command, environment),
        Command::Fork { command } => fork_cmd::run_fork(command, environment),
        Command::Explore { command } => explore_cmd::run_explore(command, environment),
        Command::Completion { command } => run_completion(command),
        Command::Debt { command } => debt::run_debt(command, environment),
        // ── First-class facade commands (D2 shadow routing) ───────────────
        Command::Status { cycle, format } => status::run_status(cycle, format, environment),
        Command::Plan {
            name,
            path,
            branch,
            format,
        } => plan::run_plan(name, path, branch, format, environment),
        Command::Run { name, format } => run::run_run(name, format, environment),
        Command::Ship { tag, cycle, format } => ship::run_ship(tag, cycle, format, environment),
        Command::Recover {
            cycle,
            dry_run,
            format,
        } => recover::run_recover(cycle, dry_run, format, environment),
    }
}

/// Renders or installs shell completion scripts for the `sddk` command line.
fn run_completion(command: CompletionCommand) -> CommandOutput {
    match command {
        CompletionCommand::Bash => completion_print(clap_complete::Shell::Bash),
        CompletionCommand::Zsh => completion_print(clap_complete::Shell::Zsh),
        CompletionCommand::Fish => completion_print(clap_complete::Shell::Fish),
        CompletionCommand::Elvish => completion_print(clap_complete::Shell::Elvish),
        CompletionCommand::PowerShell => completion_print(clap_complete::Shell::PowerShell),
        CompletionCommand::Install(args) => completion_install(args),
    }
}

/// Prints a completion script for a shell to stdout.
fn completion_print(shell: clap_complete::Shell) -> CommandOutput {
    let mut command = Cli::command();
    let mut stdout = Vec::new();
    clap_complete::generate(shell, &mut command, "sddk", &mut stdout);
    CommandOutput {
        stdout: String::from_utf8(stdout).unwrap_or_default(),
        ..CommandOutput::default()
    }
}

/// Resolves the completion target path for a shell given config/home dirs.
fn completion_install_path(
    shell: CompletionShell,
    xdg_config: &Path,
    home: &Path,
) -> Option<PathBuf> {
    match shell {
        CompletionShell::Fish => Some(xdg_config.join("fish/completions/sddk.fish")),
        CompletionShell::Bash => Some(home.join(".bash_completion.d/sddk.bash")),
        CompletionShell::Zsh => Some(home.join(".zfunc/_sddk")),
        CompletionShell::Elvish | CompletionShell::PowerShell => None,
    }
}

/// Activation hint printed after a successful install.
fn completion_hint(shell: CompletionShell) -> &'static str {
    match shell {
        CompletionShell::Bash => "add to ~/.bashrc: source ~/.bash_completion.d/sddk.bash",
        CompletionShell::Zsh => "add to ~/.zshrc: fpath=(~/.zfunc $fpath); compinit",
        CompletionShell::Fish => "already active for new fish sessions",
        CompletionShell::Elvish | CompletionShell::PowerShell => "",
    }
}

/// Installs the completion script for the detected or requested shell.
fn completion_install(args: CompletionInstallArgs) -> CommandOutput {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("HOME is not set"));
    let xdg_config = home.as_ref().ok().map(|home| {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"))
    });
    match (home, xdg_config) {
        (Ok(home), Some(xdg)) => match completion_install_to(args.shell, args.dry_run, &xdg, &home)
        {
            Ok(stdout) => CommandOutput {
                stdout: format!("{stdout}\n"),
                ..CommandOutput::default()
            },
            Err(error) => failure(error.to_string()),
        },
        (Err(error), _) => failure(error.to_string()),
        _ => failure("HOME is not set".to_string()),
    }
}

/// Pure install logic: resolves the shell, writes the script, prints hints.
fn completion_install_to(
    requested_shell: Option<CompletionShell>,
    dry_run: bool,
    xdg_config: &Path,
    home: &Path,
) -> anyhow::Result<String> {
    let shell = match requested_shell {
        Some(shell) => shell,
        None => {
            let shell = std::env::var("SHELL")
                .ok()
                .and_then(|value| {
                    PathBuf::from(value)
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                })
                .unwrap_or_default();
            match shell.as_str() {
                "bash" => CompletionShell::Bash,
                "zsh" => CompletionShell::Zsh,
                "fish" => CompletionShell::Fish,
                "elvish" => CompletionShell::Elvish,
                other => {
                    anyhow::bail!(
                        "cannot detect a supported shell from $SHELL ({other:?}); pass --shell"
                    )
                }
            }
        }
    };
    let path = completion_install_path(shell, xdg_config, home)
        .ok_or_else(|| anyhow::anyhow!("completion install is not supported for {shell:?}"))?;

    if dry_run {
        return Ok(format!(
            "dry-run: would write {}\n  {}",
            path.display(),
            completion_hint(shell)
        ));
    }

    let mut command = Cli::command();
    let mut script = Vec::new();
    clap_complete::generate(
        clap_complete::Shell::from(shell),
        &mut command,
        "sddk",
        &mut script,
    );
    atomic_write_path(&path, &script)?;
    Ok(format!(
        "installed: {}\n  {}",
        path.display(),
        completion_hint(shell)
    ))
}

impl From<CompletionShell> for clap_complete::Shell {
    fn from(shell: CompletionShell) -> Self {
        match shell {
            CompletionShell::Bash => clap_complete::Shell::Bash,
            CompletionShell::Zsh => clap_complete::Shell::Zsh,
            CompletionShell::Fish => clap_complete::Shell::Fish,
            CompletionShell::Elvish => clap_complete::Shell::Elvish,
            CompletionShell::PowerShell => clap_complete::Shell::PowerShell,
        }
    }
}

/// Writes bytes atomically via a temporary sibling file and rename.
fn atomic_write_path(destination: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    use std::io::Write;
    let parent = destination
        .parent()
        .ok_or_else(|| anyhow::anyhow!("destination has no parent: {destination:?}"))?;
    std::fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(
        ".{}.tmp-{}",
        destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("sddk"),
        std::process::id()
    ));
    let mut file = std::fs::File::create(&temporary)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    std::fs::rename(&temporary, destination)?;
    Ok(())
}

/// Resolve the generation destination: the repo (`docs/generated/`) only when
/// `--in-repo` (dogfooding); otherwise the project XDG `generated/` dir.
/// When the project identity cannot be resolved (not adopted, no remote), fall
/// back to in-repo so `generate` never fails on unadopted repos.
fn generation_destination(
    root: &Path,
    environment: &CliEnvironment,
    in_repo: bool,
) -> anyhow::Result<PathBuf> {
    if in_repo {
        return Ok(root.to_path_buf());
    }
    // Resolve the project identity to find its XDG generated dir. We reuse the
    // same resolution as adoption so the destination is stable per project.
    let remote = resolve_remote(root, None)?;
    let fallback_seed = if remote.is_none() {
        find_persisted_fallback_seed(environment, root, ".")?
    } else {
        None
    };
    let identity = match resolve_project_identity(remote.as_deref(), ".", fallback_seed.as_deref())
    {
        Ok(identity) => identity,
        Err(_) => return Ok(root.to_path_buf()), // not adopted → in-repo fallback
    };
    let workspace_id = stable_workspace_id(&identity.project_id, &path_string(root)?);
    let paths = sddk_engine::resolve_xdg_paths(
        &environment.xdg(),
        identity.project_id.as_str(),
        &workspace_id,
    )?;
    std::fs::create_dir_all(&paths.generated)?;
    Ok(paths.generated)
}

fn generate_workflow_docs_dest(
    root: &Path,
    environment: &CliEnvironment,
    in_repo: bool,
    check: bool,
) -> Result<GenerationStatus, docs::GenerationError> {
    let destination = generation_destination(root, environment, in_repo).map_err(|error| {
        docs::GenerationError::Io {
            path: root.to_path_buf(),
            source: std::io::Error::other(error.to_string()),
        }
    })?;
    if in_repo {
        generate_workflow_docs(root, check)
    } else {
        docs::generate_workflow_docs_to(root, destination, check)
    }
}

fn generate_inventory_dest(
    root: &Path,
    environment: &CliEnvironment,
    in_repo: bool,
    check: bool,
) -> Result<GenerationStatus, inventory::InventoryError> {
    let destination = generation_destination(root, environment, in_repo).map_err(|error| {
        inventory::InventoryError::Io {
            path: root.to_path_buf(),
            source: std::io::Error::other(error.to_string()),
        }
    })?;
    if in_repo {
        generate_inventory(root, check)
    } else {
        inventory::generate_inventory_to(root, destination, check)
    }
}

fn run_generation<E: std::fmt::Display>(
    result: Result<GenerationStatus, E>,
    generated_path: &str,
    command: &str,
    root: &Path,
) -> CommandOutput {
    match result {
        Ok(GenerationStatus::Current) => CommandOutput {
            stdout: format!("{generated_path} is current\n"),
            ..CommandOutput::default()
        },
        Ok(GenerationStatus::Written) => CommandOutput {
            stdout: format!("wrote {generated_path}\n"),
            ..CommandOutput::default()
        },
        Ok(GenerationStatus::Stale) => CommandOutput {
            status: 1,
            stderr: format!(
                "{generated_path} is missing or stale; run `sddk generate {command} --root {}`\n",
                root.display()
            ),
            ..CommandOutput::default()
        },
        Err(error) => failure(error.to_string()),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ProjectResolution {
    project_id: String,
    workspace_id: String,
    canonical_workspace_path: String,
    identity_source: IdentitySource,
    remote_url: Option<String>,
    scope: String,
    fallback_seed: Option<String>,
}

#[derive(Clone, Copy)]
enum AdoptionOperation {
    Plan,
    Apply,
    Status,
    Repair,
    Refresh,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct VersionResolution {
    /// CLI binary version.
    binary: String,
    /// Where the version was resolved from: `.sddk-versions` | `current` | `path:` | `none`.
    source: String,
    /// The resolved framework version or path target.
    resolved: String,
    /// Whether the target exists on disk.
    present: bool,
}

/// Resolves the framework version for a directory, asdf-style:
/// `$PWD/.sddk-versions` → parents → `$SDDK_DATA_DIR/framework/current`.
fn run_version(args: VersionArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<VersionResolution> {
        let start = args
            .root
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let start = if start.is_absolute() {
            start
        } else {
            std::env::current_dir()?.join(start)
        };

        // 1. Walk up from start looking for .sddk-versions.
        let mut dir: Option<&Path> = Some(start.as_path());
        let mut declared: Option<(String, String)> = None; // (value, file path)
        while let Some(current) = dir {
            let candidate = current.join(".sddk-versions");
            if candidate.is_file()
                && let Ok(content) = std::fs::read_to_string(&candidate)
                && let Some(value) = content.lines().find_map(|line| {
                    let line = line.trim();
                    line.strip_prefix("sddk ")
                        .or_else(|| line.strip_prefix("sddk\t"))
                })
            {
                declared = Some((
                    value.trim().to_owned(),
                    candidate.to_string_lossy().into_owned(),
                ));
                break;
            }
            dir = current.parent();
        }

        let binary = env!("CARGO_PKG_VERSION").to_owned();

        // 2. Resolve the declared value or fall back to `current`.
        let (source, resolved, present) = match declared {
            Some((value, file)) => {
                if let Some(path) = value.strip_prefix("path:") {
                    let path = path.to_owned();
                    let present = std::path::Path::new(&path).exists();
                    (format!(".sddk-versions ({file})"), path, present)
                } else if value == "current" || value == "system" {
                    match resolve_current(environment) {
                        Some((target, _)) => (format!(".sddk-versions ({file})"), target, true),
                        None => (format!(".sddk-versions ({file})"), value, false),
                    }
                } else {
                    // Version dir under the framework root.
                    let dir = sddk_framework_dir(environment)?;
                    let target = dir.join(&value);
                    let present = target.is_dir();
                    (
                        format!(".sddk-versions ({file})"),
                        target.to_string_lossy().into_owned(),
                        present,
                    )
                }
            }
            None => match resolve_current(environment) {
                Some((target, label)) => (label, target, true),
                None => ("none".to_owned(), "no version configured".to_owned(), false),
            },
        };

        Ok(VersionResolution {
            binary,
            source,
            resolved,
            present,
        })
    })();
    render_result(result, format, version_text)
}

/// Resolve `$SDDK_DATA_DIR/framework/current` to its target path.
fn resolve_current(environment: &CliEnvironment) -> Option<(String, String)> {
    let dir = sddk_framework_dir(environment).ok()?;
    let current = dir.join("current");
    let target = std::fs::read_link(&current).ok()?;
    let resolved = if target.is_absolute() {
        target
    } else {
        dir.join(target)
    };
    Some((
        resolved.to_string_lossy().into_owned(),
        "current".to_owned(),
    ))
}

/// `$SDDK_DATA_DIR/framework` — same resolution as `dev use`.
fn sddk_framework_dir(environment: &CliEnvironment) -> anyhow::Result<PathBuf> {
    let data_root = if let Some(dir) = &environment.sddk_data_dir {
        dir.clone()
    } else {
        let data_home = match (&environment.data_home, &environment.home) {
            (Some(data), _) => data.clone(),
            (None, Some(home)) => home.join(".local/share"),
            (None, None) => dirs::data_dir().ok_or_else(|| {
                anyhow::anyhow!("no data root: set HOME, XDG_DATA_HOME or SDDK_DATA_DIR")
            })?,
        };
        data_home.join("sddk")
    };
    Ok(data_root.join("framework"))
}

fn version_text(output: &VersionResolution) -> String {
    format!(
        "binary: {}\nsource: {}\nresolved: {}\npresent: {}\n",
        output.binary, output.source, output.resolved, output.present
    )
}

fn run_project_resolve(args: ProjectResolveArgs) -> CommandOutput {
    let result = (|| -> anyhow::Result<ProjectResolution> {
        let root = canonical_root(&args.root)?;
        let remote = resolve_remote(&root, args.remote)?;
        let fallback_seed = match (remote.as_ref(), args.fallback_seed) {
            (None, None) => Some(Uuid::new_v4().hyphenated().to_string()),
            (_, seed) => seed,
        };
        let identity =
            resolve_project_identity(remote.as_deref(), &args.scope, fallback_seed.as_deref())?;
        let canonical_workspace_path = path_string(&root)?;
        let workspace_id = stable_workspace_id(&identity.project_id, &canonical_workspace_path);
        Ok(ProjectResolution {
            project_id: identity.project_id.to_string(),
            workspace_id,
            canonical_workspace_path,
            identity_source: identity.identity_source,
            remote_url: identity.remote_url,
            scope: identity.scope,
            fallback_seed: identity.fallback_seed,
        })
    })();
    render_result(result, args.format, project_resolution_text)
}

fn run_adopt(command: AdoptCommand, environment: &CliEnvironment) -> CommandOutput {
    let (operation, args) = match command {
        AdoptCommand::Plan(args) => (AdoptionOperation::Plan, args),
        AdoptCommand::Apply(args) => (AdoptionOperation::Apply, args),
        AdoptCommand::Status(args) => (AdoptionOperation::Status, args),
        AdoptCommand::Repair(args) => (AdoptionOperation::Repair, args),
        AdoptCommand::Refresh(args) => (AdoptionOperation::Refresh, args),
    };
    let format = args.format;
    let result = (|| -> anyhow::Result<AdoptionCommandResult> {
        let plan = prepare_adoption_plan(args, operation, environment)?;
        let (mut storage, _plane) = compose(environment, &plan.paths.ledger)?;
        Ok(match operation {
            AdoptionOperation::Plan => AdoptionCommandResult::Plan(Box::new(plan)),
            AdoptionOperation::Apply => {
                let status = apply_adoption(&plan, &mut storage)?;
                AdoptionCommandResult::Status(Box::new(status))
            }
            AdoptionOperation::Status => {
                AdoptionCommandResult::Status(Box::new(adoption_status(&plan, &storage)?))
            }
            AdoptionOperation::Repair => {
                AdoptionCommandResult::Status(Box::new(repair_adoption(&plan, &mut storage)?))
            }
            AdoptionOperation::Refresh => {
                let status = sddk_engine::refresh_adoption(&plan, &mut storage)?;
                AdoptionCommandResult::Status(Box::new(status))
            }
        })
    })();
    match result {
        Ok(result) => {
            let status = match &result {
                AdoptionCommandResult::Plan(_) => 0,
                AdoptionCommandResult::Status(status) => {
                    i32::from(status.status != AdoptionStatusKind::Complete)
                }
            };
            match render(&result, format, adoption_result_text) {
                Ok(stdout) => CommandOutput {
                    status,
                    stdout,
                    stderr: String::new(),
                },
                Err(error) => failure(error.to_string()),
            }
        }
        Err(error) => failure(error.to_string()),
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum AdoptionCommandResult {
    Plan(Box<AdoptionPlan>),
    Status(Box<AdoptionStatus>),
}

fn prepare_adoption_plan(
    args: AdoptionArgs,
    operation: AdoptionOperation,
    environment: &CliEnvironment,
) -> anyhow::Result<AdoptionPlan> {
    let root = canonical_root(&args.root)?;
    let remote = resolve_remote(&root, args.remote)?;
    let mut fallback_seed = args.fallback_seed;
    if remote.is_none() && fallback_seed.is_none() {
        fallback_seed = find_persisted_fallback_seed(environment, &root, &args.scope)?;
    }
    if remote.is_none() && fallback_seed.is_none() {
        fallback_seed = match operation {
            AdoptionOperation::Plan | AdoptionOperation::Apply => {
                Some(Uuid::new_v4().hyphenated().to_string())
            }
            AdoptionOperation::Status | AdoptionOperation::Repair | AdoptionOperation::Refresh => {
                anyhow::bail!(
                    "fallback seed is required because no remote or matching adoption receipt exists"
                )
            }
        };
    }
    let display_name = root
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| anyhow::anyhow!("root has no UTF-8 display name: {root:?}"))?
        .to_owned();
    let timestamp = match args.timestamp {
        Some(timestamp) => timestamp,
        None => OffsetDateTime::now_utc().format(&Rfc3339)?,
    };
    let actor = args
        .actor
        .or_else(|| environment.sddk_actor.clone())
        .or_else(|| environment.user.clone())
        .unwrap_or_else(|| "sddk-cli".into());
    Ok(plan_adoption(AdoptionPlanInput {
        remote_url: remote,
        scope: args.scope,
        fallback_seed,
        canonical_workspace_path: root,
        display_name,
        xdg: environment.xdg(),
        sddk_version: "3.6".into(),
        runtime_version: env!("CARGO_PKG_VERSION").into(),
        timestamp,
        actor,
    })?)
}

pub(crate) fn find_persisted_fallback_seed(
    environment: &CliEnvironment,
    root: &Path,
    scope: &str,
) -> anyhow::Result<Option<String>> {
    let data_home = match (
        &environment.sddk_data_dir,
        &environment.data_home,
        &environment.home,
    ) {
        (Some(data), _, _) | (None, Some(data), _) => data.clone(),
        (None, None, Some(home)) => home.join(".local/share"),
        (None, None, None) => return Ok(None),
    };
    if !data_home.is_absolute() {
        anyhow::bail!("XDG_DATA_HOME must be absolute: {data_home:?}");
    }
    let projects = data_home.join("sddk/projects");
    if !projects.exists() {
        return Ok(None);
    }
    let root = path_string(root)?;
    let scope = normalize_scope(scope)?;
    let mut found = None;
    for entry in WalkDir::new(projects)
        .min_depth(4)
        .max_depth(4)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file() && entry.file_name() == "adoption.json")
    {
        let receipt = match read_adoption_receipt(entry.path()) {
            Ok(receipt) => receipt,
            Err(_) => continue,
        };
        if receipt.identity_source == IdentitySource::Fallback
            && receipt.canonical_workspace_path == root
            && receipt.scope == scope
        {
            if found.is_some() {
                anyhow::bail!("multiple fallback adoption receipts match this workspace");
            }
            found = receipt.fallback_seed;
        }
    }
    Ok(found)
}

pub(crate) fn resolve_remote(
    root: &Path,
    explicit: Option<String>,
) -> anyhow::Result<Option<String>> {
    if explicit.is_some() {
        return Ok(explicit);
    }
    let output = match ProcessCommand::new("git")
        .arg("-C")
        .arg(root)
        .args(["config", "--get", "remote.origin.url"])
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !output.status.success() {
        return Ok(None);
    }
    let remote = String::from_utf8(output.stdout)?.trim().to_owned();
    Ok((!remote.is_empty()).then_some(remote))
}

fn canonical_root(root: &Path) -> anyhow::Result<PathBuf> {
    let root = std::fs::canonicalize(root)?;
    if !root.is_dir() {
        anyhow::bail!("root is not a directory: {root:?}");
    }
    Ok(root)
}

fn project_resolution_text(resolution: &ProjectResolution) -> String {
    format!(
        "project_id: {}\nworkspace_id: {}\ncanonical_workspace_path: {}\nidentity_source: {}\nremote_url: {}\nscope: {}\nfallback_seed: {}\n",
        resolution.project_id,
        resolution.workspace_id,
        resolution.canonical_workspace_path,
        identity_source_text(resolution.identity_source),
        resolution.remote_url.as_deref().unwrap_or("null"),
        resolution.scope,
        resolution.fallback_seed.as_deref().unwrap_or("null")
    )
}

fn adoption_result_text(result: &AdoptionCommandResult) -> String {
    match result {
        AdoptionCommandResult::Plan(plan) => format!(
            "status: planned\nproject_id: {}\nworkspace_id: {}\nconfiguration_hash: {}\nvault: {}\nartifacts: {}\nledger: {}\ncache: {}\nreceipt: {}\n",
            plan.receipt.project_id,
            plan.receipt.workspace_id,
            plan.receipt.configuration_hash,
            plan.knowledge.vault_path.display(),
            plan.paths.artifacts.display(),
            plan.paths.ledger.display(),
            plan.paths.cache.display(),
            plan.paths.receipt.display()
        ),
        AdoptionCommandResult::Status(status) => format!(
            "status: {}\nproject_id: {}\nworkspace_id: {}\nreceipt: {}\nledger: {}\n{}",
            adoption_status_text(status.status),
            status.project_id,
            status.workspace_id,
            status.receipt_path.display(),
            status.ledger_path.display(),
            status
                .detail
                .as_ref()
                .map(|detail| format!("detail: {detail}\n"))
                .unwrap_or_default()
        ),
    }
}

fn identity_source_text(source: IdentitySource) -> &'static str {
    match source {
        IdentitySource::Remote => "remote",
        IdentitySource::Fallback => "fallback",
    }
}

fn adoption_status_text(status: AdoptionStatusKind) -> &'static str {
    match status {
        AdoptionStatusKind::Absent => "absent",
        AdoptionStatusKind::Complete => "complete",
        AdoptionStatusKind::ReceiptOnly => "receipt_only",
        AdoptionStatusKind::LedgerOnly => "ledger_only",
        AdoptionStatusKind::Conflict => "conflict",
        AdoptionStatusKind::Corrupt => "corrupt",
    }
}

fn render_result<T: Serialize>(
    result: anyhow::Result<T>,
    format: OutputFormat,
    text: fn(&T) -> String,
) -> CommandOutput {
    match result {
        Ok(value) => match render(&value, format, text) {
            Ok(stdout) => CommandOutput {
                stdout,
                ..CommandOutput::default()
            },
            Err(error) => failure(error.to_string()),
        },
        Err(error) => failure_envelope(&error),
    }
}

fn render<T: Serialize>(
    value: &T,
    format: OutputFormat,
    text: fn(&T) -> String,
) -> anyhow::Result<String> {
    match format {
        OutputFormat::Text => Ok(text(value)),
        OutputFormat::Json => Ok(format!("{}\n", serde_json::to_string_pretty(value)?)),
    }
}

fn path_string(path: &Path) -> anyhow::Result<String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| anyhow::anyhow!("path is not valid UTF-8: {path:?}"))
}

fn nonempty_env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn nonempty_env_string(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn failure(message: String) -> CommandOutput {
    CommandOutput {
        status: 1,
        stdout: String::new(),
        stderr: format!("error: {message}\n"),
    }
}

/// Renders a runtime error with the RNF-006 envelope when the concrete type
/// supports it: stable code, message, first cause, and a recovery hint.
pub(crate) fn failure_envelope(error: &anyhow::Error) -> CommandOutput {
    let envelope = error
        .downcast_ref::<sddk_domain::StorageError>()
        .map(|e| (e.code(), e.recovery()))
        .or_else(|| {
            // Storage inherent methods (e.g. `Storage::get_cycle`) still
            // return the concrete `sddk_storage::StorageError` type. Until
            // those signatures migrate to `sddk_domain::StorageError` via
            // the `impl Ledger for Storage` blanket, the runtime downcast
            // chain needs this fallback so error envelopes stay actionable.
            // Waived under ADR-0015 (ARCH003 composition-root waiver).
            error
                .downcast_ref::<sddk_storage::StorageError>()
                .map(|e| (e.code(), e.recovery()))
        })
        .or_else(|| {
            error
                .downcast_ref::<sddk_engine::EngineError>()
                .map(|e| (e.code(), e.recovery()))
        })
        .or_else(|| {
            error
                .downcast_ref::<sddk_gateway::GatewayError>()
                .map(|e| (e.code(), e.recovery()))
        })
        .or_else(|| {
            error
                .downcast_ref::<sddk_gateway::ReleaseError>()
                .map(|e| (e.code(), e.recovery()))
        });
    let Some((code, recovery)) = envelope else {
        return failure(error.to_string());
    };
    let mut stderr = format!("error[{code}]: {error}\n");
    if let Some(source) = error.source() {
        stderr.push_str(&format!("  cause: {source}\n"));
    }
    stderr.push_str(&format!("  recovery: {recovery}\n"));
    CommandOutput {
        status: 1,
        stdout: String::new(),
        stderr,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn completion_paths_are_shell_specific() {
        let home = PathBuf::from("/home/user");
        let xdg = PathBuf::from("/home/user/.config");
        assert_eq!(
            completion_install_path(CompletionShell::Fish, &xdg, &home),
            Some(PathBuf::from(
                "/home/user/.config/fish/completions/sddk.fish"
            ))
        );
        assert_eq!(
            completion_install_path(CompletionShell::Bash, &xdg, &home),
            Some(PathBuf::from("/home/user/.bash_completion.d/sddk.bash"))
        );
        assert_eq!(
            completion_install_path(CompletionShell::Zsh, &xdg, &home),
            Some(PathBuf::from("/home/user/.zfunc/_sddk"))
        );
        assert_eq!(
            completion_install_path(CompletionShell::Elvish, &xdg, &home),
            None
        );
        assert_eq!(
            completion_install_path(CompletionShell::PowerShell, &xdg, &home),
            None
        );
    }

    #[test]
    fn completion_install_dry_run_does_not_write() {
        let dir = tempfile::tempdir().unwrap();
        let xdg = dir.path().join("cfg");
        let out =
            completion_install_to(Some(CompletionShell::Fish), true, &xdg, dir.path()).unwrap();
        assert!(out.contains("dry-run: would write"));
        assert!(!xdg.join("fish/completions/sddk.fish").exists());
    }

    #[test]
    fn completion_install_writes_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let xdg = dir.path().join("cfg");
        let out =
            completion_install_to(Some(CompletionShell::Fish), false, &xdg, dir.path()).unwrap();
        assert!(out.contains("installed:"));
        let target = xdg.join("fish/completions/sddk.fish");
        assert!(target.exists());
        let content = std::fs::read_to_string(&target).unwrap();
        assert!(content.contains("_sddk"));
    }

    #[test]
    fn completion_print_still_works() {
        let output = run_from(["sddk", "completion", "bash"]);
        assert_eq!(output.status, 0);
        assert!(output.stdout.contains("_sddk"));
        let output = run_from(["sddk", "completion", "zsh"]);
        assert_eq!(output.status, 0);
        assert!(output.stdout.contains("#compdef"));
    }
}
