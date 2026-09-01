//! Cycle and lease commands exposing the local workflow authority.

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};
use sddk_domain::{
    ActorKind, ArtifactRef, ControlPlane, CycleId, CycleManifest, CyclePath, WorkflowManifest,
    normalize_scope,
};
use sddk_engine::{
    AdoptionPaths, CycleStartInput, Engine, EventContext, GateEvaluationInput, ReplanDelta,
    RestageTo, SupersedeReason, TransitionEvidence, TransitionOutcome, WorkflowLoadError,
    event_bus::{self, OutcomeEventInput, PhaseEventInput},
};
use sddk_storage::SqliteEventStore;
use serde::Serialize;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::{CliEnvironment, CommandOutput, OutputFormat, render_result};

const CYCLE_START_REQUIREMENTS: [&str; 4] = [
    "project.adopted",
    "project.initialized",
    "worktree.clean",
    "cycle.no_active_conflict",
];

/// Shared runtime resolution inputs for cycle and ledger commands.
#[derive(Debug, Clone, Args)]
pub(crate) struct RuntimeArgs {
    /// Checkout or worktree root.
    #[arg(long)]
    pub(crate) root: PathBuf,
    /// Required monorepo scope, using `.` for the repository root.
    #[arg(long)]
    pub(crate) scope: String,
    /// Explicit remote URL instead of read-only Git discovery.
    #[arg(long)]
    pub(crate) remote: Option<String>,
    /// Stable UUID for a repository without a remote.
    #[arg(long)]
    pub(crate) fallback_seed: Option<String>,
}
/// Resolved identity, storage, and engine for one runtime invocation.
pub(crate) struct RuntimeContext {
    pub(crate) root: PathBuf,
    pub(crate) identity: sddk_domain::ResolvedProjectIdentity,
    pub(crate) workspace_id: String,
    pub(crate) engine: Engine<crate::Storage>,
    pub(crate) storage: crate::Storage,
    /// Control-plane port (boxed to allow dynamic dispatch).
    // `dead_code` allow: retained for future CP polling/injection;
    // tracked for cleanup in phase2-hygiene-baseline.
    #[allow(dead_code)]
    pub(crate) control_plane: Box<dyn ControlPlane>,
    pub(crate) artifacts_path: PathBuf,
    pub(crate) cycle_artifacts_path: PathBuf,
    /// Resolved XDG storage paths for the project workspace.
    pub(crate) paths: AdoptionPaths,
}

impl RuntimeContext {
    /// Resolves identity and opens the project ledger and workflow engine.
    ///
    /// `generate_seed` permits state-changing commands to mint a fallback UUID
    /// when the repository has no remote and no persisted adoption receipt.
    pub(crate) fn open(
        args: &RuntimeArgs,
        environment: &CliEnvironment,
        generate_seed: bool,
    ) -> anyhow::Result<Self> {
        let root = crate::canonical_root(&args.root)?;
        let remote = crate::resolve_remote(&root, args.remote.clone())?;
        let mut fallback_seed = args.fallback_seed.clone();
        if remote.is_none() && fallback_seed.is_none() {
            fallback_seed = crate::find_persisted_fallback_seed(environment, &root, &args.scope)?;
        }
        if remote.is_none() && fallback_seed.is_none() && generate_seed {
            fallback_seed = Some(Uuid::new_v4().hyphenated().to_string());
        }
        let identity = sddk_domain::resolve_project_identity(
            remote.as_deref(),
            &args.scope,
            fallback_seed.as_deref(),
        )?;
        let canonical_workspace_path = crate::path_string(&root)?;
        let workspace_id =
            crate::stable_workspace_id(&identity.project_id, &canonical_workspace_path);
        let paths = sddk_engine::resolve_xdg_paths(
            &environment.xdg(),
            identity.project_id.as_str(),
            &workspace_id,
        )?;
        let (storage, plane) = crate::compose(environment, &paths.ledger)?;
        let workflow = load_workflow(&root)?;
        // Engine takes ownership; original pattern opens Storage twice to satisfy both.
        let engine = Engine::new(workflow, crate::Storage::open(&paths.ledger)?)?;
        Ok(Self {
            root,
            identity,
            workspace_id,
            engine,
            storage,
            control_plane: Box::new(plane),
            artifacts_path: paths.artifacts.clone(),
            cycle_artifacts_path: paths.cycle_artifacts.clone(),
            paths,
        })
    }
}

/// Loads the repository workflow manifest, falling back to the canonical
/// embedded manifest when the repository has none (non-intrusive policy,
/// ADR-0011: projects are never required to carry workflow files).
fn load_workflow(root: &std::path::Path) -> anyhow::Result<WorkflowManifest> {
    match sddk_engine::load_workflow_path(root.join(crate::WORKFLOW_MANIFEST)) {
        Ok(manifest) => Ok(manifest),
        Err(WorkflowLoadError::Io { source, .. })
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            Ok(sddk_engine::load_workflow_str(crate::CANONICAL_WORKFLOW)?)
        }
        Err(error) => Err(error.into()),
    }
}

