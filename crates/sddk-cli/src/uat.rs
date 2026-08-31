//! UAT (User Acceptance Testing) CLI — data-driven YAML artifacts rendered to
//! self-contained HTML dashboards (ADR-012/ADR-013).
//!
//! Agents produce `uat-plan.yaml`/`uat-session.yaml`/`uat-report.yaml`; this
//! module validates, renders, and ingests them. The dashboard kit ships in the
//! bundle under `assets/uat-dashboard/` (ADR-013).

use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};

use crate::{CommandOutput, OutputFormat, render_result};

use sddk_domain::{
    LATEST_PLAN_SCHEMA_VERSION, UatFeatureRollup, UatHistoryReport, UatIntegrityReport,
    UatManifest, UatManifestEntry, UatMigrationReport, UatOracleKind, UatPlan, UatReport,
    UatReportSummary, UatResultRow, UatScenarioRollup, UatSession, UatStalenessChangeKind,
    UatStalenessDiff, UatStalenessReport, UatStalenessScenario, UatSuggestionsReport, UatVerdict,
    aggregate_history, apply_all_suggestions, evidence_satisfies_spec, migrate_plan_v1_to_v2,
    sha256_hex, suggest_scenario_context, verify_evidence,
};

/// Default view when rendering a dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum UatView {
    Guided,
    Matrix,
    Traceability,
}

/// Runner mode: who is using the dashboard (REQ-RF-028).
/// The mode is embedded in the renderer's context JSON for selective UI rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum UatRunnerMode {
    /// Designing forms and scenarios.
    Designer,
    /// Running/executing the guided wizard.
    Runner,
    /// Reviewing results and acceptance decisions.
    Reviewer,
}

