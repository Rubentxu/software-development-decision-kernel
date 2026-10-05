//! Testable command surface for the SDDK CLI.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(missing_docs)]

mod admission;
pub mod agent_profile;
pub mod agent_surface_golden;
mod analytics;
mod approval;
pub mod arg_schema;
mod artifact;
pub mod audit_cmd;
mod backlog;
mod capability;
pub mod change;
pub mod cheat_sheet;
pub mod command_spec;
pub mod command_surface;
pub mod config_cmd;
pub mod context_cmd;
pub mod cosign;
mod cycle;
mod debt;
pub mod dev;
pub mod examples_walker;
pub mod execution_receipt;
pub mod instruction_compiler;
pub mod skill_definition;
pub mod skill_registry_bridge;

mod architecture_cmd;
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
mod memory_cmd;
mod metrics;
mod pack_cmd;
mod permission;
mod plan;
pub mod project_alias;
mod recover;
mod release_cmd;
mod result_cmd;
mod rules_cmd;
mod run;
pub mod run_view;
mod ship;
mod stale_cmd;
mod status;
pub mod surface_integration;
mod target_cmd;
mod target_handlers;
mod telemetry;
mod uat;
mod uat_common;
mod uat_discover;
mod uat_enrich;
mod uat_generate;
mod uat_quality;
mod uat_serve;
mod vault_cmd;
pub mod verify_cmd;
pub mod verify_kernel_cmd;
mod why_cmd;
mod writer;

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
use memory_cmd::MemoryCommand;
use metrics::MetricsCommand;
use pack_cmd::PackCommand;
use permission::PermissionCommand;
use release_cmd::ReleaseCommand;
use result_cmd::{AgentResultCommand, ValidateCommand};
use rules_cmd::RulesCommand;
use sddk_domain::{
    IdentitySource, LedgerFactory, ProjectId, ResolvedProjectIdentity, SddkErrorCode,
    normalize_scope, resolve_project_identity, stable_workspace_id,
};
use sddk_storage::{SqliteControlPlane, SqliteLedgerFactory};

use sddk_engine::{
    AdoptionPlan, AdoptionPlanInput, AdoptionStatus, AdoptionStatusKind, XdgEnvironment,
    adoption_status, apply_adoption, plan_adoption, read_adoption_receipt, repair_adoption,
};
/// Re-exports `Storage` so that `cycle.rs` can use it via `crate::Storage`
/// without a direct `use sddk_storage::Storage` import (ARCH003 edge elimination).
pub use sddk_storage::Storage;

/// Re-exported alongside `Storage` because `emit_canonical_event` takes it, and
/// a type that is not reachable from the crate that re-exports its constructor's
/// argument forces every caller to add a second dependency for no reason.
pub use sddk_domain::LedgerEventInput;

/// Wall-clock milliseconds since the UNIX epoch, computed once at the
/// CLI composition root. Engine code MUST call this instead of
/// dereferencing `SystemTime::now()` directly so that production paths
/// and test paths can both be made deterministic (golden replays,
/// repro harnesses) by injecting a fixed value at this boundary.
/// Closes INC-DEBT-019 (CL-CC-01: hidden time-randomness coupling).
pub fn now_ms_since_epoch() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(i64::MAX)
}
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use vault_cmd::VaultCommand;
use walkdir::WalkDir;