#[derive(Debug, Subcommand)]
pub(crate) enum CycleCommand {
    /// Create a cycle through the declared `cycle.start` transition.
    Start(CycleStartArgs),
    /// Show the current cycle snapshot and lease.
    Status(CycleStatusArgs),
    /// Apply one declared transition with caller evidence.
    Transition(CycleTransitionArgs),
    /// Authorize and persist one gate evaluation receipt.
    EvaluateGate(CycleEvaluateGateArgs),
    /// Restore a missing cycle snapshot from its ledger events.
    Rebuild(CycleRebuildArgs),
    /// Close a cycle with a successor or a reason.
    Supersede(CycleSupersedeArgs),
    /// Bounded in-place revision of a cycle.
    Replan(CycleReplanArgs),
    /// Print the XDG artifact directory for a cycle (created on demand).
    ArtifactsDir(CycleArtifactsDirArgs),
    /// Acquire, release, or inspect the exclusive cycle lease.
    #[command(subcommand)]
    Lock(CycleLockCommand),
    /// Build the cycle-scoped files inventory artifact (`sddk.inventory/v1`).
    Inventory(crate::inventory_cycle::CycleInventoryArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CycleArtifactsDirArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CycleStartArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Display name used to derive the stable cycle identifier.
    #[arg(long)]
    pub(crate) name: String,
    /// Workflow path applied to the cycle (defaults to F3 tuning when available).
    #[arg(long, value_enum)]
    pub(crate) path: Option<CyclePathArg>,
    /// Git branch associated with the cycle.
    #[arg(long)]
    pub(crate) branch: Option<String>,
    /// Base commit SHA.
    #[arg(long)]
    pub(crate) base: Option<String>,
    /// Explicit RFC 3339 timestamp for deterministic execution.
    #[arg(long)]
    pub(crate) timestamp: Option<String>,
    /// Explicit actor for deterministic execution.
    #[arg(long)]
    pub(crate) actor: Option<String>,
    /// Acquire the exclusive cycle lease for this owner after creation.
    #[arg(long)]
    pub(crate) lease_owner: Option<String>,
    /// Lease duration in milliseconds when acquiring a lease.
    #[arg(long, default_value_t = 3_600_000)]
    pub(crate) lease_ms: i64,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CycleStatusArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CycleTransitionArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Declared transition identifier, for example `phase.build.complete`.
    #[arg(long)]
    pub(crate) transition: String,
    /// Satisfied non-artifact requirement.
    #[arg(long, action = clap::ArgAction::Append)]
    pub(crate) requirement: Vec<String>,
    /// Persisted gate receipt issued by `cycle evaluate-gate`.
    #[arg(long, action = clap::ArgAction::Append)]
    pub(crate) gate_receipt: Vec<String>,
    /// Produced artifact as `kind=path`.
    #[arg(long, action = clap::ArgAction::Append)]
    pub(crate) artifact: Vec<String>,
    /// Lease owner required by the fencing check.
    #[arg(long)]
    pub(crate) lease_owner: Option<String>,
    /// Fencing token required by the fencing check.
    #[arg(long)]
    pub(crate) fencing_token: Option<i64>,
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
pub(crate) struct CycleRebuildArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Dry-run: validate the ledger state without persisting any changes.
    #[arg(long)]
    pub(crate) dry_run: bool,
    /// Lease owner required by the fencing check; `rebuild` refuses to run
    /// without an unexpired lease so silent snapshot restoration cannot
    /// happen during a read-only audit.
    #[arg(long)]
    pub(crate) lease_owner: Option<String>,
    /// Fencing token of the lease the caller currently holds.
    #[arg(long)]
    pub(crate) fencing_token: Option<i64>,
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

/// Argument for the `cycle supersede` command.
#[derive(Debug, Clone, Args)]
pub(crate) struct CycleSupersedeArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier to supersede.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Successor cycle identifier (mutually exclusive with --reason).
    #[arg(long)]
    pub(crate) successor: Option<String>,
    /// Reason for superseding (scope_invalid | goal_replaced | external_obsolete).
    #[arg(long, value_enum)]
    pub(crate) reason: Option<SupersedeReasonArg>,
    /// Evidence references as JSON array string.
    #[arg(long, default_value = "[]")]
    pub(crate) evidence_refs: String,
    /// Lease owner required by the fencing check.
    #[arg(long)]
    pub(crate) lease_owner: String,
    /// Fencing token of the lease the caller currently holds.
    #[arg(long)]
    pub(crate) fencing_token: i64,
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

/// CLI representation of supersede reason.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum SupersedeReasonArg {
    /// The cycle scope became invalid.
    ScopeInvalid,
    /// The cycle goal was replaced by a new direction.
    GoalReplaced,
    /// External circumstances made the cycle obsolete.
    ExternalObsolete,
}

impl From<SupersedeReasonArg> for SupersedeReason {
    fn from(value: SupersedeReasonArg) -> Self {
        match value {
            SupersedeReasonArg::ScopeInvalid => SupersedeReason::ScopeInvalid,
            SupersedeReasonArg::GoalReplaced => SupersedeReason::GoalReplaced,
            SupersedeReasonArg::ExternalObsolete => SupersedeReason::ExternalObsolete,
        }
    }
}

/// Argument for the `cycle replan` command.
#[derive(Debug, Clone, Args)]
pub(crate) struct CycleReplanArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier to replan.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Target phase to restage to (propose | specify | design | tasks | apply).
    #[arg(long, value_enum)]
    pub(crate) restage_to: RestageToArg,
    /// JSON delta object with changed_files and reason.
    #[arg(long)]
    pub(crate) delta: String,
    /// Evidence references as JSON array string.
    #[arg(long, default_value = "[]")]
    pub(crate) evidence_refs: String,
    /// Lease owner required by the fencing check.
    #[arg(long)]
    pub(crate) lease_owner: String,
    /// Fencing token of the lease the caller currently holds.
    #[arg(long)]
    pub(crate) fencing_token: i64,
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

/// CLI representation of restage target.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum RestageToArg {
    Propose,
    Specify,
    Design,
    Tasks,
    Apply,
}