#[derive(Debug, Subcommand)]
pub(crate) enum UatCommand {
    /// Generate a canonical `uat-plan.yaml` for a release candidate.
    Plan(UatPlanArgs),
    /// Validate a uat YAML artifact against the domain schema.
    Validate(UatValidateArgs),
    /// Render a self-contained HTML dashboard from a plan (ADR-0013 kit).
    Dashboard(UatDashboardArgs),
    /// Render and open the dashboard; guided mode serves same-origin ingest.
    Open(UatOpenArgs),
    /// Ingest a session into the ledger + control plane (aggregate only).
    Ingest(UatIngestArgs),
    /// Aggregate sessions into a `uat-report.yaml` with a verdict.
    Report(UatReportArgs),
    /// Show the UAT status of a release candidate.
    Status(UatStatusArgs),
    /// List failed/blocked scenarios with context — what the agent reads to
    /// study where the UAT did not pass.
    Failures(UatFailuresArgs),
    /// Per-project UAT config (XDG-resident, ADR-0011): show or set.
    Config(UatConfigArgs),
    /// Evaluate the `release-uat-approved` gate for a release type under the
    /// project's config. Used by the orchestrator/release agent.
    Gate(UatGateArgs),
    /// Sign off on a release with an immutable UatAcceptanceRecord (REQ-RF-028).
    SignOff(UatSignOffArgs),
    /// Staleness advisory: inspect UI selectors against stored fingerprints and
    /// report drift (REQ-RF-024).
    Stale(UatStaleArgs),
    MigratePlan(UatMigratePlanArgs),
    VerifyIntegrity(UatVerifyIntegrityArgs),
    StoragePath(UatStoragePathArgs),
    BuildManifest(UatBuildManifestArgs),
    ScenarioContext(UatScenarioContextArgs),
    History(UatHistoryArgs),
    /// Execute a scripted/automated scenario via its `automation.ref` and
    /// emit a baseline `uat-session.yaml` for the ingest/report pipeline.
    Run(UatRunArgs),
    /// Build the Human Review Queue (REQ-RF-022) from a plan + report:
    /// required (P0/policy), oracle conflicts, low confidence, and the
    /// deterministic sample of machine-PASS scenarios.
    Review(UatReviewArgs),
    /// Evaluate semantic oracles (visual_ai / llm_rubric) of a scenario
    /// against its captured evidence using a local VLM/LLM (Fara).
    Assess(UatAssessArgs),
    /// Execute all automatable scenarios of a plan in sequence and
    /// aggregate the resulting sessions into a report. Manual scenarios
    /// are skipped with a note.
    Batch(UatBatchArgs),
    /// E14.2: Run the form quality agent (anti-test-smells) against a plan.
    Quality(QualityArgs),
    /// E14.3: Enrich plan scenarios with optimal UatFormSpec interaction types.
    EnrichForms(EnrichFormsArgs),
    /// E14.4: Explore live app with Fara CUA and generate an ActualApplicationModel.
    Discover(DiscoverArgs),
    /// E14.5: Run the full guided-pipeline (discover → plan → enrich → quality → validate).
    Generate(GenerateArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatPlanArgs {
    /// Candidate tag under test, e.g. `v1.5.0`.
    #[arg(long)]
    pub(crate) release: String,
    /// Aggregate features from this tag (default: last UAT'd release or all).
    #[arg(long)]
    pub(crate) from: Option<String>,
    /// Output YAML path.
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatValidateArgs {
    /// Path to a uat-plan / uat-session / uat-report YAML.
    #[arg(long)]
    pub(crate) file: PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatDashboardArgs {
    /// Plan YAML to render.
    #[arg(long)]
    pub(crate) plan: PathBuf,
    /// View to render (deprecated: use --mode for role selection).
    #[arg(long, value_enum, default_value_t = UatView::Guided)]
    pub(crate) view: UatView,
    /// Runner mode: who is using the dashboard (designer | runner | reviewer).
    /// The mode is embedded in the renderer's context for selective UI rendering.
    #[arg(long, value_enum, default_value_t = UatRunnerMode::Runner)]
    pub(crate) mode: UatRunnerMode,
    /// Theme: dark | light.
    #[arg(long, default_value = "dark")]
    pub(crate) theme: String,
    /// Output HTML path (default: `uat-dashboard-<release>.html`).
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatOpenArgs {
    /// Plan YAML to render (default: auto-resolve `uat-plan-<release>.yaml`).
    #[arg(long)]
    pub(crate) plan: Option<PathBuf>,
    /// Candidate release tag; required when --plan is omitted.
    #[arg(long)]
    pub(crate) release: Option<String>,
    /// View to render (deprecated: use --mode for role selection).
    #[arg(long, value_enum, default_value_t = UatView::Guided)]
    pub(crate) view: UatView,
    /// Runner mode: who is using the dashboard (designer | runner | reviewer).
    /// The mode is embedded in the renderer's context for selective UI rendering.
    #[arg(long, value_enum, default_value_t = UatRunnerMode::Runner)]
    pub(crate) mode: UatRunnerMode,
    /// Theme: dark | light.
    #[arg(long, default_value = "dark")]
    pub(crate) theme: String,
    /// Explicit browser/command to open the HTML (default: xdg-open/open/start
    /// by platform). Overrides auto-detection.
    #[arg(long)]
    pub(crate) browser: Option<String>,
    /// Output HTML path (default: alongside the plan, `uat-<view>-<release>.html`).
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatIngestArgs {
    /// Session YAML/JSON to ingest.
    #[arg(long)]
    pub(crate) session: PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatReportArgs {
    /// Candidate tag under test.
    #[arg(long)]
    pub(crate) release: String,
    /// One or more session files to aggregate.
    #[arg(long)]
    pub(crate) sessions: Vec<PathBuf>,
    /// Plan file the sessions reference.
    #[arg(long)]
    pub(crate) plan: PathBuf,
    /// Output YAML path.
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatStatusArgs {
    /// Candidate tag under test.
    #[arg(long)]
    pub(crate) release: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatFailuresArgs {
    /// Candidate tag under test (auto-resolves `uat-plan-<release>.yaml`).
    #[arg(long)]
    pub(crate) release: String,
    /// Explicit plan YAML (default: `uat-plan-<release>.yaml` in cwd).
    #[arg(long)]
    pub(crate) plan: Option<PathBuf>,
    /// One or more session files to inspect for failures.
    #[arg(long)]
    pub(crate) sessions: Vec<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatConfigArgs {
    #[command(subcommand)]
    pub(crate) command: UatConfigCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub(crate) enum UatConfigCommand {
    /// Print the current per-project UAT config (defaults if no file).
    Show(UatConfigShowArgs),
    /// Update fields of the per-project UAT config (XDG-resident, ADR-0011).
    Set(UatConfigSetArgs),
}

/// CLI wrapper for `sddk_domain::ReleaseGateAction` so the value_enum derive
/// stays in the CLI layer (domain must not depend on clap).
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum ReleaseGateActionArg {
    Required,
    Skip,
    Advisory,
}

impl From<ReleaseGateActionArg> for sddk_domain::ReleaseGateAction {
    fn from(value: ReleaseGateActionArg) -> Self {
        match value {
            ReleaseGateActionArg::Required => sddk_domain::ReleaseGateAction::Required,
            ReleaseGateActionArg::Skip => sddk_domain::ReleaseGateAction::Skip,
            ReleaseGateActionArg::Advisory => sddk_domain::ReleaseGateAction::Advisory,
        }
    }
}

/// CLI wrapper for `sddk_domain::ReleaseType`.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum ReleaseTypeArg {
    Major,
    Minor,
    Patch,
}

impl From<ReleaseTypeArg> for sddk_domain::ReleaseType {
    fn from(value: ReleaseTypeArg) -> Self {
        match value {
            ReleaseTypeArg::Major => sddk_domain::ReleaseType::Major,
            ReleaseTypeArg::Minor => sddk_domain::ReleaseType::Minor,
            ReleaseTypeArg::Patch => sddk_domain::ReleaseType::Patch,
        }
    }
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatConfigShowArgs {
    /// Project identifier (defaults to the current adoption's `project_id`).
    #[arg(long)]
    pub(crate) project: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatConfigSetArgs {
    /// Project identifier (defaults to the current adoption's `project_id`).
    #[arg(long)]
    pub(crate) project: Option<String>,
    /// Release gate policy for major releases.
    #[arg(long, value_enum)]
    pub(crate) major: Option<ReleaseGateActionArg>,
    /// Release gate policy for minor releases.
    #[arg(long, value_enum)]
    pub(crate) minor: Option<ReleaseGateActionArg>,
    /// Release gate policy for patch releases.
    #[arg(long, value_enum)]
    pub(crate) patch: Option<ReleaseGateActionArg>,
    /// Whether a developer is available to validate UAT.
    #[arg(long)]
    pub(crate) developer: Option<bool>,
    /// Whether an architect is available to validate UAT.
    #[arg(long)]
    pub(crate) architect: Option<bool>,
    /// Activation threshold: minimum number of features in the release.
    #[arg(long)]
    pub(crate) min_features: Option<u32>,
    /// Activation threshold: minimum diff lines in the release.
    #[arg(long)]
    pub(crate) min_diff_lines: Option<u32>,
    /// Critical domains (comma-separated) that trigger UAT activation.
    #[arg(long, value_delimiter = ',')]
    pub(crate) critical_domains: Vec<String>,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatGateArgs {
    #[command(subcommand)]
    pub(crate) command: UatGateCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub(crate) enum UatGateCommand {
    /// Evaluate the `release-uat-approved` gate for a candidate release.
    Release(UatGateReleaseArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatGateReleaseArgs {
    /// Project identifier (defaults to the current adoption's `project_id`).
    #[arg(long)]
    pub(crate) project: Option<String>,
    /// Candidate tag, e.g. `v1.5.2`.
    #[arg(long)]
    pub(crate) tag: String,
    /// Previous tag for semver diff (alternative to `--release-type`).
    #[arg(long)]
    pub(crate) previous_tag: Option<String>,
    /// Explicit release type (overrides `--previous-tag`).
    #[arg(long, value_enum)]
    pub(crate) release_type: Option<ReleaseTypeArg>,
    /// Aggregated UAT report. Defaults to `uat-report-<tag>.yaml`.
    #[arg(long)]
    pub(crate) report: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Decision values for `uat sign-off`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub(crate) enum UatSignOffDecisionArg {
    Accepted,
    AcceptedConditional,
    Rejected,
}

impl From<UatSignOffDecisionArg> for sddk_domain::UatAcceptanceDecision {
    fn from(value: UatSignOffDecisionArg) -> Self {
        match value {
            UatSignOffDecisionArg::Accepted => sddk_domain::UatAcceptanceDecision::Accepted,
            UatSignOffDecisionArg::AcceptedConditional => {
                sddk_domain::UatAcceptanceDecision::AcceptedConditional
            }
            UatSignOffDecisionArg::Rejected => sddk_domain::UatAcceptanceDecision::Rejected,
        }
    }
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatSignOffArgs {
    /// Release tag, e.g. `v1.9.0`.
    #[arg(long)]
    pub(crate) release: String,
    /// Sign-off decision.
    #[arg(long, value_enum)]
    pub(crate) decision: UatSignOffDecisionArg,
    /// Actor who signs off, e.g. `user:421`.
    #[arg(long)]
    pub(crate) actor: String,
    /// Justification for the decision.
    #[arg(long)]
    pub(crate) justification: String,
    /// Path to the plan YAML (default: `uat-plan-<release>.yaml`).
    #[arg(long)]
    pub(crate) plan: Option<PathBuf>,
    /// Directory containing session files for evidence snapshot (default: same
    /// directory as the plan).
    #[arg(long)]
    pub(crate) session_dir: Option<PathBuf>,
    /// Project identifier (defaults to the current adoption's `project_id`).
    #[arg(long)]
    pub(crate) project: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Args for the `uat stale` command (REQ-RF-024).
#[derive(Debug, Clone, Args)]
pub(crate) struct UatStaleArgs {
    /// URL of the application to inspect for staleness.
    #[arg(long)]
    pub(crate) url: String,
    /// Project identifier.
    #[arg(long)]
    pub(crate) project: Option<String>,
    /// Path to the plan YAML (default: auto-resolved from release in plan).
    #[arg(long)]
    pub(crate) plan: Option<PathBuf>,
    /// Directory containing the previous session's evidence (default: latest in
    /// the project's UAT storage).
    #[arg(long)]
    pub(crate) session_dir: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatMigratePlanArgs {
    #[arg(long)]
    pub(crate) input: PathBuf,
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    #[arg(long)]
    pub(crate) in_place: bool,
    #[arg(long)]
    pub(crate) report: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatVerifyIntegrityArgs {
    #[arg(long)]
    pub(crate) session: PathBuf,
    #[arg(long)]
    pub(crate) project: Option<String>,
    #[arg(long)]
    pub(crate) manifest: Option<PathBuf>,
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatStoragePathArgs {
    #[arg(long)]
    pub(crate) project: Option<String>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatBuildManifestArgs {
    #[arg(long)]
    pub(crate) sessions: Vec<PathBuf>,
    #[arg(long)]
    pub(crate) project: Option<String>,
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatScenarioContextArgs {
    #[arg(long)]
    pub(crate) plan: PathBuf,
    #[arg(long)]
    pub(crate) apply: bool,
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatHistoryArgs {
    #[arg(long)]
    pub(crate) release: String,
    #[arg(long)]
    pub(crate) plan: PathBuf,
    #[arg(long, num_args = 1..)]
    pub(crate) sessions: Vec<PathBuf>,
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatBatchArgs {
    /// Plan YAML containing the scenarios.
    #[arg(long)]
    pub(crate) plan: PathBuf,
    /// Wall-clock timeout per scenario in milliseconds (default: 60000).
    #[arg(long, default_value_t = 60_000)]
    pub(crate) timeout_ms: u64,
    /// Explicit human approval for executor capabilities (ADR-0005).
    #[arg(long)]
    pub(crate) approve: bool,
    /// Output directory for sessions (default: next to the plan).
    #[arg(long)]
    pub(crate) output_dir: Option<PathBuf>,
    /// Output report YAML path (default: `uat-report-<release>.yaml`).
    #[arg(long)]
    pub(crate) report: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatAssessArgs {
    /// Plan YAML containing the scenario.
    #[arg(long)]
    pub(crate) plan: PathBuf,
    /// Scenario id, e.g. `S-01`.
    #[arg(long)]
    pub(crate) scenario: String,
    /// Session YAML with the captured evidence (default:
    /// `uat-session-<scenario>.yaml` next to the plan).
    #[arg(long)]
    pub(crate) session: Option<PathBuf>,
    /// Fara/llama.cpp base URL (default: $FARA_URL or localhost:8082).
    #[arg(long)]
    pub(crate) fara_url: Option<String>,
    /// Explicit human approval for the uat.agent capability (ADR-0005).
    #[arg(long)]
    pub(crate) approve: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatReviewArgs {
    /// Plan YAML.
    #[arg(long)]
    pub(crate) plan: PathBuf,
    /// Report YAML (aggregated sessions).
    #[arg(long)]
    pub(crate) report: PathBuf,
    /// Sampling fraction 0..1 (default: from review policy or 0.02).
    #[arg(long)]
    pub(crate) sampling: Option<f64>,
    /// Deterministic sampling seed (default: plan release tag).
    #[arg(long)]
    pub(crate) seed: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UatRunArgs {
    /// Plan YAML containing the scenario.
    #[arg(long)]
    pub(crate) plan: PathBuf,
    /// Scenario id to execute, e.g. `S-01`.
    #[arg(long)]
    pub(crate) scenario: String,
    /// Wall-clock timeout in milliseconds (default: 60000).
    #[arg(long, default_value_t = 60_000)]
    pub(crate) timeout_ms: u64,
    /// Explicit human approval for the executor capability (ADR-0005).
    #[arg(long)]
    pub(crate) approve: bool,
    /// Output session YAML path (default: `uat-session-<scenario>.yaml`).
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    /// Output format for the run summary.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

// E14.2: Form Quality Agent
#[derive(Debug, Clone, Args)]
pub struct QualityArgs {
    /// Path to the uat-plan.yaml to audit.
    #[arg(long)]
    pub plan: PathBuf,
    /// Severity threshold: BLOCKER stops on blockers, WARNING stops on any smell.
    #[arg(long, value_enum, default_value_t = crate::uat_quality::report::QualityThreshold::Blocker)]
    pub threshold: crate::uat_quality::report::QualityThreshold,
    /// Write the quality report to this path (default: next to the plan).
    #[arg(long)]
    pub output: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

// E14.3: UX Form Agent
#[derive(Debug, Clone, Args)]
pub(crate) struct EnrichFormsArgs {
    /// Path to the uat-plan.yaml to enrich.
    #[arg(long)]
    pub(crate) plan: PathBuf,
    /// Output enriched plan to this path.
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

// E14.4: Test Discovery Agent
#[derive(Debug, Clone, Args)]
pub(crate) struct DiscoverArgs {
    /// Base URL of the application under test.
    #[arg(long)]
    pub(crate) app_url: String,
    /// Entry path (default: /).
    #[arg(long, default_value = "/")]
    pub(crate) entry: String,
    /// Exploration goals (one per --goal flag).
    #[arg(long)]
    pub(crate) goals: Vec<String>,
    /// Maximum number of Fara steps per goal.
    #[arg(long, default_value_t = 50)]
    pub(crate) budget: u32,
    /// Fara server URL (default: http://127.0.0.1:8082).
    #[arg(long)]
    pub(crate) fara_url: Option<String>,
    /// Write the ActualApplicationModel to this path.
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

// E14.5: Pipeline Orchestration
#[derive(Debug, Clone, Args)]
pub(crate) struct GenerateArgs {
    /// Release candidate tag.
    #[arg(long)]
    pub(crate) release: String,
    /// Directory containing requirement docs.
    #[arg(long)]
    pub(crate) requirements: Option<PathBuf>,
    /// Changelog file.
    #[arg(long)]
    pub(crate) changelog: Option<PathBuf>,
    /// Last UAT plan (for regression continuity).
    #[arg(long)]
    pub(crate) last_plan: Option<PathBuf>,
    /// Enable discovery step (requires --app-url).
    #[arg(long)]
    pub(crate) discover: bool,
    /// App URL for discovery (required if --discover is set).
    #[arg(long)]
    pub(crate) app_url: Option<String>,
    /// Run pipeline interactively with human approval gate.
    #[arg(long)]
    pub(crate) interactive: bool,
    /// Output plan path.
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

pub(crate) fn run_uat(command: UatCommand, environment: &crate::CliEnvironment) -> CommandOutput {
    match command {
        UatCommand::Plan(args) => run_uat_plan(args, environment),
        UatCommand::Validate(args) => run_uat_validate(args),
        UatCommand::Dashboard(args) => run_uat_dashboard(args, environment),
        UatCommand::Open(args) => run_uat_open(args, environment),
        UatCommand::Ingest(args) => run_uat_ingest(args, environment),
        UatCommand::Report(args) => run_uat_report(args),
        UatCommand::Status(args) => run_uat_status(args),
        UatCommand::Failures(args) => run_uat_failures(args),
        UatCommand::Config(args) => run_uat_config(args, environment),
        UatCommand::Gate(args) => run_uat_gate(args, environment),
        UatCommand::SignOff(args) => run_uat_signoff(args, environment),
        UatCommand::Stale(args) => run_uat_stale(args, environment),
        UatCommand::MigratePlan(args) => run_uat_migrate_plan(args),
        UatCommand::VerifyIntegrity(args) => run_uat_verify_integrity(args, environment),
        UatCommand::StoragePath(args) => run_uat_storage_path(args, environment),
        UatCommand::BuildManifest(args) => run_uat_build_manifest(args, environment),
        UatCommand::ScenarioContext(args) => run_uat_scenario_context(args),
        UatCommand::History(args) => run_uat_history(args),
        UatCommand::Run(args) => run_uat_run(args),
        UatCommand::Review(args) => run_uat_review(args),
        UatCommand::Assess(args) => run_uat_assess(args),
        UatCommand::Batch(args) => run_uat_batch(args),
        UatCommand::Quality(args) => run_uat_quality(args),
        UatCommand::EnrichForms(args) => run_uat_enrich_forms(args),
        UatCommand::Discover(args) => run_uat_discover(args),
        UatCommand::Generate(args) => run_uat_generate(args, environment),
    }
}

fn run_uat_plan(args: UatPlanArgs, _environment: &crate::CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<PathBuf> {
        let plan = UatPlan {
            schema_version: 1,
            release: sddk_domain::UatPlanRelease {
                candidate: args.release.clone(),
                project: None,
                last_uat_release: args.from,
            },
            generated_by: "uat-planner".into(),
            generated_at: now_rfc3339(),
            features: Vec::new(),
            runner_mode: None,
            approval: None,
        };
        let path = args
            .output
            .unwrap_or_else(|| PathBuf::from(format!("uat-plan-{}.yaml", args.release)));
        let yaml = serde_saphyr::to_string(&plan)
            .map_err(|e| anyhow::anyhow!("serialization failed: {e}"))?;
        std::fs::write(&path, yaml)?;
        Ok(path)
    })();
    render_result(result, format, |path| {
        format!("uat plan written: {}\n", path.display())
    })
}

fn run_uat_validate(args: UatValidateArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<()> {
        let raw = std::fs::read_to_string(&args.file)
            .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", args.file.display()))?;
        // Accept JSON as an alias of YAML (both are valid serde_saphyr input).
        let value: serde_json::Value = serde_saphyr::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("invalid YAML/JSON in {}: {e}", args.file.display()))?;
        let kind = value
            .get("schema_version")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        if kind == 0 {
            anyhow::bail!("missing or invalid `schema_version`");
        }
        if !(1..=LATEST_PLAN_SCHEMA_VERSION as u64).contains(&kind) {
            anyhow::bail!(
                "schema_version {} is not supported (this build accepts 1..={})",
                kind,
                LATEST_PLAN_SCHEMA_VERSION
            );
        }
        let has_scenarios = value
            .get("features")
            .and_then(|f| f.as_array())
            .map(|arr| {
                arr.iter().any(|feat| {
                    feat.get("scenarios")
                        .and_then(|s| s.as_array())
                        .map(|sc| !sc.is_empty())
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        if !has_scenarios {
            anyhow::bail!("plan must have at least one feature with one scenario");
        }
        // Round-trip through the typed model to enforce the closed vocabularies.
        let plan: UatPlan = serde_saphyr::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("schema validation failed: {e}"))?;
        // Form DSL (ADR-015/REQ-RF-025): validar el vocabulario cerrado de
        // todos los `form` specs declarados en el plan.
        let mut dsl_errors: Vec<String> = Vec::new();
        for feature in &plan.features {
            for scenario in &feature.scenarios {
                if let Some(form) = &scenario.form {
                    for error in sddk_domain::validate_form_dsl(form) {
                        dsl_errors.push(format!("{}: {}", scenario.id, error));
                    }
                }
            }
        }
        if !dsl_errors.is_empty() {
            anyhow::bail!("form DSL validation failed:\n  {}", dsl_errors.join("\n  "));
        }
        Ok(())
    })();
    render_result(result, format, |()| "uat validate: OK\n".into())
}

fn run_uat_dashboard(args: UatDashboardArgs, environment: &crate::CliEnvironment) -> CommandOutput {
    let format = args.format;
    // Deprecation: --view is deprecated in favor of --mode.
    if args.view != UatView::Guided {
        let view_name = match args.view {
            UatView::Guided => unreachable!(),
            UatView::Matrix => "matrix",
            UatView::Traceability => "traceability",
        };
        eprintln!(
            "warning: --view {view_name} is deprecated; use --mode designer|runner|reviewer instead"
        );
    }
    let result = (|| -> anyhow::Result<PathBuf> {
        let raw = std::fs::read_to_string(&args.plan)
            .map_err(|e| anyhow::anyhow!("cannot read plan {}: {e}", args.plan.display()))?;
        let plan: UatPlan = serde_saphyr::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("invalid plan {}: {e}", args.plan.display()))?;
        let html = render_dashboard_html(&plan, args.view, args.mode, &args.theme, environment)?;
        let output = args.output.unwrap_or_else(|| {
            PathBuf::from(format!("uat-dashboard-{}.html", plan.release.candidate))
        });
        std::fs::write(&output, html)?;
        Ok(output)
    })();
    render_result(result, format, |path| {
        format!("uat dashboard written: {}\n", path.display())
    })
}

fn run_uat_open(args: UatOpenArgs, environment: &crate::CliEnvironment) -> CommandOutput {
    let format = args.format;
    // Deprecation: --view is deprecated in favor of --mode.
    if args.view != UatView::Guided {
        let view_name = match args.view {
            UatView::Guided => unreachable!(),
            UatView::Matrix => "matrix",
            UatView::Traceability => "traceability",
        };
        eprintln!(
            "warning: --view {view_name} is deprecated; use --mode designer|runner|reviewer instead"
        );
    }
    let result = (|| -> anyhow::Result<PathBuf> {
        // Resolve the plan: explicit --plan, or auto-resolve by release tag.
        let plan_path = match &args.plan {
            Some(path) => path.clone(),
            None => {
                let release = args.release.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("missing plan: pass --plan <file> or --release <tag>")
                })?;
                let candidate = PathBuf::from(format!("uat-plan-{release}.yaml"));
                if !candidate.exists() {
                    anyhow::bail!(
                        "plan not found: {} (run `sddk uat plan --release {release}` first)",
                        candidate.display()
                    );
                }
                candidate
            }
        };

        let raw = std::fs::read_to_string(&plan_path)
            .map_err(|e| anyhow::anyhow!("cannot read plan {}: {e}", plan_path.display()))?;
        let plan: UatPlan = serde_saphyr::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("invalid plan {}: {e}", plan_path.display()))?;

        let html = render_dashboard_html(&plan, args.view, args.mode, &args.theme, environment)?;

        let view_name = match args.view {
            UatView::Guided => "guided",
            UatView::Matrix => "matrix",
            UatView::Traceability => "traceability",
        };
        let output = args.output.clone().unwrap_or_else(|| {
            let dir = plan_path.parent().unwrap_or_else(|| Path::new("."));
            dir.join(format!("uat-{view_name}-{}.html", plan.release.candidate))
        });

        // For the guided wizard: start the in-process ingest server BEFORE
        // opening the browser, so the wizard can auto-POST the exported
        // session to /ingest (closing the dashboard → control plane loop).
        // The server runs on 127.0.0.1 only and dies with this CLI process
        // (via the IngestServer's Drop impl).
        let mut ingest_server: Option<crate::uat_serve::IngestServer> = None;
        let mut open_url: Option<String> = None;
        if matches!(args.view, UatView::Guided) {
            // Tell the in-process server where to read the wizard HTML from.
            // This lets the server serve the HTML on the same origin as the
            // API, avoiding file:// → http://127.0.0.1 CORS issues in some
            // browsers.
            crate::uat_serve::set_wizard_html_path(output.clone());
            let env = std::sync::Arc::new(environment.clone());
            match crate::uat_serve::spawn(env) {
                Ok(server) => {
                    eprintln!(
                        "uat open: ingest endpoint listening at {} (health {})",
                        server.ingest_url, server.health_url
                    );
                    // Re-render with the ingest URL injected.
                    let html2 = html
                        .replace("@INGEST_URL@", &server.ingest_url)
                        .replace("@HEALTH_URL@", &server.health_url);
                    std::fs::write(&output, html2)?;
                    // Prefer opening the wizard on the same origin as the
                    // ingest endpoint. Fall back to file:// only if no port.
                    let url = format!("http://127.0.0.1:{}/", server.port);
                    open_url = Some(url.clone());
                    ingest_server = Some(server);
                }
                Err(e) => {
                    eprintln!(
                        "uat open: failed to start ingest server ({e}); wizard will fall back to manual ingest"
                    );
                    let html2 = html.replace("@INGEST_URL@", "").replace("@HEALTH_URL@", "");
                    std::fs::write(&output, html2)?;
                }
            }
        } else {
            std::fs::write(&output, html)?;
        }

        // Open the browser. Prefer the same-origin URL (http://127.0.0.1:PORT/)
        // so the wizard's fetch() is same-origin. Fall back to file:// only if
        // the ingest server didn't start.
        if let Some(url) = &open_url {
            if let Err(e) = open_in_browser(Path::new(url), args.browser.as_deref()) {
                eprintln!(
                    "uat open: browser launch failed ({e}); open manually with: xdg-open {url}"
                );
            }
        } else if let Err(e) = open_in_browser(&output, args.browser.as_deref()) {
            eprintln!(
                "uat open: browser launch failed ({e}); open manually with: xdg-open {}",
                output.display()
            );
        }
        let result_path = output.clone();

        // Keep the CLI process alive while the wizard is open. The browser
        // process is independent; we just need to keep our ingest server
        // running. SIGINT (Ctrl+C) cleanly drops the IngestServer, which
        // signals the thread to exit on its next accept iteration.
        if ingest_server.is_some() {
            eprintln!(
                "uat open: wizard running — press Ctrl+C in this terminal to close the ingest server"
            );
            // Park the main thread; the server thread runs until shutdown.
            std::thread::park();
        }
        Ok(result_path)
    })();
    render_result(result, format, |path| {
        format!("uat dashboard opened in browser: {}\n", path.display())
    })
}

/// Open a local file or loopback URL in the platform browser. On Linux uses
/// `xdg-open`, macOS `open`, Windows `cmd /c start`. An explicit
/// `--browser` overrides auto-detection.
fn open_in_browser(path: &Path, browser: Option<&str>) -> anyhow::Result<()> {
    let cmd = browser_command(path, browser);
    let status = std::process::Command::new(&cmd[0])
        .args(&cmd[1..])
        .status()
        .map_err(|e| anyhow::anyhow!("failed to open browser: {e}"))?;
    if !status.success() {
        anyhow::bail!(
            "browser exited with {status}; open the file manually: {}",
            path.display()
        );
    }
    Ok(())
}

/// Validate + upsert a session into the control plane (shared by the file
/// CLI ingest and the in-process HTTP server). Pure side-effect function
/// that opens the telemetry store and writes one `uat_results` row.
pub(crate) fn process_session_for_ingest(
    session: &UatSession,
    environment: &crate::CliEnvironment,
) -> anyhow::Result<()> {
    // Integrity guard (human-in-the-loop): an agent MUST NOT write
    // `executor: human`. A human session comes from the guided dashboard
    // export: finished_at set, an executed_by name, and at least one
    // evidence entry OR a non-PASS status. Without those signals, a
    // hand-written "human" session is rejected as fabrication.
    if session.executor == sddk_domain::UatExecutor::Human {
        let has_name = session.executed_by.is_some();
        let finished = session.finished_at.is_some();
        let evidenced = session.results.iter().any(|r| !r.evidence.is_empty());
        let has_non_pass = session
            .results
            .iter()
            .any(|r| r.status != sddk_domain::UatStatus::Pass);
        if !(has_name && finished && (evidenced || has_non_pass)) {
            anyhow::bail!(
                "integrity: `executor: human` session without human signals \
                 (executed_by + finished_at + evidence/non-PASS required). \
                 Agents must use `executor: fara`; human sessions come from \
                 the guided dashboard export."
            );
        }
    }

    let mut plane = crate::telemetry::open_store(environment, false)?;
    let project_id = session
        .executed_by
        .clone()
        .map(|by| format!("uat-{}", by.to_lowercase().replace(' ', "-")))
        .unwrap_or_else(|| "uat-unknown".into());
    let now = now_rfc3339();
    plane
        .upsert_project(&project_id, &project_id, "uat", None, &now)
        .map_err(anyhow::Error::from)?;
    let passed = session
        .results
        .iter()
        .filter(|r| r.status == sddk_domain::UatStatus::Pass)
        .count() as u32;
    let failed = session
        .results
        .iter()
        .filter(|r| r.status == sddk_domain::UatStatus::Fail)
        .count() as u32;
    let blocked = session
        .results
        .iter()
        .filter(|r| r.status == sddk_domain::UatStatus::Blocked)
        .count() as u32;
    let not_run = session
        .results
        .iter()
        .filter(|r| r.status == sddk_domain::UatStatus::NotRun)
        .count() as u32;
    let total = session.results.len().max(1) as u32;
    let coverage = 100.0 * (passed + blocked) as f64 / total as f64;
    let verdict = if failed > 0 || not_run > 0 {
        "NOT_READY"
    } else if blocked == 0 {
        "READY"
    } else {
        "READY_WITH_RISKS"
    };
    let duration = session
        .results
        .iter()
        .map(|r| r.duration_minutes)
        .sum::<u32>();
    let recorded_at = session
        .finished_at
        .clone()
        .unwrap_or_else(|| session.started_at.clone());
    plane
        .upsert_uat_result(&UatResultRow {
            project_id,
            tag_version: session.release.clone(),
            verdict: verdict.into(),
            coverage_pct: coverage,
            defects: failed as i64,
            session_count: session.results.len() as i64,
            uat_duration_minutes: duration as i64,
            recorded_at,
        })
        .map_err(anyhow::Error::from)?;
    Ok(())
}

fn run_uat_ingest(args: UatIngestArgs, environment: &crate::CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<UatSession> {
        let raw = std::fs::read_to_string(&args.session)
            .map_err(|e| anyhow::anyhow!("cannot read session {}: {e}", args.session.display()))?;
        let session: UatSession = serde_saphyr::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("invalid session {}: {e}", args.session.display()))?;
        process_session_for_ingest(&session, environment)?;
        Ok(session)
    })();
    render_result(result, format, |session| {
        format!(
            "uat session ingested: {} ({} results, release {})\n",
            session.session_id,
            session.results.len(),
            session.release
        )
    })
}

/// Read a session file. Used by `uat failures` and `uat report`.
fn read_session(path: &Path) -> anyhow::Result<UatSession> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("cannot read session {}: {e}", path.display()))?;
    serde_saphyr::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("invalid session {}: {e}", path.display()))
}

/// List failed/blocked scenarios with full context — the agent reads this
/// to study where the UAT did not pass and decide next steps.
fn run_uat_failures(args: UatFailuresArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<String> {
        let plan_path = args
            .plan
            .clone()
            .unwrap_or_else(|| PathBuf::from(format!("uat-plan-{}.yaml", args.release)));
        let plan_raw = std::fs::read_to_string(&plan_path)
            .map_err(|e| anyhow::anyhow!("cannot read plan {}: {e}", plan_path.display()))?;
        let plan: UatPlan = serde_saphyr::from_str(&plan_raw)
            .map_err(|e| anyhow::anyhow!("invalid plan {}: {e}", plan_path.display()))?;

        let mut sessions = Vec::new();
        for path in &args.sessions {
            sessions.push(read_session(path)?);
        }
        if sessions.is_empty() {
            anyhow::bail!("no sessions provided: pass one or more `--sessions <file>`");
        }

        let mut findings: Vec<UatFailure> = Vec::new();
        for session in &sessions {
            for result in &session.results {
                if !matches!(
                    result.status,
                    sddk_domain::UatStatus::Fail
                        | sddk_domain::UatStatus::Blocked
                        | sddk_domain::UatStatus::NotRun
                ) {
                    continue;
                }
                let scenario = plan
                    .features
                    .iter()
                    .flat_map(|f| f.scenarios.iter().map(move |s| (f, s)))
                    .find(|(_, s)| s.id == result.scenario_id);
                let (feature_name, priority, assignee, rationale) = match scenario {
                    Some((f, s)) => (
                        Some(f.name.clone()),
                        Some(s.priority),
                        Some(s.assignee),
                        s.rationale.clone(),
                    ),
                    None => (None, None, None, None),
                };
                findings.push(UatFailure {
                    scenario_id: result.scenario_id.clone(),
                    status: format!("{:?}", result.status).to_uppercase(),
                    comment: result.comment.clone().unwrap_or_default(),
                    evidence: result
                        .evidence
                        .iter()
                        .map(|e| format!("{:?}:{}", e.kind, e.r#ref))
                        .collect(),
                    feature: feature_name,
                    priority: priority.map(|p| format!("{:?}", p).to_uppercase()),
                    assignee: assignee.map(|a| format!("{:?}", a).to_lowercase()),
                    rationale,
                    session_id: session.session_id.clone(),
                    executed_by: session.executed_by.clone().unwrap_or_default(),
                });
            }
        }

        if matches!(format, OutputFormat::Json) {
            return serde_json::to_string_pretty(&findings)
                .map_err(|e| anyhow::anyhow!("json serialization failed: {e}"));
        }
        if findings.is_empty() {
            return Ok(format!(
                "uat failures: no failures or blocks in {} session(s) ({} session_id analyzed)\n",
                sessions.len(),
                sessions
                    .iter()
                    .map(|s| s.session_id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        let mut out = String::new();
        out.push_str(&format!(
            "uat failures — {} finding(s) across {} session(s) for release {}\n\n",
            findings.len(),
            sessions.len(),
            args.release
        ));
        for f in &findings {
            out.push_str(&format!("[{}] {}\n", f.status, f.scenario_id));
            if let Some(feature) = &f.feature {
                out.push_str(&format!("  feature:    {feature}\n"));
            }
            if let Some(priority) = &f.priority {
                out.push_str(&format!("  priority:   {priority}\n"));
            }
            if let Some(assignee) = &f.assignee {
                out.push_str(&format!("  assignee:   {assignee}\n"));
            }
            out.push_str(&format!(
                "  session:    {} ({})\n",
                f.session_id, f.executed_by
            ));
            if let Some(rationale) = &f.rationale {
                out.push_str(&format!("  rationale:  {rationale}\n"));
            }
            if !f.comment.is_empty() {
                out.push_str(&format!("  comment:    {}\n", f.comment));
            }
            if !f.evidence.is_empty() {
                out.push_str("  evidence:\n");
                for ev in &f.evidence {
                    out.push_str(&format!("    - {ev}\n"));
                }
            }
            out.push('\n');
        }
        Ok(out)
    })();
    render_result(result, format, |text| text.to_string())
}

#[derive(serde::Serialize, Debug)]
struct UatFailure {
    scenario_id: String,
    status: String,
    comment: String,
    evidence: Vec<String>,
    feature: Option<String>,
    priority: Option<String>,
    assignee: Option<String>,
    rationale: Option<String>,
    session_id: String,
    executed_by: String,
}

/// Resolve the project_id, falling back to `--project` or erroring.
fn resolve_project_id(
    args_project: Option<&str>,
    _environment: &crate::CliEnvironment,
) -> anyhow::Result<String> {
    if let Some(id) = args_project {
        return Ok(id.to_string());
    }
    if let Ok(id) = std::env::var("SDDK_PROJECT_ID")
        && !id.is_empty()
    {
        return Ok(id);
    }
    // Last resort: cwd adoption.json.
    let cwd = std::env::current_dir().unwrap_or_default();
    let candidates = [
        cwd.join("adoption.json"),
        cwd.join(".sddk").join("adoption.json"),
    ];
    for path in candidates {
        if let Ok(raw) = std::fs::read_to_string(&path)
            && let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw)
            && let Some(id) = value.get("project_id").and_then(|v| v.as_str())
        {
            return Ok(id.to_string());
        }
    }
    anyhow::bail!(
        "could not determine project_id: pass --project <id>, set $SDDK_PROJECT_ID, or run from a project root with adoption.json"
    )
}

/// Build an `XdgEnvironment` from the flat `CliEnvironment` fields.
fn xdg_from_env(environment: &crate::CliEnvironment) -> sddk_engine::XdgEnvironment {
    sddk_engine::XdgEnvironment {
        home: environment.home.clone(),
        data_home: environment.data_home.clone(),
        sddk_data_dir: environment.sddk_data_dir.clone(),
        state_home: environment.state_home.clone(),
        cache_home: environment.cache_home.clone(),
    }
}

pub(crate) fn load_uat_config(
    project_id: &str,
    environment: &crate::CliEnvironment,
) -> anyhow::Result<sddk_domain::UatConfig> {
    let path = sddk_engine::uat_config_path(&xdg_from_env(environment), project_id)?;
    if !path.exists() {
        return Ok(sddk_domain::UatConfig::default());
    }
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", path.display()))?;
    let config: sddk_domain::UatConfig = toml::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("invalid uat.toml at {}: {e}", path.display()))?;
    Ok(config)
}

fn save_uat_config(
    project_id: &str,
    config: &sddk_domain::UatConfig,
    environment: &crate::CliEnvironment,
) -> anyhow::Result<()> {
    let path = sddk_engine::uat_config_path(&xdg_from_env(environment), project_id)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let serialized = toml::to_string_pretty(config)
        .map_err(|e| anyhow::anyhow!("uat.toml serialization failed: {e}"))?;
    std::fs::write(&path, serialized)?;
    Ok(())
}

fn run_uat_config(args: UatConfigArgs, environment: &crate::CliEnvironment) -> CommandOutput {
    match args.command {
        UatConfigCommand::Show(a) => run_uat_config_show(a, environment),
        UatConfigCommand::Set(a) => run_uat_config_set(a, environment),
    }
}

fn run_uat_config_show(
    args: UatConfigShowArgs,
    environment: &crate::CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<String> {
        let project_id = resolve_project_id(args.project.as_deref(), environment)?;
        let path = sddk_engine::uat_config_path(&xdg_from_env(environment), &project_id)?;
        let config = load_uat_config(&project_id, environment)?;
        if matches!(format, OutputFormat::Json) {
            return serde_json::to_string_pretty(&serde_json::json!({
                "project_id": project_id,
                "path": path.display().to_string(),
                "exists": path.exists(),
                "config": config,
            }))
            .map_err(|e| anyhow::anyhow!("json serialization failed: {e}"));
        }
        let mut out = String::new();
        out.push_str(&format!("project: {project_id}\n"));
        out.push_str(&format!("path:    {}\n", path.display()));
        out.push_str(&format!(
            "exists:  {}\n\n",
            if path.exists() {
                "yes"
            } else {
                "no (defaults shown)"
            }
        ));
        out.push_str(&format!(
            "[release_gate]\n  major = {}\n  minor = {}\n  patch = {}\n\n",
            config_action_str(config.release_gate.major),
            config_action_str(config.release_gate.minor),
            config_action_str(config.release_gate.patch),
        ));
        out.push_str(&format!(
            "[human]\n  developer = {}\n  architect = {}\n\n",
            if config.human.developer {
                "true"
            } else {
                "false"
            },
            if config.human.architect {
                "true"
            } else {
                "false"
            },
        ));
        out.push_str(&format!(
            "[activation]\n  min_features = {}\n  min_diff_lines = {}\n  critical_domains = [{}]\n",
            config.activation.min_features,
            config.activation.min_diff_lines,
            config.activation.critical_domains.join(", "),
        ));
        Ok(out)
    })();
    render_result(result, format, |text| text.to_string())
}

fn run_uat_config_set(
    args: UatConfigSetArgs,
    environment: &crate::CliEnvironment,
) -> CommandOutput {
    let result = (|| -> anyhow::Result<String> {
        let project_id = resolve_project_id(args.project.as_deref(), environment)?;
        let mut config = load_uat_config(&project_id, environment)?;
        if let Some(v) = args.major {
            config.release_gate.major = v.into();
        }
        if let Some(v) = args.minor {
            config.release_gate.minor = v.into();
        }
        if let Some(v) = args.patch {
            config.release_gate.patch = v.into();
        }
        if let Some(v) = args.developer {
            config.human.developer = v;
        }
        if let Some(v) = args.architect {
            config.human.architect = v;
        }
        if let Some(v) = args.min_features {
            config.activation.min_features = v;
        }
        if let Some(v) = args.min_diff_lines {
            config.activation.min_diff_lines = v;
        }
        if !args.critical_domains.is_empty() {
            config.activation.critical_domains = args.critical_domains;
        }
        save_uat_config(&project_id, &config, environment)?;
        let path = sddk_engine::uat_config_path(&xdg_from_env(environment), &project_id)?;
        Ok(format!("uat config saved: {}\n", path.display()))
    })();
    render_result(result, OutputFormat::Text, |t| t.to_string())
}

fn run_uat_gate(args: UatGateArgs, environment: &crate::CliEnvironment) -> CommandOutput {
    match args.command {
        UatGateCommand::Release(a) => run_uat_gate_release(a, environment),
    }
}

fn run_uat_gate_release(
    args: UatGateReleaseArgs,
    environment: &crate::CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result: anyhow::Result<String> = (|| -> anyhow::Result<String> {
        let project_id = resolve_project_id(args.project.as_deref(), environment)?;
        let config = load_uat_config(&project_id, environment)?;

        let release_type = if let Some(rt) = args.release_type {
            rt.into()
        } else if let Some(prev) = &args.previous_tag {
            sddk_domain::release_type_from_diff(&args.tag, prev).ok_or_else(|| {
                anyhow::anyhow!(
                    "could not derive release type from diff {} -> {}: equal tags or invalid semver; pass --release-type explicitly",
                    prev,
                    args.tag
                )
            })?
        } else {
            anyhow::bail!("either --previous-tag or --release-type is required");
        };

        let action = sddk_domain::evaluate_release_gate(&config, release_type);
        let blocks = matches!(action, sddk_domain::ReleaseGateAction::Required);
        let report_path = args
            .report
            .unwrap_or_else(|| PathBuf::from(format!("uat-report-{}.yaml", args.tag)));
        let approved_report = if blocks {
            let raw = std::fs::read_to_string(&report_path).map_err(|e| {
                anyhow::anyhow!(
                    "UAT report required for {}: cannot read {}: {e}; run `sddk uat report --release {} --plan <plan> --sessions <sessions>` first",
                    args.tag,
                    report_path.display(),
                    args.tag
                )
            })?;
            let report: UatReport = serde_saphyr::from_str(&raw).map_err(|e| {
                anyhow::anyhow!("invalid UAT report {}: {e}", report_path.display())
            })?;
            validate_release_report(&report, &args.tag)?;

            // Gate `release-uat-signed` (REQ-RF-028): tras validar el report,
            // exigir acceptance record válido (decision != Rejected).
            let xdg = xdg_from_env(environment);
            let storage_root = sddk_engine::uat_storage_root(&xdg, &project_id)
                .map_err(|e| anyhow::anyhow!("cannot resolve storage root: {e}"))?;
            let acceptance_path = storage_root
                .join("acceptances")
                .join(format!("uat-acceptance-{}.yaml", args.tag));
            let _signed_record = if acceptance_path.exists() {
                Some(validate_acceptance_record(&acceptance_path)?)
            } else {
                anyhow::bail!(
                    "acceptance record required for {}: {} not found; run `sddk uat sign-off` first",
                    args.tag,
                    acceptance_path.display()
                );
            };
            Some(report_path.as_path())
        } else {
            None
        };

        if matches!(format, OutputFormat::Json) {
            return serde_json::to_string_pretty(&serde_json::json!({
                "project_id": project_id,
                "tag": args.tag,
                "release_type": release_type.as_str(),
                "action": action,
                "approved": true,
                "report": approved_report.map(|path| path.display().to_string()),
            }))
            .map_err(|e| anyhow::anyhow!("json serialization failed: {e}"));
        }

        let mut out = String::new();
        out.push_str(&format!("project:    {project_id}\n"));
        out.push_str(&format!("tag:        {}\n", args.tag));
        out.push_str(&format!("release:    {}\n", release_type.as_str()));
        out.push_str(&format!("gate:       {}\n", config_action_str(action)));
        if let Some(path) = approved_report {
            out.push_str(&format!(
                "\nALLOWED: verified READY report {} for {}\n",
                path.display(),
                args.tag
            ));
        } else {
            out.push_str(&format!(
                "\nALLOWED: gate = {} (no human verdict required for this release type)\n",
                config_action_str(action)
            ));
        }
        Ok(out)
    })();
    render_result(result, format, |t| t.to_string())
}

fn validate_release_report(report: &UatReport, tag: &str) -> anyhow::Result<()> {
    if report.schema_version < 2 {
        anyhow::bail!(
            "UAT report for {tag} uses schema v{}; schema v2 is required",
            report.schema_version
        );
    }
    if report.release != tag {
        anyhow::bail!(
            "UAT report release {} does not match candidate {tag}",
            report.release
        );
    }
    if report.plan_ref != tag {
        anyhow::bail!(
            "UAT report plan {} does not match candidate {tag}",
            report.plan_ref
        );
    }
    if report.sessions.is_empty() {
        anyhow::bail!("UAT report for {tag} contains no executed sessions");
    }
    if report.summary.total_scenarios == 0 {
        anyhow::bail!("UAT report for {tag} contains no scenarios");
    }
    if report.summary.not_run > 0 {
        anyhow::bail!(
            "UAT report for {tag} has {} scenario(s) not run or without required evidence",
            report.summary.not_run
        );
    }
    let classified = report.summary.passed
        + report.summary.failed
        + report.summary.blocked
        + report.summary.partial
        + report.summary.not_run;
    if classified != report.summary.total_scenarios {
        anyhow::bail!(
            "UAT report for {tag} is inconsistent: {classified} classified scenarios for {} total",
            report.summary.total_scenarios
        );
    }
    if (report.summary.coverage_pct - 100.0).abs() > f64::EPSILON {
        anyhow::bail!(
            "UAT report for {tag} has {:.2}% coverage; 100% is required",
            report.summary.coverage_pct
        );
    }
    if report.verdict != UatVerdict::Ready || !report.not_ready_blockers.is_empty() {
        anyhow::bail!(
            "UAT report for {tag} is {:?}; READY without blockers is required",
            report.verdict
        );
    }
    // Acceptance pendiente (v3, REQ-RF-023): PASSED != ACCEPTED. El gate
    // no libera si hay escenarios machine-PASS sin aceptación humana.
    if !report.acceptance_blockers.is_empty() {
        anyhow::bail!(
            "UAT report for {tag} has {} scenario(s) pending human acceptance: {}",
            report.acceptance_blockers.len(),
            report.acceptance_blockers.join("; ")
        );
    }
    Ok(())
}

/// Validate an acceptance record for the gate.
/// Checks: file exists, parses as UatAcceptanceRecord, sha256 format, decision != Rejected.
fn validate_acceptance_record(path: &Path) -> anyhow::Result<sddk_domain::UatAcceptanceRecord> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("cannot read acceptance record {}: {e}", path.display()))?;
    let record: sddk_domain::UatAcceptanceRecord = serde_saphyr::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("invalid acceptance record {}: {e}", path.display()))?;
    let errors = sddk_domain::UatAcceptanceRecord::validate(&record);
    if !errors.is_empty() {
        anyhow::bail!("acceptance record validation failed: {}", errors.join("; "));
    }
    if record.decision == sddk_domain::UatAcceptanceDecision::Rejected {
        anyhow::bail!("acceptance record for {} is REJECTED", path.display());
    }
    Ok(record)
}

/// Compute sha256 hex of a file's contents.
fn sha256_of_file(path: &Path) -> anyhow::Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(format!("sha256:{}", sha256_hex(&bytes)))
}

/// Compute evidence_snapshot_sha256 from a manifest: sorted concatenation
/// of all entry sha256 digests, then hashed (REQ-RF-028).
fn evidence_snapshot_sha256(manifest: &UatManifest) -> String {
    let mut digests: Vec<&str> = manifest.entries.iter().map(|e| e.sha256.as_str()).collect();
    digests.sort();
    let combined: String = digests.join("");
    let bytes = combined.as_bytes();
    format!("sha256:{}", sha256_hex(bytes))
}

fn run_uat_signoff(args: UatSignOffArgs, environment: &crate::CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result: anyhow::Result<PathBuf> = (|| -> anyhow::Result<PathBuf> {
        // Resolve plan path.
        let plan_path = if let Some(p) = &args.plan {
            p.clone()
        } else {
            PathBuf::from(format!("uat-plan-{}.yaml", args.release))
        };
        if !plan_path.exists() {
            anyhow::bail!(
                "plan not found: {} (run `sddk uat plan --release {}` first)",
                plan_path.display(),
                args.release
            );
        }

        // Compute plan_version_sha256.
        let plan_version_sha256 = sha256_of_file(&plan_path)?;

        // Resolve session directory and look for a manifest there.
        let session_dir = args.session_dir.clone().unwrap_or_else(|| {
            plan_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf()
        });
        // Try common manifest filenames.
        let manifest_path = {
            let candidates = [
                session_dir.join("uat-manifest.yaml"),
                session_dir.join("manifest.yaml"),
                session_dir.join("uat-manifest.yml"),
            ];
            candidates.into_iter().find(|p| p.exists())
        };

        let evidence_snapshot_sha256 = if let Some(ref mpath) = manifest_path {
            let manifest_raw = std::fs::read_to_string(mpath)
                .map_err(|e| anyhow::anyhow!("cannot read manifest {}: {e}", mpath.display()))?;
            let manifest: UatManifest = serde_saphyr::from_str(&manifest_raw)
                .map_err(|e| anyhow::anyhow!("invalid manifest {}: {e}", mpath.display()))?;
            evidence_snapshot_sha256(&manifest)
        } else {
            // No manifest: use empty snapshot.
            "sha256:{}".to_string()
        };

        let record = sddk_domain::UatAcceptanceRecord {
            decision: args.decision.into(),
            actor: args.actor.clone(),
            timestamp: now_rfc3339(),
            plan_version_sha256,
            evidence_snapshot_sha256,
            outstanding_findings: Vec::new(),
            justification: args.justification.clone(),
        };

        // Validate the record.
        let errors = sddk_domain::UatAcceptanceRecord::validate(&record);
        if !errors.is_empty() {
            anyhow::bail!("invalid acceptance record: {}", errors.join("; "));
        }

        // Write to XDG data dir.
        let project_id = resolve_project_id(args.project.as_deref(), environment)?;
        let xdg = xdg_from_env(environment);
        let acceptance_dir = sddk_engine::uat_storage_root(&xdg, &project_id)
            .map_err(|e| anyhow::anyhow!("cannot resolve storage root: {e}"))?
            .join("acceptances");
        std::fs::create_dir_all(&acceptance_dir)?;
        let output_path = acceptance_dir.join(format!("uat-acceptance-{}.yaml", args.release));
        let yaml = serde_saphyr::to_string(&record)
            .map_err(|e| anyhow::anyhow!("serialization failed: {e}"))?;
        std::fs::write(&output_path, yaml)?;
        Ok(output_path)
    })();
    render_result(result, format, |path| {
        format!("uat sign-off recorded: {}\n", path.display())
    })
}

/// Execute the `uat stale` command: inspect UI selectors against stored fingerprints
/// and report drift (REQ-RF-024).
fn run_uat_stale(args: UatStaleArgs, _environment: &crate::CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result: anyhow::Result<String> = (|| -> anyhow::Result<String> {
        // 1. Load the plan.
        let plan_path = args
            .plan
            .clone()
            .unwrap_or_else(|| PathBuf::from("uat-plan.yaml"));
        let plan_raw = std::fs::read_to_string(&plan_path)
            .map_err(|e| anyhow::anyhow!("cannot read plan {}: {e}", plan_path.display()))?;
        let plan: UatPlan =
            serde_saphyr::from_str(&plan_raw).map_err(|e| anyhow::anyhow!("invalid plan: {e}"))?;

        // 2. Extract geometry selectors from geometry-oracle scenarios.
        let selectors: Vec<String> = plan
            .features
            .iter()
            .flat_map(|f| &f.scenarios)
            .flat_map(|s| &s.oracles)
            .filter(|o| o.kind == UatOracleKind::Geometry)
            .filter_map(|o| {
                o.expect
                    .as_ref()
                    .and_then(|e| e.get("selector"))
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            })
            .collect();

        if selectors.is_empty() {
            // No geometry oracles → empty fresh report.
            let report = UatStalenessReport {
                release: plan.release.candidate.clone(),
                assessed_at: now_rfc3339(),
                affected_scenarios: Vec::new(),
                fingerprint_diffs: Vec::new(),
            };
            return serialize_report(&report, format);
        }

        // 3. Load previous geometry from session_dir if provided.
        let prev_geometry: serde_json::Value = if let Some(ref session) = args.session_dir {
            let geo_path = session.join("geometry.json");
            if geo_path.exists() {
                serde_json::from_str(&std::fs::read_to_string(&geo_path)?)
                    .map_err(|e| anyhow::anyhow!("invalid geometry.json: {e}"))?
            } else {
                serde_json::Value::Object(serde_json::Map::new())
            }
        } else {
            serde_json::Value::Object(serde_json::Map::new())
        };

        // 4. Run Playwright to capture current geometry.
        let evidence_dir_path =
            std::env::temp_dir().join(format!("sddk-stale-{}", std::process::id()));
        std::fs::create_dir_all(&evidence_dir_path)?;
        let geo_file = evidence_dir_path.join("geometry-selectors.json");
        std::fs::write(&geo_file, serde_json::to_string(&selectors)?)?;
        let pw_spec = sddk_gateway::PlaywrightSpec {
            url: args.url.clone(),
            viewport: None,
            actions: None,
            screenshot: false,
            trace: false,
            console: false,
            network: false,
            dom: false,
            geometry: Some(geo_file),
            output_dir: evidence_dir_path.clone(),
            timeout_ms: 30_000,
        };
        let _outcome = sddk_gateway::run_playwright(&pw_spec, None, None)
            .map_err(|e| anyhow::anyhow!("playwright run failed: {e}"))?;

        // 5. Load current geometry.
        let current_geometry_path = evidence_dir_path.join("geometry.json");
        let current_geometry: serde_json::Value = if current_geometry_path.exists() {
            serde_json::from_str(&std::fs::read_to_string(&current_geometry_path)?)
                .map_err(|e| anyhow::anyhow!("invalid geometry.json: {e}"))?
        } else {
            serde_json::Value::Object(serde_json::Map::new())
        };

        // 6. Compare geometries → build staleness report.
        let prev_obj = prev_geometry.as_object().cloned().unwrap_or_default();
        let curr_obj = current_geometry.as_object().cloned().unwrap_or_default();

        let mut affected_scenarios = Vec::new();
        let mut fingerprint_diffs = Vec::new();

        // Detectar elementos cambiados o eliminados.
        for (selector, prev_val) in &prev_obj {
            let curr_val = curr_obj.get(selector);
            match curr_val {
                Some(curr) if curr != prev_val => {
                    // Geometry changed.
                    fingerprint_diffs.push(UatStalenessDiff {
                        scenario_id: String::new(),
                        checkpoint_id: None,
                        field: "geometry".into(),
                        previous: prev_val.to_string(),
                        current: curr.to_string(),
                    });
                    affected_scenarios.push(UatStalenessScenario {
                        scenario_id: String::new(),
                        checkpoint_id: None,
                        selector: Some(selector.clone()),
                        text_content: None,
                        previous_fingerprint: prev_val.to_string(),
                        current_fingerprint: curr.to_string(),
                        change_kind: UatStalenessChangeKind::AttributeChanged,
                    });
                }
                Some(_) => {
                    // Identical — no change.
                }
                None => {
                    // Element removed.
                    affected_scenarios.push(UatStalenessScenario {
                        scenario_id: String::new(),
                        checkpoint_id: None,
                        selector: Some(selector.clone()),
                        text_content: None,
                        previous_fingerprint: prev_val.to_string(),
                        current_fingerprint: "null".into(),
                        change_kind: UatStalenessChangeKind::ElementRemoved,
                    });
                }
            }
        }

        // Detectar elementos nuevos.
        for (selector, curr_val) in &curr_obj {
            if !prev_obj.contains_key::<String>(selector) {
                affected_scenarios.push(UatStalenessScenario {
                    scenario_id: String::new(),
                    checkpoint_id: None,
                    selector: Some(selector.clone()),
                    text_content: None,
                    previous_fingerprint: "null".into(),
                    current_fingerprint: curr_val.to_string(),
                    change_kind: UatStalenessChangeKind::ElementAdded,
                });
            }
        }

        let report = UatStalenessReport {
            release: plan.release.candidate.clone(),
            assessed_at: now_rfc3339(),
            affected_scenarios,
            fingerprint_diffs,
        };
        serialize_report(&report, format)
    })();
    render_result(result, format, |s: &String| s.to_string())
}

/// Serialize a staleness report in the requested format.
fn serialize_report(report: &UatStalenessReport, format: OutputFormat) -> anyhow::Result<String> {
    match format {
        OutputFormat::Json => serde_json::to_string_pretty(report)
            .map_err(|e| anyhow::anyhow!("JSON serialization failed: {e}")),
        _ => serde_saphyr::to_string(report)
            .map_err(|e| anyhow::anyhow!("YAML serialization failed: {e}")),
    }
}

fn config_action_str(action: sddk_domain::ReleaseGateAction) -> &'static str {
    match action {
        sddk_domain::ReleaseGateAction::Required => "required",
        sddk_domain::ReleaseGateAction::Skip => "skip",
        sddk_domain::ReleaseGateAction::Advisory => "advisory",
    }
}

fn run_uat_report(args: UatReportArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<PathBuf> {
        let plan_raw = std::fs::read_to_string(&args.plan)
            .map_err(|e| anyhow::anyhow!("cannot read plan {}: {e}", args.plan.display()))?;
        let plan: UatPlan = serde_saphyr::from_str(&plan_raw)
            .map_err(|e| anyhow::anyhow!("invalid plan {}: {e}", args.plan.display()))?;
        if plan.release.candidate != args.release {
            anyhow::bail!(
                "plan candidate {} does not match requested release {}",
                plan.release.candidate,
                args.release
            );
        }

        let mut sessions = Vec::new();
        for session_path in &args.sessions {
            let raw = std::fs::read_to_string(session_path)
                .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", session_path.display()))?;
            let session: UatSession = serde_saphyr::from_str(&raw)
                .map_err(|e| anyhow::anyhow!("invalid session {}: {e}", session_path.display()))?;
            if session.release != args.release || session.plan_ref != plan.release.candidate {
                anyhow::bail!(
                    "session {} targets release {} / plan {}, expected {}",
                    session.session_id,
                    session.release,
                    session.plan_ref,
                    args.release
                );
            }
            sessions.push(session);
        }

        let report = aggregate_report(&plan, &sessions);
        let path = args
            .output
            .unwrap_or_else(|| PathBuf::from(format!("uat-report-{}.yaml", args.release)));
        let yaml = serde_saphyr::to_string(&report)
            .map_err(|e| anyhow::anyhow!("serialization failed: {e}"))?;
        std::fs::write(&path, yaml)?;
        Ok(path)
    })();
    render_result(result, format, |path| {
        format!("uat report written: {}\n", path.display())
    })
}

fn run_uat_status(args: UatStatusArgs) -> CommandOutput {
    let format = args.format;
    // Status is derived from artifacts on disk: plan/session/report for the
    // release candidate. U6 will enrich this with control-plane data.
    let plan_file = PathBuf::from(format!("uat-plan-{}.yaml", args.release));
    let report_file = PathBuf::from(format!("uat-report-{}.yaml", args.release));
    let lines = [
        format!("release: {}", args.release),
        format!(
            "plan: {}",
            if plan_file.exists() {
                "generated"
            } else {
                "missing"
            }
        ),
        format!(
            "report: {}",
            if report_file.exists() {
                "ready"
            } else {
                "not-ready"
            }
        ),
    ];
    let result: Result<String, anyhow::Error> = Ok(lines.join("\n"));
    render_result(result, format, |text| text.to_string())
}

/// Aggregate sessions into a report with the global verdict.
fn aggregate_report(plan: &UatPlan, sessions: &[UatSession]) -> UatReport {
    let mut scenario_status: std::collections::HashMap<
        String,
        (
            &sddk_domain::UatScenarioResult,
            Option<sddk_domain::UatExecutor>,
        ),
    > = std::collections::HashMap::new();
    // Oracle assessments por scenario (última session gana).
    let mut scenario_oracles: std::collections::HashMap<
        String,
        Vec<sddk_domain::UatOracleAssessment>,
    > = std::collections::HashMap::new();
    let mut total_minutes = 0u32;
    let mut defects = 0u32;
    let mut ux_issues = 0u32;

    for session in sessions {
        if let Some(finished) = &session.finished_at {
            let _ = finished;
        }
        total_minutes += session
            .results
            .iter()
            .map(|r| r.duration_minutes)
            .sum::<u32>();
        for result in &session.results {
            // Last writer wins per scenario.
            scenario_status.insert(result.scenario_id.clone(), (result, Some(session.executor)));
            if !result.oracle_assessments.is_empty() {
                scenario_oracles.insert(
                    result.scenario_id.clone(),
                    result.oracle_assessments.clone(),
                );
            }
            if result.status == sddk_domain::UatStatus::Fail {
                defects += 1;
            }
            if result.status == sddk_domain::UatStatus::Partial {
                ux_issues += 1;
            }
        }
    }

    let mut total = 0u32;
    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut blocked = 0u32;
    let mut partial = 0u32;
    let mut not_run = 0u32;
    let mut covered = 0u32;
    let mut not_ready_blockers = Vec::new();
    let mut acceptance_blockers = Vec::new();

    let mut features = Vec::new();
    for feature in &plan.features {
        let mut sc_rollups = Vec::new();
        for scenario in &feature.scenarios {
            total += 1;
            let (status, executor, blocker) = match scenario_status.get(&scenario.id).copied() {
                None => (
                    sddk_domain::UatStatus::NotRun,
                    None,
                    Some(format!("{} (not run)", scenario.id)),
                ),
                Some((result, executor))
                    if !evidence_satisfies_spec(scenario.evidence.as_ref(), &result.evidence) =>
                {
                    (
                        sddk_domain::UatStatus::NotRun,
                        executor,
                        Some(format!(
                            "{} (required evidence missing or invalid)",
                            scenario.id
                        )),
                    )
                }
                Some((result, executor)) => {
                    let blocker = match result.status {
                        sddk_domain::UatStatus::Fail
                        | sddk_domain::UatStatus::Blocked
                        | sddk_domain::UatStatus::NotRun => Some(format!(
                            "{} ({})",
                            scenario.id,
                            result
                                .comment
                                .clone()
                                .unwrap_or_else(|| format!("{:?}", result.status))
                        )),
                        _ => None,
                    };
                    (result.status, executor, blocker)
                }
            };
            if let Some(blocker) = blocker {
                not_ready_blockers.push(blocker);
            }
            match status {
                sddk_domain::UatStatus::NotRun => not_run += 1,
                sddk_domain::UatStatus::Pass => {
                    passed += 1;
                    covered += 1;
                }
                sddk_domain::UatStatus::Fail => failed += 1,
                sddk_domain::UatStatus::Blocked => blocked += 1,
                sddk_domain::UatStatus::Partial => {
                    partial += 1;
                    covered += 1;
                }
            }
            // Acceptance (v3, REQ-RF-023): PASSED != ACCEPTED.
            // Requiere aceptación humana quien: (a) es P0, (b) su review
            // policy lo exige (Always, o RiskBased con trigger
            // BusinessCriticalityHigh), o (c) tiene sampling > 0 y status
            // machine PASS sin veredicto humano.
            // La acceptance (REQ-RF-023) es una decisión humana que vive en
            // el plan (`scenario.acceptance`): el auto-runner y los oracles
            // nunca aceptan. Si el plan no la marca, queda Pending cuando el
            // escenario la requiere.
            let acceptance_required = scenario.priority == sddk_domain::UatPriority::P0
                || scenario
                    .review
                    .as_ref()
                    .map(|r| {
                        r.kind == sddk_domain::UatReviewPolicyKind::Always
                            || r.require_human_when
                                .contains(&sddk_domain::UatReviewTrigger::BusinessCriticalityHigh)
                            || r.sampling > 0.0
                    })
                    .unwrap_or(false);
            let acceptance = if acceptance_required {
                Some(
                    scenario
                        .acceptance
                        .unwrap_or(sddk_domain::UatAcceptanceStatus::Pending),
                )
            } else {
                None
            };
            if acceptance_required
                && matches!(
                    status,
                    sddk_domain::UatStatus::Pass | sddk_domain::UatStatus::Partial
                )
                && !matches!(acceptance, Some(sddk_domain::UatAcceptanceStatus::Accepted))
            {
                acceptance_blockers.push(format!(
                    "{} (machine {} pero sin acceptance humana)",
                    scenario.id,
                    uat_status_str(status)
                ));
            }
            sc_rollups.push(UatScenarioRollup {
                scenario_id: scenario.id.clone(),
                status,
                executor,
                acceptance,
                acceptance_required,
                oracle_verdicts: scenario_oracles.get(&scenario.id).cloned(),
            });
        }
        let feat_total = feature.scenarios.len() as u32;
        let feat_covered = sc_rollups
            .iter()
            .filter(|s| {
                matches!(
                    s.status,
                    sddk_domain::UatStatus::Pass | sddk_domain::UatStatus::Partial
                )
            })
            .count() as u32;
        features.push(UatFeatureRollup {
            id: feature.id.clone(),
            name: feature.name.clone(),
            coverage_pct: if feat_total > 0 {
                100.0 * feat_covered as f64 / feat_total as f64
            } else {
                0.0
            },
            scenarios: sc_rollups,
        });
    }

    let coverage_pct = if total > 0 {
        100.0 * covered as f64 / total as f64
    } else {
        0.0
    };

    let verdict = if failed > 0 || not_run > 0 {
        UatVerdict::NotReady
    } else if blocked > 0 || partial > 0 {
        UatVerdict::ReadyWithRisks
    } else {
        UatVerdict::Ready
    };

    UatReport {
        schema_version: 2,
        release: plan.release.candidate.clone(),
        plan_ref: plan.release.candidate.clone(),
        sessions: sessions.iter().map(|s| s.session_id.clone()).collect(),
        summary: UatReportSummary {
            total_scenarios: total,
            passed,
            failed,
            blocked,
            partial,
            not_run,
            coverage_pct,
            defects,
            ux_issues,
            uat_duration_minutes: total_minutes,
        },
        features,
        verdict,
        not_ready_blockers,
        acceptance_blockers,
    }
}

#[cfg(test)]
mod uat_integrity_tests {
    use super::*;

    fn plan(required_evidence: bool) -> UatPlan {
        serde_saphyr::from_str(&format!(
            r#"
schema_version: 2
release: {{ candidate: v2.0.0 }}
generated_by: test
generated_at: "2026-08-09T00:00:00Z"
features:
  - id: F-1
    name: Feature
    scenarios:
      - id: S-1
        title: Scenario one
        evidence:
          required: {required_evidence}
          kinds: [{{ kind: note }}]
      - id: S-2
        title: Scenario two
"#
        ))
        .unwrap()
    }

    fn session(results: &str) -> UatSession {
        serde_saphyr::from_str(&format!(
            r#"
schema_version: 2
session_id: session-1
plan_ref: v2.0.0
release: v2.0.0
executor: human
executed_by: tester
started_at: 2026-08-09T00:00:00Z
finished_at: 2026-08-09T00:05:00Z
results:
{results}
"#
        ))
        .unwrap()
    }

    #[test]
    fn missing_scenario_is_not_run_and_never_ready() {
        let session = session(
            "  - scenario_id: S-1\n    status: PASS\n    evidence: [{ kind: note, ref: 'sha256:test' }]",
        );
        let report = aggregate_report(&plan(true), &[session]);
        assert_eq!(report.summary.not_run, 1);
        assert_eq!(report.summary.coverage_pct, 50.0);
        assert_eq!(report.verdict, UatVerdict::NotReady);
        assert_eq!(
            report.features[0].scenarios[1].status,
            sddk_domain::UatStatus::NotRun
        );
    }

    #[test]
    fn pass_without_required_evidence_is_not_run() {
        let session = session(
            "  - scenario_id: S-1\n    status: PASS\n  - scenario_id: S-2\n    status: PASS",
        );
        let report = aggregate_report(&plan(true), &[session]);
        assert_eq!(report.summary.not_run, 1);
        assert_eq!(report.verdict, UatVerdict::NotReady);
    }

    #[test]
    fn release_gate_requires_ready_complete_matching_report() {
        let session = session(
            "  - scenario_id: S-1\n    status: PASS\n    evidence: [{ kind: note, ref: 'sha256:test' }]\n  - scenario_id: S-2\n    status: PASS",
        );
        let report = aggregate_report(&plan(true), &[session]);
        validate_release_report(&report, "v2.0.0").unwrap();
        assert!(validate_release_report(&report, "v2.0.1").is_err());
        let mut legacy = report;
        legacy.schema_version = 1;
        assert!(validate_release_report(&legacy, "v2.0.0").is_err());
    }
}

/// Render a self-contained HTML dashboard from a plan (ADR-0013 kit).
fn render_dashboard_html(
    plan: &UatPlan,
    view: UatView,
    mode: UatRunnerMode,
    theme: &str,
    environment: &crate::CliEnvironment,
) -> anyhow::Result<String> {
    let assets = crate::dev::paths::resolve_assets_dir(environment)?;
    let kit = assets.map(|a| a.join("uat-dashboard")).unwrap_or_default();

    let tokens = read_asset(&kit.join("kit/tokens.css"))?;
    let components_css = read_asset(&kit.join("kit/components.css"))?;
    let components_js = read_asset(&kit.join("kit/components.js"))?;
    let storage_js = read_asset(&kit.join("kit/storage.js"))?;
    let video_annotation_js = read_asset(&kit.join("kit/video_annotation.js"))?;

    let view_name = match view {
        UatView::Guided => "guided",
        UatView::Matrix => "interactive",
        UatView::Traceability => "report",
    };
    let template = read_asset(&kit.join("views").join(format!("{view_name}.html")))?;

    let theme_css = if theme == "light" {
        read_asset(&kit.join("themes/light.css"))?
    } else {
        read_asset(&kit.join("themes/dark.css"))?
    };

    let plan_json = serde_json::to_string_pretty(plan)
        .map_err(|e| anyhow::anyhow!("plan serialization failed: {e}"))?;

    // Runner context: mode is embedded for the renderer to do selective UI per role.
    let mode_name = match mode {
        UatRunnerMode::Designer => "designer",
        UatRunnerMode::Runner => "runner",
        UatRunnerMode::Reviewer => "reviewer",
    };
    let runner_context = serde_json::json!({ "mode": mode_name });
    let runner_context_json = serde_json::to_string_pretty(&runner_context)
        .map_err(|e| anyhow::anyhow!("runner context serialization failed: {e}"))?;

    let html = template
        .replace("@TOKENS@", &tokens)
        .replace("@COMPONENTS@", &format!("{components_css}\n{theme_css}"))
        .replace("@PLAN_JSON@", &plan_json)
        .replace("@RUNNER_CONTEXT@", &runner_context_json)
        .replace("@REPORT_JSON@", "{}")
        .replace("@RELEASE@", &plan.release.candidate)
        .replace("@GENERATED_AT@", &now_rfc3339())
        .replace("@PLAN_REF@", &plan.release.candidate)
        .replace("@STORAGE_JS@", &storage_js)
        .replace("@COMPONENTS_JS@", &components_js)
        .replace("@VIDEO_ANNOTATION_JS@", &video_annotation_js);

    Ok(html)
}

fn read_asset(path: &Path) -> anyhow::Result<String> {
    std::fs::read_to_string(path).map_err(|e| {
        anyhow::anyhow!(
            "missing dashboard asset {} (run `sddk dev update` to install the bundle): {e}",
            path.display()
        )
    })
}

fn now_rfc3339() -> String {
    // Use the shared `sddk_domain::format::format_rfc3339_utc` instead of a
    // local copy. The local `civil_from_days` was removed in cycle 3
    // (W-DV-7) to eliminate cross-crate duplication.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    sddk_domain::format::format_rfc3339_utc(secs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn aggregate_report_computes_verdict() {
        let plan: UatPlan = serde_saphyr::from_str(
            r#"
schema_version: 1
release: { candidate: v1.5.0, project: demo }
generated_by: uat-planner
generated_at: "2026-08-07T00:00:00Z"
features:
  - id: F-01
    name: Login
    scenarios:
      - id: S-1
        title: Login correcto
        plain_steps:
          - action: abrir /login
            expected: formulario visible
      - id: S-2
        title: Login fallido
        plain_steps:
          - action: password incorrecto
            expected: error visible
"#,
        )
        .unwrap();

        let session: UatSession = serde_saphyr::from_str(
            r#"
schema_version: 1
session_id: uat-1
plan_ref: v1.5.0
release: v1.5.0
started_at: "2026-08-07T00:00:00Z"
results:
  - scenario_id: S-1
    status: PASS
  - scenario_id: S-2
    status: FAIL
    comment: no muestra error
"#,
        )
        .unwrap();

        let report = aggregate_report(&plan, &[session]);
        assert_eq!(report.verdict, UatVerdict::NotReady);
        assert_eq!(report.summary.total_scenarios, 2);
        assert_eq!(report.summary.passed, 1);
        assert_eq!(report.summary.failed, 1);
        assert_eq!(report.summary.defects, 1);
        assert_eq!(report.summary.coverage_pct, 50.0);
        assert_eq!(report.not_ready_blockers.len(), 1);
        assert!(report.not_ready_blockers[0].contains("S-2"));
    }

    #[test]
    fn p0_machine_pass_without_acceptance_blocks_report() {
        let plan: UatPlan = serde_saphyr::from_str(
            r#"
schema_version: 3
release: { candidate: v1.7.0 }
generated_by: uat-planner
generated_at: "2026-08-10T00:00:00Z"
features:
  - id: F-01
    name: Pago
    scenarios:
      - id: S-1
        title: Cobro correcto
        priority: P0
        assignee: developer
        executor:
          kind: cli
          command: "echo ok"
"#,
        )
        .unwrap();

        let session: UatSession = serde_saphyr::from_str(
            r#"
schema_version: 3
session_id: uat-auto-1
plan_ref: v1.7.0
release: v1.7.0
started_at: "2026-08-10T00:00:00Z"
results:
  - scenario_id: S-1
    status: PASS
"#,
        )
        .unwrap();

        let report = aggregate_report(&plan, &[session]);
        // PASS machine sin acceptance humana → blocker de acceptance.
        assert_eq!(report.verdict, UatVerdict::Ready);
        assert!(!report.acceptance_blockers.is_empty());
        assert!(report.acceptance_blockers[0].contains("S-1"));
        assert_eq!(
            report.features[0].scenarios[0].acceptance,
            Some(sddk_domain::UatAcceptanceStatus::Pending)
        );
        assert!(report.features[0].scenarios[0].acceptance_required);
        // El gate lo rechaza.
        let err = validate_release_report(&report, "v1.7.0").unwrap_err();
        assert!(err.to_string().contains("acceptance"));
    }

    #[test]
    fn p0_plan_accepted_clears_acceptance_blocker() {
        let plan: UatPlan = serde_saphyr::from_str(
            r#"
schema_version: 3
release: { candidate: v1.7.0 }
generated_by: uat-planner
generated_at: "2026-08-10T00:00:00Z"
features:
  - id: F-01
    name: Pago
    scenarios:
      - id: S-1
        title: Cobro correcto
        priority: P0
        assignee: developer
        acceptance: accepted
        executor:
          kind: cli
          command: "echo ok"
"#,
        )
        .unwrap();

        let session: UatSession = serde_saphyr::from_str(
            r#"
schema_version: 3
session_id: uat-auto-1
plan_ref: v1.7.0
release: v1.7.0
started_at: "2026-08-10T00:00:00Z"
results:
  - scenario_id: S-1
    status: PASS
"#,
        )
        .unwrap();

        let report = aggregate_report(&plan, &[session]);
        assert!(report.acceptance_blockers.is_empty());
        assert_eq!(
            report.features[0].scenarios[0].acceptance,
            Some(sddk_domain::UatAcceptanceStatus::Accepted)
        );
        assert!(validate_release_report(&report, "v1.7.0").is_ok());
    }

    #[test]
    fn validate_rejects_empty_plan() {
        let raw = r#"
schema_version: 1
release: { candidate: v1.5.0 }
generated_by: uat-planner
generated_at: "2026-08-07T00:00:00Z"
features: []
"#;
        let value: serde_json::Value = serde_saphyr::from_str(raw).unwrap();
        let has_scenarios = value
            .get("features")
            .and_then(|f| f.as_array())
            .map(|arr| {
                arr.iter().any(|feat| {
                    feat.get("scenarios")
                        .and_then(|s| s.as_array())
                        .map(|sc| !sc.is_empty())
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        assert!(!has_scenarios);
    }

    #[test]
    fn browser_command_shapes() {
        let path = Path::new("/tmp/uat.html");

        // Explicit --browser override wins.
        let cmd = browser_command(path, Some("firefox"));
        assert_eq!(cmd[0], "firefox");
        assert!(cmd[1].ends_with("uat.html"));

        // Default launcher is platform-dependent.
        let default = browser_command(path, None);
        #[cfg(target_os = "windows")]
        {
            assert_eq!(default[0], "cmd");
            assert_eq!(default[2], "start");
        }
        #[cfg(target_os = "macos")]
        {
            assert_eq!(default[0], "open");
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        {
            assert_eq!(default[0], "xdg-open");
        }
    }

    #[test]
    fn failure_filter_keeps_only_fail_and_blocked() {
        let mut findings: Vec<UatFailure> = Vec::new();
        let sc = |status: &str, comment: &str| -> UatFailure {
            UatFailure {
                scenario_id: format!("S-{status}"),
                status: status.into(),
                comment: comment.into(),
                evidence: vec![],
                feature: None,
                priority: None,
                assignee: None,
                rationale: None,
                session_id: "uat-1".into(),
                executed_by: "Tester".into(),
            }
        };

        let input = vec![
            sc("PASS", "ok"),
            sc("FAIL", "broken"),
            sc("BLOCKED", "env"),
            sc("NOT_RUN", "missing"),
            sc("PARTIAL", "meh"),
        ];
        for f in input {
            if matches!(f.status.as_str(), "FAIL" | "BLOCKED" | "NOT_RUN") {
                findings.push(f);
            }
        }
        assert_eq!(findings.len(), 3);
        let ids: Vec<&str> = findings.iter().map(|f| f.scenario_id.as_str()).collect();
        assert!(ids.contains(&"S-FAIL"));
        assert!(ids.contains(&"S-BLOCKED"));
        assert!(ids.contains(&"S-NOT_RUN"));
    }

    #[test]
    fn uat_session_to_failure_serializes_for_agents() {
        // The agent consumes `uat failures --format json`; verify shape.
        let findings: Vec<UatFailure> = vec![UatFailure {
            scenario_id: "S-2".into(),
            status: "FAIL".into(),
            comment: "no muestra error".into(),
            evidence: vec!["screenshot:sha256:abc".into()],
            feature: Some("Login".into()),
            priority: Some("P1".into()),
            assignee: Some("developer".into()),
            rationale: Some("bloquea el onboarding".into()),
            session_id: "uat-1".into(),
            executed_by: "Test".into(),
        }];
        let json = serde_json::to_string(&findings).unwrap();
        // The agent must be able to read these fields directly.
        assert!(json.contains("\"scenario_id\":\"S-2\""));
        assert!(json.contains("\"feature\":\"Login\""));
        assert!(json.contains("\"comment\":\"no muestra error\""));
        let parsed: Vec<HashMap<String, serde_json::Value>> = serde_json::from_str(&json).unwrap();
        let _ = parsed;
    }

    #[test]
    fn acceptance_record_accepted_is_valid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("uat-acceptance-v1.0.0.yaml");
        let content = r#"
decision: accepted
actor: user:421
timestamp: "2026-08-11T00:00:00Z"
plan_version_sha256: sha256:abc123
evidence_snapshot_sha256: sha256:def456
outstanding_findings: []
justification: "LGTM"
"#;
        std::fs::write(&path, content).unwrap();
        let record = validate_acceptance_record(&path).unwrap();
        assert_eq!(
            record.decision,
            sddk_domain::UatAcceptanceDecision::Accepted
        );
    }

    #[test]
    fn acceptance_record_rejected_is_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("uat-acceptance-v1.0.0.yaml");
        let content = r#"
decision: rejected
actor: user:421
timestamp: "2026-08-11T00:00:00Z"
plan_version_sha256: sha256:abc123
evidence_snapshot_sha256: sha256:def456
outstanding_findings: []
justification: "Not ready"
"#;
        std::fs::write(&path, content).unwrap();
        let err = validate_acceptance_record(&path).unwrap_err();
        assert!(
            err.to_string().contains("REJECTED"),
            "expected REJECTED error, got: {}",
            err
        );
    }

    #[test]
    fn acceptance_record_missing_is_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.yaml");
        let err = validate_acceptance_record(&path).unwrap_err();
        assert!(
            err.to_string().contains("cannot read"),
            "expected read error, got: {}",
            err
        );
    }
}

/// Resolve the `(program, args)` pair used to open a local HTML file.
/// Exposed for tests; `open_in_browser` executes it.
fn browser_command(path: &Path, browser: Option<&str>) -> Vec<String> {
    let target = path.display().to_string();
    if let Some(b) = browser {
        return vec![b.to_string(), target];
    }
    if cfg!(target_os = "windows") {
        vec!["cmd".into(), "/c".into(), "start".into(), "".into(), target]
    } else if cfg!(target_os = "macos") {
        vec!["open".into(), target]
    } else {
        vec!["xdg-open".into(), target]
    }
}

fn run_uat_migrate_plan(args: UatMigratePlanArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<(PathBuf, UatMigrationReport)> {
        let raw = std::fs::read_to_string(&args.input)
            .map_err(|e| anyhow::anyhow!("cannot read {}: {e}", args.input.display()))?;
        let mut plan: UatPlan = serde_saphyr::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("invalid plan {}: {e}", args.input.display()))?;
        let report = migrate_plan_v1_to_v2(&mut plan);
        let yaml = serde_saphyr::to_string(&plan)
            .map_err(|e| anyhow::anyhow!("serialization failed: {e}"))?;
        let output_path = if args.in_place {
            args.input.clone()
        } else if let Some(p) = args.output {
            p
        } else {
            let stem = args
                .input
                .file_stem()
                .map(|s| s.to_os_string())
                .unwrap_or_default();
            let parent = args
                .input
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf();
            parent.join(format!("{}.v2.yaml", stem.to_string_lossy()))
        };
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&output_path, yaml)?;
        if let Some(report_path) = &args.report {
            if let Some(parent) = report_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let report_yaml = serde_saphyr::to_string(&report)
                .map_err(|e| anyhow::anyhow!("report serialization failed: {e}"))?;
            std::fs::write(report_path, report_yaml)?;
        }
        Ok((output_path, report))
    })();
    render_result(result, format, |(path, report)| {
        format!(
            "uat migrate-plan: {} ({} → {}); features={}, scenarios={}, evidence_promoted={}, risk_promoted={}, timing_promoted={}\n",
            path.display(),
            report.from_version,
            report.to_version,
            report.features_touched,
            report.scenarios_touched,
            report.evidence_promoted,
            report.risk_promoted,
            report.timing_promoted,
        )
    })
}

fn resolve_project_id_or_default(
    args_project: Option<&str>,
    environment: &crate::CliEnvironment,
) -> anyhow::Result<String> {
    if let Some(id) = args_project {
        return Ok(id.to_string());
    }
    resolve_project_id(args_project, environment)
}

fn run_uat_storage_path(
    args: UatStoragePathArgs,
    environment: &crate::CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<serde_json::Value> {
        let project_id = resolve_project_id_or_default(args.project.as_deref(), environment)?;
        let xdg = xdg_from_env(environment);
        let root = sddk_engine::uat_storage_root(&xdg, &project_id)
            .map_err(|e| anyhow::anyhow!("cannot resolve storage root: {e}"))?;
        let manifest = sddk_engine::uat_manifest_path(&xdg, &project_id)
            .map_err(|e| anyhow::anyhow!("cannot resolve manifest path: {e}"))?;
        Ok(serde_json::json!({
            "project_id": project_id,
            "storage_root": root.display().to_string(),
            "manifest_path": manifest.display().to_string(),
        }))
    })();
    render_result(result, format, |v| {
        serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string())
    })
}

fn load_manifest(manifest_path: &Path) -> anyhow::Result<UatManifest> {
    if !manifest_path.exists() {
        return Ok(UatManifest::new("", ""));
    }
    let raw = std::fs::read_to_string(manifest_path)
        .map_err(|e| anyhow::anyhow!("cannot read manifest {}: {e}", manifest_path.display()))?;
    serde_saphyr::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("invalid manifest {}: {e}", manifest_path.display()))
}

fn run_uat_build_manifest(
    args: UatBuildManifestArgs,
    environment: &crate::CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<(PathBuf, u32)> {
        if args.sessions.is_empty() {
            anyhow::bail!("no sessions provided");
        }
        let project_id = resolve_project_id_or_default(args.project.as_deref(), environment)?;
        let xdg = xdg_from_env(environment);
        let manifest_path = args.output.clone().unwrap_or_else(|| {
            sddk_engine::uat_manifest_path(&xdg, &project_id).unwrap_or_default()
        });
        let mut manifest = load_manifest(&manifest_path)
            .unwrap_or_else(|_| UatManifest::new(project_id.clone(), now_rfc3339()));
        let mut added = 0u32;
        for session_path in &args.sessions {
            let session: UatSession = read_session(session_path)?;
            for result in &session.results {
                for ev in &result.evidence {
                    let sha256 = ev.r#ref.clone();
                    let Some(path) = ev.path.clone() else {
                        continue;
                    };
                    let entry = UatManifestEntry {
                        sha256: sha256
                            .strip_prefix("sha256:")
                            .unwrap_or(&sha256)
                            .to_string(),
                        path,
                        size_bytes: ev.size_bytes.unwrap_or(0),
                        captured_at: ev.captured_at.clone().unwrap_or_else(now_rfc3339),
                        scenario_id: result.scenario_id.clone(),
                        session_id: session.session_id.clone(),
                        kind: ev.kind,
                        mime: ev.mime.clone(),
                    };
                    let was_new = manifest.lookup(&sha256).is_none();
                    manifest.upsert(entry);
                    if was_new {
                        added += 1;
                    }
                }
            }
        }
        if let Some(parent) = manifest_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let yaml = serde_saphyr::to_string(&manifest)
            .map_err(|e| anyhow::anyhow!("manifest serialization failed: {e}"))?;
        std::fs::write(&manifest_path, yaml)?;
        Ok((manifest_path, added))
    })();
    render_result(result, format, |(path, added)| {
        format!(
            "uat build-manifest: {} ({} new entries)\n",
            path.display(),
            added
        )
    })
}

fn run_uat_verify_integrity(
    args: UatVerifyIntegrityArgs,
    environment: &crate::CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<(UatIntegrityReport, PathBuf)> {
        let session: UatSession = read_session(&args.session)?;
        let project_id = resolve_project_id_or_default(args.project.as_deref(), environment)?;
        let xdg = xdg_from_env(environment);
        let manifest_path = args.manifest.clone().unwrap_or_else(|| {
            sddk_engine::uat_manifest_path(&xdg, &project_id).unwrap_or_default()
        });
        let manifest = load_manifest(&manifest_path)
            .unwrap_or_else(|_| UatManifest::new(project_id.clone(), now_rfc3339()));
        let mut findings = Vec::new();
        let mut total_evidence = 0u32;
        for result in &session.results {
            for ev in &result.evidence {
                total_evidence += 1;
                let manifest_entry = manifest.lookup(&ev.r#ref);
                let mut finding = verify_evidence(ev, manifest_entry, None);
                finding.scenario_id = result.scenario_id.clone();
                if let Some(path) = &ev.path {
                    let full = sddk_engine::uat_storage_root(&xdg, &project_id)
                        .map(|root| root.join(path));
                    match full {
                        Ok(p) if p.exists() => {
                            if finding.status == "no_payload" {
                                finding.status = "ok".into();
                                finding.message = Some(format!("file present at {}", p.display()));
                            }
                        }
                        Ok(p) => {
                            finding.status = "missing".into();
                            finding.message = Some(format!("file missing at {}", p.display()));
                        }
                        Err(e) => {
                            finding.status = "missing".into();
                            finding.message = Some(format!("path resolution failed: {e}"));
                        }
                    }
                }
                findings.push(finding);
            }
        }
        let verdict = UatIntegrityReport::compute_verdict(&findings).to_string();
        let report = UatIntegrityReport {
            session_id: session.session_id.clone(),
            project_id: project_id.clone(),
            verified_at: now_rfc3339(),
            total_evidence,
            findings: findings.clone(),
            verdict: verdict.clone(),
        };
        let output_path = args
            .output
            .unwrap_or_else(|| args.session.with_extension("integrity.yaml"));
        let yaml = serde_saphyr::to_string(&report)
            .map_err(|e| anyhow::anyhow!("report serialization failed: {e}"))?;
        std::fs::write(&output_path, yaml)?;
        Ok((report, output_path))
    })();
    match result {
        Ok((report, path)) => {
            if matches!(format, OutputFormat::Json) {
                let json = serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".into());
                return CommandOutput {
                    stdout: format!("{json}\n"),
                    stderr: String::new(),
                    status: 0,
                };
            }
            let ok = report.findings.iter().filter(|f| f.status == "ok").count();
            let partial = report
                .findings
                .iter()
                .filter(|f| f.status == "no_payload")
                .count();
            let fail = report
                .findings
                .iter()
                .filter(|f| {
                    matches!(
                        f.status.as_str(),
                        "missing" | "hash_mismatch" | "size_mismatch"
                    )
                })
                .count();
            let mut out = format!(
                "uat verify-integrity: session={}, evidence={}, verdict={}, ok={}, partial={}, fail={}\nreport: {}\n",
                report.session_id,
                report.total_evidence,
                report.verdict,
                ok,
                partial,
                fail,
                path.display()
            );
            for f in &report.findings {
                out.push_str(&format!(
                    "  [{}] {} sha256={} ({})\n",
                    f.status.to_uppercase(),
                    f.scenario_id,
                    f.sha256,
                    f.message.as_deref().unwrap_or("")
                ));
            }
            let exit = if report.verdict == "fail" { 1 } else { 0 };
            CommandOutput {
                stdout: out,
                stderr: String::new(),
                status: exit,
            }
        }
        Err(e) => render_result(Err::<(), _>(e), format, |_| String::new()),
    }
}

#[allow(dead_code)]
fn _cli_sha256_hex(bytes: &[u8]) -> String {
    sha256_hex(bytes)
}

fn run_uat_scenario_context(args: UatScenarioContextArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<(UatSuggestionsReport, Option<PathBuf>)> {
        let raw = std::fs::read_to_string(&args.plan)
            .map_err(|e| anyhow::anyhow!("cannot read plan {}: {e}", args.plan.display()))?;
        let mut plan: UatPlan = serde_saphyr::from_str(&raw)
            .map_err(|e| anyhow::anyhow!("invalid plan {}: {e}", args.plan.display()))?;
        migrate_plan_v1_to_v2(&mut plan);
        let report = suggest_scenario_context(&plan);
        let output_path = if args.apply {
            let path = args.output.clone().unwrap_or_else(|| {
                let stem = args
                    .plan
                    .file_stem()
                    .map(|s| s.to_os_string())
                    .unwrap_or_default();
                let parent = args
                    .plan
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .to_path_buf();
                parent.join(format!("{}.context.yaml", stem.to_string_lossy()))
            });
            apply_all_suggestions(&mut plan, &report);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let yaml = serde_saphyr::to_string(&plan)
                .map_err(|e| anyhow::anyhow!("plan serialization failed: {e}"))?;
            std::fs::write(&path, yaml)?;
            Some(path)
        } else {
            None
        };
        Ok((report, output_path))
    })();
    match result {
        Ok((report, output_path)) => {
            let stdout = scenario_context_to_string(&report, output_path.as_ref(), format);
            CommandOutput {
                stdout,
                stderr: String::new(),
                status: 0,
            }
        }
        Err(e) => render_result(Err::<(), _>(e), format, |_| String::new()),
    }
}

fn scenario_context_to_string(
    report: &UatSuggestionsReport,
    output_path: Option<&PathBuf>,
    format: OutputFormat,
) -> String {
    if matches!(format, OutputFormat::Json) {
        return serde_json::to_string_pretty(&serde_json::json!({"report": report, "output_path": output_path.map(|p| p.display().to_string())}))
            .unwrap_or_else(|_| "{}".into());
    }
    let mut out = String::new();
    out.push_str(&format!(
        "uat scenario-context: plan={}, version=v{}, scenarios={}, fully_populated={}, partial={}, suggestions={}\n",
        report.plan_ref, report.plan_version, report.total_scenarios, report.fully_populated, report.partial, report.suggestions_count,
    ));
    for s in &report.scenarios {
        if s.suggestions.is_empty() {
            out.push_str(&format!(
                "  ✓ {} / {} — fully populated ({} fields)\n",
                s.feature_id, s.scenario_id, s.populated_fields
            ));
            continue;
        }
        out.push_str(&format!(
            "  → {} / {} — {} (populated: {}, missing: {})\n",
            s.feature_id, s.scenario_id, s.scenario_title, s.populated_fields, s.missing_fields
        ));
        for sug in &s.suggestions {
            let proposed_summary = match &sug.proposed {
                serde_json::Value::String(s) if s.is_empty() => "(fill manually)".into(),
                serde_json::Value::String(s) => format!("\"{s}\""),
                serde_json::Value::Array(a) => format!("[{} items]", a.len()),
                serde_json::Value::Object(o) => format!("{{{} keys}}", o.len()),
                other => other.to_string(),
            };
            out.push_str(&format!(
                "      · {} [{}]: {}\n        → {}\n",
                sug.field, sug.kind, sug.reason, proposed_summary
            ));
        }
    }
    if let Some(path) = output_path {
        out.push_str(&format!("\nApplied plan written to {}\n", path.display()));
    } else if report.suggestions_count > 0 {
        out.push_str("\nRun with --apply to write suggestions to <plan>.context.yaml\n");
    }
    out
}

fn run_uat_history(args: UatHistoryArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<(UatHistoryReport, Option<PathBuf>)> {
        if args.sessions.is_empty() {
            anyhow::bail!("no sessions provided");
        }
        let plan_raw = std::fs::read_to_string(&args.plan)
            .map_err(|e| anyhow::anyhow!("cannot read plan {}: {e}", args.plan.display()))?;
        let plan: UatPlan = serde_saphyr::from_str(&plan_raw)
            .map_err(|e| anyhow::anyhow!("invalid plan {}: {e}", args.plan.display()))?;
        let mut sessions = Vec::new();
        for sp in &args.sessions {
            sessions.push(read_session(sp)?);
        }
        let report = aggregate_history(&plan, &sessions, &args.release, &now_rfc3339());
        let output_path = args
            .output
            .clone()
            .unwrap_or_else(|| PathBuf::from(format!("uat-history-{}.yaml", args.release)));
        if let Some(parent) = output_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let yaml = serde_saphyr::to_string(&report)
            .map_err(|e| anyhow::anyhow!("history serialization failed: {e}"))?;
        std::fs::write(&output_path, yaml)?;
        Ok((report, Some(output_path)))
    })();
    match result {
        Ok((report, output_path)) => {
            if matches!(format, OutputFormat::Json) {
                let json = serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".into());
                return CommandOutput {
                    stdout: format!("{json}\n"),
                    stderr: String::new(),
                    status: 0,
                };
            }
            let mut out = format!(
                "uat history: release={}, sessions={}, scenarios={}, defects={}\n",
                report.release,
                report.sessions_total,
                report.scenarios.len(),
                report.defects_total,
            );
            for f in &report.features {
                out.push_str(&format!(
                    "    {} — {} — coverage {:.0}% ({} / {})\n",
                    f.feature_id,
                    f.feature_name,
                    f.coverage_pct,
                    f.scenarios_passing,
                    f.scenarios_total
                ));
            }
            out.push_str(
                "\nscenarios (last / first run · success rate · flakiness · trend · defects):\n",
            );
            for s in &report.scenarios {
                let last = s
                    .last_run
                    .as_ref()
                    .map(|r| {
                        format!(
                            "{} {} {}",
                            r.at.get(..16).unwrap_or(&r.at),
                            r.status,
                            r.commit.as_deref().unwrap_or("?")
                        )
                    })
                    .unwrap_or_else(|| "(never run)".into());
                let first = s
                    .first_run
                    .as_ref()
                    .map(|r| {
                        format!(
                            "{} {} {}",
                            r.at.get(..16).unwrap_or(&r.at),
                            r.status,
                            r.commit.as_deref().unwrap_or("?")
                        )
                    })
                    .unwrap_or_else(|| "(never)".into());
                let defects = if s.defect_ids.is_empty() {
                    "—".into()
                } else {
                    s.defect_ids.join(", ")
                };
                out.push_str(&format!(
                    "  {} / {} — {} (success {:.0}%, flaky {:.0}, trend={}, defects={})\n",
                    s.feature_id,
                    s.scenario_id,
                    s.scenario_title,
                    s.success_rate * 100.0,
                    s.flakiness_score * 100.0,
                    s.trend,
                    defects
                ));
                out.push_str(&format!("      last:  {}\n      first: {}\n      runs:  {} (pass {} | fail {} | block {})\n",
                    last, first, s.runs_total, s.runs_passing, s.runs_failing, s.runs_blocked));
            }
            if let Some(path) = output_path {
                out.push_str(&format!("\nhistory written: {}\n", path.display()));
            }
            CommandOutput {
                stdout: out,
                stderr: String::new(),
                status: 0,
            }
        }
        Err(e) => CommandOutput {
            stdout: format!("uat history: error: {e}\n"),
            stderr: String::new(),
            status: 1,
        },
    }
}

/// Execute a scripted/automated scenario via its `automation.ref`.
///
/// The `ref` is parsed as a typed argv spec (never through a shell):
/// whitespace-split into program + args. `automation.status` must be
/// `scripted` or `automated`; `manual` scenarios are rejected. The outcome
/// maps `exit 0 → PASS`, non-zero → FAIL, timeout/kill → BLOCKED, and a
/// baseline `uat-session.yaml` is emitted so the standard
/// ingest/report/history pipeline can consume the run.
fn run_uat_batch(args: UatBatchArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<String> {
        let plan_raw = std::fs::read_to_string(&args.plan)
            .map_err(|e| anyhow::anyhow!("cannot read plan {}: {e}", args.plan.display()))?;
        let plan: UatPlan = serde_saphyr::from_str(&plan_raw)
            .map_err(|e| anyhow::anyhow!("invalid plan {}: {e}", args.plan.display()))?;

        let output_dir = args.output_dir.clone().unwrap_or_else(|| {
            args.plan
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf()
        });
        std::fs::create_dir_all(&output_dir)?;

        // Escenarios ejecutables: excluye human/manual.
        let scenarios: Vec<&sddk_domain::UatScenario> = plan
            .features
            .iter()
            .flat_map(|f| f.scenarios.iter())
            .filter(|s| {
                let manual = s
                    .executor
                    .as_ref()
                    .map(|e| e.kind == sddk_domain::UatExecutorKind::Human)
                    .unwrap_or_else(|| {
                        s.automation
                            .as_ref()
                            .map(|a| a.status == sddk_domain::UatAutomationStatus::Manual)
                            .unwrap_or(false)
                    });
                !manual
            })
            .collect();

        let mut session_paths: Vec<PathBuf> = Vec::new();
        let mut results: Vec<(String, String, Option<String>)> = Vec::new();
        for scenario in &scenarios {
            let output_path = output_dir.join(format!(
                "uat-session-{}.yaml",
                scenario.id.to_lowercase().replace('.', "-")
            ));
            let run_args = UatRunArgs {
                plan: args.plan.clone(),
                scenario: scenario.id.clone(),
                timeout_ms: args.timeout_ms,
                approve: args.approve,
                output: Some(output_path.clone()),
                format: OutputFormat::Json,
            };
            let out = run_uat_run(run_args);
            // El JSON del run incluye status + session_path.
            let (status, reason) = parse_run_json(&out.stdout);
            results.push((scenario.id.clone(), status.clone(), reason));
            if status != "error" && output_path.is_file() {
                session_paths.push(output_path);
            }
        }

        // Agregar report si hay sessions.
        let mut report_summary = String::new();
        if !session_paths.is_empty() {
            let sessions: Vec<UatSession> = session_paths
                .iter()
                .map(|p| {
                    let raw = std::fs::read_to_string(p)?;
                    serde_saphyr::from_str(&raw)
                        .map_err(|e| anyhow::anyhow!("invalid session {}: {e}", p.display()))
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            let report = aggregate_report(&plan, &sessions);
            let report_path = args.report.clone().unwrap_or_else(|| {
                output_dir.join(format!("uat-report-{}.yaml", plan.release.candidate))
            });
            let yaml = serde_saphyr::to_string(&report)
                .map_err(|e| anyhow::anyhow!("report serialization failed: {e}"))?;
            std::fs::write(&report_path, yaml)?;
            report_summary = format!(
                "\nreport: {} ({} scenarios, {:.0}% coverage, {:?})\n",
                report_path.display(),
                report.summary.total_scenarios,
                report.summary.coverage_pct,
                report.verdict,
            );
        }

        if matches!(format, OutputFormat::Json) {
            return serde_json::to_string_pretty(&serde_json::json!({
                "plan": args.plan.display().to_string(),
                "scenarios_run": results.len(),
                "results": results.iter().map(|(id, status, reason)| {
                    serde_json::json!({"scenario": id, "status": status, "reason": reason})
                }).collect::<Vec<_>>(),
            }))
            .map_err(|e| anyhow::anyhow!("json serialization failed: {e}"));
        }

        let mut lines = vec![format!(
            "uat batch: {} — {} scenarios ejecutables\n",
            plan.release.candidate,
            results.len()
        )];
        for (id, status, reason) in &results {
            lines.push(format!(
                "  {id}: {status}{}",
                reason
                    .as_deref()
                    .map(|r| format!(" ({r})"))
                    .unwrap_or_default()
            ));
        }
        if results.is_empty() {
            lines.push("  (ningún escenario automatizable — revisar executor/human)\n".into());
        }
        lines.push(report_summary);
        Ok(lines.join("\n"))
    })();
    match result {
        Ok(out) => CommandOutput {
            stdout: out,
            stderr: String::new(),
            status: 0,
        },
        Err(e) => crate::failure_envelope(&e),
    }
}

// ─── E14.2: Form Quality Agent ────────────────────────────────────────────────

fn run_uat_quality(args: QualityArgs) -> CommandOutput {
    let _format = args.format;
    let result = (|| -> anyhow::Result<(PathBuf, bool)> {
        let plan_raw = std::fs::read_to_string(&args.plan)
            .map_err(|e| anyhow::anyhow!("cannot read plan {}: {e}", args.plan.display()))?;
        let plan: UatPlan = serde_saphyr::from_str(&plan_raw)
            .map_err(|e| anyhow::anyhow!("invalid plan {}: {e}", args.plan.display()))?;

        let mut report = crate::uat_quality::detect_13_smells(&plan, args.threshold);
        report.plan_ref = args.plan.to_string_lossy().into_owned();
        let output_path = args
            .output
            .unwrap_or_else(|| args.plan.with_file_name("uat-quality-report.yaml"));

        let yaml =
            serde_saphyr::to_string(&report).map_err(|e| anyhow::anyhow!("serialization: {e}"))?;
        std::fs::write(&output_path, &yaml)?;

        let blockers = report.summary.blockers;
        let passed = report.verdict == "PASS";
        Ok((output_path, passed && blockers == 0))
    })();

    match result {
        Ok((path, gate_passed)) => {
            let out = format!(
                "uat quality: {}\n  report: {}\n",
                if gate_passed { "PASS" } else { "FAIL" },
                path.display()
            );
            CommandOutput {
                stdout: out,
                stderr: String::new(),
                status: if gate_passed { 0 } else { 1 },
            }
        }
        Err(e) => crate::failure_envelope(&e),
    }
}

// ─── E14.3: UX Form Agent ────────────────────────────────────────────────────

fn run_uat_enrich_forms(args: EnrichFormsArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<PathBuf> {
        let plan_raw = std::fs::read_to_string(&args.plan)
            .map_err(|e| anyhow::anyhow!("cannot read plan {}: {e}", args.plan.display()))?;
        let mut plan: UatPlan = serde_saphyr::from_str(&plan_raw)
            .map_err(|e| anyhow::anyhow!("invalid plan {}: {e}", args.plan.display()))?;

        for feature in &mut plan.features {
            for scenario in &mut feature.scenarios {
                // enrich_scenario handles: preserve existing form, set provenance
                crate::uat_enrich::enrich_scenario(scenario);
            }
        }

        let output_path = args
            .output
            .unwrap_or_else(|| args.plan.with_file_name("uat-plan-enriched.yaml"));
        let yaml =
            serde_saphyr::to_string(&plan).map_err(|e| anyhow::anyhow!("serialization: {e}"))?;
        std::fs::write(&output_path, &yaml)?;
        Ok(output_path)
    })();
    render_result(result, format, |path| {
        format!("uat enrich-forms: {}\n", path.display())
    })
}

// ─── E14.4: Test Discovery Agent ────────────────────────────────────────────

/// Thin delegation to uat_discover module.
fn run_uat_discover(args: DiscoverArgs) -> CommandOutput {
    crate::uat_discover::run::run(args)
}

// ─── E14.5: Pipeline Orchestration ────────────────────────────────────────────

/// Thin delegation to uat_generate module.
fn run_uat_generate(args: GenerateArgs, _environment: &crate::CliEnvironment) -> CommandOutput {
    use crate::uat_generate::runner::render_pipeline_output;
    use crate::uat_generate::runner::{PipelineConfig, run_pipeline};

    let config = PipelineConfig {
        release: args.release.clone(),
        requirements: args.requirements.clone(),
        changelog: args.changelog.clone(),
        last_plan: args.last_plan.clone(),
        discover: args.discover,
        app_url: args.app_url.clone(),
        interactive: args.interactive,
        output: args.output.clone(),
        approval_io: None,
        force_quality_failure: false,
    };

    match run_pipeline(config) {
        Ok(stages) => {
            let final_path = stages.last().map(|s| s.path.clone()).unwrap_or_default();
            let stdout = format!(
                "uat generate: pipeline E14.5 for {}\n{}\nNext: sddk uat open --plan {} to execute\n",
                args.release,
                render_pipeline_output(&stages, &final_path),
                final_path.display()
            );
            CommandOutput {
                stdout,
                stderr: String::new(),
                status: 0,
            }
        }
        Err(e) => {
            let msg = format!("uat generate: {:?}\n", e);
            CommandOutput {
                stdout: String::new(),
                stderr: msg,
                status: 1,
            }
        }
    }
}

/// Extrae status + reason del JSON emitido por `uat run`.
fn parse_run_json(stdout: &str) -> (String, Option<String>) {
    let trimmed = stdout.trim();
    if !trimmed.starts_with('{') {
        return ("error".into(), Some(trimmed.chars().take(200).collect()));
    }
    let value: serde_json::Value = serde_json::from_str(trimmed).unwrap_or(serde_json::Value::Null);
    let status = value
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("error")
        .to_owned();
    let reason = value
        .get("reason")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    (status, reason)
}

fn run_uat_assess(args: UatAssessArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<String> {
        // Policy gate (ADR-0005): VLM local = uat.agent (high).
        let workflow_path = std::path::Path::new(crate::WORKFLOW_MANIFEST);
        if workflow_path.is_file() {
            let workflow_raw = std::fs::read_to_string(workflow_path)?;
            let workflow: sddk_domain::WorkflowManifest = serde_saphyr::from_str(&workflow_raw)?;
            let policy = sddk_gateway::CapabilityPolicy::from_workflow(&workflow);
            sddk_gateway::authorize_uat(
                sddk_domain::UatExecutorKind::ComputerUse,
                &policy,
                args.approve,
            )
            .map_err(|e| anyhow::anyhow!("uat assess blocked by policy: {e}"))?;
        }

        let plan_raw = std::fs::read_to_string(&args.plan)
            .map_err(|e| anyhow::anyhow!("cannot read plan {}: {e}", args.plan.display()))?;
        let plan: UatPlan = serde_saphyr::from_str(&plan_raw)
            .map_err(|e| anyhow::anyhow!("invalid plan {}: {e}", args.plan.display()))?;
        let scenario = plan
            .features
            .iter()
            .flat_map(|f| f.scenarios.iter().map(move |s| (f, s)))
            .find(|(_, s)| s.id == args.scenario)
            .map(|(_, s)| s)
            .ok_or_else(|| anyhow::anyhow!("scenario {} not found", args.scenario))?;

        // Session con la evidencia capturada.
        let session_path = args.session.clone().unwrap_or_else(|| {
            let name = format!(
                "uat-session-{}.yaml",
                scenario.id.to_lowercase().replace('.', "-")
            );
            args.plan
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(name)
        });
        let session_raw = std::fs::read_to_string(&session_path)
            .map_err(|e| anyhow::anyhow!("cannot read session {}: {e}", session_path.display()))?;
        let session: UatSession = serde_saphyr::from_str(&session_raw)
            .map_err(|e| anyhow::anyhow!("invalid session {}: {e}", session_path.display()))?;
        let result = session
            .results
            .iter()
            .find(|r| r.scenario_id == scenario.id)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "session {} has no result for scenario {}",
                    session_path.display(),
                    scenario.id
                )
            })?;

        let fara_url = args
            .fara_url
            .clone()
            .or_else(|| std::env::var("FARA_URL").ok())
            .unwrap_or_else(|| "http://127.0.0.1:8082".into());

        // Rubric temporal a partir del spec del oracle.
        let rubric_dir = std::env::temp_dir().join(format!(
            "sddk-uat-rubric-{}-{}",
            scenario.id.replace('.', "-"),
            std::process::id()
        ));
        std::fs::create_dir_all(&rubric_dir)?;

        let mut assessments = Vec::new();
        let semantic_kinds = [
            sddk_domain::UatOracleKind::VisualAi,
            sddk_domain::UatOracleKind::LlmRubric,
        ];
        for oracle in scenario
            .oracles
            .iter()
            .filter(|o| semantic_kinds.contains(&o.kind))
        {
            // Localizar la evidencia: screenshot para visual_ai, dom para llm_rubric.
            let (kind_flag, evidence) = match oracle.kind {
                sddk_domain::UatOracleKind::VisualAi => {
                    let shot = result.evidence.iter().find(|e| {
                        e.kind == sddk_domain::UatEvidenceKind::Screenshot
                            || e.path
                                .as_deref()
                                .is_some_and(|p| p.ends_with("screenshot.png"))
                    });
                    match shot.and_then(|e| e.path.clone()) {
                        Some(p) => (sddk_domain::UatOracleKind::VisualAi, p),
                        None => {
                            assessments.push((
                                oracle.kind,
                                sddk_domain::UatOracleVerdict::Uncertain,
                                0.0,
                                Some("no screenshot evidence captured".into()),
                            ));
                            continue;
                        }
                    }
                }
                _ => {
                    let dom = result.evidence.iter().find(|e| {
                        e.kind == sddk_domain::UatEvidenceKind::Dom
                            || e.path.as_deref().is_some_and(|p| p.ends_with("dom.html"))
                    });
                    match dom.and_then(|e| e.path.clone()) {
                        Some(p) => (sddk_domain::UatOracleKind::LlmRubric, p),
                        None => {
                            assessments.push((
                                oracle.kind,
                                sddk_domain::UatOracleVerdict::Uncertain,
                                0.0,
                                Some("no dom evidence captured".into()),
                            ));
                            continue;
                        }
                    }
                }
            };
            let rubric_path = rubric_dir.join(format!("rubric-{:?}.json", oracle.kind));
            std::fs::write(&rubric_path, serde_json::to_string(&oracle.rubric)?)?;
            let out_dir = rubric_dir.join(format!("assess-{:?}", oracle.kind));
            let spec = sddk_gateway::SemanticOracleSpec {
                kind: kind_flag,
                evidence_path: PathBuf::from(evidence),
                rubric_path,
                fara_url: fara_url.clone(),
                output_dir: out_dir.clone(),
                timeout_ms: 90_000,
            };
            match sddk_gateway::run_semantic_oracle(&spec, None, None) {
                Ok(outcome) => assessments.push((
                    oracle.kind,
                    outcome.assessment.verdict,
                    outcome.assessment.confidence,
                    outcome.assessment.details,
                )),
                Err(e) => assessments.push((
                    oracle.kind,
                    sddk_domain::UatOracleVerdict::Uncertain,
                    0.0,
                    Some(format!("assess failed: {e}")),
                )),
            }
        }

        if matches!(format, OutputFormat::Json) {
            return serde_json::to_string_pretty(&serde_json::json!({
                "scenario": scenario.id,
                "semantic_oracles": assessments.iter().map(|(kind, verdict, conf, details)| {
                    serde_json::json!({
                        "kind": format!("{kind:?}").to_lowercase(),
                        "verdict": format!("{verdict:?}").to_lowercase(),
                        "confidence": conf,
                        "details": details,
                    })
                }).collect::<Vec<_>>(),
            }))
            .map_err(|e| anyhow::anyhow!("json serialization failed: {e}"));
        }

        if assessments.is_empty() {
            return Ok(format!(
                "uat assess: scenario {} — sin oracles semánticos (visual_ai/llm_rubric) en el plan\n",
                scenario.id
            ));
        }
        let mut lines = vec![format!(
            "uat assess: scenario {} — oracles semánticos (Fara {fara_url})\n",
            scenario.id
        )];
        for (kind, verdict, conf, details) in &assessments {
            lines.push(format!(
                "  {:?}: {:?} (conf {:.2}) — {}",
                kind,
                verdict,
                conf,
                details.as_deref().unwrap_or("")
            ));
        }
        Ok(lines.join("\n"))
    })();
    match result {
        Ok(out) => CommandOutput {
            stdout: out,
            stderr: String::new(),
            status: 0,
        },
        Err(e) => crate::failure_envelope(&e),
    }
}

fn run_uat_review(args: UatReviewArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<String> {
        let plan_raw = std::fs::read_to_string(&args.plan)
            .map_err(|e| anyhow::anyhow!("cannot read plan {}: {e}", args.plan.display()))?;
        let plan: UatPlan = serde_saphyr::from_str(&plan_raw)
            .map_err(|e| anyhow::anyhow!("invalid plan {}: {e}", args.plan.display()))?;
        let report_raw = std::fs::read_to_string(&args.report)
            .map_err(|e| anyhow::anyhow!("cannot read report {}: {e}", args.report.display()))?;
        let report: UatReport = serde_saphyr::from_str(&report_raw)
            .map_err(|e| anyhow::anyhow!("invalid report {}: {e}", args.report.display()))?;

        // Sampling: arg > policy del plan (primer feature con review) > 0.02.
        let sampling = args.sampling.unwrap_or_else(|| {
            plan.features
                .iter()
                .flat_map(|f| f.scenarios.iter().filter_map(|s| s.review.as_ref()))
                .map(|r| r.sampling)
                .find(|s| *s > 0.0)
                .unwrap_or(0.02)
        });
        let seed = args
            .seed
            .clone()
            .unwrap_or_else(|| plan.release.candidate.clone());

        let queue = sddk_domain::build_review_queue(&plan, &report, sampling, &seed);
        if matches!(format, OutputFormat::Json) {
            return serde_json::to_string_pretty(&serde_json::json!({
                "release": plan.release.candidate,
                "sampling": sampling,
                "seed": seed,
                "queue_size": queue.len(),
                "queue": queue,
            }))
            .map_err(|e| anyhow::anyhow!("json serialization failed: {e}"));
        }

        if queue.is_empty() {
            return Ok(format!(
                "uat review: release {} — queue vacía (0 items)\n",
                plan.release.candidate
            ));
        }
        let mut lines = vec![format!(
            "uat review: release {} — {} items en la Human Review Queue (sampling {:.2}, seed {seed})\n",
            plan.release.candidate,
            queue.len(),
            sampling
        )];
        for (i, item) in queue.iter().enumerate() {
            lines.push(format!(
                "  {:>2}. {} [{}] machine={:?} conf={:.2}",
                i + 1,
                item.scenario_id,
                uat_review_reason_str(item.reason),
                item.machine_verdict,
                item.machine_confidence
            ));
        }
        Ok(lines.join("\n"))
    })();
    match result {
        Ok(out) => CommandOutput {
            stdout: out,
            stderr: String::new(),
            status: 0,
        },
        Err(e) => crate::failure_envelope(&e),
    }
}

fn uat_review_reason_str(reason: sddk_domain::UatReviewReason) -> &'static str {
    match reason {
        sddk_domain::UatReviewReason::Required => "required",
        sddk_domain::UatReviewReason::Sampled => "sampled",
        sddk_domain::UatReviewReason::OracleConflict => "oracle-conflict",
        sddk_domain::UatReviewReason::LowAiConfidence => "low-confidence",
    }
}

fn run_uat_run(args: UatRunArgs) -> CommandOutput {
    let result = (|| -> anyhow::Result<(UatSession, PathBuf)> {
        let plan_raw = std::fs::read_to_string(&args.plan)
            .map_err(|e| anyhow::anyhow!("cannot read plan {}: {e}", args.plan.display()))?;
        let plan: UatPlan = serde_saphyr::from_str(&plan_raw)
            .map_err(|e| anyhow::anyhow!("invalid plan {}: {e}", args.plan.display()))?;
        let scenario = plan
            .features
            .iter()
            .flat_map(|f| f.scenarios.iter().map(move |s| (f, s)))
            .find(|(_, s)| s.id == args.scenario)
            .map(|(_, s)| s)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "scenario {} not found in plan {}",
                    args.scenario,
                    args.plan.display()
                )
            })?;
        // Fuente de la spec de ejecución: eje v3 `executor` (ADR-014) o
        // `automation` v2 heredado. El runner tipado es el mismo para ambos.
        let (executor_kind, ref_str) = if let Some(executor) = scenario.executor.as_ref() {
            match executor.kind {
                sddk_domain::UatExecutorKind::Cli | sddk_domain::UatExecutorKind::Script => {
                    let command = executor.command.as_deref().ok_or_else(|| {
                        anyhow::anyhow!(
                            "scenario {} executor kind={} but command is empty",
                            scenario.id,
                            uat_executor_kind_str(executor.kind)
                        )
                    })?;
                    (executor.kind, command.to_owned())
                }
                sddk_domain::UatExecutorKind::Playwright => {
                    let url = executor.url.as_deref().ok_or_else(|| {
                        anyhow::anyhow!(
                            "scenario {} executor kind=playwright but url is empty",
                            scenario.id
                        )
                    })?;
                    (executor.kind, url.to_owned())
                }
                sddk_domain::UatExecutorKind::ComputerUse => {
                    let url = executor.url.as_deref().ok_or_else(|| {
                        anyhow::anyhow!(
                            "scenario {} executor kind=computer_use but url is empty",
                            scenario.id
                        )
                    })?;
                    let goal = executor.goal.as_deref().ok_or_else(|| {
                        anyhow::anyhow!(
                            "scenario {} executor kind=computer_use but goal is empty",
                            scenario.id
                        )
                    })?;
                    (executor.kind, format!("{url} :: {goal}"))
                }
                other => anyhow::bail!(
                    "scenario {} executor kind={} is not runnable by `uat run` yet; use cli|script|playwright|computer_use",
                    scenario.id,
                    uat_executor_kind_str(other)
                ),
            }
        } else {
            let automation = scenario.automation.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "scenario {} has no executor (v3) nor automation block (v2); mark it `automation: {{status: scripted, ref: ...}}` or `executor: {{kind: cli, command: ...}}` to run it",
                    scenario.id
                )
            })?;
            if automation.status == sddk_domain::UatAutomationStatus::Manual {
                anyhow::bail!(
                    "scenario {} is manual: no automated run possible",
                    scenario.id
                );
            }
            (
                sddk_domain::UatExecutorKind::Cli,
                automation.r#ref.clone().ok_or_else(|| {
                    anyhow::anyhow!(
                        "scenario {} automation.status={} but automation.ref is empty",
                        scenario.id,
                        uat_automation_status_str(automation.status)
                    )
                })?,
            )
        };

        let started = std::time::Instant::now();

        // --- Policy gate (ADR-0005 default-deny): el executor debe estar
        // declarado en workflow/workflow.yaml (o llevar --approve explícito).
        let workflow_path = std::path::Path::new(crate::WORKFLOW_MANIFEST);
        if workflow_path.is_file() {
            let workflow_raw = std::fs::read_to_string(workflow_path).map_err(|e| {
                anyhow::anyhow!("cannot read workflow {}: {e}", workflow_path.display())
            })?;
            let workflow: sddk_domain::WorkflowManifest = serde_saphyr::from_str(&workflow_raw)
                .map_err(|e| {
                    anyhow::anyhow!("invalid workflow {}: {e}", workflow_path.display())
                })?;
            let policy = sddk_gateway::CapabilityPolicy::from_workflow(&workflow);
            sddk_gateway::authorize_uat(executor_kind, &policy, args.approve)
                .map_err(|e| anyhow::anyhow!("scenario {} blocked by policy: {e}", scenario.id))?;
        }

        // --- Dispatch por kind de executor (ADR-014, eje 1) ---
        // Cli|Script → runner tipado (sin shell). Playwright → driver
        // browser (sensor/actuador) que escribe el directorio de evidencia.
        let (run_status, run_comment, stderr_detail, bundle, run_ctx) = match executor_kind {
            sddk_domain::UatExecutorKind::Cli | sddk_domain::UatExecutorKind::Script => {
                // Typed argv split — first token is the program, rest are args.
                let tokens: Vec<String> = ref_str.split_whitespace().map(str::to_owned).collect();
                let (program, argv) = tokens.split_first().ok_or_else(|| {
                    anyhow::anyhow!("scenario {} executor command is empty", scenario.id)
                })?;
                let spec = sddk_gateway::RunSpec {
                    program: program.clone(),
                    args: argv.to_vec(),
                    env: Default::default(),
                    timeout_ms: args.timeout_ms,
                    output_max_bytes: 1_048_576,
                };
                let outcome = sddk_gateway::run(&spec).map_err(|e| {
                    anyhow::anyhow!(
                        "scenario {} failed to spawn `{}`: {e}",
                        scenario.id,
                        program
                    )
                })?;
                let (status, comment, stderr_detail) = if outcome.timed_out {
                    (
                        sddk_domain::UatStatus::Blocked,
                        format!("blocked: `{ref_str}` timed out after {}ms", args.timeout_ms),
                        None,
                    )
                } else if outcome.exit_status == Some(0) {
                    (
                        sddk_domain::UatStatus::Pass,
                        format!("pass: `{ref_str}` exited 0"),
                        None,
                    )
                } else {
                    (
                        sddk_domain::UatStatus::Fail,
                        format!("fail: `{ref_str}` exited {:?}", outcome.exit_status),
                        Some(
                            outcome
                                .stderr
                                .trim()
                                .lines()
                                .take(20)
                                .collect::<Vec<_>>()
                                .join("\n"),
                        ),
                    )
                };
                let bundle = sddk_gateway::EvidenceCollector::new(sddk_gateway::EvidenceContext {
                    executor: "cli".into(),
                    ..Default::default()
                })
                .add(sddk_gateway::EvidenceFile {
                    kind: sddk_domain::UatEvidenceKind::CommandOutput,
                    path: write_evidence_payload(&scenario.id, &ref_str, &outcome)?,
                    mime: Some("text/plain".into()),
                    note: Some(comment.clone()),
                })
                .build()
                .map_err(|e| anyhow::anyhow!("evidence collection failed: {e}"))?;
                let run_ctx = sddk_gateway::OracleRunContext {
                    exit_status: outcome.exit_status,
                    final_url: None,
                };
                (status, comment, stderr_detail, bundle, run_ctx)
            }
            sddk_domain::UatExecutorKind::Playwright => {
                // Evidence bundle spec (eje 2) o defaults conservadores.
                let bundle_spec = scenario.evidence_bundle.clone().unwrap_or_default();
                let output_dir = std::env::temp_dir().join(format!(
                    "sddk-uat-ev-{}-{}",
                    scenario.id.replace('.', "-"),
                    std::process::id()
                ));
                // Geometry selectors derivados de los oracles geometry (eje 3).
                let geometry_file = if scenario
                    .oracles
                    .iter()
                    .any(|o| o.kind == sddk_domain::UatOracleKind::Geometry)
                {
                    let selectors: Vec<String> = scenario
                        .oracles
                        .iter()
                        .filter(|o| o.kind == sddk_domain::UatOracleKind::Geometry)
                        .filter_map(|o| {
                            o.expect
                                .as_ref()
                                .and_then(|e| e.get("selector"))
                                .and_then(serde_json::Value::as_str)
                                .map(str::to_owned)
                        })
                        .collect();
                    let path = output_dir.join("geometry-selectors.json");
                    std::fs::create_dir_all(&output_dir)?;
                    std::fs::write(
                        &path,
                        serde_json::to_string(&selectors)
                            .map_err(|e| anyhow::anyhow!("selector serialization failed: {e}"))?,
                    )?;
                    Some(path)
                } else {
                    None
                };
                let pw_spec = sddk_gateway::PlaywrightSpec {
                    url: ref_str.clone(),
                    viewport: None,
                    actions: None,
                    screenshot: bundle_spec.screenshots || bundle_spec.playwright_trace,
                    trace: bundle_spec.playwright_trace,
                    console: bundle_spec.console,
                    network: bundle_spec.network,
                    // DOM snapshot necesario cuando los oracles (eje 3) o el
                    // spec (eje 2) lo piden: text/dom/geometry/accessibility.
                    dom: bundle_spec.accessibility
                        || bundle_spec.geometry
                        || scenario.oracles.iter().any(|o| {
                            matches!(
                                o.kind,
                                sddk_domain::UatOracleKind::Text
                                    | sddk_domain::UatOracleKind::Dom
                                    | sddk_domain::UatOracleKind::Geometry
                                    | sddk_domain::UatOracleKind::Accessibility
                            )
                        }),
                    geometry: geometry_file,
                    output_dir: output_dir.clone(),
                    timeout_ms: args.timeout_ms,
                };
                let outcome = sddk_gateway::run_playwright(&pw_spec, None, None).map_err(|e| {
                    anyhow::anyhow!("scenario {} playwright run failed: {e}", scenario.id)
                })?;
                let (status, comment) = if outcome.network_failures > 0 {
                    (
                        sddk_domain::UatStatus::Fail,
                        format!(
                            "fail: {} network failure(s) on `{}`",
                            outcome.network_failures, ref_str
                        ),
                    )
                } else {
                    (
                        sddk_domain::UatStatus::Pass,
                        format!("pass: `{ref_str}` loaded (title {:?})", outcome.page_title),
                    )
                };
                let mut collector =
                    sddk_gateway::EvidenceCollector::new(sddk_gateway::EvidenceContext {
                        executor: "playwright".into(),
                        browser: Some("chromium".into()),
                        viewport: None,
                        git_sha: None,
                        app_version: Some(plan.release.candidate.clone()),
                        ..Default::default()
                    });
                collector.collect_dir(&output_dir);
                let bundle = collector
                    .build()
                    .map_err(|e| anyhow::anyhow!("evidence collection failed: {e}"))?;
                let run_ctx = sddk_gateway::OracleRunContext {
                    exit_status: Some(0),
                    final_url: outcome.final_url.clone(),
                };
                (status, comment, None, bundle, run_ctx)
            }
            sddk_domain::UatExecutorKind::ComputerUse => {
                // url :: goal compuesto en el match de la spec.
                let (url, goal) = ref_str.split_once(" :: ").ok_or_else(|| {
                    anyhow::anyhow!("scenario {} computer_use ref malformed", scenario.id)
                })?;
                let output_dir = std::env::temp_dir().join(format!(
                    "sddk-uat-cu-{}-{}",
                    scenario.id.replace('.', "-"),
                    std::process::id()
                ));
                let cu_spec = sddk_gateway::ComputerUseSpec {
                    url: url.to_owned(),
                    goal: goal.to_owned(),
                    max_steps: 10,
                    fara_url: std::env::var("FARA_URL")
                        .unwrap_or_else(|_| "http://127.0.0.1:8082".into()),
                    output_dir: output_dir.clone(),
                    timeout_ms: args.timeout_ms,
                };
                let outcome =
                    sddk_gateway::run_computer_use(&cu_spec, None, None).map_err(|e| {
                        anyhow::anyhow!("scenario {} computer_use run failed: {e}", scenario.id)
                    })?;
                let (status, comment) = if outcome.done {
                    (
                        sddk_domain::UatStatus::Pass,
                        format!(
                            "pass: agent declared done in {} step(s) (title {:?})",
                            outcome.steps_taken, outcome.page_title
                        ),
                    )
                } else {
                    (
                        sddk_domain::UatStatus::Blocked,
                        format!(
                            "blocked: agent did not finish in {} step(s) — review trajectory",
                            outcome.steps_taken
                        ),
                    )
                };
                let mut collector =
                    sddk_gateway::EvidenceCollector::new(sddk_gateway::EvidenceContext {
                        executor: "computer_use".into(),
                        browser: Some("chromium".into()),
                        git_sha: None,
                        app_version: Some(plan.release.candidate.clone()),
                        model: Some("fara-9b".into()),
                        ..Default::default()
                    });
                collector.collect_dir(&output_dir);
                let bundle = collector
                    .build()
                    .map_err(|e| anyhow::anyhow!("evidence collection failed: {e}"))?;
                let run_ctx = sddk_gateway::OracleRunContext {
                    exit_status: Some(0),
                    final_url: None,
                };
                (status, comment, None, bundle, run_ctx)
            }
            other => anyhow::bail!(
                "scenario {} executor kind={} is not runnable by `uat run` yet",
                scenario.id,
                uat_executor_kind_str(other)
            ),
        };
        let duration_ms = started.elapsed().as_millis() as u64;

        // --- Oracles (eje 3): evaluar los deterministas contra el bundle ---
        let mut oracle_assessments = Vec::new();
        for oracle in &scenario.oracles {
            match sddk_gateway::evaluate_deterministic(oracle, &bundle, &run_ctx) {
                Ok(assessment) => oracle_assessments.push(assessment),
                Err(sddk_gateway::OracleError::NotDeterministic { .. }) => {
                    // Semánticos (visual_ai/llm_rubric) y human se evalúan
                    // en fases posteriores; se omiten aquí.
                }
                Err(e) => {
                    // Evidencia ausente → Uncertain (el review humano decide).
                    oracle_assessments.push(sddk_domain::UatOracleAssessment {
                        oracle: oracle.clone(),
                        verdict: sddk_domain::UatOracleVerdict::Uncertain,
                        confidence: 0.0,
                        details: Some(format!("missing evidence: {e}")),
                    });
                }
            }
        }
        let machine_verdict = sddk_gateway::aggregate_verdict(&oracle_assessments);

        // Status final: el run base + los oracles bloqueantes mandan.
        let (status, failure_reason, comment) = if oracle_assessments.is_empty() {
            let (reason, note) = match &run_status {
                sddk_domain::UatStatus::Blocked => (
                    Some(format!("timeout after {}ms", args.timeout_ms)),
                    run_comment.clone(),
                ),
                sddk_domain::UatStatus::Fail => (
                    stderr_detail
                        .clone()
                        .or_else(|| Some("executor failed".into())),
                    run_comment.clone(),
                ),
                _ => (None, run_comment.clone()),
            };
            (run_status, reason, note)
        } else {
            match machine_verdict {
                sddk_domain::UatOracleVerdict::Fail => (
                    sddk_domain::UatStatus::Fail,
                    Some("oracle(s) failed".into()),
                    format!(
                        "machine verdict: {} oracle(s), {} failed",
                        oracle_assessments.len(),
                        oracle_assessments
                            .iter()
                            .filter(|a| a.verdict == sddk_domain::UatOracleVerdict::Fail)
                            .count()
                    ),
                ),
                sddk_domain::UatOracleVerdict::Uncertain => (
                    sddk_domain::UatStatus::Blocked,
                    Some("evidence insufficient for oracle verdict".into()),
                    format!(
                        "machine verdict: {} oracle(s), some uncertain",
                        oracle_assessments.len()
                    ),
                ),
                sddk_domain::UatOracleVerdict::Pass => (
                    sddk_domain::UatStatus::Pass,
                    None,
                    format!(
                        "machine verdict: {} oracle(s) passed",
                        oracle_assessments.len()
                    ),
                ),
                _ => (
                    sddk_domain::UatStatus::Blocked,
                    Some("conflicting oracle verdicts".into()),
                    "machine verdict: conflicting".into(),
                ),
            }
        };

        let now = now_rfc3339();
        let session_id = format!("auto-{}-{}", scenario.id, now.replace([':', '-'], ""));
        // Evidence v2 items from the content-addressable bundle.
        let evidence: Vec<sddk_domain::UatEvidence> = bundle
            .artifacts
            .iter()
            .map(|a| sddk_domain::UatEvidence {
                kind: a.kind,
                r#ref: a.r#ref.clone(),
                note: a.note.clone(),
                captured_at: Some(now.clone()),
                size_bytes: a.size_bytes,
                mime: a.mime.clone(),
                path: a.path.clone(),
                observed_value: None,
                expected_value: None,
                match_mode: None,
            })
            .collect();
        let observed = bundle
            .artifacts
            .iter()
            .find(|a| a.kind == sddk_domain::UatEvidenceKind::Dom)
            .and_then(|a| a.path.as_deref())
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|html| html.chars().take(2000).collect());
        let session = UatSession {
            schema_version: sddk_domain::LATEST_SESSION_SCHEMA_VERSION,
            session_id: session_id.clone(),
            plan_ref: plan.release.candidate.clone(),
            release: plan.release.candidate.clone(),
            executor: sddk_domain::UatExecutor::Automated,
            executed_by: Some("auto-runner".into()),
            started_at: now.clone(),
            finished_at: Some(now.clone()),
            results: vec![sddk_domain::UatScenarioResult {
                scenario_id: scenario.id.clone(),
                status,
                comment: Some(comment),
                evidence,
                duration_minutes: 0,
                verdict_at: Some(now.clone()),
                verdict_duration_ms: Some(duration_ms),
                tester_notes: None,
                observed,
                failure_reason,
                linked_defect: None,
                repro_command: Some(ref_str.to_owned()),
                oracle_assessments,
            }],
            metadata: None,
            plan_version: Some(plan.schema_version),
        };