pub use docs::{GENERATED_WORKFLOW_DOC, GenerationStatus, generate_workflow_docs};
pub use inventory::{GENERATED_INVENTORY_DOC, generate_inventory};
pub use lint::{
    Diagnostic, LintReport, Severity, lint_repository, validate_classifications_registry,
    validate_vault_export_routes_through_writer,
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
    about = "Deterministic SDDK workflow tooling — uses `sddk agent-help` for the operator-facing surface"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show the resolved framework version for the current directory.
    Version(VersionArgs),
    /// M7.1 — render the agent-facing command cheat sheet (SPEC-015).
    AgentHelp {
        #[command(subcommand)]
        command: HelpCommand,
    },
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
    /// Bootstrap durable context: identity, adoption convergence, cycle
    /// inference and session binding in one typed operation.
    Context {
        #[command(subcommand)]
        command: ContextCommand,
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
    /// Backlog ledger: capture, triage, list, and show items
    /// (cycle 2/4 — REQ-Backlog-Item-Capture,
    /// REQ-Backlog-Item-Triage-Priority).
    Backlog {
        #[command(subcommand)]
        command: backlog::BacklogCommand,
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
    /// Architecture conformance: emit the architecture-conformance receipt.
    Architecture {
        #[command(subcommand)]
        command: architecture_cmd::ArchitectureCommand,
    },
    /// Explain why SDDK claims something, from the substrate it claims it against.
    Why {
        #[command(subcommand)]
        command: why_cmd::WhyCommand,
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
    /// Decision memory: status, log, tree, show, diff, merge-base, reflog, audit (SPEC-004).
    Memory {
        #[command(subcommand)]
        command: MemoryCommand,
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
    /// Plan a new cycle from a goal (DEPRECATED: use `sddk plan workitem` instead).
    Plan {
        /// Subcommand (workitem, dep, evidence, decision, graph).
        /// When absent, falls back to legacy `cycle start` with deprecation warning.
        #[command(subcommand)]
        command: Option<plan::PlanCommand>,
        /// Runtime inference flags: --root, --scope, --no-infer, --remote, --fallback-seed.
        #[command(flatten)]
        runtime: crate::cycle::RuntimeArgs,
        /// Legacy: display name used to derive the stable cycle identifier.
        /// DEPRECATED — emits warning and delegates to `cycle start`.
        #[arg(long)]
        name: Option<String>,
        /// Legacy: workflow path applied to the cycle.
        #[arg(long, value_enum)]
        path: Option<crate::cycle::CyclePathArg>,
        /// Legacy: git branch associated with the cycle.
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
    /// View a workflow run (DEC-PLANE-001). Emits RunStateView and
    /// ActionSurfaceView.
    RunView {
        /// Run id.
        run_id: String,
        /// Build the view at a given event sequence (default: latest).
        #[arg(long)]
        as_of: Option<u64>,
        /// Policy name (default: `default`).
        #[arg(long, default_value = "default")]
        policy: String,
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
        /// How a release reference names a product version. See
        /// `release --naming`.
        #[arg(long, default_value = "v_prefixed")]
        naming: String,
        /// What this target is responsible for. See `release --role`.
        #[arg(long, default_value = "full_publisher")]
        role: String,
        /// Ask the build tool what its model says. See `release --evaluate-build`.
        #[arg(long)]
        evaluate_build: bool,
        /// The build tool to ask. See `release --build-tool`.
        ///
        /// Sin default, y por el motivo medido que esta en
        /// `ReleaseArgs::build_tool`: un nombre que SDDK no sepa preguntar es un
        /// error de la linea de comandos, no un Gradle silencioso.
        #[arg(long)]
        build_tool: Option<String>,
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
    /// Create a planning work item in a cycle (M6.1 facade). Delegates to `plan workitem create`.
    Change {
        /// Cycle identifier.
        #[arg(long)]
        cycle_id: String,
        /// Work item title.
        #[arg(long)]
        title: String,
        /// Work item description.
        #[arg(long)]
        description: String,
        /// Actor id.
        #[arg(long)]
        actor: Option<String>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Verify ledger continuity and capability policy snapshot (M6.1 facade).
    Verify {
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Generic Verify kernel: drive verification for a specific domain.
    VerifyKernel {
        #[command(flatten)]
        args: verify_kernel_cmd::VerifyArgs,
    },
    /// Audit memory reflog plus ledger events (M6.1 facade).
    Audit {
        /// Optional RFC3339 filter (currently a no-op — kept for forward compat).
        #[arg(long)]
        since: Option<String>,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Inspect configuration precedence chain (M6.1).
    Config {
        #[command(subcommand)]
        command: config_cmd::ConfigCommand,
    },
    /// Enumerate commands and their typed specs (M6.1).
    Introspect {
        #[command(subcommand)]
        command: command_spec::IntrospectCommand,
    },
    /// List and resolve Target/Task DAGs (M6.2).
    Target {
        #[command(subcommand)]
        command: target_cmd::TargetCommand,
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
    /// Pin this checkout to a project_id (`.sddk/project-pin.json`).
    Pin(ProjectPinArgs),
    /// Remove the project pin from this checkout.
    Unpin(ProjectUnpinArgs),
    /// Declare that a retired project_id resolves to another one (ADR-0152).
    ///
    /// Global and convergent, unlike the pin: the pin fixes ONE checkout and
    /// does not follow the project anywhere else, which is why re-adoption left
    /// 25 receipts pointing at ids that no longer resolve to anything.
    Alias(ProjectAliasArgs),
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

#[derive(Debug, Subcommand)]
enum ContextCommand {
    /// Resolve identity, converge adoption, infer the cycle, rebuild the
    /// context basis and bind the session (SPEC-005 CTX-003).
    Bootstrap(ContextBootstrapArgsCli),
    /// Publish or drain material context changes for a bound session
    /// (SPEC-005 CTX-008).
    Delta(ContextDeltaArgsCli),
    /// Expand one capsule reference of a bound session, reading its content
    /// from the ledger and recording the read (C3j objetivo 4, CTX-UAT-015).
    Expand(ContextExpandArgsCli),
}

#[derive(Debug, Args)]
struct ContextBootstrapArgsCli {
    /// Checkout or worktree root. Inferred from the current directory when absent.
    #[arg(long)]
    root: Option<PathBuf>,
    /// Required monorepo scope, using `.` for the repository root.
    #[arg(long)]
    scope: Option<String>,
    /// Stable host session identity (opaque to SDDK).
    #[arg(long)]
    session: String,
    /// Explicit cycle id, skipping inference.
    #[arg(long)]
    cycle: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct ContextExpandArgsCli {
    /// Checkout or worktree root. Inferred from the current directory when absent.
    #[arg(long)]
    root: Option<PathBuf>,
    /// Required monorepo scope, using `.` for the repository root.
    #[arg(long)]
    scope: Option<String>,
    /// Host session identity; must already have a durable cycle binding.
    #[arg(long)]
    session: String,
    /// Capsule reference to expand (`work-item:<id>`, bare decision id, or
    /// `cycle:<id>`), exactly as the capsule names it.
    #[arg(long = "ref")]
    r#ref: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, Args)]
struct ContextDeltaArgsCli {
    /// Checkout or worktree root. Inferred from the current directory when absent.
    #[arg(long)]
    root: Option<PathBuf>,
    /// Required monorepo scope, using `.` for the repository root.
    #[arg(long)]
    scope: Option<String>,
    /// Host session identity; must already have a durable binding.
    #[arg(long)]
    session: String,
    /// Publish a change to the session's durable delta stream.
    #[arg(long)]
    publish: bool,
    /// Semantic content to deliver. Repeatable.
    #[arg(long = "add")]
    add: Vec<String>,
    /// Semantic content that is no longer true. Repeatable.
    #[arg(long = "remove")]
    remove: Vec<String>,
    /// Why this change is relevant to the session (CDD-002).
    #[arg(long)]
    reason: Option<String>,
    /// The revision this delta produces. Required when publishing.
    #[arg(long)]
    to_revision: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
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

/// M7.1 — `sddk help agent` subcommand (SPEC-015 + ADR-014).
#[derive(Debug, Clone, Subcommand)]
pub enum HelpCommand {
    /// Render the agent-facing cheat sheet (text or JSON).
    Agent {
        /// Restrict the surface to a single target (e.g., "verify", "ship").
        #[arg(long)]
        target: Option<String>,
        /// Agent profile tag (default | read_only | approver).
        #[arg(long, default_value = "default")]
        profile: String,
        /// Include Deprecated commands (off by default).
        #[arg(long, default_value_t = false)]
        include_deprecated: bool,
        /// Include Experimental commands (off by default).
        #[arg(long, default_value_t = false)]
        include_experimental: bool,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
}

/// Output format selector for first-class CLI commands.
///
/// `Text` produces a human-friendly rendering with section headers;
/// `Json` produces a machine-readable envelope suitable for agents.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    /// Human-friendly text rendering.
    Text,
    /// Machine-readable JSON envelope.
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
    /// `SDDK_FRAMEWORK_DIR`, when set and non-empty (installer contract,
    /// ADR-0011; overrides the `framework/` dir inside the data root).
    pub framework_dir: Option<PathBuf>,
    /// `XDG_STATE_HOME`, when set and non-empty.
    pub state_home: Option<PathBuf>,
    /// `SDDK_STATE_HOME` override. Takes precedence over `XDG_STATE_HOME`
    /// and governs where the project ledger is opened
    /// (`crates/sddk-engine/src/paths.rs`). INC-DEBT-037.
    pub sddk_state_home: Option<PathBuf>,
    /// `XDG_CACHE_HOME`, when set and non-empty.
    pub cache_home: Option<PathBuf>,
    /// `SDDK_ACTOR`, when set and non-empty.
    pub sddk_actor: Option<String>,
    /// `USER`, when set and non-empty.
    pub user: Option<String>,
}

impl CliEnvironment {
    /// A fully-populated environment with no real host paths, so tests
    /// exercise isolation behaviour instead of inheriting the operator's
    /// `HOME`, `XDG_*` and `SDDK_*` values.
    pub fn for_test() -> Self {
        Self {
            home: None,
            data_home: None,
            sddk_data_dir: None,
            framework_dir: None,
            state_home: None,
            sddk_state_home: None,
            cache_home: None,
            sddk_actor: None,
            user: None,
        }
    }

    fn current() -> Self {
        Self {
            home: nonempty_env_path("HOME"),
            data_home: nonempty_env_path("XDG_DATA_HOME"),
            sddk_data_dir: nonempty_env_path("SDDK_DATA_DIR"),
            framework_dir: nonempty_env_path("SDDK_FRAMEWORK_DIR"),
            state_home: nonempty_env_path("XDG_STATE_HOME"),
            sddk_state_home: nonempty_env_path("SDDK_STATE_HOME"),
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
            sddk_state_home: self.sddk_state_home.clone(),
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
    // M7.7 — runtime admission gate. Wire the gate at the dispatch
    // boundary so every command execution passes through it before the
    // matched arm runs. Only three top-level commands carry
    // `required_skill` placeholders today (`lint`, `cycle`, `release`),
    // so we dispatch the gate by command-name, not by post-parse state.
    if let Some(blocked) = enforce_command_admission(&cli, environment) {
        return blocked;
    }
    match cli.command {
        Command::Version(args) => run_version(args, environment),
        Command::AgentHelp { command } => run_help(command),
        Command::Project { command } => match command {
            ProjectCommand::Resolve(args) => run_project_resolve(args),
            ProjectCommand::Pin(args) => run_project_pin(args),
            ProjectCommand::Unpin(args) => run_project_unpin(args),
            ProjectCommand::Alias(args) => run_project_alias(args),
        },
        Command::Adopt { command } => run_adopt(command, environment),
        Command::Context { command } => run_context(command, environment),
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
        Command::Backlog { command } => backlog::run_backlog(command, environment),
        Command::Knowledge { command } => knowledge_cmd::run_knowledge(command, environment),
        Command::Dev { command } => dev::run_dev(command, environment),
        Command::Pack { command } => pack_cmd::run_pack(command, environment),
        Command::Architecture { command } => {
            architecture_cmd::run_architecture(command, environment)
        }
        Command::Graph { command } => graph_cmd::run_graph(command, environment),
        Command::Why { command } => why_cmd::run_why(command, environment),
        Command::Metrics { command } => metrics::run_metrics(command, environment),
        Command::Analytics { command } => analytics::run_analytics(command, environment),
        Command::Telemetry { command } => telemetry::run_telemetry(command, environment),
        Command::Uat { command } => uat::run_uat(command, environment),
        Command::Approval { command } => approval::run_approval(command, environment),
        Command::Rules { command } => rules_cmd::run_rules(command, environment),
        Command::Stale { command } => stale_cmd::run_stale(command, environment),
        Command::Fork { command } => fork_cmd::run_fork(command, environment),
        Command::Memory { command } => memory_cmd::run_memory(command, environment),
        Command::Explore { command } => explore_cmd::run_explore(command, environment),
        Command::Completion { command } => run_completion(command),
        Command::Debt { command } => debt::run_debt(command, environment),
        // ── First-class facade commands (D2 shadow routing) ───────────────
        Command::Status { cycle, format } => status::run_status(cycle, format, environment),
        Command::Plan {
            command,
            runtime,
            name: _,
            path: _,
            branch: _,
            format: _,
        } => {
            if let Some(cmd) = command {
                plan::run_plan(cmd, &runtime, environment)
            } else {
                // No subcommand provided — show error
                crate::failure("sddk plan requires a subcommand: workitem, dep, evidence, decision, graph, or roadmap. Use 'sddk plan --help' for more information.".to_string())
            }
        }
        Command::Run { name, format } => run::run_run(name, format, environment),
        Command::RunView {
            run_id,
            as_of,
            policy,
            format,
        } => run_view::run_run_view(run_id, as_of, policy, format, environment),
        Command::Ship {
            tag,
            cycle,
            naming,
            role,
            evaluate_build,
            build_tool,
            format,
        } => {
            // La pregunta al build tool se VALIDA aqui, antes de construir nada.
            // Un nombre que SDDK no sepa preguntar es un error de la linea de
            // comandos y tiene que salir como tal: si llegara mas adentro se
            // reportaria como una fuente que no se pudo leer, que es otro hecho
            // con otra reparacion.
            let ask = match release_cmd::BuildAsk::of_parts(evaluate_build, build_tool.as_deref()) {
                Ok(ask) => ask,
                Err(error) => {
                    return CommandOutput {
                        status: 1,
                        stdout: String::new(),
                        stderr: format!("{error}"),
                    };
                }
            };
            ship::run_ship(tag, cycle, naming, role, ask, format, environment)
        }
        Command::Recover {
            cycle,
            dry_run,
            format,
        } => recover::run_recover(cycle, dry_run, format, environment),
        Command::Change {
            cycle_id,
            title,
            description,
            actor,
            format,
        } => change::run_change(cycle_id, title, description, actor, format, environment),
        Command::Verify { format } => verify_cmd::run_verify(format, environment),
        Command::VerifyKernel { args } => verify_kernel_cmd::run_verify(args, environment),
        Command::Audit { since, format } => audit_cmd::run_audit(since, format, environment),
        Command::Config { command } => config_cmd::run_config(command, environment),
        Command::Introspect { command } => command_spec::run_introspect(command, environment),
        Command::Target { command } => target_cmd::run_target(command, environment),
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
    //
    // That sentence was a third false claim of the same family, and the
    // falsifier found it while looking for a second resolver: it reused the
    // *shape* of the adoption resolution, not the resolution itself. It called
    // `resolve_project_identity` directly, so neither the pin nor the alias
    // applied. OBSERVED on a checkout with a declared alias
    // (`repro-c3d.sh`): `project resolve` reports the surviving id while
    // `sddk generate docs` writes to
    // `…/projects/<retired-id>/generated/docs/generated/` — generated
    // documentation for a project nobody reads any more. INC-DEBT-059.
    let remote = resolve_remote(root, None)?;
    // And a **pre-existing** defect, found while measuring the above and not
    // caused by it: `find_persisted_fallback_seed` only matched receipts whose
    // `identity_source` was `Fallback`, so a receipt written **under a pin** was
    // invisible to it. With no remote, that left the seed `None`, the
    // resolution failed, and `generate docs` silently fell back to in-repo on
    // every pinned remote-less checkout — the INC-DEBT-049 shape, in a
    // different function. The fix is one clause in the predicate, not a new
    // source of seeds: see the note on that function.
    let fallback_seed = if remote.is_none() {
        find_persisted_fallback_seed(environment, root, ".")?
    } else {
        None
    };
    let identity = match resolve_identity_honoring_pin(root, ".", remote, fallback_seed) {
        Ok(identity) => identity,
        // Unresolvable identity → in-repo fallback, so `generate` never fails on
        // a repo that genuinely has no project. That set is strictly smaller
        // than before: it no longer includes pinned remote-less checkouts,
        // which are perfectly resolvable.
        Err(_) => return Ok(root.to_path_buf()),
    };
    let canonical = path_string(root)?;
    let workspace_id = stable_workspace_id(&identity.project_id, &canonical);
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
    /// Ids the resolution was redirected from, in order (ADR-0152).
    ///
    /// Always present, empty when nothing redirected. It is emitted even in
    /// the quiet case because "this field is absent" and "nothing happened"
    /// are different claims: a consumer must be able to tell that a build
    /// which knows about aliases checked, and found none.
    alias_hops: Vec<String>,
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
/// Dispatcher for `sddk help agent` (SPEC-015 + ADR-014).
fn run_help(command: HelpCommand) -> CommandOutput {
    use crate::cheat_sheet::{render_json, render_text};
    use crate::command_spec::{AgentProfileTag, SurfaceFilter};
    use crate::command_surface::surface_with_filter;

    match command {
        HelpCommand::Agent {
            target,
            profile,
            include_deprecated,
            include_experimental,
            format,
        } => {
            let profile_tag = match profile.as_str() {
                "read_only" => AgentProfileTag::ReadOnly,
                "approver" => AgentProfileTag::Approver,
                _ => AgentProfileTag::Default,
            };
            let surface = surface_with_filter(SurfaceFilter {
                target,
                profile: profile_tag,
                include_deprecated,
                include_experimental,
                ..Default::default()
            });
            let stdout = match format {
                OutputFormat::Text => render_text(&surface),
                OutputFormat::Json => render_json(&surface),
            };
            CommandOutput {
                status: 0,
                stdout,
                stderr: String::new(),
            }
        }
    }
}

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

/// SDDK data root (the directory that holds `framework/` and `mode-index`).
///
/// `$SDDK_DATA_DIR` wins; otherwise `$XDG_DATA_HOME/sddk`; otherwise
/// `~/.local/share/sddk`.
pub(crate) fn sddk_data_root(environment: &CliEnvironment) -> anyhow::Result<PathBuf> {
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
    Ok(data_root)
}

/// `$SDDK_FRAMEWORK_DIR` → `$SDDK_DATA_DIR/framework` — same resolution as
/// `dev use` (INC-A5-FWDIR).
fn sddk_framework_dir(environment: &CliEnvironment) -> anyhow::Result<PathBuf> {
    // INC-A5-FWDIR: same precedence as `dev use` (dev/paths.rs): the explicit
    // `$SDDK_FRAMEWORK_DIR` override wins over the data-root derived path.
    if let Some(dir) = &environment.framework_dir {
        return Ok(dir.clone());
    }
    let data_root = sddk_data_root(environment)?;
    Ok(data_root.join("framework"))
}

fn version_text(output: &VersionResolution) -> String {
    format!(
        "binary: {}\nsource: {}\nresolved: {}\npresent: {}\n",
        output.binary, output.source, output.resolved, output.present
    )
}

/// The one place project identity is decided for every CLI surface.
///
/// A durable pin (`.sddk/project-pin.json`) wins over remote/seed derivation:
/// once an operator pins a `project_id`, a renamed or case-drifted remote can
/// no longer fork the ledger (W2c, agent-secretless report D2).
///
/// **Why this function exists.** The pin used to be honoured by only two of
/// five independent resolvers in this crate — `project resolve` and
/// `RuntimeContext::open`. The other three (`resolve_project_ids`, cycle
/// inference, `prepare_adoption_plan`) re-derived the identity from
/// remote/seed, so a pinned checkout still split across two `project_id`s:
/// `sddk cycle status` and `sddk adopt status` reported the unpinned one while
/// `sddk project resolve` reported the pinned one. The `ProjectPin` doc comment
/// claimed "every runtime context honors it"; that claim was false and nothing
/// tested it. See INC-DEBT-049.
///
/// Every resolver must go through here. A surface that legitimately needs a
/// different *output* shape (notably `project resolve`, which reports
/// `remote_url` alongside the id) may call the pin lookup directly, but it
/// must not re-derive the identity.
///
/// **This is also where an identity alias is applied** (ADR-0152), to the
/// pinned and the derived branch alike. Pin wins over derivation because the
/// pin is per-checkout; the alias runs after both because it is global and
/// convergent. The order is not interchangeable: applying the alias first
/// would let a pin re-introduce a retired id, and applying the pin after
/// would let a stale checkout opt out of the redirect entirely.
pub(crate) fn resolve_identity_honoring_pin(
    root: &Path,
    scope: &str,
    remote: Option<String>,
    fallback_seed: Option<String>,
) -> anyhow::Result<ResolvedProjectIdentity> {
    // The alias table lives in the XDG state base, resolved with the *ledger's*
    // precedence (INC-DEBT-037) via the engine's own `state_base`. A second
    // resolver here would let an alias redirect against a different ledger
    // than the one the runtime opens — a silent wrong-project redirect, the
    // exact class ADR-0152 closes.
    let state_base = sddk_engine::state_base(&CliEnvironment::current().xdg())?;
    let table = crate::project_alias::load_alias_table(&state_base)?;
    resolve_identity_honoring_pin_with(root, scope, remote, fallback_seed, &table)
}

/// The resolution itself, with the alias table supplied.
///
/// Split so the chain logic is testable without fabricating an XDG tree: the
/// outer function is two lines of environment reading, and every behaviour
/// that matters is in here.
///
/// **The alias applies to the pinned branch too, and that is deliberate.** A
/// pin names one checkout's id and does not converge: a checkout pinned before
/// the re-adoption keeps resolving to the pre-adoption id forever, which is
/// INC-DEBT-049. If the alias were applied only to derived identities, the one
/// case that most needs redirecting — a pinned, stale id — would be the one
/// case that could not reach it. The pin answers "which id is this checkout",
/// the alias answers "which project is that id"; a pin naming a retired id is
/// exactly the input the alias exists to correct.
pub(crate) fn resolve_identity_honoring_pin_with(
    root: &Path,
    scope: &str,
    remote: Option<String>,
    fallback_seed: Option<String>,
    table: &sddk_domain::identity::AliasTable,
) -> anyhow::Result<ResolvedProjectIdentity> {
    let derived = resolve_project_identity(remote.as_deref(), scope, fallback_seed.as_deref());
    let pin = load_project_pin(root)?;

    // Derive FIRST, then let the pin override the id. The other order — pin
    // short-circuit, derive never — is what forced `project resolve` to build
    // its own second resolver to recover the remote/seed it displays: the pin
    // branch used to throw them away. Reporting them from here keeps the pin's
    // meaning ("this checkout is X, whatever the remote says") without costing
    // the surface its display data, and it removes a resolver.
    let mut identity = match (derived, pin) {
        (Ok(mut d), Some(pin)) => {
            d.project_id = ProjectId::new(pin.project_id.clone())?;
            d.identity_source = IdentitySource::Pinned;
            d
        }
        (Ok(d), None) => d,
        // A pin still resolves when derivation cannot (no remote, no seed).
        // That case worked before and must keep working: the pin is exactly
        // what makes a checkout resolvable without a usable remote.
        (Err(_), Some(pin)) => ResolvedProjectIdentity {
            project_id: ProjectId::new(pin.project_id.clone())?,
            remote_url: None,
            scope: normalize_scope(scope)?,
            identity_source: IdentitySource::Pinned,
            fallback_seed: None,
            alias_hops: Vec::new(),
        },
        (Err(e), None) => return Err(e.into()),
    };

    // A cycle or an over-long chain is a hard error here, not a fallback to
    // the un-aliased id: silently resolving the pre-alias id would report a
    // project the operator no longer uses, which is the silent-redirect
    // failure ADR-0152 rule 2 forbids.
    let resolution = table
        .resolve(identity.project_id.clone())
        .map_err(|e| anyhow::anyhow!("project identity alias cannot be applied: {e}"))?;
    identity.alias_hops = resolution.hops;
    identity.project_id = resolution.project_id;
    Ok(identity)
}

/// Resolve `(project_id, workspace_id)` for a root. Single source shared by
/// `sddk project resolve` and `sddk config resolve`.
pub(crate) fn resolve_project_ids(
    root: &Path,
    scope: &str,
    explicit_remote: Option<String>,
    explicit_seed: Option<String>,
) -> anyhow::Result<(String, String)> {
    let root = canonical_root(root)?;
    let remote = resolve_remote(&root, explicit_remote)?;
    let fallback_seed = match (remote.as_ref(), explicit_seed) {
        // Derive from the canonical path: a random seed makes every call a
        // different project. INC-DEBT-028.
        (None, None) => Some(sddk_domain::stable_fallback_seed(&path_string(&root)?)),
        (_, seed) => seed,
    };
    // Goes through the canonical resolver so a pinned checkout reports the
    // pinned project here too. It used to call `resolve_project_identity`
    // directly and silently ignore the pin — INC-DEBT-049.
    let identity = resolve_identity_honoring_pin(&root, scope, remote, fallback_seed)?;
    let canonical_workspace_path = path_string(&root)?;
    let workspace_id = stable_workspace_id(&identity.project_id, &canonical_workspace_path);
    Ok((identity.project_id.to_string(), workspace_id))
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ProjectPinArgs {
    /// Checkout root holding `.sddk/project-pin.json`.
    #[arg(long)]
    pub(crate) root: PathBuf,
    /// The project_id to pin, as reported by `sddk project resolve`.
    #[arg(long)]
    pub(crate) project_id: String,
    /// Why the pin exists (remote renamed, case drift, monorepo split...).
    #[arg(long)]
    pub(crate) reason: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ProjectUnpinArgs {
    /// Checkout root holding `.sddk/project-pin.json`.
    #[arg(long)]
    pub(crate) root: PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ProjectAliasArgs {
    /// The retired project_id that should resolve elsewhere.
    #[arg(long)]
    pub(crate) from: String,
    /// The surviving project_id `from` should resolve to. Must already exist:
    /// an alias to an id with no ledger behind it redirects a project into
    /// empty state, which is worse than not redirecting.
    #[arg(long)]
    pub(crate) to: String,
    /// Why the alias exists. Mandatory, not optional.
    ///
    /// This table decides which ledger a project is read from. A redirect with
    /// no recorded reason is a redirect nobody can audit a year later, and
    /// `--reason` being optional is what would make that the path of least
    /// resistance. ADR-0152 rule 4.
    #[arg(long)]
    pub(crate) reason: String,
    /// Preview the declaration without writing it.
    #[arg(long)]
    pub(crate) dry_run: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Durable project pin: `.sddk/project-pin.json` in the checkout.
/// When present, every project-identity resolution honors it over
/// remote/seed derivation, so a renamed or case-drifted remote cannot fork
/// the ledger (agent-secretless report D2, work item W2c).
///
/// **This doc used to be a false claim, twice.** It first said "every runtime
/// context honors it" while only two of five independent resolvers did;
/// `adopt status`, `config resolve` and cycle inference silently re-derived
/// from the remote. Nothing tested the contract it asserted. The repair then
/// closed with "All resolvers now go through [`resolve_identity_honoring_pin`]" —
/// which was **also false**, for the same function it names earlier in the
/// paragraph: `adopt` re-derived a second time, and this time not for the pin
/// but for the alias, so `sddk adopt status` and `sddk context bootstrap`
/// reported a retired `project_id` and `adopt apply` wrote a second adoption
/// receipt under it. INC-DEBT-049, then INC-DEBT-059.
///
/// What is true now, and is enforced rather than asserted: the pin is applied
/// in exactly one function, `resolve_identity_honoring_pin_with`, and both
/// `adopt` and `context bootstrap` reach their identity only through it. The
/// engine no longer decides identity at all — it cannot, being filesystem-free,
/// so any identity it derived was systematically the pre-alias one. The pin
/// *itself* is read by `load_project_pin` below, which is why a malformed one
/// still fails closed: a checkout-local concern is validated where the
/// checkout-local file is read.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ProjectPin {
    pub(crate) schema_version: u32,
    pub(crate) project_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) reason: Option<String>,
    pub(crate) pinned_at: String,
}

pub(crate) const PROJECT_PIN_SCHEMA_VERSION: u32 = 1;

pub(crate) fn project_pin_path(root: &Path) -> PathBuf {
    root.join(".sddk").join("project-pin.json")
}

/// Reads and validates the pin for a root. Errors name the problem so a
/// stale pin fails loud instead of being silently ignored.
pub(crate) fn load_project_pin(root: &Path) -> anyhow::Result<Option<ProjectPin>> {
    let path = project_pin_path(root);
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", path.display()))?;
    let pin: ProjectPin = serde_json::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("invalid project pin {}: {e}", path.display()))?;
    if pin.schema_version != PROJECT_PIN_SCHEMA_VERSION {
        anyhow::bail!(
            "project pin {} has schema_version {}, this build accepts {}",
            path.display(),
            pin.schema_version,
            PROJECT_PIN_SCHEMA_VERSION
        );
    }
    if !pin.project_id.starts_with("p-") {
        anyhow::bail!(
            "project pin {} holds '{}', which is not a project_id (expected p-*)",
            path.display(),
            pin.project_id
        );
    }
    Ok(Some(pin))
}

fn run_project_pin(args: ProjectPinArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<String> {
        let root = canonical_root(&args.root)?;
        if !args.project_id.starts_with("p-") {
            anyhow::bail!(
                "--project-id '{}' is not a project_id (expected p-*); run `sddk project resolve` first",
                args.project_id
            );
        }
        let path = project_pin_path(&root);
        if let Some(existing) = load_project_pin(&root)?
            && existing.project_id != args.project_id
        {
            anyhow::bail!(
                "{} already pins {} (refusing to silently repin to {}); run `sddk project unpin` first if the move is intentional",
                path.display(),
                existing.project_id,
                args.project_id
            );
        }
        let pin = ProjectPin {
            schema_version: PROJECT_PIN_SCHEMA_VERSION,
            project_id: args.project_id.clone(),
            reason: args.reason.clone(),
            pinned_at: crate::git_cmd::default_timestamp(),
        };
        std::fs::create_dir_all(root.join(".sddk"))?;
        std::fs::write(&path, serde_json::to_string_pretty(&pin)? + "\n")?;
        Ok(format!(
            "project pinned: {}\npin file: {}\n",
            args.project_id,
            path.display()
        ))
    })();
    render_result(result, format, |text| text.to_string())
}

fn run_project_unpin(args: ProjectUnpinArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<String> {
        let root = canonical_root(&args.root)?;
        let path = project_pin_path(&root);
        if path.exists() {
            std::fs::remove_file(&path)?;
            Ok(format!("project pin removed: {}\n", path.display()))
        } else {
            Ok(format!("no project pin at {}\n", path.display()))
        }
    })();
    render_result(result, format, |text| text.to_string())
}

/// Declare one identity alias (ADR-0152).
///
/// The target must already own a ledger. That check is the difference between
/// "merge two histories" and "point this project at nothing": an alias to a
/// `project_id` with no state behind it resolves every command to an empty
/// ledger, and it would do so *successfully*. Failing at declaration is the
/// only place that mistake is still cheap.
fn run_project_alias(args: ProjectAliasArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<String> {
        let state_base = sddk_engine::state_base(&CliEnvironment::current().xdg())?;
        let path = crate::project_alias::project_aliases_path(&state_base);
        let alias = crate::project_alias::parse_alias(
            &args.from,
            &args.to,
            &args.reason,
            &crate::git_cmd::default_timestamp(),
        )?;

        let target_ledger = state_base
            .join("sddk")
            .join("projects")
            .join(&args.to)
            .join("ledger.sqlite");
        if !target_ledger.exists() {
            anyhow::bail!(
                "--to '{}' has no ledger at {}; an alias must point at a project that already exists, otherwise it redirects this project into empty state",
                args.to,
                target_ledger.display()
            );
        }

        // Preview against the current table so `--dry-run` reports the real
        // outcome, including the case where the alias closes a cycle.
        let table = crate::project_alias::load_alias_table_at(&path)?;
        let mut candidate = table.clone();
        crate::project_alias::declare_alias(&mut candidate, alias.clone())?;
        candidate.resolve(alias.from_id.clone()).map_err(|e| {
            anyhow::anyhow!(
                "--from {} would not resolve after this alias: {e}",
                args.from
            )
        })?;

        if args.dry_run {
            return Ok(format!(
                "would declare: {} -> {}\nreason: {}\nalias file: {}\nresolves to: {}\n",
                alias.from_id,
                alias.to_id,
                alias.reason,
                path.display(),
                candidate.resolve(alias.from_id.clone())?.project_id
            ));
        }

        crate::project_alias::declare_alias_at(&path, alias.clone())?;
        Ok(format!(
            "declared: {} -> {}\nreason: {}\nalias file: {}\n",
            alias.from_id,
            alias.to_id,
            alias.reason,
            path.display()
        ))
    })();
    render_result(result, format, |text| text.to_string())
}

fn run_project_resolve(args: ProjectResolveArgs) -> CommandOutput {
    let result = (|| -> anyhow::Result<ProjectResolution> {
        let state_base = sddk_engine::state_base(&CliEnvironment::current().xdg())?;
        let table = crate::project_alias::load_alias_table(&state_base)?;
        run_project_resolve_with(&args, &table)
    })();
    render_result(result, args.format, project_resolution_text)
}

/// The body of `sddk project resolve`, with the alias table supplied.
///
/// Split for the same reason as the resolver: the property that matters here is
/// that this surface goes through the **one** resolver, and a property you can
/// only exercise through a real `$XDG_STATE_HOME` is a property nobody tests.
/// The first falsification run proved it — reintroducing the two-resolver shape
/// here left the whole suite green.
fn run_project_resolve_with(
    args: &ProjectResolveArgs,
    table: &sddk_domain::identity::AliasTable,
) -> anyhow::Result<ProjectResolution> {
    let root = canonical_root(&args.root)?;
    let remote = resolve_remote(&root, args.remote.clone())?;
    let fallback_seed = match (remote.as_ref(), args.fallback_seed.clone()) {
        // Derive from the canonical path: a random seed makes every
        // invocation a different project, so the reported identity
        // drifts between commands. INC-DEBT-028.
        (None, None) => Some(sddk_domain::stable_fallback_seed(&path_string(&root)?)),
        (_, seed) => seed,
    };
    // One resolver, pin and alias both included. This function used to
    // branch on the pin itself and re-derive in the other branch, which is
    // a second identity resolver — the exact shape INC-DEBT-049 was about,
    // one level up, and it would have reported the pre-alias id here and
    // the post-alias id everywhere else.
    let identity =
        resolve_identity_honoring_pin_with(&root, &args.scope, remote, fallback_seed, table)?;
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
        alias_hops: identity
            .alias_hops
            .iter()
            .map(|id| id.to_string())
            .collect(),
    })
}

/// Dispatch `sddk context <subcommand>` (C3j objetivo 3).
fn run_context(command: ContextCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        ContextCommand::Bootstrap(args) => {
            let service_args = context_cmd::ContextBootstrapArgs {
                root: args.root,
                scope: args.scope,
                session: args.session,
                cycle: args.cycle,
                format: args.format,
                now_ms: now_ms_since_epoch(),
            };
            match context_cmd::bootstrap(&service_args, environment) {
                Ok(result) => match render(&result, service_args.format, context_bootstrap_text) {
                    Ok(stdout) => {
                        // A bootstrap that compiled no capsule leaves CTX-003
                        // step 5 unsatisfied. Identity, adoption and the binding
                        // were still resolved and rendered, so the payload is
                        // emitted in full — but the exit code must not claim
                        // success, or a consumer reading only the exit status
                        // proceeds on a capsule that does not exist
                        // (INC-DEBT-042).
                        let status = if result.status == "complete" { 0 } else { 4 };
                        CommandOutput {
                            status,
                            stdout,
                            stderr: String::new(),
                        }
                    }
                    Err(error) => failure(error.to_string()),
                },
                Err(error) => failure(error.to_string()),
            }
        }
        ContextCommand::Expand(args) => {
            let service_args = context_cmd::ContextExpandArgs {
                root: args.root,
                scope: args.scope,
                session: args.session,
                r#ref: args.r#ref,
                format: args.format,
            };
            match context_cmd::expand(&service_args, environment) {
                Ok(result) => match render(&result, service_args.format, context_expand_text) {
                    Ok(stdout) => CommandOutput {
                        status: 0,
                        stdout,
                        stderr: String::new(),
                    },
                    Err(error) => failure(error.to_string()),
                },
                Err(error) => failure(error.to_string()),
            }
        }
        ContextCommand::Delta(args) => {
            let service_args = context_cmd::ContextDeltaArgs {
                root: args.root,
                scope: args.scope,
                session: args.session,
                add: args.add,
                remove: args.remove,
                reason: args.reason,
                to_revision: args.to_revision,
                mode: if args.publish {
                    context_cmd::DeltaMode::Publish
                } else {
                    context_cmd::DeltaMode::Drain
                },
                format: args.format,
            };
            match context_cmd::delta(&service_args, environment) {
                Ok(result) => match render(&result, service_args.format, context_delta_text) {
                    Ok(stdout) => CommandOutput {
                        status: 0,
                        stdout,
                        stderr: String::new(),
                    },
                    Err(error) => failure(error.to_string()),
                },
                Err(error) => failure(error.to_string()),
            }
        }
    }
}

fn context_bootstrap_text(result: &context_cmd::ContextBootstrapResult) -> String {
    use context_cmd::BootstrapCycleState;
    let mut out = String::new();
    out.push_str(&format!("status: {}\n", result.status));
    out.push_str(&format!("project: {}\n", result.project_id));
    out.push_str(&format!("workspace: {}\n", result.workspace_id));
    out.push_str(&format!("adoption: {}\n", result.adoption));
    match &result.cycle {
        BootstrapCycleState::Resolved { cycle_id } => out.push_str(&format!("cycle: {cycle_id}\n")),
        BootstrapCycleState::Explicit { cycle_id } => {
            out.push_str(&format!("cycle: {cycle_id} (explicit)\n"));
        }
        BootstrapCycleState::NoActiveCycle { hint, .. } => {
            out.push_str(&format!("cycle: none ({hint})\n"));
        }
        BootstrapCycleState::Ambiguous { candidates, .. } => {
            let ids: Vec<&str> = candidates.iter().map(|c| c.cycle_id.as_str()).collect();
            out.push_str(&format!("cycle: ambiguous [{}]\n", ids.join(", ")));
        }
    }
    out.push_str(&format!("context: {}\n", result.context_source));
    if let Some(capsule_id) = &result.capsule_id {
        out.push_str(&format!("capsule: {capsule_id}\n"));
    }
    out.push_str(&format!("basis: {}\n", result.basis_revision));
    out.push_str(&format!(
        "binding: {} (written: {})\n",
        result.binding_ref, result.binding_written
    ));
    out
}

fn context_expand_text(result: &context_cmd::ContextExpandResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("status: {}\n", result.status));
    out.push_str(&format!("session: {}\n", result.session));
    out.push_str(&format!("ref: {}\n", result.r#ref));
    out.push_str(&format!("kind: {}\n", result.kind));
    out.push_str(&format!("capsule: {}\n", result.capsule_id));
    out.push_str(&format!("reads_recorded: {}\n", result.reads_recorded));
    out.push_str(&format!("content_sha256: {}\n", result.content_sha256));
    out.push_str("--- content ---\n");
    out.push_str(&result.content);
    out.push('\n');
    out
}

fn context_delta_text(result: &context_cmd::ContextDeltaResult) -> String {
    let mut out = String::new();
    out.push_str(&format!("status: {}\n", result.status));
    out.push_str(&format!("project: {}\n", result.project_id));
    out.push_str(&format!("session: {}\n", result.session));
    out.push_str(&format!("operation: {}\n", result.operation));
    out.push_str(&format!("basis: {}\n", result.basis_revision));
    out.push_str(&format!("last_seq: {}\n", result.last_seq));
    out.push_str(&format!("applied: {}\n", result.applied));
    out.push_str(&format!("rejected: {}\n", result.rejected.len()));
    for rejected in &result.rejected {
        out.push_str(&format!("  - seq {}: {}\n", rejected.seq, rejected.reason));
    }
    if !result.replay_skipped.is_empty() {
        out.push_str(&format!("skipped: {}\n", result.replay_skipped.join(", ")));
    }
    out.push_str(&format!("facts: {}\n", result.facts));
    out.push_str(&format!("advisory: {}\n", result.advisory));
    out
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
    // The pin is NOT read here. It used to be, so the engine could honour it —
    // and that arrangement left this path with no way to see the alias table,
    // so `adopt` re-derived the identity independently of the canonical
    // resolver and landed on the retired id (INC-DEBT-059). Both the pin and
    // the alias are now applied by the one resolver, below, in the one order
    // that is correct.
    let remote = resolve_remote(&root, args.remote)?;
    let mut fallback_seed = args.fallback_seed;
    if remote.is_none() && fallback_seed.is_none() {
        fallback_seed = find_persisted_fallback_seed(environment, &root, &args.scope)?;
    }
    if remote.is_none() && fallback_seed.is_none() {
        fallback_seed = match operation {
            AdoptionOperation::Plan | AdoptionOperation::Apply => {
                // Derive the seed from the canonical path instead of minting a
                // random UUID: a random seed writes a receipt that no later
                // invocation can rediscover, so the workspace immediately
                // reads back as "not adopted". INC-DEBT-028.
                let canonical = path_string(&root)?;
                Some(sddk_domain::stable_fallback_seed(&canonical))
            }
            AdoptionOperation::Status | AdoptionOperation::Repair | AdoptionOperation::Refresh => {
                // No remote and no persisted receipt, but the seed is now
                // derivable from the canonical path, so these read-only
                // operations resolve to the same identity `adopt apply` would
                // write. Bailing here is what made a freshly-adopted
                // workspace report "not adopted" on the next command.
                // INC-DEBT-028.
                let canonical = path_string(&root)?;
                Some(sddk_domain::stable_fallback_seed(&canonical))
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
    // The one decision point. The pin is applied over the derived id, and then
    // the alias over both branches — the order is not interchangeable, and
    // `resolve_identity_honoring_pin_with` is where that is enforced and
    // explained. A cycle or an over-long chain is a hard error here rather than
    // a silent fallback to the pre-alias id.
    let identity = resolve_identity_honoring_pin(&root, &args.scope, remote, fallback_seed)?;
    Ok(plan_adoption(AdoptionPlanInput {
        // Resolved once, by the resolver that `sddk project resolve` and
        // `sddk context bootstrap` also go through. `identity_source` and
        // `alias_hops` travel with it, so a checkout that reached its id
        // through a redirect still says so in `adopt status`.
        identity,
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
        // A `Pinned` receipt counts, and its absence here is what made
        // `sddk generate docs` fall back to in-repo on every pinned remote-less
        // checkout: the pin overwrites `identity_source`, so a receipt written
        // under one was invisible to this lookup, the seed came back `None`, the
        // resolution failed, and the caller swallowed it as "not adopted". The
        // seed is still there — the pin replaces the `project_id`, not the seed.
        // INC-DEBT-059.
        let seeded = matches!(
            receipt.identity_source,
            IdentitySource::Fallback | IdentitySource::Pinned
        );
        if seeded && receipt.canonical_workspace_path == root && receipt.scope == scope {
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
    // The alias is declared as an explicit arrow, not as a bare field, because
    // the whole point of ADR-0152 rule 4 is that an operator reading this can
    // see which id they stopped using. A `project_id:` line with a second id
    // somewhere above it is a fact; `from -> to` is a statement about a change.
    let alias = if resolution.alias_hops.is_empty() {
        "none".to_string()
    } else {
        format!(
            "{} -> {}",
            resolution.alias_hops.join(" -> "),
            resolution.project_id
        )
    };
    format!(
        "project_id: {}\nworkspace_id: {}\ncanonical_workspace_path: {}\nidentity_source: {}\nremote_url: {}\nscope: {}\nfallback_seed: {}\nidentity_alias: {}\n",
        resolution.project_id,
        resolution.workspace_id,
        resolution.canonical_workspace_path,
        identity_source_text(resolution.identity_source),
        resolution.remote_url.as_deref().unwrap_or("null"),
        resolution.scope,
        resolution.fallback_seed.as_deref().unwrap_or("null"),
        alias
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
            "status: {}\nproject_id: {}\nworkspace_id: {}\nreceipt: {}\nledger: {}\nidentity_alias: {}\n{}",
            adoption_status_text(status.status),
            status.project_id,
            status.workspace_id,
            status.receipt_path.display(),
            status.ledger_path.display(),
            // Criterion 3 of ADR-0152: the declaration names the `from` and the
            // `to`. The `to` is the `project_id` above; without this line the
            // `from` appeared nowhere, and a redirected identity read exactly
            // like one that was never redirected. `none` is a real answer, not
            // a missing one: `alias_origin()` returns `None` rather than
            // guessing when no alias applied.
            status
                .alias_origin
                .as_ref()
                .map(|from| format!("{from} -> {}", status.project_id))
                .unwrap_or_else(|| "none".to_string()),
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
        IdentitySource::Pinned => "pinned",
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

// ── M7.7 — Runtime admission gate ─────────────────────────────────────────────

/// Resolve the top-level command name from a parsed `Cli` so the
/// admission gate can look up the matching `CommandSpec`. Returns
/// `None` for commands that should bypass the gate (help, completion,
/// version, facade commands).
fn cli_top_level_name(cli: &Cli) -> Option<&'static str> {
    use Command::*;
    Some(match &cli.command {
        Version(_) => "version",
        // help always passes
        AgentHelp { .. } => return None,
        Project { .. } => "project",
        Adopt { .. } => "adopt",
        Context { .. } => "context",
        Lint { .. } => "lint",
        Generate { .. } => "generate",
        Cycle { .. } => "cycle",
        Architecture { .. } => "architecture",
        Why { .. } => "why",
        Ledger { .. } => "ledger",
        Capability { .. } => "capability",
        Git { .. } => "git",
        Artifact { .. } => "artifact",
        Permission { .. } => "permission",
        Validate { .. } => "validate",
        AgentResult { .. } => "agent-result",
        Release { .. } => "release",
        Vault { .. } => "vault",
        Backlog { .. } => "backlog",
        Knowledge { .. } => "knowledge",
        Dev { .. } => "dev",
        Pack { .. } => "pack",
        Graph { .. } => "graph",
        Metrics { .. } => "metrics",
        Analytics { .. } => "analytics",
        Telemetry { .. } => "telemetry",
        Uat { .. } => "uat",
        Approval { .. } => "approval",
        Rules { .. } => "rules",
        Stale { .. } => "stale",
        Fork { .. } => "fork",
        Memory { .. } => "memory",
        Explore { .. } => "explore",
        // shells always pass
        Completion { .. } => return None,
        Debt { .. } => "debt",
        // facade commands — bypass gate (route to canonical commands).
        Status { .. }
        | Plan { .. }
        | Run { .. }
        | RunView { .. }
        | Ship { .. }
        | Recover { .. }
        | Change { .. }
        | Verify { .. }
        | VerifyKernel { .. }
        | Audit { .. }
        | Config { .. }
        | Introspect { .. }
        | Target { .. } => return None,
    })
}

/// M7.7 — enforce runtime skill admission before dispatching to the
/// command's executor. Returns `Some(CommandOutput)` only when the gate
/// refuses execution (missing required skill that is not a known
/// placeholder). Help/completion/version always pass through.
fn enforce_command_admission(cli: &Cli, environment: &CliEnvironment) -> Option<CommandOutput> {
    let name = cli_top_level_name(cli)?;
    let spec = crate::command_spec::find_spec_by_name(name)?;
    if spec.required_skills.is_empty() {
        return None;
    }
    match crate::skill_registry_bridge::gate_command_for_environment(&spec, environment) {
        Ok(blocked) => blocked,
        Err(error) => {
            // Gate itself failed (IO, no framework installed, parse
            // error). Fail open with a stderr warning — blocking the
            // entire CLI on a missing skill bundle would be worse than
            // missing the audit.
            eprintln!(
                "sddk: skill admission gate unavailable ({error}); running {name} without gate."
            );
            None
        }
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

    /// INC-DEBT-049: `resolve_project_ids` (la vía que usa `config set`)
    /// rederivaba la identidad desde el remote e ignoraba el pin. Este es el
    /// tercer resolver roto; los dos primeros los cubre el e2e de pin.
    #[test]
    fn resolve_project_ids_honors_the_pin() {
        let root =
            std::env::temp_dir().join(format!("sddk-unit-pin-{}-{}", std::process::id(), line!()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let remote = Some("https://example.com/acme/repo.git".to_string());

        // Sin pin: el id viene del remote derivado.
        let (derived, _) = resolve_project_ids(&root, ".", remote.clone(), None).unwrap();
        assert!(derived.starts_with("p-"), "{derived}");
        assert_ne!(
            derived, "p-unithonorspin001",
            "el id derivado debe diferir del pin, si no el test no prueba nada"
        );

        // Con pin presente, el pinneado gana.
        std::fs::create_dir_all(root.join(".sddk")).unwrap();
        std::fs::write(
            root.join(".sddk/project-pin.json"),
            r#"{"schema_version":1,"project_id":"p-unithonorspin001","reason":"test","pinned_at":"2026-10-01T12:00:00Z"}"#,
        )
        .unwrap();
        let (pinned, _) = resolve_project_ids(&root, ".", remote, None).unwrap();
        assert_eq!(
            pinned, "p-unithonorspin001",
            "resolve_project_ids debe honourar el pin"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    // ── ADR-0152: el cableado del alias en el resolutor único ────────────
    //
    // Los tests del dominio (lote 1) y del store (lote 2) demuestran que la
    // tabla resuelve y que el fichero se escribe. Nada de eso demuestra que
    // el cableado exista: una tabla perfecta a la que nadie llama deja todo
    // verde. Estos tests atacan el enlace.

    mod alias_wiring {
        use super::*;
        use sddk_domain::identity::{AliasTable, ProjectAlias};

        fn pid(s: &str) -> sddk_domain::identity::ProjectId {
            sddk_domain::identity::ProjectId::new(s).unwrap()
        }

        fn alias(from: &str, to: &str) -> ProjectAlias {
            ProjectAlias::new(pid(from), pid(to), "test", "2026-10-02T00:00:00Z").unwrap()
        }

        fn table(entries: Vec<ProjectAlias>) -> AliasTable {
            AliasTable::new(entries)
        }

        /// The id a given remote derives to, so a test can alias *the derived
        /// id* without hardcoding a hash that would change if derivation did.
        fn derived_id(remote: &str) -> sddk_domain::identity::ProjectId {
            resolve_project_identity(Some(remote), ".", None)
                .unwrap()
                .project_id
        }

        const REMOTE: &str = "https://example.com/acme/alias-wiring.git";
        const SURVIVOR: &str = "p-aliaswiresurvivor01";
        const RETIRED: &str = "p-aliaswireretired01";

        #[test]
        fn an_alias_redirects_a_derived_identity() {
            let root = tempfile::tempdir().unwrap();
            let derived = derived_id(REMOTE);
            let t = table(vec![alias(&derived.to_string(), SURVIVOR)]);
            let identity =
                resolve_identity_honoring_pin_with(root.path(), ".", Some(REMOTE.into()), None, &t)
                    .unwrap();
            assert_eq!(identity.project_id, pid(SURVIVOR));
            assert_eq!(identity.alias_hops, vec![derived.clone()]);
            assert!(identity.redirected());
            assert_eq!(identity.alias_origin(), Some(&derived));
        }

        /// The case the alias exists for. A pin is per-checkout and does not
        /// converge, so a checkout pinned before the re-adoption resolves to a
        /// retired id forever. If the alias were applied only to derived
        /// identities, the one case that most needs redirecting could not
        /// reach it — and the pin would be a permanent escape hatch from the
        /// redirect.
        #[test]
        fn an_alias_redirects_a_pinned_retired_identity() {
            let root = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(root.path().join(".sddk")).unwrap();
            std::fs::write(
                root.path().join(".sddk/project-pin.json"),
                format!(
                    r#"{{"schema_version":1,"project_id":"{RETIRED}","reason":"pre-re-adoption pin","pinned_at":"2026-10-01T12:00:00Z"}}"#
                ),
            )
            .unwrap();
            let t = table(vec![alias(RETIRED, SURVIVOR)]);
            let identity =
                resolve_identity_honoring_pin_with(root.path(), ".", Some(REMOTE.into()), None, &t)
                    .unwrap();
            assert_eq!(identity.project_id, pid(SURVIVOR));
            assert_eq!(identity.identity_source, IdentitySource::Pinned);
            assert_eq!(identity.alias_hops, vec![pid(RETIRED)]);
        }

        #[test]
        fn an_unrelated_table_changes_nothing_and_declares_nothing() {
            let root = tempfile::tempdir().unwrap();
            let t = table(vec![alias(RETIRED, SURVIVOR)]);
            let identity =
                resolve_identity_honoring_pin_with(root.path(), ".", Some(REMOTE.into()), None, &t)
                    .unwrap();
            assert_eq!(identity.project_id, derived_id(REMOTE));
            assert!(identity.alias_hops.is_empty());
            assert!(!identity.redirected());
            assert_eq!(identity.alias_origin(), None);
        }

        #[test]
        fn an_empty_table_is_the_same_as_no_table() {
            let root = tempfile::tempdir().unwrap();
            let t = table(vec![]);
            let identity =
                resolve_identity_honoring_pin_with(root.path(), ".", Some(REMOTE.into()), None, &t)
                    .unwrap();
            assert_eq!(identity.project_id, derived_id(REMOTE));
            assert!(identity.alias_hops.is_empty());
        }

        /// Chains resolve transitively, and the hops record the WHOLE path, not
        /// just the first link: a two-step redirect that reported one hop would
        /// leave the middle id unexplained in the declaration.
        #[test]
        fn a_chain_resolves_and_records_every_hop() {
            let root = tempfile::tempdir().unwrap();
            let derived = derived_id(REMOTE);
            let middle = "p-aliaswiremiddle0001";
            let t = table(vec![
                alias(&derived.to_string(), middle),
                alias(middle, SURVIVOR),
            ]);
            let identity =
                resolve_identity_honoring_pin_with(root.path(), ".", Some(REMOTE.into()), None, &t)
                    .unwrap();
            assert_eq!(identity.project_id, pid(SURVIVOR));
            assert_eq!(identity.alias_hops, vec![derived, pid(middle)]);
        }

        /// The failure mode that must NOT be a silent success: a cyclic table
        /// makes the resolution non-terminating in meaning, so it has to fail
        /// loud. Falling back to the un-aliased id here would report a project
        /// the operator deliberately retired, with exit 0 and no warning —
        /// ADR-0152 rule 2.
        #[test]
        fn a_cycle_in_the_table_fails_loud_instead_of_returning_the_old_id() {
            let root = tempfile::tempdir().unwrap();
            let derived = derived_id(REMOTE);
            let t = table(vec![
                alias(&derived.to_string(), RETIRED),
                alias(RETIRED, &derived.to_string()),
            ]);
            let err =
                resolve_identity_honoring_pin_with(root.path(), ".", Some(REMOTE.into()), None, &t)
                    .expect_err("un ciclo debe fallar, no devolver el id pre-alias");
            let msg = format!("{err:#}");
            assert!(msg.contains("cannot be applied"), "{msg}");
            assert!(
                !msg.contains(&derived.to_string()) || msg.contains("cycle"),
                "el error debe nombrar el ciclo, no solo el id: {msg}"
            );
        }

        /// A pin must keep working when derivation cannot (no remote, no
        /// seed). That path existed before the alias and the refactor to
        /// derive-first must not have taken it away.
        #[test]
        fn a_pin_still_resolves_when_derivation_cannot() {
            let root = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(root.path().join(".sddk")).unwrap();
            std::fs::write(
                root.path().join(".sddk/project-pin.json"),
                format!(
                    r#"{{"schema_version":1,"project_id":"{SURVIVOR}","reason":"no remote","pinned_at":"2026-10-01T12:00:00Z"}}"#
                ),
            )
            .unwrap();
            let t = table(vec![]);
            let identity =
                resolve_identity_honoring_pin_with(root.path(), ".", None, None, &t).unwrap();
            assert_eq!(identity.project_id, pid(SURVIVOR));
            assert_eq!(identity.identity_source, IdentitySource::Pinned);
        }

        /// End-to-end through the `sddk project resolve` surface itself.
        ///
        /// **This test exists because a mutation escaped.** Reinstating the
        /// old two-resolver shape in `run_project_resolve` — a pin branch
        /// that returns the pin's id without ever consulting the alias — left
        /// every other test in the suite green. The resolver tests exercise
        /// the resolver; the render tests exercise the renderer; nothing
        /// exercised the seam between them, which is precisely the seam the
        /// defect lived in.
        #[test]
        fn project_resolve_reports_the_aliased_id_not_the_pinned_one() {
            let root = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(root.path().join(".sddk")).unwrap();
            std::fs::write(
                root.path().join(".sddk/project-pin.json"),
                format!(
                    r#"{{"schema_version":1,"project_id":"{RETIRED}","reason":"pre-re-adoption pin","pinned_at":"2026-10-01T12:00:00Z"}}"#
                ),
            )
            .unwrap();
            let t = table(vec![alias(RETIRED, SURVIVOR)]);
            let args = ProjectResolveArgs {
                root: root.path().to_path_buf(),
                scope: ".".into(),
                remote: Some(REMOTE.into()),
                fallback_seed: None,
                format: OutputFormat::Text,
            };
            let resolution = run_project_resolve_with(&args, &t).expect("must resolve");

            assert_eq!(
                resolution.project_id, SURVIVOR,
                "la superficie debe reportar el id POST-alias"
            );
            assert_eq!(
                resolution.alias_hops,
                vec![RETIRED.to_string()],
                "y declarar de dónde vino"
            );
            let text = project_resolution_text(&resolution);
            assert!(
                text.contains(&format!("identity_alias: {RETIRED} -> {SURVIVOR}")),
                "la salida visible debe llevar el salto: {text}"
            );
        }

        /// The complement: with no alias, `project resolve` reports the pin and
        /// says so. Without this, a test could "pass" by always printing an
        /// arrow.
        #[test]
        fn project_resolve_reports_the_pinned_id_when_no_alias_applies() {
            let root = tempfile::tempdir().unwrap();
            std::fs::create_dir_all(root.path().join(".sddk")).unwrap();
            std::fs::write(
                root.path().join(".sddk/project-pin.json"),
                format!(
                    r#"{{"schema_version":1,"project_id":"{SURVIVOR}","reason":"pin vigente","pinned_at":"2026-10-01T12:00:00Z"}}"#
                ),
            )
            .unwrap();
            let t = table(vec![]);
            let args = ProjectResolveArgs {
                root: root.path().to_path_buf(),
                scope: ".".into(),
                remote: Some(REMOTE.into()),
                fallback_seed: None,
                format: OutputFormat::Text,
            };
            let resolution = run_project_resolve_with(&args, &t).expect("must resolve");
            assert_eq!(resolution.project_id, SURVIVOR);
            assert!(resolution.alias_hops.is_empty());
            assert!(project_resolution_text(&resolution).contains("identity_alias: none"));
        }
    }

    /// ADR-0152 rule 4: the redirect is **declared**, not merely applied. A
    /// resolution that lands on a different project without saying so is the
    /// false green INC-DEBT-049 was about.
    #[test]
    fn the_resolve_output_declares_the_alias_it_followed() {
        let resolution = ProjectResolution {
            project_id: SURVIVOR_ID.to_string(),
            workspace_id: "ws-test".into(),
            canonical_workspace_path: "/tmp/x".into(),
            identity_source: IdentitySource::Pinned,
            remote_url: None,
            scope: ".".into(),
            fallback_seed: None,
            alias_hops: vec![RETIRED_ID.to_string()],
        };
        let text = project_resolution_text(&resolution);
        assert!(
            text.contains(&format!("identity_alias: {RETIRED_ID} -> {SURVIVOR_ID}")),
            "la salida debe declarar el salto: {text}"
        );
    }

    #[test]
    fn the_resolve_output_declares_none_when_nothing_redirected() {
        let resolution = ProjectResolution {
            project_id: SURVIVOR_ID.to_string(),
            workspace_id: "ws-test".into(),
            canonical_workspace_path: "/tmp/x".into(),
            identity_source: IdentitySource::Remote,
            remote_url: Some("https://example.com/a/b.git".into()),
            scope: ".".into(),
            fallback_seed: None,
            alias_hops: vec![],
        };
        let text = project_resolution_text(&resolution);
        assert!(
            text.contains("identity_alias: none"),
            "el caso quieto tambien se declara: {text}"
        );
    }

    const SURVIVOR_ID: &str = "p-declsurvivor0001";
    const RETIRED_ID: &str = "p-declretired00001";
}