impl From<RestageToArg> for RestageTo {
    fn from(value: RestageToArg) -> Self {
        match value {
            RestageToArg::Propose => RestageTo::Propose,
            RestageToArg::Specify => RestageTo::Specify,
            RestageToArg::Design => RestageTo::Design,
            RestageToArg::Tasks => RestageTo::Tasks,
            RestageToArg::Apply => RestageTo::Apply,
        }
    }
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CycleEvaluateGateArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Transition that declares the gate.
    #[arg(long)]
    pub(crate) transition: String,
    /// Gate name being evaluated.
    #[arg(long)]
    pub(crate) gate: String,
    /// Evaluator identifier registered for the gate.
    #[arg(long, default_value = "sddk.cli")]
    pub(crate) evaluator: String,
    /// Sanitized evaluation evidence as JSON.
    #[arg(long, default_value = "{}")]
    pub(crate) evidence: String,
    /// Required. The gate outcome (passed | failed). Must be passed explicitly.
    /// See CHANGELOG.md v1.9.15 for details on this breaking change.
    #[arg(long, value_enum)]
    pub(crate) outcome: GateOutcomeArg,
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

/// CLI representation of the gate outcome; defaults to `Failed` (fail-closed).
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum GateOutcomeArg {
    /// Authorize the transition past this gate.
    Passed,
    /// Reject the transition; the workflow routes through `on_failure`.
    Failed,
    /// Explicitly waive the gate (does not apply). Satisfies transitions,
    /// but NOT release gates (those require `passed`).
    Waived,
}

impl From<GateOutcomeArg> for sddk_domain::GateOutcomeStatus {
    fn from(value: GateOutcomeArg) -> Self {
        match value {
            GateOutcomeArg::Passed => sddk_domain::GateOutcomeStatus::Passed,
            GateOutcomeArg::Failed => sddk_domain::GateOutcomeStatus::Failed,
            GateOutcomeArg::Waived => sddk_domain::GateOutcomeStatus::Waived,
        }
    }
}

#[derive(Debug, Subcommand)]
pub(crate) enum CycleLockCommand {
    /// Acquire an absent or expired lease; replaces bump the fencing token (reacquire semantics).
    Acquire(CycleLockAcquireArgs),
    /// Extend the expiry of the lease you already hold; keeps the fencing token (reuse / renew semantics).
    Renew(CycleLockRenewArgs),
    /// Release the lease you hold; emits a `lease.released` ledger event when the row is actually deleted.
    Release(CycleLockReleaseArgs),
    /// Show the current lease.
    Status(CycleLockStatusArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CycleLockAcquireArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Lease owner.
    #[arg(long)]
    pub(crate) owner: String,
    /// Lease duration in milliseconds.
    #[arg(long, default_value_t = 3_600_000)]
    pub(crate) lease_ms: i64,
    /// Explicit RFC 3339 timestamp for deterministic execution.
    #[arg(long)]
    pub(crate) timestamp: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CycleLockRenewArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Lease owner.
    #[arg(long)]
    pub(crate) owner: String,
    /// Fencing token currently held by the caller.
    #[arg(long)]
    pub(crate) fencing_token: i64,
    /// Lease duration in milliseconds.
    #[arg(long, default_value_t = 3_600_000)]
    pub(crate) lease_ms: i64,
    /// Explicit RFC 3339 timestamp for deterministic execution.
    #[arg(long)]
    pub(crate) timestamp: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CycleLockReleaseArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Lease owner.
    #[arg(long)]
    pub(crate) owner: String,
    /// Fencing token issued at acquisition.
    #[arg(long)]
    pub(crate) fencing_token: i64,
    /// Explicit actor for deterministic execution.
    #[arg(long)]
    pub(crate) actor: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CycleLockStatusArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum CyclePathArg {
    AMin,
    ALite,
    AFull,
    BDirect,
}

impl From<CyclePathArg> for CyclePath {
    fn from(value: CyclePathArg) -> Self {
        match value {
            CyclePathArg::AMin => CyclePath::AMin,
            CyclePathArg::ALite => CyclePath::ALite,
            CyclePathArg::AFull => CyclePath::AFull,
            CyclePathArg::BDirect => CyclePath::BDirect,
        }
    }
}

/// Parse a tuning `path_bias` value into a cycle path argument.
fn parse_tuned_path(bias: &str) -> Option<CyclePathArg> {
    match bias.trim().to_ascii_lowercase().as_str() {
        "a-min" | "amin" => Some(CyclePathArg::AMin),
        "a-lite" | "alite" => Some(CyclePathArg::ALite),
        "a-full" | "afull" => Some(CyclePathArg::AFull),
        "b-direct" | "bdirect" => Some(CyclePathArg::BDirect),
        _ => None,
    }
}

pub(crate) fn run_cycle(command: CycleCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        CycleCommand::Start(args) => run_cycle_start(args, environment),
        CycleCommand::Status(args) => run_cycle_status(args, environment),
        CycleCommand::Transition(args) => run_cycle_transition(args, environment),
        CycleCommand::Rebuild(args) => run_cycle_rebuild(args, environment),
        CycleCommand::Supersede(args) => run_cycle_supersede(args, environment),
        CycleCommand::Replan(args) => run_cycle_replan(args, environment),
        CycleCommand::ArtifactsDir(args) => run_cycle_artifacts_dir(args, environment),
        CycleCommand::EvaluateGate(args) => run_cycle_evaluate_gate(args, environment),
        CycleCommand::Lock(command) => run_cycle_lock(command, environment),
        CycleCommand::Inventory(args) => {
            crate::inventory_cycle::run_cycle_inventory(args, environment)
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct CycleStartOutput {
    cycle_id: String,
    status: String,
    phase: String,
    path: String,
    branch: String,
    sequence: i64,
    event_id: String,
    event_hash: String,
    lease: Option<LeaseOutput>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct LeaseOutput {
    owner: String,
    acquired_at_ms: i64,
    expires_at_ms: i64,
    fencing_token: i64,
}

fn run_cycle_start(args: CycleStartArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<CycleStartOutput> {
        let mut context = RuntimeContext::open(&args.runtime, environment, true)?;
        let scope = normalize_scope(&args.runtime.scope)?;
        let cycle_id = CycleId::from_parts(&context.identity.project_id, &args.name)?;
        let mut manifest = CycleManifest::new(
            context.identity.project_id.to_string(),
            context.workspace_id.clone(),
            cycle_id,
            args.name.clone(),
            args.branch.clone().unwrap_or_else(|| match args.path {
                Some(CyclePathArg::AMin) | None => "main".to_owned(),
                _ => format!("feat/{}", args.name),
            }),
            args.base.clone().unwrap_or_else(|| "HEAD".to_owned()),
        );
        // Resolve the workflow path: explicit --path wins; otherwise the F3
        // tuning recommendation (path_bias) when present; else the A-full default.
        let effective_path = match args.path {
            Some(path) => path,
            None => crate::metrics::read_tuning_path_bias(&context)
                .and_then(|bias| parse_tuned_path(&bias))
                .unwrap_or(CyclePathArg::AFull),
        };
        manifest.path = effective_path.into();
        manifest.remote_url = context.identity.remote_url.clone();
        manifest.scope = Some(scope);
        let input = CycleStartInput {
            manifest,
            requirements: CYCLE_START_REQUIREMENTS
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
        };
        let plan = context.engine.plan_cycle_start(input)?;
        let timestamp = args.timestamp.clone().unwrap_or_else(default_timestamp);
        let command_id = format!("cycle.start-{}", Uuid::new_v4().hyphenated());
        let started = context.engine.apply_cycle_start(
            &plan,
            &event_context(
                &command_id,
                &format!("evt-{}", Uuid::new_v4().hyphenated()),
                &args.actor,
                environment,
                &timestamp,
            ),
        )?;
        let lease = match &args.lease_owner {
            Some(owner) => {
                let now_ms = timestamp_ms(args.timestamp.as_deref())?;
                Some(context.storage.acquire_cycle_lease(
                    &started.manifest.cycle_id,
                    owner,
                    now_ms,
                    now_ms + args.lease_ms,
                )?)
            }
            None => None,
        };
        Ok(CycleStartOutput {
            cycle_id: started.manifest.cycle_id,
            status: wire(&started.manifest.status),
            phase: wire(&started.manifest.phase),
            path: cycle_path_text(&started.manifest.path),
            branch: started.manifest.branch,
            sequence: started.event.sequence,
            event_id: started.event.event_id,
            event_hash: started.event.event_hash,
            lease: lease.map(Into::into),
        })
    })();
    render_result(result, format, cycle_start_text)
}

fn run_cycle_status(args: CycleStatusArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<CycleStatusOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let record = context.storage.get_cycle(&args.cycle)?;
        let lease = context.storage.get_cycle_lease(&args.cycle).ok();
        Ok(CycleStatusOutput {
            cycle_id: record.manifest.cycle_id,
            status: wire(&record.manifest.status),
            phase: wire(&record.manifest.phase),
            path: cycle_path_text(&record.manifest.path),
            updated_at: record.updated_at,
            artifacts: record.manifest.artifacts.len(),
            lease: lease.map(Into::into),
        })
    })();
    render_result(result, format, cycle_status_text)
}

fn run_cycle_transition(args: CycleTransitionArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<CycleTransitionOutput> {
        let mut context = RuntimeContext::open(&args.runtime, environment, false)?;
        let now_ms = timestamp_ms(args.timestamp.as_deref())?;
        match context
            .storage
            .get_cycle_lease(&args.cycle)
            .map_err(Into::into)
        {
            Ok(_) => {
                let owner = args.lease_owner.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("cycle {} is leased; --lease-owner is required", args.cycle)
                })?;
                let token = args.fencing_token.ok_or_else(|| {
                    anyhow::anyhow!(
                        "cycle {} is leased; --fencing-token is required",
                        args.cycle
                    )
                })?;
                context
                    .engine
                    .require_lease_fence(&args.cycle, owner, token, now_ms)?;
            }
            Err(sddk_domain::StorageError::NotFound { .. }) => {
                if args.lease_owner.is_some() || args.fencing_token.is_some() {
                    anyhow::bail!(
                        "cycle {} has no lease; fencing arguments are not applicable",
                        args.cycle
                    );
                }
            }
            Err(error) => return Err(error.into()),
        }
        let mut evidence = TransitionEvidence {
            requirements: args
                .requirement
                .iter()
                .map(|value| value.to_owned())
                .collect(),
            ..TransitionEvidence::default()
        };
        for artifact in &args.artifact {
            let (kind, path) = split_artifact(artifact)?;
            evidence
                .artifacts
                .insert(kind.clone(), ArtifactRef::new(kind, path));
        }
        for receipt_id in &args.gate_receipt {
            let receipt = context.storage.get_gate_receipt(receipt_id)?;
            evidence.gates.insert(
                receipt.gate.clone(),
                sddk_engine::GateReceiptRef {
                    receipt_id: receipt.receipt_id.clone(),
                },
            );
        }
        let plan = context
            .engine
            .plan_transition(&args.cycle, &args.transition, evidence)?;
        let timestamp = args.timestamp.clone().unwrap_or_else(default_timestamp);
        let command_id = format!("cycle.transition-{}", Uuid::new_v4().hyphenated());
        let actor_id = args
            .actor
            .clone()
            .or_else(|| environment.sddk_actor.clone())
            .or_else(|| environment.user.clone())
            .unwrap_or_else(|| "sddk-cli".into());
        let actor_kind = if actor_id.starts_with("user:") {
            ActorKind::Human
        } else if actor_id.starts_with("agent:") {
            ActorKind::Agent
        } else {
            ActorKind::System
        };
        let event_id_prefix = format!("tr-{}", args.cycle);
        let outcome_input = OutcomeEventInput {
            project_id: context.identity.project_id.to_string(),
            cycle_id: args.cycle.clone(),
            transition_id: args.transition.clone(),
            from_phase: None,
            to_phase: None,
            transition_at: timestamp.clone(),
            actor_id: actor_id.clone(),
            actor_kind,
            event_id_prefix,
            failed_gates: vec![],
        };
        let applied = match context.engine.apply_transition(
            &plan,
            &event_context(
                &command_id,
                &format!("evt-{}", Uuid::new_v4().hyphenated()),
                &args.actor,
                environment,
                &timestamp,
            ),
        ) {
            Ok(applied) => {
                // Emit workflow.transition.succeeded
                let mut outcome_input = outcome_input;
                outcome_input.from_phase = Some(wire(&plan.state_before().phase));
                outcome_input.to_phase = Some(wire(&plan.state_after().phase));
                outcome_input.failed_gates = plan.failed_gates().to_vec();
                let outcome_store_path = context.paths.ledger.parent().unwrap();
                if let Ok(mut store) = SqliteEventStore::open(outcome_store_path) {
                    let _ = event_bus::emit_outcome_event(
                        &mut store,
                        &outcome_input,
                        TransitionOutcome::Succeeded,
                    );
                }
                // Emit workflow.phase events on successful phase transitions (fail-soft)
                if plan.state_before().phase != plan.state_after().phase {
                    let phase_input = PhaseEventInput {
                        project_id: context.identity.project_id.to_string(),
                        cycle_id: plan.state_before().cycle_id.clone(),
                        from_phase: wire(&plan.state_before().phase),
                        to_phase: wire(&plan.state_after().phase),
                        transition_at: timestamp.clone(),
                        actor_id,
                        actor_kind,
                        event_id_prefix: format!("ph-{}", plan.state_before().cycle_id),
                    };
                    if let Ok(mut store) = SqliteEventStore::open(outcome_store_path) {
                        let _ = event_bus::emit_phase_event(&mut store, &phase_input);
                    }
                }
                applied
            }
            Err(engine_err) => {
                // Emit workflow.transition.failed for EngineError
                let outcome_store_path = context.paths.ledger.parent().unwrap();
                if let Ok(mut store) = SqliteEventStore::open(outcome_store_path) {
                    let _ = event_bus::emit_outcome_event(
                        &mut store,
                        &outcome_input,
                        TransitionOutcome::Failed,
                    );
                }
                return Err(engine_err.into());
            }
        };
        if applied.manifest.status == sddk_domain::CycleStatus::Closed
            && let Err(error) = crate::metrics::capture_cycle_metrics(&context, &applied.manifest)
        {
            eprintln!("warning: auto metrics capture failed: {error}");
        }
        Ok(CycleTransitionOutput {
            cycle_id: applied.manifest.cycle_id,
            transition_id: applied.transition_id,
            outcome: transition_outcome_text(applied.outcome),
            status: wire(&applied.manifest.status),
            phase: wire(&applied.manifest.phase),
            sequence: applied.event.sequence,
            event_id: applied.event.event_id,
            event_hash: applied.event.event_hash,
        })
    })();
    render_result(result, format, cycle_transition_text)
}