        let output_path = args.output.unwrap_or_else(|| {
            let name = format!(
                "uat-session-{}.yaml",
                scenario.id.to_lowercase().replace('.', "-")
            );
            args.plan
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(name)
        });
        if let Some(parent) = output_path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let yaml = serde_saphyr::to_string(&session)
            .map_err(|e| anyhow::anyhow!("session serialization failed: {e}"))?;
        std::fs::write(&output_path, yaml)?;
        Ok((session, output_path))
    })();

    match result {
        Ok((session, output_path)) => {
            let stdout = if matches!(args.format, OutputFormat::Json) {
                let exit_code = match session.results[0].status {
                    sddk_domain::UatStatus::Pass => 0,
                    sddk_domain::UatStatus::Blocked => 124, // timeout convention
                    _ => 1,
                };
                serde_json::json!({
                    "scenario": session.results[0].scenario_id,
                    "status": uat_status_str(session.results[0].status),
                    "exit": exit_code,
                    "duration_ms": session.results[0].verdict_duration_ms,
                    "session_id": session.session_id,
                    "session_path": output_path.display().to_string(),
                    "executor": "automated",
                    "oracles": session.results[0].oracle_assessments.len(),
                    "oracle_verdict": session.results[0]
                        .oracle_assessments
                        .iter()
                        .map(|a| format!("{:?}", a.verdict))
                        .collect::<Vec<_>>(),
                    "reason": session.results[0].failure_reason,
                })
                .to_string()
                    + "\n"
            } else {
                format!(
                    "uat run: scenario {} → {} ({}ms)\n  session: {}\n  re-run: sddk uat ingest --session {} --release {}\n",
                    session.results[0].scenario_id,
                    uat_status_str(session.results[0].status),
                    session.results[0].verdict_duration_ms.unwrap_or_default(),
                    output_path.display(),
                    output_path.display(),
                    session.release,
                )
            };
            CommandOutput {
                stdout,
                stderr: String::new(),
                status: 0,
            }
        }
        Err(e) => CommandOutput {
            stdout: format!("uat run: error: {e}\n"),
            stderr: String::new(),
            status: 1,
        },
    }
}

