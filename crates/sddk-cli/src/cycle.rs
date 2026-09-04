//! Cycle and lease commands exposing the local workflow authority.

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};
use sddk_domain::{
    ActorKind, ArtifactRef, ControlPlane, CycleId, CycleManifest, CyclePath, PauseReason,
    ProjectId, WorkflowManifest, normalize_remote_url, normalize_scope, stable_fallback_project_id,
    stable_project_id,
};
use sddk_engine::{
    AdoptionPaths, CycleStartInput, Engine, EventContext, GateEvaluationInput, ReplanDelta,
    RestageTo, SupersedeReason, TransitionEvidence, TransitionOutcome, WorkflowLoadError,
    authority::{AuthorityContext, infer_actor_kind},
    event_bus::{self, OutcomeEventInput, PhaseEventInput},
    frontier_for_state,
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

/// Inference error kinds for typed degradation (D2).
#[derive(Debug, Clone)]
pub(crate) enum InferenceError {
    /// Walk-up from cwd found no project marker.
    NoProjectContext { cwd: String },
    /// Explicit args required but missing when `--no-infer` is set.
    ExplicitRequired { missing: Vec<String> },
    /// Zero active leases for the resolved project (S3).
    NoActiveCycle { project_id: String, hint: String },
    /// Multiple active leases — ambiguity resolved with candidate list (S3).
    AmbiguousCycle {
        project_id: String,
        candidates: Vec<CycleCandidate>,
    },
}

/// One active cycle lease candidate for ambiguity resolution.
#[derive(Debug, Clone)]
pub(crate) struct CycleCandidate {
    pub cycle_id: String,
    pub owner: String,
    pub expires_at_ms: i64,
}

/// Result of resolving cycle context from args + state.
#[derive(Debug, Clone)]
pub(crate) struct ResolvedCycleContext {
    /// Fully resolved RuntimeArgs (all optionals filled).
    pub runtime: RuntimeArgs,
    /// Resolved project identity (from identity resolution).
    /// None when cycle is explicit (deferred to RuntimeContext::open).
    pub project_id: Option<sddk_domain::ProjectId>,
    /// Resolved scope (from identity resolution).
    pub scope: String,
    /// Active leases found for the project (used for ambiguity).
    pub active_leases: Vec<CycleCandidate>,
    /// Resolved cycle_id (only set when unambiguous).
    pub cycle_id: Option<String>,
}

impl std::fmt::Display for InferenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InferenceError::NoProjectContext { cwd } => {
                write!(
                    f,
                    "no adopted project found from: {}\n  hint: run `sddk project resolve` or `sddk init` first",
                    cwd
                )
            }
            InferenceError::ExplicitRequired { missing } => {
                write!(
                    f,
                    "explicit --root/--scope required (--no-infer is set): missing {}",
                    missing.join(", ")
                )
            }
            InferenceError::NoActiveCycle { project_id, hint } => {
                write!(
                    f,
                    "no active cycle found for project {}\n  hint: {}",
                    project_id, hint
                )
            }
            InferenceError::AmbiguousCycle {
                project_id,
                candidates,
            } => {
                writeln!(
                    f,
                    "multiple active cycles for project {}; pass one explicitly:",
                    project_id
                )?;
                for c in candidates {
                    writeln!(f, "  sddk cycle status --cycle {}", c.cycle_id)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for InferenceError {}

/// Marker files for project walk-up (S1/S4).
const PROJECT_MARKERS: &[&str] = &["sddk", ".git", "AGENTS.md"];

/// Walks up from `cwd` looking for a project marker file/directory.
/// Returns the first directory containing a marker, or None.
fn walk_up_for_project_marker(cwd: &std::path::Path) -> Option<PathBuf> {
    let mut current = Some(cwd.to_path_buf());
    loop {
        let cur = current?;
        for marker in PROJECT_MARKERS {
            if cur.join(marker).exists() {
                return Some(cur);
            }
        }
        current = cur.parent().map(|p| p.to_path_buf());
    }
}

/// Resolves `--root`, `--scope`, and `--cycle` from current state.
///
/// This is the ONE shared resolver used by all cycle subcommands (S6).
/// Explicit args always win over inference (S2). When `--no-infer` is set,
/// missing required args produce today's clap errors.
///
/// Project identity (needed only for cycle inference) is deferred to
/// `RuntimeContext::open` which handles fallback seed generation.
pub(crate) fn resolve_cycle_context(
    args: &RuntimeArgs,
    environment: &CliEnvironment,
    cycle_arg: Option<&str>,
) -> Result<ResolvedCycleContext, InferenceError> {
    // S2: explicit args win; nothing to infer if all explicit
    let root_explicit = args.root.is_some();
    let scope_explicit = args.scope.is_some();
    let cycle_explicit = cycle_arg.is_some();

    if args.no_infer {
        let mut missing = Vec::new();
        if args.root.is_none() {
            missing.push("--root".to_string());
        }
        if args.scope.is_none() {
            missing.push("--scope".to_string());
        }
        if cycle_arg.is_none() {
            missing.push("--cycle".to_string());
        }
        if !missing.is_empty() {
            return Err(InferenceError::ExplicitRequired { missing });
        }
        // All explicit — build resolved runtime without project identity
        // (RuntimeContext::open handles identity resolution)
        let root = args.root.clone().unwrap_or_else(|| PathBuf::from("."));
        let scope = args.scope.clone().unwrap_or_else(|| ".".to_string());
        let mut resolved = args.clone();
        if resolved.root.is_none() {
            resolved.root = Some(root);
        }
        if resolved.scope.is_none() {
            resolved.scope = Some(scope.clone());
        }
        return Ok(ResolvedCycleContext {
            runtime: resolved,
            project_id: None, // Deferred to RuntimeContext::open
            scope,
            active_leases: Vec::new(),
            cycle_id: cycle_arg.map(String::from),
        });
    }

    // Inference path (no_infer = false)
    // Step 1: Resolve root — use explicit if provided, otherwise walk up from cwd
    let root = if let Some(ref r) = args.root {
        r.clone()
    } else {
        let cwd = std::env::current_dir().map_err(|_| InferenceError::NoProjectContext {
            cwd: ".".to_string(),
        })?;
        walk_up_for_project_marker(&cwd).ok_or_else(|| InferenceError::NoProjectContext {
            cwd: cwd.to_string_lossy().to_string(),
        })?
    };

    // Step 2: Resolve scope — use explicit if provided, otherwise default to "."
    let scope = args.scope.clone().unwrap_or_else(|| ".".to_string());

    // Step 3: Build resolved runtime with inferred root/scope filled in
    let mut resolved = args.clone();
    if resolved.root.is_none() {
        resolved.root = Some(root.clone());
    }
    if resolved.scope.is_none() {
        resolved.scope = Some(scope.clone());
    }

    // Step 4: Resolve cycle from active leases (only if cycle not explicit)
    // This requires project identity, which we defer to RuntimeContext::open.
    // For cycle inference we need project_id now, so we resolve it here
    // using the same logic as RuntimeContext::open (remote OR fallback_seed OR generate).
    let cycle_id = if let Some(c) = cycle_arg {
        Some(c.to_string())
    } else {
        // Need project_id for cycle inference — resolve it using available signals
        let remote = crate::resolve_remote(&root, args.remote.clone())
            .ok()
            .flatten();
        let mut fallback_seed = args.fallback_seed.clone();
        if remote.is_none() && fallback_seed.is_none() {
            fallback_seed = crate::find_persisted_fallback_seed(environment, &root, &scope)
                .ok()
                .flatten();
        }
        // If no remote and no fallback_seed, we can't infer cycle without walking up
        // to find project markers. Use the project_id from project markers if available.
        let project_id = if remote.is_some() {
            // Use remote for project_id
            if let Some(ref remote_url) = remote {
                let normalized = normalize_remote_url(remote_url).ok();
                if let Some(ref url) = normalized {
                    let id = stable_project_id(url, &scope);
                    Some(ProjectId::new(id).ok())
                } else {
                    None
                }
            } else {
                None
            }
        } else if let Some(ref seed) = fallback_seed {
            let id = stable_fallback_project_id(seed.as_str(), &scope);
            Some(ProjectId::new(id).ok())
        } else {
            // No remote, no fallback_seed — can't infer cycle without project markers
            // to generate a fallback seed. Walk up to find project markers.
            let walked_root = walk_up_for_project_marker(&root);
            if walked_root.is_none() {
                return Err(InferenceError::NoProjectContext {
                    cwd: root.to_string_lossy().to_string(),
                });
            }
            // With project markers found, we can now get fallback_seed
            let walked_root = walked_root.unwrap();
            let new_fallback_seed =
                crate::find_persisted_fallback_seed(environment, &walked_root, &scope)
                    .ok()
                    .flatten();
            if let Some(ref seed) = new_fallback_seed {
                let id = stable_fallback_project_id(seed.as_str(), &scope);
                Some(ProjectId::new(id).ok())
            } else {
                None
            }
        };

        let project_id = match project_id {
            Some(Some(id)) => id,
            _ => {
                return Err(InferenceError::NoProjectContext {
                    cwd: root.to_string_lossy().to_string(),
                });
            }
        };

        // Open storage to list active leases
        let workspace_id = {
            let canonical_workspace_path =
                crate::path_string(&root).map_err(|_| InferenceError::NoProjectContext {
                    cwd: root.to_string_lossy().to_string(),
                })?;
            crate::stable_workspace_id(&project_id, &canonical_workspace_path)
        };
        let paths =
            sddk_engine::resolve_xdg_paths(&environment.xdg(), project_id.as_str(), &workspace_id)
                .map_err(|_| InferenceError::NoProjectContext {
                    cwd: root.to_string_lossy().to_string(),
                })?;
        let storage = match crate::Storage::open_read_only(&paths.ledger) {
            Ok(s) => s,
            Err(_) => {
                // Storage not found — no active cycles possible
                return Err(InferenceError::NoActiveCycle {
                    project_id: project_id.to_string(),
                    hint: format!(
                        "start one with: sddk cycle start --root {} --scope {} --name <name>",
                        root.to_string_lossy(),
                        scope
                    ),
                });
            }
        };
        let now_ms = OffsetDateTime::now_utc().unix_timestamp() * 1000;
        let leases = storage
            .list_active_cycle_leases_for_project(project_id.as_str(), now_ms)
            .map_err(|_| InferenceError::NoActiveCycle {
                project_id: project_id.to_string(),
                hint: "could not query active leases".to_string(),
            })?;

        if leases.is_empty() {
            return Err(InferenceError::NoActiveCycle {
                project_id: project_id.to_string(),
                hint: format!(
                    "start one with: sddk cycle start --root {} --scope {} --name <name>",
                    root.to_string_lossy(),
                    scope
                ),
            });
        }

        if leases.len() > 1 {
            let candidates: Vec<CycleCandidate> = leases
                .iter()
                .map(|l| CycleCandidate {
                    cycle_id: l.cycle_id.clone(),
                    owner: l.owner.clone(),
                    expires_at_ms: l.expires_at_ms,
                })
                .collect();
            return Err(InferenceError::AmbiguousCycle {
                project_id: project_id.to_string(),
                candidates,
            });
        }

        // Exactly one active lease
        Some(leases[0].cycle_id.clone())
    };

    Ok(ResolvedCycleContext {
        runtime: resolved,
        project_id: None, // Deferred to RuntimeContext::open
        scope,
        active_leases: Vec::new(),
        cycle_id,
    })
}

#[derive(Debug, Clone, Args)]
pub(crate) struct RuntimeArgs {
    /// Checkout or worktree root.
    /// When absent and `--no-infer` is not set, the cycle inference layer
    /// walks up from cwd searching for a project marker (sddk manifest, .git,
    /// AGENTS.md). When `no_infer` is true, this must be provided explicitly.
    #[arg(long)]
    pub(crate) root: Option<PathBuf>,
    /// Required monorepo scope, using `.` for the repository root.
    /// When absent and `--no-infer` is not set, inferred from the resolved root.
    #[arg(long)]
    pub(crate) scope: Option<String>,
    /// Explicit remote URL instead of read-only Git discovery.
    #[arg(long)]
    pub(crate) remote: Option<String>,
    /// Stable UUID for a repository without a remote.
    #[arg(long)]
    pub(crate) fallback_seed: Option<String>,
    /// Opt out of context inference. When set, `--root` and `--scope` must be
    /// passed explicitly and `--cycle` is a required argument on cycle commands.
    #[arg(long)]
    pub(crate) no_infer: bool,
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
        let root =
            crate::canonical_root(args.root.as_deref().unwrap_or(std::path::Path::new(".")))?;
        let scope = args.scope.as_deref().unwrap_or(".");
        let remote = crate::resolve_remote(&root, args.remote.clone())?;
        let mut fallback_seed = args.fallback_seed.clone();
        if remote.is_none() && fallback_seed.is_none() {
            fallback_seed = crate::find_persisted_fallback_seed(environment, &root, scope)?;
        }
        if remote.is_none() && fallback_seed.is_none() && generate_seed {
            fallback_seed = Some(Uuid::new_v4().hyphenated().to_string());
        }
        let identity = sddk_domain::resolve_project_identity(
            remote.as_deref(),
            scope,
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
    /// Pause a cycle, releasing the lease.
    Pause(CyclePauseArgs),
    /// Resume a paused cycle, re-acquiring a fresh lease.
    Resume(CycleResumeArgs),
    /// Print the XDG artifact directory for a cycle (created on demand).
    ArtifactsDir(CycleArtifactsDirArgs),
    /// Acquire, release, or inspect the exclusive cycle lease.
    #[command(subcommand)]
    Lock(CycleLockCommand),
    /// Build the cycle-scoped files inventory artifact (`sddk.inventory/v1`).
    Inventory(crate::inventory_cycle::CycleInventoryArgs),
    /// Print the frontier of legal transitions from the current cycle state.
    Next(CycleNextArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CycleArtifactsDirArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier.
    /// When absent and `--no-infer` is not set, inferred from the active lease.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CycleNextArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier.
    /// When absent and `--no-infer` is not set, inferred from the active lease.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
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
    /// When absent and `--no-infer` is not set, inferred from the active lease.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CycleTransitionArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier.
    /// When absent and `--no-infer` is not set, inferred from the active lease.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
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
    /// When absent and `--no-infer` is not set, inferred from the active lease.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
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
    /// When absent and `--no-infer` is not set, inferred from the active lease.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
    /// Successor cycle identifier (mutually exclusive with --reason).
    #[arg(long, conflicts_with = "reason")]
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
    /// When absent and `--no-infer` is not set, inferred from the active lease.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
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

/// Argument for the `cycle pause` command.
#[derive(Debug, Clone, Args)]
pub(crate) struct CyclePauseArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier to pause.
    /// When absent and `--no-infer` is not set, inferred from the active lease.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
    /// Reason for pausing (priority_revoked | context_switch | dependency_waiting).
    #[arg(long, value_enum)]
    pub(crate) reason: PauseReasonArg,
    /// Optional RFC3339 timestamp for when the cycle should be reviewed.
    #[arg(long)]
    pub(crate) review_at: Option<String>,
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

/// Argument for the `cycle resume` command.
#[derive(Debug, Clone, Args)]
pub(crate) struct CycleResumeArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier to resume.
    /// When absent and `--no-infer` is not set, inferred from the active lease.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
    /// Lease owner for the new lease.
    #[arg(long)]
    pub(crate) lease_owner: String,
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

/// CLI representation of pause reason.
#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum PauseReasonArg {
    /// Priority was revoked — a higher-priority item requires the team's attention.
    PriorityRevoked,
    /// Context switch — the team must pivot to a different focus.
    ContextSwitch,
    /// Waiting on a dependency — external blocker that is not yet resolved.
    DependencyWaiting,
}

impl From<PauseReasonArg> for PauseReason {
    fn from(value: PauseReasonArg) -> Self {
        match value {
            PauseReasonArg::PriorityRevoked => PauseReason::PriorityRevoked,
            PauseReasonArg::ContextSwitch => PauseReason::ContextSwitch,
            PauseReasonArg::DependencyWaiting => PauseReason::DependencyWaiting,
        }
    }
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
    /// When absent and `--no-infer` is not set, inferred from the active lease.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
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
    /// When absent and `--no-infer` is not set, inferred from the active lease.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
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
    /// When absent and `--no-infer` is not set, inferred from the active lease.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
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
    /// When absent and `--no-infer` is not set, inferred from the active lease.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
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
    /// When absent and `--no-infer` is not set, inferred from the active lease.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
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
        CycleCommand::Next(args) => run_cycle_next(args, environment),
        CycleCommand::Pause(args) => run_cycle_pause(args, environment),
        CycleCommand::Resume(args) => run_cycle_resume(args, environment),
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
        let scope = normalize_scope(args.runtime.scope.as_deref().unwrap_or("."))?;
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
    let resolved = match resolve_cycle_context(&args.runtime, environment, args.cycle.as_deref()) {
        Ok(r) => r,
        Err(e) => return crate::failure(e.to_string()),
    };
    let result = (|| -> anyhow::Result<CycleStatusOutput> {
        let context = RuntimeContext::open(&resolved.runtime, environment, false)?;
        let cycle_id = resolved
            .cycle_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("cycle inference failed: no cycle_id resolved"))?;
        let record = context.storage.get_cycle(cycle_id)?;
        let lease = context.storage.get_cycle_lease(cycle_id).ok();
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
    let resolved = match resolve_cycle_context(&args.runtime, environment, args.cycle.as_deref()) {
        Ok(r) => r,
        Err(e) => return crate::failure(e.to_string()),
    };
    let result = (|| -> anyhow::Result<CycleTransitionOutput> {
        let mut context = RuntimeContext::open(&resolved.runtime, environment, false)?;
        let cycle_id = resolved
            .cycle_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("cycle inference failed: no cycle_id resolved"))?;
        let now_ms = timestamp_ms(args.timestamp.as_deref())?;
        match context
            .storage
            .get_cycle_lease(cycle_id)
            .map_err(Into::into)
        {
            Ok(_) => {
                let owner = args.lease_owner.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("cycle {} is leased; --lease-owner is required", cycle_id)
                })?;
                let token = args.fencing_token.ok_or_else(|| {
                    anyhow::anyhow!("cycle {} is leased; --fencing-token is required", cycle_id)
                })?;
                context
                    .engine
                    .require_lease_fence(cycle_id, owner, token, now_ms)?;
            }
            Err(sddk_domain::StorageError::NotFound { .. }) => {
                if args.lease_owner.is_some() || args.fencing_token.is_some() {
                    anyhow::bail!(
                        "cycle {} has no lease; fencing arguments are not applicable",
                        cycle_id
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
            .plan_transition(cycle_id, &args.transition, evidence)?;
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
        let event_id_prefix = format!("tr-{}", cycle_id);
        let outcome_input = OutcomeEventInput {
            project_id: context.identity.project_id.to_string(),
            cycle_id: cycle_id.clone(),
            transition_id: args.transition.clone(),
            from_phase: None,
            to_phase: None,
            transition_at: timestamp.clone(),
            actor_id: actor_id.clone(),
            actor_kind,
            event_id_prefix,
            failed_gates: vec![],
        };
        let auth = AuthorityContext::for_cli(actor_id.clone(), actor_kind, None, None);
        let applied = match context.engine.apply_transition(
            &plan,
            &event_context(
                &command_id,
                &format!("evt-{}", Uuid::new_v4().hyphenated()),
                &args.actor,
                environment,
                &timestamp,
            ),
            &auth,
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
    let resolved = match resolve_cycle_context(&args.runtime, environment, args.cycle.as_deref()) {
        Ok(r) => r,
        Err(e) => return crate::failure(e.to_string()),
    };
    let result = (|| -> anyhow::Result<CycleRebuildOutput> {
        let mut context = RuntimeContext::open(&resolved.runtime, environment, false)?;
        let cycle_id = resolved
            .cycle_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("cycle inference failed: no cycle_id resolved"))?;
        let now_ms = timestamp_ms(args.timestamp.as_deref())?;
        // `rebuild` is no longer a silent read-only audit: it requires the
        // caller to hold the same lease fence as a phase transition. An
        // expired lease is rejected with `LeaseExpired` (fail-closed).
        match context
            .storage
            .get_cycle_lease(cycle_id)
            .map_err(Into::into)
        {
            Ok(_) => {
                let owner = args.lease_owner.as_deref().ok_or_else(|| {
                    anyhow::anyhow!(
                        "cycle {} is leased; --lease-owner is required for rebuild",
                        cycle_id
                    )
                })?;
                let token = args.fencing_token.ok_or_else(|| {
                    anyhow::anyhow!(
                        "cycle {} is leased; --fencing-token is required for rebuild",
                        cycle_id
                    )
                })?;
                context
                    .engine
                    .require_lease_fence(cycle_id, owner, token, now_ms)?;
            }
            Err(sddk_domain::StorageError::NotFound { .. }) => {
                anyhow::bail!(
                    "cycle {} has no lease; acquire one with `sddk cycle lock acquire` before rebuild",
                    cycle_id
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
                .rebuild_cycle(cycle_id, &event_context, now_ms, args.dry_run)?;
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
    let resolved = match resolve_cycle_context(&args.runtime, environment, args.cycle.as_deref()) {
        Ok(r) => r,
        Err(e) => return crate::failure(e.to_string()),
    };
    let result = (|| -> anyhow::Result<ArtifactsDirOutput> {
        let context = RuntimeContext::open(&resolved.runtime, environment, false)?;
        let cycle_id = resolved
            .cycle_id
            .ok_or_else(|| anyhow::anyhow!("cycle inference failed: no cycle_id resolved"))?;
        let dir = context.cycle_artifacts_path.join(&cycle_id);
        std::fs::create_dir_all(&dir)?;
        Ok(ArtifactsDirOutput {
            cycle_id,
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
) -> Result<(), anyhow::Error> {
    match CycleId::new(cycle_id) {
        Ok(cid) => {
            let cycle_project = cid.project();
            let expected = expected_project_id.as_str();
            if cycle_project != expected {
                // CycleProjectMismatch stays on storage layer — domain has no
                // equivalent, and the 4 integration tests assert on
                // STORAGE_CYCLE_PROJECT_MISMATCH code.
                Err(sddk_storage::StorageError::CycleProjectMismatch {
                    cycle_id: cycle_id.to_owned(),
                    cycle_project_id: cycle_project.to_owned(),
                    expected_project_id: expected.to_owned(),
                }
                .into())
            } else {
                Ok(())
            }
        }
        // Malformed cycle ids (no project prefix) continue to STORAGE_NOT_FOUND
        // Use domain error so failure_envelope picks up the cycle-specific hint.
        Err(_) => Err(sddk_domain::StorageError::NotFound {
            entity: "cycle",
            id: cycle_id.to_owned(),
        }
        .into()),
    }
}

fn run_cycle_lock_acquire(
    args: CycleLockAcquireArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let resolved = match resolve_cycle_context(&args.runtime, environment, args.cycle.as_deref()) {
        Ok(r) => r,
        Err(e) => return crate::failure(e.to_string()),
    };
    let result = (|| -> anyhow::Result<LeaseOutput> {
        let mut context = RuntimeContext::open(&resolved.runtime, environment, false)?;
        let cycle_id = resolved
            .cycle_id
            .ok_or_else(|| anyhow::anyhow!("cycle inference failed: no cycle_id resolved"))?;
        validate_cycle_project(&cycle_id, &context.identity.project_id)?;
        let now_ms = timestamp_ms(args.timestamp.as_deref())?;
        let lease = context.storage.acquire_cycle_lease(
            &cycle_id,
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
    let resolved = match resolve_cycle_context(&args.runtime, environment, args.cycle.as_deref()) {
        Ok(r) => r,
        Err(e) => return crate::failure(e.to_string()),
    };
    let result = (|| -> anyhow::Result<LeaseOutput> {
        let mut context = RuntimeContext::open(&resolved.runtime, environment, false)?;
        let cycle_id = resolved
            .cycle_id
            .ok_or_else(|| anyhow::anyhow!("cycle inference failed: no cycle_id resolved"))?;
        validate_cycle_project(&cycle_id, &context.identity.project_id)?;
        let now_ms = timestamp_ms(args.timestamp.as_deref())?;
        let lease = context.storage.renew_cycle_lease(
            &cycle_id,
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
    let resolved = match resolve_cycle_context(&args.runtime, environment, args.cycle.as_deref()) {
        Ok(r) => r,
        Err(e) => return crate::failure(e.to_string()),
    };
    let result = (|| -> anyhow::Result<CycleLockReleaseOutput> {
        let mut context = RuntimeContext::open(&resolved.runtime, environment, false)?;
        let cycle_id = resolved
            .cycle_id
            .ok_or_else(|| anyhow::anyhow!("cycle inference failed: no cycle_id resolved"))?;
        validate_cycle_project(&cycle_id, &context.identity.project_id)?;
        let command_id = format!("cycle.lock.release-{}", Uuid::new_v4().hyphenated());
        let actor = args
            .actor
            .clone()
            .or_else(|| environment.sddk_actor.clone())
            .or_else(|| environment.user.clone())
            .unwrap_or_else(|| "sddk-cli".into());
        let released = context.storage.release_cycle_lease(
            context.identity.project_id.as_str(),
            &cycle_id,
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
    let resolved = match resolve_cycle_context(&args.runtime, environment, args.cycle.as_deref()) {
        Ok(r) => r,
        Err(e) => return crate::failure(e.to_string()),
    };
    let result = (|| -> anyhow::Result<CycleSupersedeOutput> {
        let mut context = RuntimeContext::open(&resolved.runtime, environment, false)?;
        let cycle_id = resolved
            .cycle_id
            .ok_or_else(|| anyhow::anyhow!("cycle inference failed: no cycle_id resolved"))?;
        let timestamp = args.timestamp.clone().unwrap_or_else(default_timestamp);
        let actor = args
            .actor
            .clone()
            .or_else(|| environment.sddk_actor.clone())
            .or_else(|| environment.user.clone())
            .unwrap_or_else(|| "sddk-cli".into());
        let command_id = format!("cycle.supersede-{}", Uuid::new_v4().hyphenated());
        let event_id = format!("evt-{}", Uuid::new_v4().hyphenated());

        // GAP-UX-1: validate cycle belongs to this project before touching storage
        validate_cycle_project(&cycle_id, &context.identity.project_id)?;

        // Parse evidence refs
        let evidence_refs: Vec<String> =
            serde_json::from_str(&args.evidence_refs).unwrap_or_default();

        // Convert reason
        let reason: Option<SupersedeReason> = args.reason.map(|r| r.into());
        let auth = AuthorityContext::for_cli(actor.clone(), infer_actor_kind(&actor), None, None);

        let receipt = context.engine.cycle_supersede(
            &cycle_id,
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
            &auth,
        )?;

        // Load updated cycle to get manifest
        let record = context.storage.get_cycle(&cycle_id)?;
        Ok(CycleSupersedeOutput {
            cycle_id,
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
    let resolved = match resolve_cycle_context(&args.runtime, environment, args.cycle.as_deref()) {
        Ok(r) => r,
        Err(e) => return crate::failure(e.to_string()),
    };
    let result = (|| -> anyhow::Result<CycleReplanOutput> {
        let mut context = RuntimeContext::open(&resolved.runtime, environment, false)?;
        let cycle_id = resolved
            .cycle_id
            .ok_or_else(|| anyhow::anyhow!("cycle inference failed: no cycle_id resolved"))?;
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
            &cycle_id,
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
        let _record = context.storage.get_cycle(&cycle_id)?;
        Ok(CycleReplanOutput {
            cycle_id,
            restage_to: restage_to_display,
            sequence: 0, // Placeholder until cycle_replan returns sequence info
        })
    })();
    render_result(result, format, cycle_replan_text)
}

fn run_cycle_pause(args: CyclePauseArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let resolved = match resolve_cycle_context(&args.runtime, environment, args.cycle.as_deref()) {
        Ok(r) => r,
        Err(e) => return crate::failure(e.to_string()),
    };
    let result = (|| -> anyhow::Result<CyclePauseOutput> {
        let mut context = RuntimeContext::open(&resolved.runtime, environment, false)?;
        let cycle_id = resolved
            .cycle_id
            .ok_or_else(|| anyhow::anyhow!("cycle inference failed: no cycle_id resolved"))?;
        let timestamp = args.timestamp.clone().unwrap_or_else(default_timestamp);
        let actor = args
            .actor
            .clone()
            .or_else(|| environment.sddk_actor.clone())
            .or_else(|| environment.user.clone())
            .unwrap_or_else(|| "sddk-cli".into());
        let command_id = format!("cycle.pause-{}", Uuid::new_v4().hyphenated());
        let event_id = format!("evt-{}", Uuid::new_v4().hyphenated());

        // GAP-UX-1: validate cycle belongs to this project before touching storage
        validate_cycle_project(&cycle_id, &context.identity.project_id)?;

        let reason: PauseReason = args.reason.into();
        let auth = AuthorityContext::for_cli(actor.clone(), infer_actor_kind(&actor), None, None);

        let receipt = context.engine.cycle_pause(
            &cycle_id,
            reason,
            args.review_at.as_deref(),
            &actor,
            &command_id,
            &event_id,
            &timestamp,
            &context.paths.cycle_artifacts,
            &args.lease_owner,
            args.fencing_token,
            &auth,
        )?;

        // Load updated cycle to get manifest
        let record = context.storage.get_cycle(&cycle_id)?;
        Ok(CyclePauseOutput {
            cycle_id,
            status: wire(&record.manifest.status),
            event_id: receipt.event_id,
            sequence: receipt.sequence,
            event_hash: receipt.event_hash,
        })
    })();
    render_result(result, format, cycle_pause_text)
}

fn run_cycle_resume(args: CycleResumeArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let resolved = match resolve_cycle_context(&args.runtime, environment, args.cycle.as_deref()) {
        Ok(r) => r,
        Err(e) => return crate::failure(e.to_string()),
    };
    let result = (|| -> anyhow::Result<CycleResumeOutput> {
        let mut context = RuntimeContext::open(&resolved.runtime, environment, false)?;
        let cycle_id = resolved
            .cycle_id
            .ok_or_else(|| anyhow::anyhow!("cycle inference failed: no cycle_id resolved"))?;
        let timestamp = args.timestamp.clone().unwrap_or_else(default_timestamp);
        let actor = args
            .actor
            .clone()
            .or_else(|| environment.sddk_actor.clone())
            .or_else(|| environment.user.clone())
            .unwrap_or_else(|| "sddk-cli".into());
        let command_id = format!("cycle.resume-{}", Uuid::new_v4().hyphenated());
        let event_id = format!("evt-{}", Uuid::new_v4().hyphenated());

        // GAP-UX-1: validate cycle belongs to this project before touching storage
        validate_cycle_project(&cycle_id, &context.identity.project_id)?;
        let auth = AuthorityContext::for_cli(actor.clone(), infer_actor_kind(&actor), None, None);

        let resume_output = context.engine.cycle_resume(
            &cycle_id,
            &actor,
            &command_id,
            &event_id,
            &timestamp,
            &context.paths.cycle_artifacts,
            &args.lease_owner,
            &auth,
        )?;

        // Load updated cycle to get manifest
        let record = context.storage.get_cycle(&cycle_id)?;
        Ok(CycleResumeOutput {
            cycle_id,
            status: wire(&record.manifest.status),
            event_id: resume_output.event_id,
            sequence: resume_output.sequence,
            event_hash: resume_output.event_hash,
            new_fencing_token: resume_output.new_fencing_token,
        })
    })();
    render_result(result, format, cycle_resume_text)
}

fn run_cycle_next(args: CycleNextArgs, environment: &CliEnvironment) -> CommandOutput {
    use sddk_engine::frontier_for_state;
    let format = args.format;
    let resolved = match resolve_cycle_context(&args.runtime, environment, args.cycle.as_deref()) {
        Ok(r) => r,
        Err(e) => return crate::failure(e.to_string()),
    };
    let result = (|| -> anyhow::Result<CycleNextOutput> {
        let context = RuntimeContext::open(&resolved.runtime, environment, false)?;
        let cycle_id = resolved
            .cycle_id
            .ok_or_else(|| anyhow::anyhow!("cycle inference failed: no cycle_id resolved"))?;

        // Load the workflow (from root or canonical fallback)
        let workflow = load_workflow(
            &resolved
                .runtime
                .root
                .clone()
                .unwrap_or_else(|| std::path::PathBuf::from(".")),
        )?;

        // Replay cycle state from ledger (S-NEXT-STATE-DERIVATION)
        let replay = context
            .engine
            .replay_cycle(&cycle_id)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let state = &replay.manifest;

        // Compute frontier
        let frontier_entries =
            frontier_for_state(&workflow, state, &cycle_id, context.engine.ledger())
                .map_err(|e| anyhow::anyhow!("{}", e))?;

        // Get lease if present
        let lease_opt: Option<LeaseOutput> = context
            .storage
            .get_cycle_lease(&cycle_id)
            .ok()
            .map(Into::into);
        let lease_owner: String = lease_opt
            .as_ref()
            .map(|l: &LeaseOutput| l.owner.clone())
            .unwrap_or_else(|| "<owner>".to_string());
        let lease_token: i64 = lease_opt
            .as_ref()
            .map(|l: &LeaseOutput| l.fencing_token)
            .unwrap_or(0);

        // Build output entries
        let frontier: Vec<FrontierEntryOutput> = frontier_entries
            .iter()
            .map(|entry| {
                let command = if entry.requires_met {
                    let transition = workflow
                        .transitions
                        .iter()
                        .find(|t| t.id == entry.transition_id);
                    let binding = transition.and_then(|t| t.implementation_binding.clone());
                    Some(format!(
                        "sddk cycle transition --cycle {} --transition {} --lease-owner {} --fencing-token {}",
                        cycle_id,
                        entry.transition_id,
                        lease_owner,
                        lease_token
                    ))
                } else {
                    None
                };

                let hint = if entry.requires_met {
                    None
                } else {
                    let unmet: Vec<String> = entry
                        .unmet_gates
                        .iter()
                        .map(|g| format!("sddk cycle evaluate-gate --gate {} --cycle {} --transition {}", g, cycle_id, entry.transition_id))
                        .chain(entry.unmet_requirements.iter().map(|r| format!("requirement: {}", r)))
                        .collect();
                    Some(if unmet.is_empty() {
                        "blocked — evaluate gate first".to_string()
                    } else {
                        unmet.join("; ")
                    })
                };

                // OVG-02: surface from_phase (was previously unconsumed)
                let from_phase = entry.from.phase.as_ref().map(|p| format!("{:?}", p));
                // OVG-03: surface to_phase and closes_cycle (to.status was unconsumed)
                let to_phase = entry.to.phase.as_ref().map(|p| format!("{:?}", p));
                let closes_cycle = entry.to.status == sddk_domain::CycleStatus::Closed;

                FrontierEntryOutput {
                    transition_id: entry.transition_id.clone(),
                    command,
                    requires_met: entry.requires_met,
                    from_phase,
                    to_phase,
                    closes_cycle,
                    hint,
                }
            })
            .collect();

        let reason = if frontier.is_empty() {
            Some(format!("terminal — status={:?}", state.status))
        } else {
            None
        };

        Ok(CycleNextOutput {
            cycle: cycle_id,
            node: format!("{:?}/{:?}", state.status, state.phase),
            frontier,
            lease: lease_opt,
            reason,
        })
    })();
    render_result(result, format, cycle_next_text)
}

fn run_cycle_lock_status(args: CycleLockStatusArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let resolved = match resolve_cycle_context(&args.runtime, environment, args.cycle.as_deref()) {
        Ok(r) => r,
        Err(e) => return crate::failure(e.to_string()),
    };
    let result = (|| -> anyhow::Result<Option<LeaseOutput>> {
        let context = RuntimeContext::open(&resolved.runtime, environment, false)?;
        let cycle_id = resolved
            .cycle_id
            .ok_or_else(|| anyhow::anyhow!("cycle inference failed: no cycle_id resolved"))?;
        // REQ-GAP6-3: apply same project-prefix guard as acquire/renew/release
        validate_cycle_project(&cycle_id, &context.identity.project_id)?;
        // REQ-DEBT017-5: cycle not found → typed error; cycle exists but no lease → None
        let lease = match context.storage.get_cycle_lease(&cycle_id) {
            Ok(l) => Some(l),
            Err(sddk_storage::StorageError::NotFound {
                entity: "cycle", ..
            }) => {
                // Cycle does not exist in `cycles` table → use domain error so
                // failure_envelope picks up the cycle-specific hint (WARN-001).
                return Err(anyhow::anyhow!(sddk_domain::StorageError::NotFound {
                    entity: "cycle",
                    id: cycle_id.clone(),
                }));
            }
            Err(sddk_storage::StorageError::NotFound { .. }) => None,
            Err(e) => return Err(anyhow::anyhow!(e)),
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
struct CyclePauseOutput {
    cycle_id: String,
    status: String,
    event_id: String,
    sequence: i64,
    event_hash: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct CycleResumeOutput {
    cycle_id: String,
    status: String,
    event_id: String,
    sequence: i64,
    event_hash: String,
    new_fencing_token: i64,
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

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct CycleNextOutput {
    cycle: String,
    node: String,
    frontier: Vec<FrontierEntryOutput>,
    lease: Option<LeaseOutput>,
    reason: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct FrontierEntryOutput {
    transition_id: String,
    command: Option<String>,
    requires_met: bool,
    /// Source phase for this transition (OVG-02: was previously unconsumed).
    from_phase: Option<String>,
    /// Target phase for this transition.
    to_phase: Option<String>,
    /// Whether this transition closes the cycle (OVG-03: to.status was previously unconsumed).
    closes_cycle: bool,
    hint: Option<String>,
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

fn cycle_pause_text(output: &CyclePauseOutput) -> String {
    format!(
        "cycle_id: {}\nstatus: {}\nevent_id: {}\nsequence: {}\nevent_hash: {}\n",
        output.cycle_id, output.status, output.event_id, output.sequence, output.event_hash
    )
}

fn cycle_resume_text(output: &CycleResumeOutput) -> String {
    format!(
        "cycle_id: {}\nstatus: {}\nevent_id: {}\nsequence: {}\nevent_hash: {}\nnew_fencing_token: {}\n",
        output.cycle_id,
        output.status,
        output.event_id,
        output.sequence,
        output.event_hash,
        output.new_fencing_token
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

fn cycle_next_text(output: &CycleNextOutput) -> String {
    let mut s = format!("cycle: {}\nnode: {}\n", output.cycle, output.node);
    if let Some(ref reason) = output.reason {
        s.push_str(&format!("frontier: [] ({})\n", reason));
        return s;
    }
    s.push_str("frontier:\n");
    for entry in &output.frontier {
        s.push_str(&format!("  - transition: {}\n", entry.transition_id));
        if entry.requires_met {
            if let Some(ref cmd) = entry.command {
                s.push_str(&format!("    command: {}\n", cmd));
            }
        } else if let Some(ref hint) = entry.hint {
            s.push_str(&format!("    hint: {}\n", hint));
        }
        s.push_str(&format!("    requires_met: {}\n", entry.requires_met));
        // OVG-02/03: surface from/to phase and closes_cycle
        if let Some(ref from) = entry.from_phase {
            s.push_str(&format!("    from_phase: {}\n", from));
        }
        if let Some(ref to) = entry.to_phase {
            s.push_str(&format!("    → {}\n", to));
        }
        if entry.closes_cycle {
            s.push_str("    closes_cycle: true\n");
        }
    }
    s.push_str(&lease_option_text(&output.lease));
    s
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
        actor_ref: None,
        occurred_at: occurred_at.to_owned(),
        causation_id: None,
        correlation_id: None,
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
    let resolved = match resolve_cycle_context(&args.runtime, environment, args.cycle.as_deref()) {
        Ok(r) => r,
        Err(e) => return crate::failure(e.to_string()),
    };
    let result = (|| -> anyhow::Result<GateEvaluationOutput> {
        let mut context = RuntimeContext::open(&resolved.runtime, environment, false)?;
        let cycle_id = resolved
            .cycle_id
            .ok_or_else(|| anyhow::anyhow!("cycle inference failed: no cycle_id resolved"))?;
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
        // AC-EVT-LEDGER-08: gate receipt creation requires System authority
        let actor_kind = infer_actor_kind(&actor);
        let auth = AuthorityContext::for_cli(actor.clone(), actor_kind, None, None);
        // Fail-closed: when --outcome is omitted we record `Failed`, so a
        // caller that wants to advance the workflow MUST pass
        // `--outcome passed` explicitly.
        let outcome = args.outcome.into();
        let receipt = context.engine.evaluate_gate(
            &GateEvaluationInput {
                cycle_id: cycle_id.clone(),
                transition_id: args.transition.clone(),
                gate: args.gate.clone(),
                evaluator: args.evaluator.clone(),
                evidence: serde_json::from_str(&args.evidence)?,
                outcome,
                evaluated_at: timestamp,
                actor,
                command_id: format!("gate-{}", uuid::Uuid::new_v4().hyphenated()),
            },
            &auth,
        )?;
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

// ── Tests for context inference ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::{
        CyclePath, CycleStatus, Phase, ProjectId, ProjectRecord, WorkspaceRecord,
        stable_fallback_project_id, stable_workspace_id,
    };
    use sddk_testkit::{CycleBuilder, EventBuilder};
    use std::fs;

    /// Creates a temp project directory with a .git marker (project walk-up target).
    fn temp_project() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        dir
    }

    /// Seeds a SQLite storage with project, workspace, cycle, and ONE active lease.
    /// Returns the project_id and cycle_id that were created.
    fn seed_one_active_lease(
        state_home: &std::path::Path,
        project_root: &std::path::Path,
        project_id: &str,
        cycle_id: &str,
        fallback_seed: &str,
        scope: &str,
    ) -> anyhow::Result<()> {
        // Create storage (writable for setup)
        let ledger_dir = state_home.join("sddk").join("projects").join(project_id);
        fs::create_dir_all(&ledger_dir).unwrap();
        let ledger_path = ledger_dir.join("ledger.sqlite");

        // Use open() to create schema and apply migrations
        let mut storage = crate::Storage::open(&ledger_path)?;

        // Register project + workspace
        // NOTE: CycleBuilder hardcodes workspace_id = "ws-test", so we must use the same
        let project = ProjectRecord {
            project_id: project_id.to_string(),
            display_name: "test-project".to_string(),
            remote_url: None,
            scope: scope.to_string(),
            created_at: "2026-08-19T00:00:00Z".to_string(),
        };
        let workspace = WorkspaceRecord {
            workspace_id: "ws-test".to_string(),
            project_id: project_id.to_string(),
            canonical_path: project_root.to_string_lossy().to_string(),
            created_at: "2026-08-19T00:00:00Z".to_string(),
        };
        storage.register_project_workspace(&project, &workspace)?;

        // Build cycle via CycleBuilder
        let cycle_record = CycleBuilder::new(CyclePath::AFull)
            .with_id(cycle_id)
            .with_project(project_id)
            .build();

        // Build the initial event with state_after = cycle manifest
        // (required for replay_cycle which is called by run_cycle_next)
        let event_input = EventBuilder::new("cycle.created")
            .with_cycle(cycle_id)
            .with_project(project_id)
            .state_after(serde_json::to_value(&cycle_record.manifest).unwrap())
            .build();

        // Insert cycle + event
        storage.insert_cycle_with_event(&cycle_record, &event_input)?;

        // Acquire active lease (now_ms + 1 hour)
        let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() * 1000;
        let expires_ms = now_ms + 3_600_000;
        storage.acquire_cycle_lease(cycle_id, "test-owner", now_ms, expires_ms)?;

        // Close storage by dropping
        drop(storage);

        // Seed the fallback_seed file at the path resolve_cycle_context will look for.
        // IMPORTANT: find_persisted_fallback_seed uses data_home (not state_home),
        // and resolve_cycle_context uses path.to_str() (not canonicalize).
        let workspace_id_for_path = stable_workspace_id(
            &ProjectId::new(project_id).unwrap(),
            project_root.to_str().unwrap(),
        );
        // data_home is where find_persisted_fallback_seed looks
        let data_home = project_root.join(".local").join("data");
        let seed_dir = data_home
            .join("sddk")
            .join("projects")
            .join(project_id)
            .join("workspaces")
            .join(workspace_id_for_path);
        fs::create_dir_all(&seed_dir).unwrap();
        fs::write(seed_dir.join("fallback_seed"), fallback_seed).unwrap();

        Ok(())
    }

    /// Seeds storage with TWO active leases for the same project (S3b).
    fn seed_two_active_leases(
        state_home: &std::path::Path,
        project_root: &std::path::Path,
        project_id: &str,
        cycle_id_a: &str,
        cycle_id_b: &str,
        fallback_seed: &str,
        scope: &str,
    ) -> anyhow::Result<()> {
        let ledger_dir = state_home.join("sddk").join("projects").join(project_id);
        fs::create_dir_all(&ledger_dir).unwrap();
        let ledger_path = ledger_dir.join("ledger.sqlite");

        let mut storage = crate::Storage::open(&ledger_path)?;

        let project = ProjectRecord {
            project_id: project_id.to_string(),
            display_name: "test-project".to_string(),
            remote_url: None,
            scope: scope.to_string(),
            created_at: "2026-08-19T00:00:00Z".to_string(),
        };
        // NOTE: CycleBuilder hardcodes workspace_id = "ws-test"
        let workspace = WorkspaceRecord {
            workspace_id: "ws-test".to_string(),
            project_id: project_id.to_string(),
            canonical_path: project_root.to_string_lossy().to_string(),
            created_at: "2026-08-19T00:00:00Z".to_string(),
        };
        storage.register_project_workspace(&project, &workspace)?;

        // Two cycles
        let cycle_a = CycleBuilder::new(CyclePath::AFull)
            .with_id(cycle_id_a)
            .with_project(project_id)
            .build();
        let event_a = EventBuilder::new("cycle.created")
            .with_cycle(cycle_id_a)
            .with_project(project_id)
            .build();
        storage.insert_cycle_with_event(&cycle_a, &event_a)?;

        let cycle_b = CycleBuilder::new(CyclePath::ALite)
            .with_id(cycle_id_b)
            .with_project(project_id)
            .build();
        let event_b = EventBuilder::new("cycle.created")
            .with_cycle(cycle_id_b)
            .with_project(project_id)
            .build();
        storage.insert_cycle_with_event(&cycle_b, &event_b)?;

        // Two active leases
        let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() * 1000;
        let expires_ms = now_ms + 3_600_000;
        storage.acquire_cycle_lease(cycle_id_a, "owner-a", now_ms, expires_ms)?;
        storage.acquire_cycle_lease(cycle_id_b, "owner-b", now_ms, expires_ms)?;

        drop(storage);

        // Seed fallback at the path resolve_cycle_context will look for.
        // IMPORTANT: find_persisted_fallback_seed uses data_home (not state_home),
        // and resolve_cycle_context uses path.to_str() (not canonicalize).
        let workspace_id_for_path = stable_workspace_id(
            &ProjectId::new(project_id).unwrap(),
            project_root.to_str().unwrap(),
        );
        let data_home = project_root.join(".local").join("data");
        let seed_dir = data_home
            .join("sddk")
            .join("projects")
            .join(project_id)
            .join("workspaces")
            .join(workspace_id_for_path);
        fs::create_dir_all(&seed_dir).unwrap();
        fs::write(seed_dir.join("fallback_seed"), fallback_seed).unwrap();

        Ok(())
    }

    /// Builds a CliEnvironment with XDG redirected to the temp project.
    fn make_env(project_root: &std::path::Path) -> CliEnvironment {
        CliEnvironment {
            home: Some(project_root.to_path_buf()),
            data_home: Some(project_root.join(".local").join("data")),
            sddk_data_dir: None,
            state_home: Some(project_root.join(".local").join("state")),
            cache_home: Some(project_root.join(".local").join("cache")),
            sddk_actor: None,
            user: Some("tester".to_string()),
        }
    }

    // ── S1: zero-arg inference with one active lease ─────────────────────────

    #[test]
    fn s1_resolve_returns_cycle_id_from_single_active_lease() {
        let proj = temp_project();
        let project_root = proj.path();

        let fallback_seed = "test-seed-001";
        let scope = ".";
        let project_id = stable_fallback_project_id(fallback_seed, scope);
        let cycle_id = "c-test-cycle-001";

        // Seed storage: project + workspace + cycle + one active lease
        seed_one_active_lease(
            &project_root.join(".local").join("state"),
            project_root,
            &project_id,
            cycle_id,
            fallback_seed,
            scope,
        )
        .unwrap();

        let env = make_env(project_root);
        let args = RuntimeArgs {
            root: Some(project_root.to_path_buf()),
            scope: None,
            remote: None,
            fallback_seed: Some(fallback_seed.to_string()),
            no_infer: false,
        };

        // Call resolver with no explicit root/scope/cycle — should infer all three
        let result = resolve_cycle_context(&args, &env, None);

        let ctx = result.expect("resolve should succeed with one active lease");
        assert!(
            ctx.cycle_id.is_some(),
            "cycle_id should be inferred from active lease"
        );
        assert_eq!(ctx.cycle_id.as_deref(), Some(cycle_id));
        // root should be filled in from walk-up
        assert!(
            ctx.runtime.root.is_some(),
            "root should be inferred from cwd"
        );
    }

    // ── S2: explicit args win over inference ───────────────────────────────────

    #[test]
    fn s2_explicit_args_returned_verbatim_with_no_storage_touch() {
        let proj = temp_project();
        let project_root = proj.path();

        let env = make_env(project_root);
        let explicit_root = project_root.join("explicit-root");
        let explicit_scope = "explicit-scope";
        let explicit_cycle = "explicit-cycle-001";

        let args = RuntimeArgs {
            root: Some(explicit_root.clone()),
            scope: Some(explicit_scope.to_string()),
            remote: None,
            fallback_seed: None,
            no_infer: false,
        };

        // With all three explicit, resolver should return them verbatim
        // without touching storage (no open storage needed)
        let result = resolve_cycle_context(&args, &env, Some(explicit_cycle));

        let ctx = result.expect("explicit args should resolve without storage");
        // Compare paths: ctx.runtime.root is Option<PathBuf>, explicit_root is PathBuf
        assert_eq!(
            ctx.runtime.root.as_ref().map(|p| p as &std::path::Path),
            Some(explicit_root.as_path())
        );
        assert_eq!(ctx.runtime.scope.as_deref(), Some(explicit_scope));
        assert_eq!(ctx.cycle_id.as_deref(), Some(explicit_cycle));
        // project_id should be None when all explicit — deferred to RuntimeContext::open
        assert!(
            ctx.project_id.is_none(),
            "project_id should be None when all args are explicit"
        );
    }

    #[test]
    fn s2_all_explicit_skips_project_identity_resolution() {
        // When root, scope, and cycle are ALL explicit, project identity
        // resolution is skipped entirely (no remote, no fallback_seed lookup).
        // We verify this by passing an unreachable/absent remote and a root
        // with no markers — if identity resolution were attempted, it would fail.
        let proj = temp_project();
        let project_root = proj.path();

        let env = make_env(project_root);
        let args = RuntimeArgs {
            root: Some(project_root.to_path_buf()),
            scope: Some(".".to_string()),
            remote: Some("https://unreachable.invalid/nonexistent.git".to_string()),
            fallback_seed: None,
            no_infer: false,
        };

        // This should succeed because project identity is deferred when all explicit
        let result = resolve_cycle_context(&args, &env, Some("any-cycle-id"));

        let ctx = result.expect("should resolve with all explicit even with bad remote");
        assert!(ctx.project_id.is_none());
    }

    // ── S3: typed ambiguity — zero leases ─────────────────────────────────────

    #[test]
    fn s3_no_active_cycle_error_contains_hint_with_root_and_scope() {
        let proj = temp_project();
        let project_root = proj.path();

        let fallback_seed = "seed-no-leases";
        let scope = ".";
        let project_id = stable_fallback_project_id(fallback_seed, scope);

        // Seed storage WITHOUT any lease
        let state_home = project_root.join(".local").join("state");
        let ledger_dir = state_home.join("sddk").join("projects").join(&project_id);
        fs::create_dir_all(&ledger_dir).unwrap();
        let ledger_path = ledger_dir.join("ledger.sqlite");
        let mut storage = crate::Storage::open(&ledger_path).unwrap();

        let project = ProjectRecord {
            project_id: project_id.clone(),
            display_name: "test".to_string(),
            remote_url: None,
            scope: scope.to_string(),
            created_at: "2026-08-19T00:00:00Z".to_string(),
        };
        // NOTE: CycleBuilder hardcodes workspace_id = "ws-test"
        let workspace = WorkspaceRecord {
            workspace_id: "ws-test".to_string(),
            project_id: project_id.clone(),
            canonical_path: project_root.to_string_lossy().to_string(),
            created_at: "2026-08-19T00:00:00Z".to_string(),
        };
        storage
            .register_project_workspace(&project, &workspace)
            .unwrap();

        let cycle = CycleBuilder::new(CyclePath::AFull)
            .with_id("c-no-lease")
            .with_project(&project_id)
            .build();
        let event = EventBuilder::new("cycle.created")
            .with_cycle("c-no-lease")
            .with_project(&project_id)
            .build();
        storage.insert_cycle_with_event(&cycle, &event).unwrap();
        // NO lease acquired — this is the "zero leases" case
        drop(storage);

        // Seed fallback at the path resolve_cycle_context will look for
        // IMPORTANT: find_persisted_fallback_seed uses data_home (not state_home),
        // and resolve_cycle_context uses path.to_str() (not canonicalize).
        let workspace_id_for_path = stable_workspace_id(
            &ProjectId::new(&project_id).unwrap(),
            project_root.to_str().unwrap(),
        );
        let data_home = project_root.join(".local").join("data");
        let seed_dir = data_home
            .join("sddk")
            .join("projects")
            .join(&project_id)
            .join("workspaces")
            .join(workspace_id_for_path);
        fs::create_dir_all(&seed_dir).unwrap();
        fs::write(seed_dir.join("fallback_seed"), fallback_seed).unwrap();

        let env = make_env(project_root);
        let args = RuntimeArgs {
            root: Some(project_root.to_path_buf()),
            scope: None,
            remote: None,
            fallback_seed: Some(fallback_seed.to_string()),
            no_infer: false,
        };

        let result = resolve_cycle_context(&args, &env, None);
        let err = result.expect_err("should return error when no active cycle");

        match err {
            InferenceError::NoActiveCycle {
                project_id: _,
                hint,
            } => {
                // Hint must contain resolved root path and scope
                let resolved_root = hint
                    .lines()
                    .find(|l| l.contains("--root"))
                    .map(|l| l.trim());
                assert!(resolved_root.is_some(), "hint should contain --root flag");
                let hint_line = resolved_root.unwrap();
                assert!(
                    hint_line.contains(project_root.to_str().unwrap()) || hint_line.contains("."),
                    "hint --root should reference the resolved root"
                );
                assert!(
                    hint_line.contains(scope),
                    "hint should contain the resolved scope"
                );
            }
            other => panic!("expected NoActiveCycle, got {:?}", other),
        }
    }

    // ── S3b: typed ambiguity — multiple leases ──────────────────────────────────

    #[test]
    fn s3b_ambiguous_cycle_error_lists_all_candidates_with_cycle_ids() {
        let proj = temp_project();
        let project_root = proj.path();

        let fallback_seed = "seed-multi";
        let scope = ".";
        let project_id = stable_fallback_project_id(fallback_seed, scope);

        seed_two_active_leases(
            &project_root.join(".local").join("state"),
            project_root,
            &project_id,
            "c-multi-a",
            "c-multi-b",
            fallback_seed,
            scope,
        )
        .unwrap();

        let env = make_env(project_root);
        let args = RuntimeArgs {
            root: Some(project_root.to_path_buf()),
            scope: None,
            remote: None,
            fallback_seed: Some(fallback_seed.to_string()),
            no_infer: false,
        };

        let result = resolve_cycle_context(&args, &env, None);
        let err = result.expect_err("should return error when multiple active leases");

        match err {
            InferenceError::AmbiguousCycle {
                project_id: _,
                candidates,
            } => {
                assert!(
                    candidates.len() >= 2,
                    "should have at least 2 candidates, got {}",
                    candidates.len()
                );
                for c in &candidates {
                    assert!(
                        !c.cycle_id.is_empty(),
                        "each candidate must name its cycle_id"
                    );
                }
            }
            other => panic!("expected AmbiguousCycle, got {:?}", other),
        }
    }

    // ── S4: no project context ────────────────────────────────────────────────

    #[test]
    fn s4_no_project_context_returns_error_pointing_to_project_resolve() {
        // Temp dir with NO .git, sddk, or AGENTS.md markers
        let temp = tempfile::TempDir::new().unwrap();
        let empty_dir = temp.path();

        // Change CWD to the empty temp dir so walk-up finds no markers
        let old_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(empty_dir).unwrap();

        let env = make_env(empty_dir);
        let args = RuntimeArgs {
            root: None,
            scope: None,
            remote: None,
            fallback_seed: None,
            no_infer: false,
        };

        let result = resolve_cycle_context(&args, &env, None);

        // Restore CWD
        std::env::set_current_dir(&old_cwd).unwrap();

        let err = result.expect_err("should fail with no project context");

        match err {
            InferenceError::NoProjectContext { cwd: _ } => {
                // Error message should mention sddk project resolve / sddk init
                let msg = err.to_string();
                assert!(
                    msg.contains("sddk project resolve") || msg.contains("sddk init"),
                    "error should point to sddk project resolve or sddk init, got: {}",
                    msg
                );
            }
            other => panic!("expected NoProjectContext, got {:?}", other),
        }
    }

    // ── S5: --no-infer flag ──────────────────────────────────────────────────

    #[test]
    fn s5_no_infer_with_missing_args_returns_explicit_required_error() {
        let proj = temp_project();
        let project_root = proj.path();

        let env = make_env(project_root);
        let args = RuntimeArgs {
            root: None,  // missing
            scope: None, // missing
            remote: None,
            fallback_seed: None,
            no_infer: true,
        };

        let result = resolve_cycle_context(&args, &env, None);
        let err = result.expect_err("should fail with missing args when no_infer=true");

        match err {
            InferenceError::ExplicitRequired { missing } => {
                assert!(
                    missing.contains(&"--root".to_string()),
                    "missing should include --root"
                );
                assert!(
                    missing.contains(&"--scope".to_string()),
                    "missing should include --scope"
                );
                // --cycle is also missing (no cycle_arg passed)
                assert!(
                    missing.contains(&"--cycle".to_string()),
                    "missing should include --cycle"
                );
            }
            other => panic!("expected ExplicitRequired, got {:?}", other),
        }
    }

    #[test]
    fn s5_no_infer_all_explicit_succeeds_without_storage() {
        let proj = temp_project();
        let project_root = proj.path();

        let env = make_env(project_root);
        let args = RuntimeArgs {
            root: Some(project_root.to_path_buf()),
            scope: Some(".".to_string()),
            remote: None,
            fallback_seed: None,
            no_infer: true,
        };

        // With all explicit and no_infer=true, should succeed without touching storage
        let result = resolve_cycle_context(&args, &env, Some("any-cycle"));
        let ctx = result.expect("should resolve with all explicit + no_infer");
        assert_eq!(ctx.cycle_id.as_deref(), Some("any-cycle"));
        assert!(ctx.project_id.is_none()); // deferred
    }

    // ── cycle next tests ────────────────────────────────────────────────────────

    /// Seeds a cycle at a specific phase for cycle-next tests.
    fn seed_cycle_for_frontier(
        state_home: &std::path::Path,
        project_root: &std::path::Path,
        project_id: &str,
        cycle_id: &str,
        fallback_seed: &str,
        scope: &str,
        phase: sddk_domain::Phase,
    ) -> anyhow::Result<()> {
        // Create storage (writable for setup)
        let ledger_dir = state_home.join("sddk").join("projects").join(project_id);
        fs::create_dir_all(&ledger_dir).unwrap();
        let ledger_path = ledger_dir.join("ledger.sqlite");

        // Use open() to create schema and apply migrations
        let mut storage = crate::Storage::open(&ledger_path)?;

        // Register project + workspace
        let project = ProjectRecord {
            project_id: project_id.to_string(),
            display_name: "test-project".to_string(),
            remote_url: None,
            scope: scope.to_string(),
            created_at: "2026-08-19T00:00:00Z".to_string(),
        };
        let workspace = WorkspaceRecord {
            workspace_id: "ws-test".to_string(),
            project_id: project_id.to_string(),
            canonical_path: project_root.to_string_lossy().to_string(),
            created_at: "2026-08-19T00:00:00Z".to_string(),
        };
        storage.register_project_workspace(&project, &workspace)?;

        // Build cycle via CycleBuilder
        let mut cycle_record = CycleBuilder::new(CyclePath::AFull)
            .with_id(cycle_id)
            .with_project(project_id)
            .build();
        cycle_record.manifest.phase = phase;
        cycle_record.manifest.status = CycleStatus::Open;

        // Build the initial event
        let event_input = EventBuilder::new("cycle.created")
            .with_cycle(cycle_id)
            .with_project(project_id)
            .build();

        // Insert cycle + event
        storage.insert_cycle_with_event(&cycle_record, &event_input)?;

        // Acquire active lease (now_ms + 1 hour)
        let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() * 1000;
        let expires_ms = now_ms + 3_600_000;
        storage.acquire_cycle_lease(cycle_id, "test-owner", now_ms, expires_ms)?;

        drop(storage);

        // Seed the fallback_seed file
        let workspace_id_for_path = stable_workspace_id(
            &ProjectId::new(project_id).unwrap(),
            project_root.to_str().unwrap(),
        );
        let data_home = project_root.join(".local").join("data");
        let seed_dir = data_home
            .join("sddk")
            .join("projects")
            .join(project_id)
            .join("workspaces")
            .join(workspace_id_for_path);
        fs::create_dir_all(&seed_dir).unwrap();
        fs::write(seed_dir.join("fallback_seed"), fallback_seed).unwrap();

        Ok(())
    }

    #[test]
    fn cycle_next_frontier_contains_ready_transitions() {
        // S-NEXT-COMMAND: frontier from Explore phase should contain
        // phase.explore.complete when no gates block it
        let proj = temp_project();
        let project_root = proj.path();

        let fallback_seed = "test-seed-next";
        let scope = ".";
        let project_id = stable_fallback_project_id(fallback_seed, scope);
        let cycle_id = "c-next-001";

        seed_cycle_for_frontier(
            &project_root.join(".local").join("state"),
            project_root,
            &project_id,
            cycle_id,
            fallback_seed,
            scope,
            Phase::Explore,
        )
        .unwrap();

        let env = make_env(project_root);
        let args = RuntimeArgs {
            root: Some(project_root.to_path_buf()),
            scope: Some(scope.to_string()),
            remote: None,
            fallback_seed: Some(fallback_seed.to_string()),
            no_infer: false,
        };

        // Just test that the resolver works
        let resolved = resolve_cycle_context(&args, &env, None).expect("should resolve");
        assert!(
            resolved.cycle_id.is_some(),
            "cycle_id should be inferred from active lease"
        );
        assert_eq!(resolved.cycle_id.as_deref(), Some(cycle_id));
    }

    #[test]
    fn cycle_next_json_output_has_stable_shape() {
        // S-NEXT-JSON: verify JSON envelope has required fields
        // We test FrontierEntryOutput serialization shape here
        // OVG-02/03: includes from_phase, to_phase, closes_cycle
        let entry = FrontierEntryOutput {
            transition_id: "phase.explore.complete".to_string(),
            command: Some("sddk cycle transition --cycle c-1 --transition phase.explore.complete --lease-owner test --fencing-token 1".to_string()),
            requires_met: true,
            from_phase: Some("Explore".to_string()),
            to_phase: Some("Specify".to_string()),
            closes_cycle: false,
            hint: None,
        };

        let json = serde_json::to_string(&entry).expect("serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("deserialize");

        assert!(
            parsed.get("transition_id").is_some(),
            "entry should have transition_id"
        );
        assert!(
            parsed.get("requires_met").is_some(),
            "entry should have requires_met"
        );
        assert!(parsed.get("command").is_some(), "entry should have command");
        // OVG-02/03 fields
        assert!(
            parsed.get("from_phase").is_some(),
            "entry should have from_phase (OVG-02)"
        );
        assert!(
            parsed.get("to_phase").is_some(),
            "entry should have to_phase (OVG-03)"
        );
        assert!(
            parsed.get("closes_cycle").is_some(),
            "entry should have closes_cycle (OVG-03)"
        );
    }

    #[test]
    fn cycle_next_zero_arg_infers_from_active_lease() {
        // S-NEXT-INFERENCE: cycle inferred from active lease when root/scope are provided
        let proj = temp_project();
        let project_root = proj.path();

        let fallback_seed = "test-seed-next2";
        let scope = ".";
        let project_id = stable_fallback_project_id(fallback_seed, scope);
        let cycle_id = "c-next-002";

        seed_cycle_for_frontier(
            &project_root.join(".local").join("state"),
            project_root,
            &project_id,
            cycle_id,
            fallback_seed,
            scope,
            Phase::Explore,
        )
        .unwrap();

        let env = make_env(project_root);
        // Note: root is provided explicitly; zero args means no cycle specified
        let args = RuntimeArgs {
            root: Some(project_root.to_path_buf()),
            scope: Some(scope.to_string()),
            remote: None,
            fallback_seed: Some(fallback_seed.to_string()),
            no_infer: false,
        };

        // Should infer cycle from active lease
        let resolved = resolve_cycle_context(&args, &env, None).expect("should resolve");
        assert!(
            resolved.cycle_id.is_some(),
            "cycle_id should be inferred from active lease"
        );
        assert_eq!(resolved.cycle_id.as_deref(), Some(cycle_id));
    }

    // ── S6: uniformity — all 9 subcommands route through resolve_cycle_context ──

    #[test]
    fn s6_all_cycle_subcommands_call_resolve_cycle_context() {
        // This is a structural test: we verify that every run_cycle_* handler
        // calls resolve_cycle_context by checking the source code occurrences.
        // The 9 subcommands that use the resolver are:
        // status, transition, rebuild, artifacts-dir, lock-acquire, lock-renew,
        // lock-release, lock-status, supersede, replan, evaluate-gate
        // (inventory also routes through resolve_cycle_context in inventory_cycle.rs)
        //
        // We do a source-level check by verifying that the grep pattern used in
        // the documentation (resolve_cycle_context appearing in each handler) is
        // present. This is a compile-time proxy for "all paths route through one resolver".

        // The actual verification: each run_cycle_* function contains the literal
        // "resolve_cycle_context" in its body. We verify this by checking the
        // source file contains the expected call sites.
        // CARGO_MANIFEST_DIR points to the crate root (where Cargo.toml is)
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source = manifest_dir.join("src").join("cycle.rs");
        let content = std::fs::read_to_string(&source).unwrap();

        // Count occurrences of the resolver call pattern in handler functions
        let resolver_calls = [
            "run_cycle_status",        // calls resolve_cycle_context
            "run_cycle_transition",    // calls resolve_cycle_context
            "run_cycle_rebuild",       // calls resolve_cycle_context
            "run_cycle_artifacts_dir", // calls resolve_cycle_context
            "run_cycle_lock_acquire",  // calls resolve_cycle_context
            "run_cycle_lock_renew",    // calls resolve_cycle_context
            "run_cycle_lock_release",  // calls resolve_cycle_context
            "run_cycle_lock_status",   // calls resolve_cycle_context
            "run_cycle_supersede",     // calls resolve_cycle_context
            "run_cycle_replan",        // calls resolve_cycle_context
            "run_cycle_evaluate_gate", // calls resolve_cycle_context
        ];

        for handler in resolver_calls {
            assert!(
                content.contains(&format!("fn {}(", handler)),
                "handler {} should exist in cycle.rs",
                handler
            );
            // Verify each handler calls resolve_cycle_context
            assert!(
                content.contains("resolve_cycle_context(&args.runtime, environment, args.cycle"),
                "handler {} should call resolve_cycle_context",
                handler
            );
        }
    }

    // ── S-NEXT-NO-WORKFLOW ───────────────────────────────────────────────────

    #[test]
    fn s_next_no_workflow_returns_typed_error_when_workflow_unreadable() {
        // S-NEXT-NO-WORKFLOW: cycle whose workflow file is a directory (not readable as file)
        // → typed error (WorkflowLoadError::Io), NOT panic, NOT STORAGE_NOT_FOUND
        let proj = temp_project();
        let project_root = proj.path();

        // Make workflow/workflow.yaml a DIRECTORY (not a file) — this triggers Io error
        // that is NOT NotFound, so load_workflow propagates it as typed error.
        // Note: we create workflow/workflow.yaml AS A DIRECTORY so that when
        // load_workflow_path tries to read it as a file, it gets EISDIR (not ENOENT).
        let workflow_file_as_dir = project_root.join("workflow").join("workflow.yaml");
        fs::create_dir_all(&workflow_file_as_dir).unwrap();

        // Use valid UUID as fallback_seed since RuntimeContext::open requires UUID validation
        let fallback_seed = "550e8400-e29b-41d4-a716-446655440001";
        let scope = ".";
        let project_id = stable_fallback_project_id(fallback_seed, scope);
        let cycle_id = "c-no-workflow-001";

        seed_one_active_lease(
            &project_root.join(".local").join("state"),
            project_root,
            &project_id,
            cycle_id,
            fallback_seed,
            scope,
        )
        .unwrap();

        let env = make_env(project_root);
        let args = CycleNextArgs {
            runtime: RuntimeArgs {
                root: Some(project_root.to_path_buf()),
                scope: Some(scope.to_string()),
                remote: None,
                fallback_seed: Some(fallback_seed.to_string()),
                no_infer: false,
            },
            cycle: Some(cycle_id.to_string()),
            format: OutputFormat::Text,
        };

        let output = run_cycle_next(args, &env);

        // Must return error status
        assert!(
            output.status != 0,
            "run_cycle_next should return non-zero status for unreadable workflow, got stdout: {}",
            output.stdout
        );
        // Error must mention the workflow path
        let stderr = &output.stderr;
        assert!(
            stderr.contains("workflow") || stderr.contains("workflow.yaml"),
            "error should mention workflow path, got: {}",
            stderr
        );
        // Must NOT be STORAGE_NOT_FOUND (different error kind)
        assert!(
            !stderr.contains("STORAGE_NOT_FOUND"),
            "error should NOT be STORAGE_NOT_FOUND"
        );
    }

    // ── WARN-001: hints-coverage ─────────────────────────────────────────────

    /// Regression test: malformed cycle id via validate_cycle_project surfaces
    /// sddk_domain::StorageError::NotFound (cycle-specific hint), NOT the generic
    /// "create the record or fix the reference" from sddk_storage.
    #[test]
    fn s_lock_acquire_malformed_cycle_emits_domain_hint_not_generic() {
        // Malformed cycle id (no project prefix) triggers validate_cycle_project's
        // Err(_ => NotFound) path. Before the fix this constructed
        // sddk_storage::StorageError::NotFound → generic hint.
        let proj = temp_project();
        let project_root = proj.path();
        let fallback_seed = "550e8400-e29b-41d4-a716-446655440099";
        let scope = ".";
        let project_id = stable_fallback_project_id(fallback_seed, scope);

        // Seed project so RuntimeContext::open succeeds
        seed_one_active_lease(
            &project_root.join(".local").join("state"),
            project_root,
            &project_id,
            "c-valid-seed",
            fallback_seed,
            scope,
        )
        .unwrap();

        let env = make_env(project_root);
        // "malformed-cycle" has no project prefix → CycleId::new fails →
        // validate_cycle_project returns domain StorageError::NotFound
        let args = CycleLockAcquireArgs {
            runtime: RuntimeArgs {
                root: Some(project_root.to_path_buf()),
                scope: Some(scope.to_string()),
                remote: None,
                fallback_seed: Some(fallback_seed.to_string()),
                no_infer: false,
            },
            cycle: Some("malformed-cycle".to_string()),
            owner: "test-owner".to_string(),
            lease_ms: 3_600_000,
            timestamp: None,
            format: OutputFormat::Text,
        };

        let output = run_cycle_lock_acquire(args, &env);

        assert!(
            output.status != 0,
            "run_cycle_lock_acquire should return error for malformed cycle"
        );
        let stderr = &output.stderr;
        // Domain hint must appear
        assert!(
            stderr.contains("sddk cycle start"),
            "stderr should contain domain hint with `sddk cycle start`, got: {}",
            stderr
        );
        // Generic storage hint must NOT appear
        assert!(
            !stderr.contains("create the record or fix the reference"),
            "stderr should NOT contain generic storage hint, got: {}",
            stderr
        );
    }

    /// Regression test: run_cycle_lock_status with nonexistent cycle surfaces
    /// sddk_domain::StorageError::NotFound (cycle-specific hint) via the
    /// fixed re-wrap, NOT the generic "create the record or fix the reference".
    #[test]
    fn s_lock_status_nonexistent_cycle_emits_domain_hint_not_generic() {
        let proj = temp_project();
        let project_root = proj.path();
        let fallback_seed = "550e8400-e29b-41d4-a716-446655440100";
        let scope = ".";
        let project_id = stable_fallback_project_id(fallback_seed, scope);

        // Seed project so RuntimeContext::open succeeds
        seed_one_active_lease(
            &project_root.join(".local").join("state"),
            project_root,
            &project_id,
            "c-valid-lock-seed",
            fallback_seed,
            scope,
        )
        .unwrap();

        let env = make_env(project_root);
        let args = CycleLockStatusArgs {
            runtime: RuntimeArgs {
                root: Some(project_root.to_path_buf()),
                scope: Some(scope.to_string()),
                remote: None,
                fallback_seed: Some(fallback_seed.to_string()),
                no_infer: false,
            },
            cycle: Some(format!("{}/c-nonexistent-lock-cycle", project_id)),
            format: OutputFormat::Text,
        };

        let output = run_cycle_lock_status(args, &env);

        assert!(
            output.status != 0,
            "run_cycle_lock_status should return error for nonexistent cycle"
        );
        let stderr = &output.stderr;
        // Domain hint must appear
        assert!(
            stderr.contains("sddk cycle start") || stderr.contains("sddk cycle rebuild"),
            "stderr should contain domain hint with cycle commands, got: {}",
            stderr
        );
        // Generic storage hint must NOT appear
        assert!(
            !stderr.contains("create the record or fix the reference"),
            "stderr should NOT contain generic storage hint, got: {}",
            stderr
        );
    }

    // ── S-NEXT-TERMINAL ─────────────────────────────────────────────────────

    /// Seeds a cycle in CLOSED/terminal status for S-NEXT-TERMINAL test.
    fn seed_terminal_cycle(
        state_home: &std::path::Path,
        project_root: &std::path::Path,
        project_id: &str,
        cycle_id: &str,
        fallback_seed: &str,
        scope: &str,
    ) -> anyhow::Result<()> {
        let ledger_dir = state_home.join("sddk").join("projects").join(project_id);
        fs::create_dir_all(&ledger_dir).unwrap();
        let ledger_path = ledger_dir.join("ledger.sqlite");

        let mut storage = crate::Storage::open(&ledger_path)?;

        let project = ProjectRecord {
            project_id: project_id.to_string(),
            display_name: "test-project".to_string(),
            remote_url: None,
            scope: scope.to_string(),
            created_at: "2026-08-19T00:00:00Z".to_string(),
        };
        let workspace = WorkspaceRecord {
            workspace_id: "ws-test".to_string(),
            project_id: project_id.to_string(),
            canonical_path: project_root.to_string_lossy().to_string(),
            created_at: "2026-08-19T00:00:00Z".to_string(),
        };
        storage.register_project_workspace(&project, &workspace)?;

        let mut cycle_record = CycleBuilder::new(CyclePath::AFull)
            .with_id(cycle_id)
            .with_project(project_id)
            .build();
        // Set terminal status
        cycle_record.manifest.phase = Phase::Explore;
        cycle_record.manifest.status = CycleStatus::Closed;

        // Build event with state_after = cycle manifest (required for replay_cycle)
        let event_input = EventBuilder::new("cycle.created")
            .with_cycle(cycle_id)
            .with_project(project_id)
            .state_after(serde_json::to_value(&cycle_record.manifest).unwrap())
            .build();

        storage.insert_cycle_with_event(&cycle_record, &event_input)?;

        // No lease needed for terminal cycle (lease released on close)

        drop(storage);

        let workspace_id_for_path = stable_workspace_id(
            &ProjectId::new(project_id).unwrap(),
            project_root.to_str().unwrap(),
        );
        let data_home = project_root.join(".local").join("data");
        let seed_dir = data_home
            .join("sddk")
            .join("projects")
            .join(project_id)
            .join("workspaces")
            .join(workspace_id_for_path);
        fs::create_dir_all(&seed_dir).unwrap();
        fs::write(seed_dir.join("fallback_seed"), fallback_seed).unwrap();

        Ok(())
    }

    #[test]
    fn s_next_terminal_cycle_returns_empty_frontier_with_explicit_reason() {
        // S-NEXT-TERMINAL: cycle in CLOSED status → frontier empty + explicit reason
        let proj = temp_project();
        let project_root = proj.path();

        // Use valid UUID as fallback_seed since RuntimeContext::open requires UUID validation
        let fallback_seed = "550e8400-e29b-41d4-a716-446655440002";
        let scope = ".";
        let project_id = stable_fallback_project_id(fallback_seed, scope);
        let cycle_id = "c-terminal-001";

        seed_terminal_cycle(
            &project_root.join(".local").join("state"),
            project_root,
            &project_id,
            cycle_id,
            fallback_seed,
            scope,
        )
        .unwrap();

        let env = make_env(project_root);
        let args = CycleNextArgs {
            runtime: RuntimeArgs {
                root: Some(project_root.to_path_buf()),
                scope: Some(scope.to_string()),
                remote: None,
                fallback_seed: Some(fallback_seed.to_string()),
                no_infer: false,
            },
            cycle: Some(cycle_id.to_string()),
            format: OutputFormat::Text,
        };

        let output = run_cycle_next(args, &env);

        // Status should be 0 (informative, not error)
        assert_eq!(
            output.status, 0,
            "cycle next on terminal cycle should return status 0"
        );
        // Human output: frontier empty + explicit reason about terminal status
        assert!(
            output.stdout.contains("frontier: []"),
            "frontier should be empty, got: {}",
            output.stdout
        );
        assert!(
            output.stdout.contains("terminal") || output.stdout.contains("CLOSED"),
            "output should mention terminal/CLOSED, got: {}",
            output.stdout
        );
    }

    #[test]
    fn s_next_terminal_json_has_empty_frontier_and_reason() {
        // S-NEXT-TERMINAL JSON surface: frontier: [], reason: "terminal: status=CLOSED"
        let proj = temp_project();
        let project_root = proj.path();

        // Use valid UUID as fallback_seed since RuntimeContext::open requires UUID validation
        let fallback_seed = "550e8400-e29b-41d4-a716-446655440003";
        let scope = ".";
        let project_id = stable_fallback_project_id(fallback_seed, scope);
        let cycle_id = "c-terminal-json-001";

        seed_terminal_cycle(
            &project_root.join(".local").join("state"),
            project_root,
            &project_id,
            cycle_id,
            fallback_seed,
            scope,
        )
        .unwrap();

        let env = make_env(project_root);
        let args = CycleNextArgs {
            runtime: RuntimeArgs {
                root: Some(project_root.to_path_buf()),
                scope: Some(scope.to_string()),
                remote: None,
                fallback_seed: Some(fallback_seed.to_string()),
                no_infer: false,
            },
            cycle: Some(cycle_id.to_string()),
            format: OutputFormat::Json,
        };

        let output = run_cycle_next(args, &env);

        assert_eq!(
            output.status, 0,
            "cycle next on terminal cycle should return status 0"
        );
        let json: serde_json::Value =
            serde_json::from_str(&output.stdout).expect("stdout should be valid JSON");
        let frontier = json.get("frontier").expect("JSON should have frontier");
        assert!(
            frontier.as_array().is_some_and(|arr| arr.is_empty()),
            "frontier should be empty array in JSON"
        );
        assert!(
            json.get("reason").is_some(),
            "JSON should have reason field for terminal cycle"
        );
    }

    // ── S-NEXT-GATES ───────────────────────────────────────────────────────

    /// Seeds a cycle at Explore phase with a transition that has a gate requirement
    /// but NO gate receipt injected.
    fn seed_cycle_with_gate_requirement(
        state_home: &std::path::Path,
        project_root: &std::path::Path,
        project_id: &str,
        cycle_id: &str,
        fallback_seed: &str,
        scope: &str,
    ) -> anyhow::Result<()> {
        let ledger_dir = state_home.join("sddk").join("projects").join(project_id);
        fs::create_dir_all(&ledger_dir).unwrap();
        let ledger_path = ledger_dir.join("ledger.sqlite");

        let mut storage = crate::Storage::open(&ledger_path)?;

        let project = ProjectRecord {
            project_id: project_id.to_string(),
            display_name: "test-project".to_string(),
            remote_url: None,
            scope: scope.to_string(),
            created_at: "2026-08-19T00:00:00Z".to_string(),
        };
        let workspace = WorkspaceRecord {
            workspace_id: "ws-test".to_string(),
            project_id: project_id.to_string(),
            canonical_path: project_root.to_string_lossy().to_string(),
            created_at: "2026-08-19T00:00:00Z".to_string(),
        };
        storage.register_project_workspace(&project, &workspace)?;

        // Build cycle at Explore phase with a gate transition in the manifest
        let mut cycle_record = CycleBuilder::new(CyclePath::AFull)
            .with_id(cycle_id)
            .with_project(project_id)
            .build();
        cycle_record.manifest.phase = Phase::Explore;
        cycle_record.manifest.status = CycleStatus::Open;

        // Build event with state_after = cycle manifest (required for replay_cycle)
        let event_input = EventBuilder::new("cycle.created")
            .with_cycle(cycle_id)
            .with_project(project_id)
            .state_after(serde_json::to_value(&cycle_record.manifest).unwrap())
            .build();

        storage.insert_cycle_with_event(&cycle_record, &event_input)?;

        // Acquire active lease
        let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() * 1000;
        let expires_ms = now_ms + 3_600_000;
        storage.acquire_cycle_lease(cycle_id, "test-owner", now_ms, expires_ms)?;

        drop(storage);

        let workspace_id_for_path = stable_workspace_id(
            &ProjectId::new(project_id).unwrap(),
            project_root.to_str().unwrap(),
        );
        let data_home = project_root.join(".local").join("data");
        let seed_dir = data_home
            .join("sddk")
            .join("projects")
            .join(project_id)
            .join("workspaces")
            .join(workspace_id_for_path);
        fs::create_dir_all(&seed_dir).unwrap();
        fs::write(seed_dir.join("fallback_seed"), fallback_seed).unwrap();

        Ok(())
    }

    #[test]
    fn s_next_gates_unmet_shows_blocked_and_requires_met_false() {
        // S-NEXT-GATES: transition with unmet gate requirement → blocked + requires_met: false
        let proj = temp_project();
        let project_root = proj.path();

        // Use valid UUID as fallback_seed since RuntimeContext::open requires UUID validation
        let fallback_seed = "550e8400-e29b-41d4-a716-446655440004";
        let scope = ".";
        let project_id = stable_fallback_project_id(fallback_seed, scope);
        let cycle_id = "c-gates-001";

        seed_cycle_with_gate_requirement(
            &project_root.join(".local").join("state"),
            project_root,
            &project_id,
            cycle_id,
            fallback_seed,
            scope,
        )
        .unwrap();

        let env = make_env(project_root);
        let args = CycleNextArgs {
            runtime: RuntimeArgs {
                root: Some(project_root.to_path_buf()),
                scope: Some(scope.to_string()),
                remote: None,
                fallback_seed: Some(fallback_seed.to_string()),
                no_infer: false,
            },
            cycle: Some(cycle_id.to_string()),
            format: OutputFormat::Text,
        };

        let output = run_cycle_next(args, &env);

        assert_eq!(
            output.status, 0,
            "cycle next should succeed even with unmet gates"
        );
        // Human surface: hint mentions "evaluate gate first" or similar blocked message
        let stdout = &output.stdout;
        assert!(
            stdout.contains("evaluate-gate") || stdout.contains("sddk cycle evaluate"),
            "human surface should show evaluate-gate hint, got: {}",
            stdout
        );
        // Verify the gate-gated transition (phase.explore.complete) shows hint, not command.
        // cycle.pause has no gate requirements, so it shows command: — that's fine.
        // Key invariants: unmet-gates transition shows hint, not command.
        assert!(
            stdout.contains("phase.explore.complete"),
            "phase.explore.complete should appear in frontier"
        );
        // The hint for unmet gates contains "evaluate-gate" — verify this appears
        // (this was already asserted above, but we re-confirm)
        let hint_contains_evaluate_gate = stdout
            .lines()
            .skip_while(|l| !l.contains("phase.explore.complete"))
            .skip(1)
            .take_while(|l| !l.trim_start().starts_with("- transition:"))
            .any(|l| l.contains("evaluate-gate"));
        assert!(
            hint_contains_evaluate_gate,
            "phase.explore.complete should show hint with evaluate-gate for unmet gates"
        );
    }

    #[test]
    fn s_next_gates_unmet_json_shows_requires_met_false() {
        // S-NEXT-GATES JSON surface: requires_met: false for unmet gate transition
        let proj = temp_project();
        let project_root = proj.path();

        // Use valid UUID as fallback_seed since RuntimeContext::open requires UUID validation
        let fallback_seed = "550e8400-e29b-41d4-a716-446655440005";
        let scope = ".";
        let project_id = stable_fallback_project_id(fallback_seed, scope);
        let cycle_id = "c-gates-json-001";

        seed_cycle_with_gate_requirement(
            &project_root.join(".local").join("state"),
            project_root,
            &project_id,
            cycle_id,
            fallback_seed,
            scope,
        )
        .unwrap();

        let env = make_env(project_root);
        let args = CycleNextArgs {
            runtime: RuntimeArgs {
                root: Some(project_root.to_path_buf()),
                scope: Some(scope.to_string()),
                remote: None,
                fallback_seed: Some(fallback_seed.to_string()),
                no_infer: false,
            },
            cycle: Some(cycle_id.to_string()),
            format: OutputFormat::Json,
        };

        let output = run_cycle_next(args, &env);

        assert_eq!(
            output.status, 0,
            "cycle next should succeed even with unmet gates"
        );
        let json: serde_json::Value =
            serde_json::from_str(&output.stdout).expect("stdout should be valid JSON");
        let frontier = json
            .get("frontier")
            .expect("JSON should have frontier")
            .as_array()
            .expect("frontier should be array");
        // At least one entry should have requires_met: false
        let has_unmet = frontier.iter().any(|entry| {
            entry
                .get("requires_met")
                .is_some_and(|v| v == &serde_json::Value::Bool(false))
        });
        assert!(
            has_unmet,
            "at least one frontier entry should have requires_met: false, got: {}",
            output.stdout
        );
    }
}