fn run_cycle_rebuild(args: CycleRebuildArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<CycleRebuildOutput> {
        let mut context = RuntimeContext::open(&args.runtime, environment, false)?;
        let now_ms = timestamp_ms(args.timestamp.as_deref())?;
        // `rebuild` is no longer a silent read-only audit: it requires the
        // caller to hold the same lease fence as a phase transition. An
        // expired lease is rejected with `LeaseExpired` (fail-closed).
        match context
            .storage
            .get_cycle_lease(&args.cycle)
            .map_err(Into::into)
        {
            Ok(_) => {
                let owner = args.lease_owner.as_deref().ok_or_else(|| {
                    anyhow::anyhow!(
                        "cycle {} is leased; --lease-owner is required for rebuild",
                        args.cycle
                    )
                })?;
                let token = args.fencing_token.ok_or_else(|| {
                    anyhow::anyhow!(
                        "cycle {} is leased; --fencing-token is required for rebuild",
                        args.cycle
                    )
                })?;
                context
                    .engine
                    .require_lease_fence(&args.cycle, owner, token, now_ms)?;
            }
            Err(sddk_domain::StorageError::NotFound { .. }) => {
                anyhow::bail!(
                    "cycle {} has no lease; acquire one with `sddk cycle lock acquire` before rebuild",
                    args.cycle
                );
            }
            Err(error) => return Err(error.into()),
        }
        let occurred_at = args
            .timestamp
            .clone()
            .unwrap_or_else(crate::git_cmd::default_timestamp);
        let command_id = format!("cycle.rebuild-{}", Uuid::new_v4().hyphenated());
        let event_context = event_context(
            &command_id,
            &format!("evt-{}", Uuid::new_v4().hyphenated()),
            &args.actor,
            environment,
            &occurred_at,
        );
        let rebuilt =
            context
                .engine
                .rebuild_cycle(&args.cycle, &event_context, now_ms, args.dry_run)?;
        Ok(CycleRebuildOutput {
            cycle_id: rebuilt.manifest.cycle_id,
            status: wire(&rebuilt.manifest.status),
            phase: wire(&rebuilt.manifest.phase),
            sequence: rebuilt.sequence,
            restored: rebuilt.restored,
        })
    })();
    render_result(result, format, cycle_rebuild_text)
}