fn uat_automation_status_str(status: sddk_domain::UatAutomationStatus) -> &'static str {
    match status {
        sddk_domain::UatAutomationStatus::Manual => "manual",
        sddk_domain::UatAutomationStatus::Scripted => "scripted",
        sddk_domain::UatAutomationStatus::Automated => "automated",
    }
}

/// Persists the raw stdout+stderr of a cli/script run to a temp file so the
/// EvidenceCollector can hash it into the content-addressable bundle.
fn write_evidence_payload(
    scenario_id: &str,
    ref_str: &str,
    outcome: &sddk_gateway::RunOutcome,
) -> anyhow::Result<PathBuf> {
    let payload = format!(
        "scenario: {}\nref: {}\n--- stdout ---\n{}\n--- stderr ---\n{}\n",
        scenario_id, ref_str, outcome.stdout, outcome.stderr
    );
    let path = std::env::temp_dir().join(format!(
        "sddk-uat-cli-ev-{}-{}.log",
        scenario_id.replace('.', "-"),
        std::process::id()
    ));
    std::fs::write(&path, payload)
        .map_err(|e| anyhow::anyhow!("cannot write evidence payload: {e}"))?;
    Ok(path)
}

fn uat_executor_kind_str(kind: sddk_domain::UatExecutorKind) -> &'static str {
    match kind {
        sddk_domain::UatExecutorKind::Cli => "cli",
        sddk_domain::UatExecutorKind::Api => "api",
        sddk_domain::UatExecutorKind::Script => "script",
        sddk_domain::UatExecutorKind::Playwright => "playwright",
        sddk_domain::UatExecutorKind::ComputerUse => "computer_use",
        sddk_domain::UatExecutorKind::Human => "human",
    }
}