fn run_cycle_artifacts_dir(
    args: CycleArtifactsDirArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<ArtifactsDirOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let dir = context.cycle_artifacts_path.join(&args.cycle);
        std::fs::create_dir_all(&dir)?;
        Ok(ArtifactsDirOutput {
            cycle_id: args.cycle,
            path: dir,
        })
    })();
    render_result(result, format, artifacts_dir_text)
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ArtifactsDirOutput {
    cycle_id: String,
    path: PathBuf,
}

fn artifacts_dir_text(output: &ArtifactsDirOutput) -> String {
    format!("{}\n", output.path.display())
}

fn run_cycle_lock(command: CycleLockCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        CycleLockCommand::Acquire(args) => run_cycle_lock_acquire(args, environment),
        CycleLockCommand::Renew(args) => run_cycle_lock_renew(args, environment),
        CycleLockCommand::Release(args) => run_cycle_lock_release(args, environment),
        CycleLockCommand::Status(args) => run_cycle_lock_status(args, environment),
    }
}

/// Validates that the given cycle_id belongs to the expected project.
///
/// Returns `Ok(())` if the cycle's project prefix matches `expected_project_id`.
/// Returns `Err(StorageError::CycleProjectMismatch)` if the cycle belongs to a
/// different project (fail-fast before any SQL).
/// Returns `Err(StorageError::NotFound)` if the cycle_id is malformed (per
/// REQ-GAP6-4: malformed cycle ids keep `STORAGE_NOT_FOUND`).
fn validate_cycle_project(
    cycle_id: &str,
    expected_project_id: &sddk_domain::ProjectId,
) -> Result<(), sddk_storage::StorageError> {
    match CycleId::new(cycle_id) {
        Ok(cid) => {
            let cycle_project = cid.project();
            let expected = expected_project_id.as_str();
            if cycle_project != expected {
                Err(sddk_storage::StorageError::CycleProjectMismatch {
                    cycle_id: cycle_id.to_owned(),
                    cycle_project_id: cycle_project.to_owned(),
                    expected_project_id: expected.to_owned(),
                })
            } else {
                Ok(())
            }
        }
        // Malformed cycle ids (no project prefix) continue to STORAGE_NOT_FOUND
        Err(_) => Err(sddk_storage::StorageError::NotFound {
            entity: "cycle",
            id: cycle_id.to_owned(),
        }),
    }
}