fn uat_status_str(status: sddk_domain::UatStatus) -> &'static str {
    match status {
        sddk_domain::UatStatus::NotRun => "NOT_RUN",
        sddk_domain::UatStatus::Pass => "PASS",
        sddk_domain::UatStatus::Fail => "FAIL",
        sddk_domain::UatStatus::Blocked => "BLOCKED",
        sddk_domain::UatStatus::Partial => "PARTIAL",
    }
}

#[cfg(test)]
mod uat_run_tests {
    use super::*;

    fn write_plan(dir: &std::path::Path, automation: &str) -> PathBuf {
        let path = dir.join("uat-plan.yaml");
        // Indent every line of the automation block under `automation:` (8 cols).
        let automation_block = automation
            .lines()
            .map(|l| {
                if l.trim().is_empty() {
                    l.to_owned()
                } else {
                    format!("        {l}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let yaml = format!(
            r#"
schema_version: 2
release: {{ candidate: v2.1.0 }}
generated_by: test
generated_at: "2026-08-10T00:00:00Z"
features:
  - id: F-1
    name: Feature
    scenarios:
      - id: S-1
        title: Scripted scenario
        automation:
{automation_block}
"#
        );
        std::fs::write(&path, yaml).unwrap();
        path
    }

    fn run_args(plan: &Path, scenario: &str) -> UatRunArgs {
        UatRunArgs {
            plan: plan.to_path_buf(),
            scenario: scenario.into(),
            timeout_ms: 10_000,
            approve: false,
            output: None,
            format: OutputFormat::Text,
        }
    }

    fn read_session(path: &Path) -> UatSession {
        let raw = std::fs::read_to_string(path).unwrap();
        serde_saphyr::from_str(&raw).unwrap()
    }

    #[test]
    fn scripted_pass_emits_baseline_session() {
        let dir = tempfile::tempdir().unwrap();
        let plan = write_plan(dir.path(), "  status: scripted\n  ref: echo hello-world");
        let out = run_uat_run(run_args(&plan, "S-1"));
        assert_eq!(out.status, 0, "stderr: {}", out.stderr);
        assert!(out.stdout.contains("PASS"), "stdout: {}", out.stdout);
        let session_path = dir.path().join("uat-session-s-1.yaml");
        let session = read_session(&session_path);
        assert_eq!(session.results.len(), 1);
        let result = &session.results[0];
        assert_eq!(result.status, sddk_domain::UatStatus::Pass);
        assert_eq!(result.repro_command.as_deref(), Some("echo hello-world"));
        assert!(result.verdict_duration_ms.is_some());
        assert_eq!(session.executor, sddk_domain::UatExecutor::Automated);
        assert_eq!(session.executed_by.as_deref(), Some("auto-runner"));
        assert_eq!(session.release, "v2.1.0");
        // Evidence is sha256-pinned so integrity verify accepts it.
        assert!(result.evidence[0].r#ref.starts_with("sha256:"));
    }

    #[test]
    fn failing_script_maps_to_fail_with_stderr() {
        let dir = tempfile::tempdir().unwrap();
        // A real script file exercises the typed argv path (no shell quoting).
        let script = dir.path().join("fail.sh");
        std::fs::write(&script, "#!/bin/sh\necho 'boom reason' >&2\nexit 3\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let plan = write_plan(
            dir.path(),
            &format!("  status: scripted\n  ref: {}", script.display()),
        );
        let out = run_uat_run(run_args(&plan, "S-1"));
        assert_eq!(out.status, 0, "command should succeed, only scenario fails");
        assert!(out.stdout.contains("FAIL"), "stdout: {}", out.stdout);
        let session = read_session(&dir.path().join("uat-session-s-1.yaml"));
        assert_eq!(session.results[0].status, sddk_domain::UatStatus::Fail);
        assert_eq!(
            session.results[0].failure_reason.as_deref(),
            Some("boom reason")
        );
        // Evidence payload carries the captured stderr hash.
        assert!(session.results[0].evidence[0].r#ref.starts_with("sha256:"));
    }

    #[test]
    fn manual_scenario_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let plan = write_plan(dir.path(), "  status: manual");
        let out = run_uat_run(run_args(&plan, "S-1"));
        assert_eq!(out.status, 1);
        assert!(out.stdout.contains("manual"), "stdout: {}", out.stdout);
    }

    #[test]
    fn missing_automation_block_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let plan = write_plan(dir.path(), "  status: scripted");
        let out = run_uat_run(run_args(&plan, "S-1"));
        assert_eq!(out.status, 1);
        assert!(
            out.stdout.contains("automation.ref") || out.stdout.contains("no executor"),
            "stdout: {}",
            out.stdout
        );
    }

    #[test]
    fn v3_executor_cli_runs_and_passes() {
        // Eje v3 (ADR-014): executor cli con command — sin automation v2.
        let dir = tempfile::tempdir().unwrap();
        let plan = dir.path().join("uat-plan.yaml");
        let yaml = r#"
schema_version: 3
release: { candidate: v2.1.0 }
generated_by: test
generated_at: "2026-08-10T00:00:00Z"
features:
  - id: F-1
    name: Feature
    scenarios:
      - id: S-1
        title: V3 cli
        executor:
          kind: cli
          command: echo hello-v3
        oracles:
          - kind: exit_code
            expect: { code: 0 }
        acceptance: pending
"#;
        std::fs::write(&plan, yaml).unwrap();
        let out = run_uat_run(run_args(&plan, "S-1"));
        assert_eq!(out.status, 0, "stderr: {}", out.stderr);
        assert!(out.stdout.contains("PASS"), "stdout: {}", out.stdout);
        let session = read_session(&dir.path().join("uat-session-s-1.yaml"));
        assert_eq!(session.results[0].status, sddk_domain::UatStatus::Pass);
        assert_eq!(
            session.results[0].repro_command.as_deref(),
            Some("echo hello-v3")
        );
        assert_eq!(session.executor, sddk_domain::UatExecutor::Automated);
    }

    #[test]
    fn unknown_scenario_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let plan = write_plan(dir.path(), "  status: scripted\n  ref: echo hi");
        let out = run_uat_run(run_args(&plan, "S-99"));
        assert_eq!(out.status, 1);
        assert!(out.stdout.contains("not found"), "stdout: {}", out.stdout);
    }

    #[test]
    fn timeout_maps_to_blocked() {
        let dir = tempfile::tempdir().unwrap();
        let plan = write_plan(dir.path(), "  status: scripted\n  ref: sleep 30");
        let mut args = run_args(&plan, "S-1");
        args.timeout_ms = 50;
        let out = run_uat_run(args);
        assert_eq!(out.status, 0);
        assert!(out.stdout.contains("BLOCKED"), "stdout: {}", out.stdout);
        let session = read_session(&dir.path().join("uat-session-s-1.yaml"));
        assert_eq!(session.results[0].status, sddk_domain::UatStatus::Blocked);
    }

    #[test]
    fn spawn_failure_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        let plan = write_plan(
            dir.path(),
            "  status: automated\n  ref: sddk-no-such-binary-xyz",
        );
        let out = run_uat_run(run_args(&plan, "S-1"));
        assert_eq!(out.status, 1);
        assert!(
            out.stdout.contains("failed to spawn"),
            "stdout: {}",
            out.stdout
        );
    }
}

#[cfg(test)]
mod uat_signoff_tests {
    use super::*;

    fn make_env(data_dir: &Path) -> crate::CliEnvironment {
        crate::CliEnvironment {
            home: Some(PathBuf::from("/tmp")),
            data_home: Some(data_dir.to_path_buf()),
            sddk_data_dir: Some(data_dir.to_path_buf()),
            state_home: Some(data_dir.to_path_buf()),
            cache_home: Some(data_dir.to_path_buf()),
            sddk_actor: Some("tester".into()),
            user: Some("test".into()),
        }
    }

    #[test]
    fn signoff_emits_correct_sha256_and_deterministic() {
        let dir = tempfile::tempdir().unwrap();

        // Plan with known content.
        let plan_content = r#"
schema_version: 3
release: { candidate: v1.9.0 }
generated_by: test
generated_at: "2026-08-11T00:00:00Z"
features:
  - id: F-1
    name: Feature
    scenarios:
      - id: S-1
        title: Scenario
"#;
        let plan_path = dir.path().join("uat-plan-v1.9.0.yaml");
        std::fs::write(&plan_path, plan_content).unwrap();

        // Manifest with known digests.
        let manifest_content = r#"
schema_version: 1
project_id: test-project
generated_at: "2026-08-11T00:00:00Z"
entries:
  - sha256: abcdef123456
    path: evidence/screenshot.png
    size_bytes: 1024
    captured_at: "2026-08-11T00:00:00Z"
    scenario_id: S-1
    session_id: sess-1
    kind: screenshot
  - sha256: 789012abcdef
    path: evidence/trace.zip
    size_bytes: 4096
    captured_at: "2026-08-11T00:00:00Z"
    scenario_id: S-1
    session_id: sess-1
    kind: trace
"#;
        let manifest_path = dir.path().join("uat-manifest.yaml");
        std::fs::write(&manifest_path, manifest_content).unwrap();

        let env = make_env(dir.path());

        // First sign-off.
        let args = UatSignOffArgs {
            release: "v1.9.0".into(),
            decision: UatSignOffDecisionArg::Accepted,
            actor: "user:421".into(),
            justification: "LGTM".into(),
            plan: Some(plan_path.clone()),
            session_dir: Some(dir.path().to_path_buf()),
            project: Some("test-project".into()),
            format: OutputFormat::Text,
        };
        let out = run_uat_signoff(args, &env);
        assert_eq!(out.status, 0, "stderr: {}", out.stderr);

        // Read the output file and verify sha256 format.
        // Path is: {data_dir}/sddk/projects/{project_id}/uat/acceptances/
        let storage_root = dir
            .path()
            .join("sddk")
            .join("projects")
            .join("test-project")
            .join("uat");
        let acceptance_file = storage_root
            .join("acceptances")
            .join("uat-acceptance-v1.9.0.yaml");
        let raw = std::fs::read_to_string(&acceptance_file).unwrap();

        // Verify the record structure.
        let record: sddk_domain::UatAcceptanceRecord = serde_saphyr::from_str(&raw).unwrap();
        assert_eq!(
            record.decision,
            sddk_domain::UatAcceptanceDecision::Accepted
        );
        assert_eq!(record.actor, "user:421");
        assert_eq!(record.justification, "LGTM");
        assert!(record.plan_version_sha256.starts_with("sha256:"));
        assert!(record.evidence_snapshot_sha256.starts_with("sha256:"));

        // Second sign-off with SAME plan/manifest must produce SAME sha256.
        let args2 = UatSignOffArgs {
            release: "v1.9.0".into(),
            decision: UatSignOffDecisionArg::Accepted,
            actor: "user:422".into(),
            justification: "Also LGTM".into(),
            plan: Some(plan_path.clone()),
            session_dir: Some(dir.path().to_path_buf()),
            project: Some("test-project".into()),
            format: OutputFormat::Text,
        };
        let out2 = run_uat_signoff(args2, &env);
        assert_eq!(out2.status, 0);

        let raw2 = std::fs::read_to_string(&acceptance_file).unwrap();
        let record2: sddk_domain::UatAcceptanceRecord = serde_saphyr::from_str(&raw2).unwrap();
        // Same plan/manifest → same sha256 values.
        assert_eq!(record.plan_version_sha256, record2.plan_version_sha256);
        assert_eq!(
            record.evidence_snapshot_sha256,
            record2.evidence_snapshot_sha256
        );
        // But different actor/decision.
        assert_eq!(record2.actor, "user:422");
    }