fn run_cycle_lock_acquire(
    args: CycleLockAcquireArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<LeaseOutput> {
        let mut context = RuntimeContext::open(&args.runtime, environment, false)?;
        validate_cycle_project(&args.cycle, &context.identity.project_id)?;
        let now_ms = timestamp_ms(args.timestamp.as_deref())?;
        let lease = context.storage.acquire_cycle_lease(
            &args.cycle,
            &args.owner,
            now_ms,
            now_ms + args.lease_ms,
        )?;
        Ok(lease.into())
    })();
    render_result(result, format, lease_text)
}

fn run_cycle_lock_renew(args: CycleLockRenewArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<LeaseOutput> {
        let mut context = RuntimeContext::open(&args.runtime, environment, false)?;
        validate_cycle_project(&args.cycle, &context.identity.project_id)?;
        let now_ms = timestamp_ms(args.timestamp.as_deref())?;
        let lease = context.storage.renew_cycle_lease(
            &args.cycle,
            &args.owner,
            args.fencing_token,
            now_ms,
            now_ms + args.lease_ms,
        )?;
        Ok(LeaseOutput::from(lease))
    })();
    render_result(result, format, lease_text)
}

fn run_cycle_lock_release(
    args: CycleLockReleaseArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<CycleLockReleaseOutput> {
        let mut context = RuntimeContext::open(&args.runtime, environment, false)?;
        validate_cycle_project(&args.cycle, &context.identity.project_id)?;
        let command_id = format!("cycle.lock.release-{}", Uuid::new_v4().hyphenated());
        let actor = args
            .actor
            .clone()
            .or_else(|| environment.sddk_actor.clone())
            .or_else(|| environment.user.clone())
            .unwrap_or_else(|| "sddk-cli".into());
        let released = context.storage.release_cycle_lease(
            context.identity.project_id.as_str(),
            &args.cycle,
            &args.owner,
            args.fencing_token,
            &actor,
            &command_id,
            &default_timestamp(),
        )?;
        Ok(CycleLockReleaseOutput { released })
    })();
    render_result(result, format, cycle_lock_release_text)
}

fn run_cycle_supersede(args: CycleSupersedeArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<CycleSupersedeOutput> {
        let mut context = RuntimeContext::open(&args.runtime, environment, false)?;
        let timestamp = args.timestamp.clone().unwrap_or_else(default_timestamp);
        let actor = args
            .actor
            .clone()
            .or_else(|| environment.sddk_actor.clone())
            .or_else(|| environment.user.clone())
            .unwrap_or_else(|| "sddk-cli".into());
        let command_id = format!("cycle.supersede-{}", Uuid::new_v4().hyphenated());
        let event_id = format!("evt-{}", Uuid::new_v4().hyphenated());

        // Parse evidence refs
        let evidence_refs: Vec<String> =
            serde_json::from_str(&args.evidence_refs).unwrap_or_default();

        // Convert reason
        let reason: Option<SupersedeReason> = args.reason.map(|r| r.into());

        let receipt = context.engine.cycle_supersede(
            &args.cycle,
            args.successor,
            reason,
            &evidence_refs,
            &actor,
            &command_id,
            &event_id,
            &timestamp,
            &context.paths.cycle_artifacts,
            &args.lease_owner,
            args.fencing_token,
        )?;

        // Load updated cycle to get manifest
        let record = context.storage.get_cycle(&args.cycle)?;
        Ok(CycleSupersedeOutput {
            cycle_id: args.cycle,
            status: wire(&record.manifest.status),
            event_id: receipt.event_id,
            sequence: receipt.sequence,
            event_hash: receipt.event_hash,
        })
    })();
    render_result(result, format, cycle_supersede_text)
}