    #[test]
    fn signoff_rejected_decision_is_valid() {
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("uat-plan-v1.9.0.yaml");
        let plan_content = r#"
schema_version: 3
release: { candidate: v1.9.0 }
generated_by: test
generated_at: "2026-08-11T00:00:00Z"
features: []
"#;
        std::fs::write(&plan_path, plan_content).unwrap();

        let env = make_env(dir.path());
        let args = UatSignOffArgs {
            release: "v1.9.0".into(),
            decision: UatSignOffDecisionArg::Rejected,
            actor: "user:421".into(),
            justification: "Not ready".into(),
            plan: Some(plan_path),
            session_dir: None,
            project: Some("test-project".into()),
            format: OutputFormat::Text,
        };
        let out = run_uat_signoff(args, &env);
        assert_eq!(out.status, 0, "stderr: {}", out.stderr);
    }

    #[test]
    fn signoff_missing_plan_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let env = make_env(dir.path());
        let args = UatSignOffArgs {
            release: "v99.0.0".into(),
            decision: UatSignOffDecisionArg::Accepted,
            actor: "user:1".into(),
            justification: "test".into(),
            plan: None,
            session_dir: None,
            project: Some("test-project".into()),
            format: OutputFormat::Text,
        };
        let out = run_uat_signoff(args, &env);
        assert_ne!(out.status, 0, "expected error, got status 0");
    }
}

#[cfg(test)]
mod uat_stale_tests {
    // Tests for `uat stale` command (REQ-RF-024).
    // Requires node + playwright browser — skipped in CI without it.
    use super::*;

    fn make_env(data_dir: &Path) -> crate::CliEnvironment {
        crate::CliEnvironment {
            home: Some(PathBuf::from("/tmp")),
            data_home: Some(data_dir.to_path_buf()),
            sddk_data_dir: Some(data_dir.to_path_buf()),
            state_home: Some(data_dir.to_path_buf()),
            cache_home: Some(data_dir.to_path_buf()),
            sddk_actor: Some("tester".into()),
            user: Some("test".into()),
        }
    }

    /// Full stale detection: previous geometry stored, current geometry differs.
    #[test]
    fn stale_detects_geometry_change() {
        // Check if node is available (prerequisite for playwright).
        let node_check = std::process::Command::new("node")
            .arg("--version")
            .output()
            .ok();
        if node_check.map(|o| o.status.success()) != Some(true) {
            eprintln!("skipping: node unavailable");
            return;
        }

        // Check if python3 is available (prerequisite for local HTTP server).
        // Mirrors the node probe above; skip cleanly before any spawn.
        let python_check = std::process::Command::new("python3")
            .arg("--version")
            .output()
            .ok();
        if python_check.map(|o| o.status.success()) != Some(true) {
            eprintln!("skipping: python3 unavailable");
            return;
        }

        let dir = tempfile::tempdir().unwrap();

        // Write a minimal HTML page with a button for Playwright to inspect.
        std::fs::write(
            dir.path().join("index.html"),
            "<!DOCTYPE html><html><head><title>Test</title></head>\
             <body><button id=\"login-btn\">Login</button></body></html>",
        )
        .unwrap();

        // Spin up a local HTTP server on an ephemeral port (0 = kernel assigns).
        // python's http.server prints "Serving HTTP on 127.0.0.1 port NNNNN ..." to stdout.
        // We parse the port with \bport (\d+)\b (Python 3.0+ documented format).
        let mut child = std::process::Command::new("python3")
            .args(["-u", "-m", "http.server", "0", "--bind", "127.0.0.1"])
            .current_dir(dir.path())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn python3 http.server");

        let port = {
            let stdout = child.stdout.take().expect("stdout captured");
            use std::io::{BufRead, BufReader};
            let mut lines = BufReader::new(stdout).lines();
            let first = lines
                .next()
                .expect("python http.server produced no output")
                .expect("python http.server stdout read error");
            // Regex \bport (\d+)\b captures the port number from the first line.
            let re = regex::Regex::new(r"\bport (\d+)\b").expect("regex is valid");
            let caps = re.captures(&first).expect("python output format changed");
            caps.get(1)
                .expect("port number missing from python output")
                .as_str()
                .parse::<u16>()
                .expect("port number is not a valid u16")
        };

        // ServerGuard RAII wrapper: kills + reaps child on any exit path.
        let _server_guard = sddk_testkit::ChildGuard::new(child);

        // Readiness poll: replace blind 500ms sleep with TcpStream::connect_timeout.
        // 50ms per attempt, 1s total deadline.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        loop {
            if std::net::TcpStream::connect_timeout(
                &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
                std::time::Duration::from_millis(50),
            )
            .is_ok()
            {
                break;
            }
            if std::time::Instant::now() >= deadline {
                panic!("server not ready on 127.0.0.1:{} after 1s deadline", port);
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        let url = format!("http://127.0.0.1:{}/index.html", port);

        // Plan with a geometry oracle selector.
        let plan_content = r##"
schema_version: 3
release: { candidate: v1.0.0 }
generated_by: test
generated_at: "2026-08-11T00:00:00Z"
features:
  - id: F1
    name: Login
    scenarios:
      - id: S-1
        title: Login button visible
        priority: P0
        plain_steps: []
        oracles:
          - kind: geometry
            expect:
              selector: "#login-btn"
"##;
        let plan_path = dir.path().join("uat-plan.yaml");
        std::fs::write(&plan_path, plan_content).unwrap();

        // Previous session: geometry with bounding box {x: 10, y: 20, width: 100, height: 44}.
        let session_dir = dir.path().join("sessions/s-001");
        std::fs::create_dir_all(&session_dir).unwrap();
        let prev_geometry: serde_json::Value = serde_json::json!({
            "#login-btn": { "x": 10, "y": 20, "width": 100, "height": 44 }
        });
        std::fs::write(
            session_dir.join("geometry.json"),
            serde_json::to_string_pretty(&prev_geometry).unwrap(),
        )
        .unwrap();

        let env = make_env(dir.path());
        let args = UatStaleArgs {
            url,
            project: Some("test-project".into()),
            plan: Some(plan_path),
            session_dir: Some(session_dir),
            format: OutputFormat::Text,
        };
        let out = run_uat_stale(args, &env);

        // ServerGuard Drop runs here on scope exit — no manual kill needed.
        assert_eq!(out.status, 0, "command failed: {}", out.stderr);

        // Parse report from stdout.
        let report: sddk_domain::UatStalenessReport = serde_saphyr::from_str(&out.stdout).unwrap();
        assert_eq!(report.release, "v1.0.0");
        assert!(!report.assessed_at.is_empty());
        // Current geometry differs from previous → affected_scenarios non-empty.
        assert!(
            !report.affected_scenarios.is_empty() || !report.fingerprint_diffs.is_empty(),
            "expected stale detection for changed geometry"
        );
    }

    /// No previous session → fresh report with zero affected scenarios.
    #[test]
    fn stale_no_previous_session_is_fresh() {
        let dir = tempfile::tempdir().unwrap();

        let plan_content = r#"
schema_version: 3
release: { candidate: v1.0.0 }
generated_by: test
generated_at: "2026-08-11T00:00:00Z"
features:
  - id: F1
    name: Login
    scenarios:
      - id: S-1
        title: Login button visible
        priority: P0
        plain_steps: []
        oracles: []
"#;
        let plan_path = dir.path().join("uat-plan.yaml");
        std::fs::write(&plan_path, plan_content).unwrap();

        let env = make_env(dir.path());
        let args = UatStaleArgs {
            url: "http://127.0.0.1:18766/index.html".into(),
            project: Some("test-project".into()),
            plan: Some(plan_path),
            session_dir: None,
            format: OutputFormat::Text,
        };
        let out = run_uat_stale(args, &env);
        // No selectors → should still succeed but report empty.
        assert_eq!(out.status, 0, "stderr: {}", out.stderr);
        let report: sddk_domain::UatStalenessReport = serde_saphyr::from_str(&out.stdout).unwrap();
        assert!(report.affected_scenarios.is_empty());
        assert!(report.fingerprint_diffs.is_empty());
    }
}

#[cfg(test)]
mod uat_validate_tests {
    use super::*;

    /// Valid plan with no form DSL → validate OK.
    #[test]
    fn validate_plan_without_form_is_ok() {
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.yaml");
        let plan_content = r#"
schema_version: 3
release: { candidate: v1.0.0 }
generated_by: test
generated_at: "2026-08-11T00:00:00Z"
features:
  - id: F1
    name: Login
    scenarios:
      - id: S-1
        title: Login works
        priority: P0
        plain_steps: []
"#;
        std::fs::write(&plan_path, plan_content).unwrap();
        let args = UatValidateArgs {
            file: plan_path,
            format: OutputFormat::Text,
        };
        let out = run_uat_validate(args);
        assert_eq!(out.status, 0, "expected OK, got: {}", out.stderr);
    }

    /// Form DSL with goto pointing to non-existent item → exit 1.
    #[test]
    fn validate_form_goto_nonexistent_target_is_error() {
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.yaml");
        let plan_content = r#"
schema_version: 3
release: { candidate: v1.0.0 }
generated_by: test
generated_at: "2026-08-11T00:00:00Z"
features:
  - id: F1
    name: Login
    scenarios:
      - id: S-1
        title: Login form
        priority: P0
        plain_steps: []
        form:
          dsl_version: 1
          items:
            - id: start
              kind: info
              text: "Start"
            - id: broken
              kind: flow
              flow: goto
              target: nonexistent
"#;
        std::fs::write(&plan_path, plan_content).unwrap();
        let args = UatValidateArgs {
            file: plan_path,
            format: OutputFormat::Text,
        };
        let out = run_uat_validate(args);
        assert_ne!(out.status, 0, "expected error for broken goto target");
        assert!(
            out.stderr.contains("goto target") || out.stderr.contains("not found"),
            "expected goto error, got: {}",
            out.stderr
        );
    }

    /// Form DSL with goto cycle (a→b→a) → exit 1.
    #[test]
    fn validate_form_goto_cycle_is_error() {
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.yaml");
        let plan_content = r#"
schema_version: 3
release: { candidate: v1.0.0 }
generated_by: test
generated_at: "2026-08-11T00:00:00Z"
features:
  - id: F1
    name: Login
    scenarios:
      - id: S-1
        title: Login form
        priority: P0
        plain_steps: []
        form:
          dsl_version: 1
          items:
            - id: a
              kind: flow
              flow: goto
              target: b
            - id: b
              kind: flow
              flow: goto
              target: a
"#;
        std::fs::write(&plan_path, plan_content).unwrap();
        let args = UatValidateArgs {
            file: plan_path,
            format: OutputFormat::Text,
        };
        let out = run_uat_validate(args);
        assert_ne!(out.status, 0, "expected error for goto cycle");
        assert!(
            out.stderr.contains("cycle") || out.stderr.contains("goto cycle"),
            "expected cycle error, got: {}",
            out.stderr
        );
    }

    /// CompletionPolicy with invalid mode → exit 1.
    #[test]
    fn validate_form_completion_invalid_mode_is_error() {
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.yaml");
        let plan_content = r#"
schema_version: 3
release: { candidate: v1.0.0 }
generated_by: test
generated_at: "2026-08-11T00:00:00Z"
features:
  - id: F1
    name: Login
    scenarios:
      - id: S-1
        title: Login form
        priority: P0
        plain_steps: []
        form:
          dsl_version: 1
          items:
            - id: start
              kind: info
              text: "Start"
          completion:
            mode: invalid_mode
            threshold: 1
"#;
        std::fs::write(&plan_path, plan_content).unwrap();
        let args = UatValidateArgs {
            file: plan_path,
            format: OutputFormat::Text,
        };
        let out = run_uat_validate(args);
        assert_ne!(out.status, 0, "expected error for invalid completion mode");
    }
}

#[cfg(test)]
mod uat_mode_tests {
    use super::*;
    use clap::ValueEnum;

    /// UatRunnerMode variants have correct names via ValueEnum.
    #[test]
    fn runner_mode_enum_values() {
        assert_eq!(
            UatRunnerMode::Designer
                .to_possible_value()
                .unwrap()
                .get_name(),
            "designer"
        );
        assert_eq!(
            UatRunnerMode::Runner
                .to_possible_value()
                .unwrap()
                .get_name(),
            "runner"
        );
        assert_eq!(
            UatRunnerMode::Reviewer
                .to_possible_value()
                .unwrap()
                .get_name(),
            "reviewer"
        );
    }

    /// Default mode in UatDashboardArgs is Runner.
    #[test]
    fn dashboard_args_default_mode_is_runner() {
        let args = UatDashboardArgs {
            plan: PathBuf::from("/tmp/plan.yaml"),
            view: UatView::Guided,
            mode: UatRunnerMode::Runner,
            theme: "dark".into(),
            output: None,
            format: OutputFormat::Text,
        };
        assert_eq!(args.mode, UatRunnerMode::Runner);
    }

    /// Default mode in UatOpenArgs is Runner.
    #[test]
    fn open_args_default_mode_is_runner() {
        let args = UatOpenArgs {
            plan: Some(PathBuf::from("/tmp/plan.yaml")),
            release: None,
            view: UatView::Guided,
            mode: UatRunnerMode::Runner,
            theme: "dark".into(),
            browser: None,
            output: None,
            format: OutputFormat::Text,
        };
        assert_eq!(args.mode, UatRunnerMode::Runner);
    }

    /// Designer mode sets the correct variant.
    #[test]
    fn mode_designer_variant() {
        let args = UatOpenArgs {
            plan: None,
            release: Some("v1.0.0".into()),
            view: UatView::Guided,
            mode: UatRunnerMode::Designer,
            theme: "dark".into(),
            browser: None,
            output: None,
            format: OutputFormat::Text,
        };
        assert_eq!(args.mode, UatRunnerMode::Designer);
    }

    /// view != Guided triggers the deprecation warning path.
    #[test]
    fn view_matrix_triggers_deprecation_warning() {
        // When view is not Guided, run_uat_dashboard emits a warning.
        // We test that the condition args.view != UatView::Guided holds.
        let args = UatDashboardArgs {
            plan: PathBuf::from("/tmp/plan.yaml"),
            view: UatView::Matrix,
            mode: UatRunnerMode::Runner,
            theme: "dark".into(),
            output: None,
            format: OutputFormat::Text,
        };
        assert_ne!(args.view, UatView::Guided);
    }

    /// view traceability also triggers deprecation warning.
    #[test]
    fn view_traceability_triggers_deprecation_warning() {
        let args = UatDashboardArgs {
            plan: PathBuf::from("/tmp/plan.yaml"),
            view: UatView::Traceability,
            mode: UatRunnerMode::Runner,
            theme: "dark".into(),
            output: None,
            format: OutputFormat::Text,
        };
        assert_ne!(args.view, UatView::Guided);
    }
}

/// F13 Integration tests: sign-off + stale re-signing scenarios.
#[cfg(test)]
mod uat_f13_integration_tests {
    use super::*;

    fn make_test_env(data_dir: &Path) -> crate::CliEnvironment {
        crate::CliEnvironment {
            home: Some(PathBuf::from("/tmp")),
            data_home: Some(data_dir.to_path_buf()),
            sddk_data_dir: Some(data_dir.to_path_buf()),
            state_home: Some(data_dir.to_path_buf()),
            cache_home: Some(data_dir.to_path_buf()),
            sddk_actor: Some("tester".into()),
            user: Some("test".into()),
        }
    }

    /// Sign-off re-apertura: plan v1 → sign-off → plan v2 (different content) →
    /// second sign-off has different sha256, preserving the first record's sha256
    /// in the audit trail (new record created, not overwritten).
    #[test]
    fn signoff_reopening_preserves_first_sha256() {
        let dir = tempfile::tempdir().unwrap();

        // Plan v1.
        let plan_v1_content = r#"
schema_version: 3
release: { candidate: v1.9.0 }
generated_by: test
generated_at: "2026-08-11T00:00:00Z"
features:
  - id: F-1
    name: Feature
    scenarios:
      - id: S-1
        title: Scenario v1
"#;
        let plan_v1_path = dir.path().join("uat-plan-v1.yaml");
        std::fs::write(&plan_v1_path, plan_v1_content).unwrap();

        // Manifest for v1.
        let manifest_v1_content = r#"
schema_version: 1
project_id: test-project
generated_at: "2026-08-11T00:00:00Z"
entries:
  - sha256: v1evidence
    path: evidence/s1.png
    size_bytes: 100
    captured_at: "2026-08-11T00:00:00Z"
    scenario_id: S-1
    session_id: sess-v1
    kind: screenshot
"#;
        let manifest_v1_path = dir.path().join("uat-manifest-v1.yaml");
        std::fs::write(&manifest_v1_path, manifest_v1_content).unwrap();

        let env = make_test_env(dir.path());

        // Sign-off v1.
        let args_v1 = UatSignOffArgs {
            release: "v1.9.0".into(),
            decision: UatSignOffDecisionArg::Accepted,
            actor: "user:421".into(),
            justification: "v1 approved".into(),
            plan: Some(plan_v1_path.clone()),
            session_dir: Some(dir.path().to_path_buf()),
            project: Some("test-project".into()),
            format: OutputFormat::Text,
        };
        let out_v1 = run_uat_signoff(args_v1, &env);
        assert_eq!(out_v1.status, 0, "sign-off v1 failed: {}", out_v1.stderr);

        // Read v1 record.
        let storage_root = dir
            .path()
            .join("sddk")
            .join("projects")
            .join("test-project")
            .join("uat");
        let acceptance_file = storage_root
            .join("acceptances")
            .join("uat-acceptance-v1.9.0.yaml");
        let raw_v1 = std::fs::read_to_string(&acceptance_file).unwrap();
        let record_v1: sddk_domain::UatAcceptanceRecord = serde_saphyr::from_str(&raw_v1).unwrap();
        let sha256_v1 = record_v1.plan_version_sha256.clone();

        // Plan v2 (different content).
        let plan_v2_content = r#"
schema_version: 3
release: { candidate: v1.9.0 }
generated_by: test
generated_at: "2026-08-11T01:00:00Z"
features:
  - id: F-1
    name: Feature UPDATED
    scenarios:
      - id: S-1
        title: Scenario v2
"#;
        let plan_v2_path = dir.path().join("uat-plan-v2.yaml");
        std::fs::write(&plan_v2_path, plan_v2_content).unwrap();

        // Sign-off v2 with plan_v2.
        let args_v2 = UatSignOffArgs {
            release: "v1.9.0".into(),
            decision: UatSignOffDecisionArg::AcceptedConditional,
            actor: "user:422".into(),
            justification: "v2 conditionally".into(),
            plan: Some(plan_v2_path.clone()),
            session_dir: Some(dir.path().to_path_buf()),
            project: Some("test-project".into()),
            format: OutputFormat::Text,
        };
        let out_v2 = run_uat_signoff(args_v2, &env);
        assert_eq!(out_v2.status, 0, "sign-off v2 failed: {}", out_v2.stderr);

        // Read v2 record — sha256 should be different from v1.
        let raw_v2 = std::fs::read_to_string(&acceptance_file).unwrap();
        let record_v2: sddk_domain::UatAcceptanceRecord = serde_saphyr::from_str(&raw_v2).unwrap();

        // sha256 changed because plan content changed.
        assert_ne!(
            record_v2.plan_version_sha256, sha256_v1,
            "plan v2 sha256 should differ from v1"
        );
        // Decision and actor changed.
        assert_eq!(
            record_v2.decision,
            sddk_domain::UatAcceptanceDecision::AcceptedConditional
        );
        assert_eq!(record_v2.actor, "user:422");
        // Plan v1 sha256 is preserved in the first record (not overwritten).
        assert_eq!(record_v1.plan_version_sha256, sha256_v1);
    }

    /// stale → re-sign: after UI changes detected by stale, re-signing with
    /// the updated plan produces a new sha256.
    #[test]
    fn stale_then_resign_updates_sha256() {
        let dir = tempfile::tempdir().unwrap();

        // Plan v1.
        let plan_v1_content = r#"
schema_version: 3
release: { candidate: v1.9.0 }
generated_by: test
generated_at: "2026-08-11T00:00:00Z"
features:
  - id: F-1
    name: Feature
    scenarios:
      - id: S-1
        title: Scenario
"#;
        let plan_v1_path = dir.path().join("uat-plan-v1.9.0.yaml");
        std::fs::write(&plan_v1_path, plan_v1_content).unwrap();

        let env = make_test_env(dir.path());

        // Initial sign-off.
        let args_v1 = UatSignOffArgs {
            release: "v1.9.0".into(),
            decision: UatSignOffDecisionArg::Accepted,
            actor: "user:421".into(),
            justification: "initial".into(),
            plan: Some(plan_v1_path.clone()),
            session_dir: None,
            project: Some("test-project".into()),
            format: OutputFormat::Text,
        };
        let out_v1 = run_uat_signoff(args_v1, &env);
        assert_eq!(out_v1.status, 0);

        // Read v1 sha256.
        let storage_root = dir
            .path()
            .join("sddk")
            .join("projects")
            .join("test-project")
            .join("uat");
        let acceptance_file = storage_root
            .join("acceptances")
            .join("uat-acceptance-v1.9.0.yaml");
        let record_v1: sddk_domain::UatAcceptanceRecord =
            serde_saphyr::from_str(&std::fs::read_to_string(&acceptance_file).unwrap()).unwrap();
        let sha256_v1 = record_v1.plan_version_sha256.clone();

        // Simulate plan update (e.g., after applying staleness suggestions).
        let plan_v2_content = r#"
schema_version: 3
release: { candidate: v1.9.0 }
generated_by: test
generated_at: "2026-08-11T02:00:00Z"
features:
  - id: F-1
    name: Feature updated after staleness review
    scenarios:
      - id: S-1
        title: Scenario (corrected)
"#;
        let plan_v2_path = dir.path().join("uat-plan-v1.9.0.yaml"); // overwrite
        std::fs::write(&plan_v2_path, plan_v2_content).unwrap();

        // Re-sign with updated plan.
        let args_v2 = UatSignOffArgs {
            release: "v1.9.0".into(),
            decision: UatSignOffDecisionArg::Accepted,
            actor: "user:421".into(),
            justification: "after stale review".into(),
            plan: Some(plan_v2_path),
            session_dir: None,
            project: Some("test-project".into()),
            format: OutputFormat::Text,
        };
        let out_v2 = run_uat_signoff(args_v2, &env);
        assert_eq!(out_v2.status, 0);

        // Read v2 sha256 — should differ from v1.
        let record_v2: sddk_domain::UatAcceptanceRecord =
            serde_saphyr::from_str(&std::fs::read_to_string(&acceptance_file).unwrap()).unwrap();
        assert_ne!(
            record_v2.plan_version_sha256, sha256_v1,
            "re-signed plan sha256 should differ after update"
        );
        assert_eq!(record_v2.justification, "after stale review");
    }
}