fn run_cycle_replan(args: CycleReplanArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<CycleReplanOutput> {
        let mut context = RuntimeContext::open(&args.runtime, environment, false)?;
        let timestamp = args.timestamp.clone().unwrap_or_else(default_timestamp);
        let actor = args
            .actor
            .clone()
            .or_else(|| environment.sddk_actor.clone())
            .or_else(|| environment.user.clone())
            .unwrap_or_else(|| "sddk-cli".into());
        let command_id = format!("cycle.replan-{}", Uuid::new_v4().hyphenated());
        let event_id = format!("evt-{}", Uuid::new_v4().hyphenated());

        // Parse delta
        let delta: ReplanDelta = serde_json::from_str(&args.delta)?;
        let evidence_refs: Vec<String> =
            serde_json::from_str(&args.evidence_refs).unwrap_or_default();

        let restage_to: RestageTo = args.restage_to.into();
        let restage_to_display = format!("{:?}", restage_to);

        // Note: cycle_replan is a stub that returns ReplanLimitExceeded.
        // When implemented, this will return Ok(()) on success.
        context.engine.cycle_replan(
            &args.cycle,
            restage_to,
            &delta,
            &evidence_refs,
            &actor,
            &command_id,
            &event_id,
            &timestamp,
            &context.paths.cycle_artifacts,
            &args.lease_owner,
            args.fencing_token,
        )?;

        // Load updated cycle to get sequence (only reached if cycle_replan succeeds)
        let _record = context.storage.get_cycle(&args.cycle)?;
        Ok(CycleReplanOutput {
            cycle_id: args.cycle,
            restage_to: restage_to_display,
            sequence: 0, // Placeholder until cycle_replan returns sequence info
        })
    })();
    render_result(result, format, cycle_replan_text)
}

fn run_cycle_lock_status(args: CycleLockStatusArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<Option<LeaseOutput>> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        // REQ-GAP6-3: apply same project-prefix guard as acquire/renew/release
        validate_cycle_project(&args.cycle, &context.identity.project_id)?;
        // REQ-DEBT017-5: cycle not found → typed error; cycle exists but no lease → None
        let lease = match context.storage.get_cycle_lease(&args.cycle) {
            Ok(l) => Some(l),
            Err(sddk_storage::StorageError::NotFound {
                entity: "cycle", ..
            }) => {
                // Cycle does not exist in `cycles` table → propagate typed error
                return Err(sddk_storage::StorageError::NotFound {
                    entity: "cycle",
                    id: args.cycle.clone(),
                }
                .into());
            }
            Err(sddk_storage::StorageError::NotFound { .. }) => None,
            Err(e) => return Err(e.into()),
        };
        Ok(lease.map(Into::into))
    })();
    render_result(result, format, lease_option_text)
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct CycleStatusOutput {
    cycle_id: String,
    status: String,
    phase: String,
    path: String,
    updated_at: String,
    artifacts: usize,
    lease: Option<LeaseOutput>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct CycleTransitionOutput {
    cycle_id: String,
    transition_id: String,
    outcome: String,
    status: String,
    phase: String,
    sequence: i64,
    event_id: String,
    event_hash: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct CycleRebuildOutput {
    cycle_id: String,
    status: String,
    phase: String,
    sequence: i64,
    restored: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct CycleSupersedeOutput {
    cycle_id: String,
    status: String,
    event_id: String,
    sequence: i64,
    event_hash: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct CycleReplanOutput {
    cycle_id: String,
    restage_to: String,
    sequence: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct CycleLockReleaseOutput {
    released: bool,
}

impl From<sddk_domain::CycleLease> for LeaseOutput {
    fn from(value: sddk_domain::CycleLease) -> Self {
        Self {
            owner: value.owner,
            acquired_at_ms: value.acquired_at_ms,
            expires_at_ms: value.expires_at_ms,
            fencing_token: value.fencing_token,
        }
    }
}

fn cycle_start_text(output: &CycleStartOutput) -> String {
    format!(
        "cycle_id: {}\nstatus: {}\nphase: {}\npath: {}\nsequence: {}\nevent_id: {}\nevent_hash: {}\n{}",
        output.cycle_id,
        output.status,
        output.phase,
        output.path,
        output.sequence,
        output.event_id,
        output.event_hash,
        output
            .lease
            .as_ref()
            .map(lease_text)
            .unwrap_or_else(|| "lease: none\n".to_owned())
    )
}

fn cycle_status_text(output: &CycleStatusOutput) -> String {
    format!(
        "cycle_id: {}\nstatus: {}\nphase: {}\npath: {}\nupdated_at: {}\nartifacts: {}\n{}",
        output.cycle_id,
        output.status,
        output.phase,
        output.path,
        output.updated_at,
        output.artifacts,
        output
            .lease
            .as_ref()
            .map(lease_text)
            .unwrap_or_else(|| "lease: none\n".to_owned())
    )
}

fn cycle_transition_text(output: &CycleTransitionOutput) -> String {
    format!(
        "cycle_id: {}\ntransition_id: {}\noutcome: {}\nstatus: {}\nphase: {}\nsequence: {}\nevent_id: {}\nevent_hash: {}\n",
        output.cycle_id,
        output.transition_id,
        output.outcome,
        output.status,
        output.phase,
        output.sequence,
        output.event_id,
        output.event_hash
    )
}

fn cycle_rebuild_text(output: &CycleRebuildOutput) -> String {
    format!(
        "cycle_id: {}\nstatus: {}\nphase: {}\nsequence: {}\nrestored: {}\n",
        output.cycle_id, output.status, output.phase, output.sequence, output.restored
    )
}

fn cycle_supersede_text(output: &CycleSupersedeOutput) -> String {
    format!(
        "cycle_id: {}\nstatus: {}\nevent_id: {}\nsequence: {}\nevent_hash: {}\n",
        output.cycle_id, output.status, output.event_id, output.sequence, output.event_hash
    )
}

fn cycle_replan_text(output: &CycleReplanOutput) -> String {
    format!(
        "cycle_id: {}\nrestage_to: {}\nsequence: {}\n",
        output.cycle_id, output.restage_to, output.sequence
    )
}

fn cycle_lock_release_text(output: &CycleLockReleaseOutput) -> String {
    format!("released: {}\n", output.released)
}

fn lease_text(lease: &LeaseOutput) -> String {
    format!(
        "lease: owner={} fencing_token={} acquired_at_ms={} expires_at_ms={}\n",
        lease.owner, lease.fencing_token, lease.acquired_at_ms, lease.expires_at_ms
    )
}

fn lease_option_text(lease: &Option<LeaseOutput>) -> String {
    match lease {
        Some(lease) => lease_text(lease),
        None => "lease: none\n".to_owned(),
    }
}

fn cycle_path_text(path: &CyclePath) -> String {
    match path {
        CyclePath::AMin => "A-min",
        CyclePath::ALite => "A-lite",
        CyclePath::AFull => "A-full",
        CyclePath::BDirect => "B-direct",
    }
    .to_owned()
}

fn transition_outcome_text(outcome: sddk_engine::TransitionOutcome) -> String {
    match outcome {
        sddk_engine::TransitionOutcome::Succeeded => "succeeded",
        sddk_engine::TransitionOutcome::Failed => "failed",
    }
    .to_owned()
}

fn split_artifact(value: &str) -> anyhow::Result<(String, String)> {
    let (kind, path) = value
        .split_once('=')
        .ok_or_else(|| anyhow::anyhow!("artifact must use kind=path: {value}"))?;
    if kind.is_empty() || path.is_empty() {
        anyhow::bail!("artifact must use kind=path: {value}");
    }
    Ok((kind.to_owned(), path.to_owned()))
}

fn event_context(
    command_id: &str,
    event_id: &str,
    explicit_actor: &Option<String>,
    environment: &CliEnvironment,
    occurred_at: &str,
) -> EventContext {
    EventContext {
        command_id: command_id.to_owned(),
        frame_id: format!("frame:{command_id}"),
        event_id: event_id.to_owned(),
        actor: explicit_actor
            .clone()
            .or_else(|| environment.sddk_actor.clone())
            .or_else(|| environment.user.clone())
            .unwrap_or_else(|| "sddk-cli".into()),
        occurred_at: occurred_at.to_owned(),
    }
}

fn wire<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .expect("workflow enums are serializable")
        .as_str()
        .expect("workflow enums serialize as strings")
        .to_owned()
}

fn default_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("RFC 3339 formatting cannot fail")
}

fn timestamp_ms(timestamp: Option<&str>) -> anyhow::Result<i64> {
    match timestamp {
        Some(value) => Ok(OffsetDateTime::parse(value, &Rfc3339)?.unix_timestamp() * 1000),
        None => Ok(OffsetDateTime::now_utc().unix_timestamp() * 1000),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct GateEvaluationOutput {
    receipt_id: String,
    gate: String,
    evaluator: String,
    transition_id: String,
    plan_hash: String,
    /// HMAC-SHA256 signature over the canonical receipt payload (Phase 9).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    signature: Option<String>,
}

/// Builds the canonical payload that gets signed for a gate receipt.
fn gate_receipt_payload(
    receipt_id: &str,
    gate: &str,
    transition_id: &str,
    plan_hash: &str,
) -> String {
    format!("{receipt_id}|{gate}|{transition_id}|{plan_hash}")
}

fn run_cycle_evaluate_gate(
    args: CycleEvaluateGateArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<GateEvaluationOutput> {
        let mut context = RuntimeContext::open(&args.runtime, environment, false)?;
        let timestamp = args
            .timestamp
            .clone()
            .unwrap_or_else(crate::git_cmd::default_timestamp);
        let actor = args
            .actor
            .clone()
            .or_else(|| environment.sddk_actor.clone())
            .or_else(|| environment.user.clone())
            .unwrap_or_else(|| "sddk-cli".into());
        // Fail-closed: when --outcome is omitted we record `Failed`, so a
        // caller that wants to advance the workflow MUST pass
        // `--outcome passed` explicitly.
        let outcome = args.outcome.into();
        let receipt = context.engine.evaluate_gate(&GateEvaluationInput {
            cycle_id: args.cycle.clone(),
            transition_id: args.transition.clone(),
            gate: args.gate.clone(),
            evaluator: args.evaluator.clone(),
            evidence: serde_json::from_str(&args.evidence)?,
            outcome,
            evaluated_at: timestamp,
            actor,
            command_id: format!("gate-{}", uuid::Uuid::new_v4().hyphenated()),
        })?;
        // Sign the receipt with the local key (fail-closed: no key → no
        // signature; release verify will reject unsigned receipts).
        let signature = {
            let keys_dir = context.paths.project_data.join("keys");
            let key = sddk_engine::load_or_create_key(&keys_dir)?;
            let payload = gate_receipt_payload(
                &receipt.receipt_id,
                &receipt.gate,
                &receipt.transition_id,
                &receipt.plan_hash,
            );
            Some(sddk_engine::sign_payload(&payload, &key)?)
        };
        Ok(GateEvaluationOutput {
            receipt_id: receipt.receipt_id,
            gate: receipt.gate,
            evaluator: receipt.evaluator,
            transition_id: receipt.transition_id,
            plan_hash: receipt.plan_hash,
            signature,
        })
    })();
    render_result(result, format, gate_evaluation_text)
}

fn gate_evaluation_text(output: &GateEvaluationOutput) -> String {
    format!(
        "receipt_id: {}\ngate: {}\nevaluator: {}\ntransition_id: {}\nplan_hash: {}\n",
        output.receipt_id, output.gate, output.evaluator, output.transition_id, output.plan_hash
    )
}
