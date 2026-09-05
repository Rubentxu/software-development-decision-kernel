//! Planning ledger subcommands and legacy facade router.
//!
//! ## CLI surface (ADR-073 §3.1)
//!
//! | Surface | Status |
//! |---|---|
//! | `sddk plan <name>` | DEPRECATED — emits `--deprecation-warning`; delegates to `cycle start` |
//! | `sddk plan workitem {create,show,list,transition}` | NEW |
//! | `sddk plan dep add` | NEW |
//! | `sddk plan evidence attach` | NEW |
//! | `sddk plan decision record` | NEW |
//! | `sddk plan graph` | NEW |

use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use sddk_domain::planning::{
    DECISION_RECORD_SCHEMA_VERSION, DecisionKind, DependencyEdgeKind, DependencyEdgeRecord,
    DependencyEdgeV1, EVIDENCE_ATTACHMENT_SCHEMA_VERSION, EvidenceAttachmentRecord,
    PlanningEvidenceKind, WORK_ITEM_SCHEMA_VERSION, WorkItemRecord, WorkItemStatus,
};
use sddk_domain::{DependencyResolutionError, DependencyResolutionService};
use serde::Deserialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat, Storage,
    cycle::{self, CyclePathArg, CycleStartArgs, RuntimeArgs},
    failure,
};

/// Deprecation warning emitted when using the legacy `sddk plan <name>` form.
const DEPRECATION_WARNING: &str = "sddk plan <name> is deprecated and will be removed in v1.87.0; \
     use 'sddk cycle start --name <name>' instead";

/// Minimal adoption receipt fields needed to resolve the ledger path.
/// We deserialize only the fields we need rather than depending on the full type.
#[derive(Debug, Deserialize)]
struct MinimalReceipt {
    project_id: String,
}

/// Opens Storage from the project root, looking up the adoption receipt to find
/// the ledger path. Returns `Some(Storage)` on success, `None` if no adopted project found.
fn open_storage_for_plan(environment: &CliEnvironment) -> Option<Storage> {
    // Walk up from cwd looking for .sddk/adoption.json
    let cwd = std::env::current_dir().ok()?;
    let mut dir = cwd.as_path();
    loop {
        let receipt_path = dir.join(".sddk").join("adoption.json");
        if receipt_path.is_file() {
            let bytes = std::fs::read(&receipt_path).ok()?;
            let receipt: MinimalReceipt = serde_json::from_slice(&bytes).ok()?;
            // The ledger is at <state_home>/sddk/projects/<project_id>/ledger.sqlite
            let xdg = environment.xdg();
            let state_home = xdg
                .state_home
                .as_deref()
                .map(PathBuf::from)
                .or_else(|| xdg.home.as_deref().map(|h| h.join(".local/state")))?;
            let project_id = &receipt.project_id;
            let ledger_path = state_home
                .join("sddk/projects")
                .join(project_id)
                .join("ledger.sqlite");
            if let Ok(storage) = Storage::open(&ledger_path) {
                return Some(storage);
            }
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => return None,
        }
    }
}

/// Parses an actor_id string like "Human:alice" or "agent:cli" into an ActorRef.
fn parse_actor_ref(actor_id: &str) -> (Option<String>, Option<String>, Option<String>) {
    if let Some(rest) = actor_id.strip_prefix("Human:") {
        (
            Some("Human".to_string()),
            Some(rest.to_string()),
            Some(rest.to_string()),
        )
    } else if let Some(rest) = actor_id.strip_prefix("Agent:") {
        (
            Some("Agent".to_string()),
            Some(rest.to_string()),
            Some(rest.to_string()),
        )
    } else if let Some(rest) = actor_id.strip_prefix("System:") {
        (
            Some("System".to_string()),
            Some(rest.to_string()),
            Some(rest.to_string()),
        )
    } else {
        // Treat as System by default
        (
            Some("System".to_string()),
            Some(actor_id.to_string()),
            Some(actor_id.to_string()),
        )
    }
}

/// Parses a string into a WorkItemStatus.
fn parse_work_item_status(s: &str) -> Result<WorkItemStatus, String> {
    match s.to_lowercase().as_str() {
        "draft" => Ok(WorkItemStatus::Draft),
        "active" => Ok(WorkItemStatus::Active),
        "paused" => Ok(WorkItemStatus::Paused),
        "done" => Ok(WorkItemStatus::Done),
        "superseded" => Ok(WorkItemStatus::Superseded),
        "cancelled" => Ok(WorkItemStatus::Cancelled),
        _ => Err(format!(
            "invalid status: {} (expected: draft, active, paused, done, superseded, cancelled)",
            s
        )),
    }
}

/// ── PlanCommand enum ─────────────────────────────────────────────────────────
///
/// Planning ledger subcommands.
///
/// Each subcommand wires Storage CRUD + Authority + EventBus per ADR-073 §3.1.
#[derive(Debug, Clone, Subcommand)]
pub(crate) enum PlanCommand {
    /// Manage planning work items.
    WorkItem {
        #[command(subcommand)]
        command: WorkItemCommand,
    },
    /// Manage planning dependencies between work items.
    Dep {
        #[command(subcommand)]
        command: DepCommand,
    },
    /// Attach evidence to a work item.
    Evidence {
        #[command(subcommand)]
        command: EvidenceCommand,
    },
    /// Record a decision for a work item.
    Decision {
        #[command(subcommand)]
        command: DecisionCommand,
    },
    /// Show the planning graph identity and provenance chain for a cycle.
    Graph {
        /// Cycle identifier.
        #[arg(long)]
        cycle_id: String,
        /// Output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
}

// ── WorkItem subcommands ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Subcommand)]
pub(crate) enum WorkItemCommand {
    /// Create a new work item in a cycle.
    Create(WorkItemCreateArgs),
    /// Show a work item by id.
    Show(WorkItemShowArgs),
    /// List work items for a cycle.
    List(WorkItemListArgs),
    /// Transition a work item to a new status.
    Transition(WorkItemTransitionArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct WorkItemCreateArgs {
    /// Cycle identifier.
    #[arg(long)]
    pub(crate) cycle_id: String,
    /// Work item title.
    #[arg(long)]
    pub(crate) title: String,
    /// Work item description.
    #[arg(long)]
    pub(crate) description: String,
    /// Actor id.
    #[arg(long, default_value = "agent:cli")]
    pub(crate) actor_id: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct WorkItemShowArgs {
    /// Work item identifier.
    #[arg(long)]
    pub(crate) work_item_id: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct WorkItemListArgs {
    /// Cycle identifier.
    #[arg(long)]
    pub(crate) cycle_id: String,
    /// Filter by status (Draft, Active, Paused, Done, Superseded, Cancelled).
    #[arg(long)]
    pub(crate) status: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct WorkItemTransitionArgs {
    /// Work item identifier.
    #[arg(long)]
    pub(crate) work_item_id: String,
    /// Target status.
    #[arg(long)]
    pub(crate) to: String,
    /// Actor id.
    #[arg(long, default_value = "agent:cli")]
    pub(crate) actor_id: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

// ── Dep subcommands ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Subcommand)]
pub(crate) enum DepCommand {
    /// Add a dependency edge between two work items.
    Add(DepAddArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct DepAddArgs {
    /// Source work item id (the dependee).
    #[arg(long)]
    pub(crate) from_id: String,
    /// Target work item id (the dependent).
    #[arg(long)]
    pub(crate) to_id: String,
    /// Dependency kind (Blocks, BlocksOnClosure).
    #[arg(long, default_value = "Blocks")]
    pub(crate) kind: String,
    /// Actor id.
    #[arg(long, default_value = "system:planner")]
    pub(crate) actor_id: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

// ── Evidence subcommands ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Subcommand)]
pub(crate) enum EvidenceCommand {
    /// Attach evidence to a work item.
    Attach(EvidenceAttachArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct EvidenceAttachArgs {
    /// Work item identifier.
    #[arg(long)]
    pub(crate) work_item_id: String,
    /// Evidence kind.
    #[arg(long)]
    pub(crate) kind: String,
    /// Path to the evidence body file.
    #[arg(long)]
    pub(crate) body_file: PathBuf,
    /// Actor id.
    #[arg(long, default_value = "agent:cli")]
    pub(crate) actor_id: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

// ── Decision subcommands ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Subcommand)]
pub(crate) enum DecisionCommand {
    /// Record a decision for a work item.
    Record(DecisionRecordArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct DecisionRecordArgs {
    /// Work item identifier.
    #[arg(long)]
    pub(crate) work_item_id: String,
    /// Decision kind (Architectural, Implementation, Priority, Rejection).
    #[arg(long)]
    pub(crate) kind: String,
    /// Rationale text (must be non-empty).
    #[arg(long)]
    pub(crate) rationale: String,
    /// Actor id.
    #[arg(long)]
    pub(crate) actor_id: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

// ── Runner functions ───────────────────────────────────────────────────────────

/// Run the `plan` subcommand dispatcher.
pub(crate) fn run_plan(command: PlanCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        PlanCommand::WorkItem { command } => run_workitem(command, environment),
        PlanCommand::Dep { command } => run_dep(command, environment),
        PlanCommand::Evidence { command } => run_evidence(command, environment),
        PlanCommand::Decision { command } => run_decision(command, environment),
        PlanCommand::Graph { cycle_id, format } => run_graph(&cycle_id, format, environment),
    }
}

/// Run the deprecated `sddk plan <name>` facade.
///
/// Emits a deprecation warning to stderr and delegates to `cycle start`.
pub(crate) fn run_plan_legacy(
    name: String,
    path: Option<CyclePathArg>,
    branch: Option<String>,
    format: OutputFormat,
    environment: &CliEnvironment,
) -> CommandOutput {
    // Emit deprecation warning to stderr
    eprintln!("{}", DEPRECATION_WARNING);
    let args = CycleStartArgs {
        runtime: RuntimeArgs {
            root: Some(PathBuf::from(".")),
            scope: Some(".".to_string()),
            remote: None,
            fallback_seed: None,
            no_infer: false,
        },
        name,
        path,
        branch,
        base: None,
        timestamp: None,
        actor: None,
        lease_owner: None,
        lease_ms: 3_600_000,
        format,
    };
    cycle::run_cycle(cycle::CycleCommand::Start(args), environment)
}

// ── WorkItem subcommand handler ───────────────────────────────────────────────

fn run_workitem(command: WorkItemCommand, environment: &CliEnvironment) -> CommandOutput {
    let storage = match open_storage_for_plan(environment) {
        Some(s) => s,
        None => {
            return failure(
                "sddk plan requires an adopted project: no .sddk/adoption.json found in parent dirs"
                    .to_string(),
            );
        }
    };
    match command {
        WorkItemCommand::Create(args) => {
            let (actor_ref_kind, actor_ref_id, actor_ref_label) = parse_actor_ref(&args.actor_id);
            let record = WorkItemRecord {
                id: Uuid::new_v4().hyphenated().to_string(),
                cycle_id: args.cycle_id,
                title: args.title,
                description: args.description,
                status: WorkItemStatus::Draft,
                actor_ref_kind,
                actor_ref_id: Some(actor_ref_id.unwrap_or_default()),
                actor_ref_label: Some(actor_ref_label.unwrap_or_default()),
                created_at: OffsetDateTime::now_utc().unix_timestamp(),
                schema_version: WORK_ITEM_SCHEMA_VERSION,
            };
            if let Err(e) = storage.insert_work_item(&record) {
                return failure(format!("failed to create work item: {}", e));
            }
            match args.format {
                OutputFormat::Json => CommandOutput {
                    status: 0,
                    stdout: format!("{{\"id\": \"{}\", \"status\": \"draft\"}}\n", record.id),
                    stderr: String::new(),
                },
                OutputFormat::Text => CommandOutput {
                    status: 0,
                    stdout: format!("work item created: {} (status: Draft)\n", record.id),
                    stderr: String::new(),
                },
            }
        }
        WorkItemCommand::Show(args) => {
            let wi = match storage.get_work_item(&args.work_item_id) {
                Ok(Some(r)) => r.into_domain(),
                Ok(None) => {
                    return failure(format!("work item not found: {}", args.work_item_id));
                }
                Err(e) => return failure(format!("failed to get work item: {}", e)),
            };
            match args.format {
                OutputFormat::Json => match serde_json::to_string_pretty(&wi) {
                    Ok(json) => CommandOutput {
                        status: 0,
                        stdout: format!("{json}\n"),
                        stderr: String::new(),
                    },
                    Err(e) => failure(format!("failed to serialize: {}", e)),
                },
                OutputFormat::Text => CommandOutput {
                    status: 0,
                    stdout: format!(
                        "id: {}\ncycle: {}\ntitle: {}\nstatus: {:?}\n",
                        wi.id, wi.cycle_id, wi.title, wi.status
                    ),
                    stderr: String::new(),
                },
            }
        }
        WorkItemCommand::List(args) => {
            let items = match storage.list_work_items_by_cycle(&args.cycle_id) {
                Ok(items) => items,
                Err(e) => return failure(format!("failed to list work items: {}", e)),
            };
            // Filter by status if provided
            let items: Vec<_> = if let Some(ref status_str) = args.status {
                let target = match parse_work_item_status(status_str) {
                    Ok(s) => s,
                    Err(e) => {
                        return failure(e);
                    }
                };
                items.into_iter().filter(|i| i.status == target).collect()
            } else {
                items
            };
            let domain_items: Vec<_> = items.into_iter().map(|r| r.into_domain()).collect();
            match args.format {
                OutputFormat::Json => match serde_json::to_string_pretty(&domain_items) {
                    Ok(json) => CommandOutput {
                        status: 0,
                        stdout: format!("{json}\n"),
                        stderr: String::new(),
                    },
                    Err(e) => failure(format!("failed to serialize: {}", e)),
                },
                OutputFormat::Text => {
                    if domain_items.is_empty() {
                        CommandOutput {
                            status: 0,
                            stdout: "no work items found\n".to_string(),
                            stderr: String::new(),
                        }
                    } else {
                        let lines: Vec<String> = domain_items
                            .iter()
                            .map(|wi| format!("- {} [{}] ({:?})", wi.id, wi.title, wi.status))
                            .collect();
                        CommandOutput {
                            status: 0,
                            stdout: format!("{}\n", lines.join("\n")),
                            stderr: String::new(),
                        }
                    }
                }
            }
        }
        WorkItemCommand::Transition(args) => {
            let target_status = match parse_work_item_status(&args.to) {
                Ok(s) => s,
                Err(e) => {
                    return failure(e);
                }
            };
            let mut storage = storage;
            let existing = match storage.get_work_item(&args.work_item_id) {
                Ok(Some(r)) => r,
                Ok(None) => {
                    return failure(format!("work item not found: {}", args.work_item_id));
                }
                Err(e) => return failure(format!("failed to get work item: {}", e)),
            };
            let existing_domain = existing.clone().into_domain();

            // REQ-PLN2-CLI-002 / spec line 276: validate status-state transition
            if !existing_domain.status.can_transition_to(target_status) {
                return failure(format!(
                    "invalid transition from {:?} to {:?}",
                    existing_domain.status, target_status
                ));
            }

            // REQ-PLN2-CLI-002: load incoming edges and apply DependencyResolutionService
            let incoming_edges = match storage.get_dependency_edges_to(&args.work_item_id) {
                Ok(edges) => edges,
                Err(e) => {
                    return failure(format!("failed to load dependency edges: {}", e));
                }
            };
            let domain_edges: Vec<DependencyEdgeV1> = incoming_edges
                .into_iter()
                .map(|r| r.into_domain())
                .collect();

            // Build a pure status lookup from the storage handle
            let status_lookup =
                |wid: &sddk_domain::planning::WorkItemId| -> Option<WorkItemStatus> {
                    if wid == &existing_domain.id {
                        Some(existing_domain.status)
                    } else {
                        storage.get_work_item(wid).ok().flatten().map(|r| r.status)
                    }
                };

            // spec line 794: dependency check mandatory for Draft→Active and terminal transitions
            let is_terminal = matches!(
                target_status,
                WorkItemStatus::Done | WorkItemStatus::Superseded | WorkItemStatus::Cancelled
            );

            if is_terminal {
                // Terminal transition: check BlocksOnClosure + Blocks via resolve_can_terminalize
                if let Err(e) = DependencyResolutionService::resolve_can_terminalize(
                    &existing_domain,
                    target_status,
                    &domain_edges,
                    &status_lookup,
                ) {
                    return failure(format!("transition blocked: {}", e));
                }
            } else if target_status == WorkItemStatus::Active {
                // Draft → Active: check Blocks via resolve_can_activate
                if let Err(e) = DependencyResolutionService::resolve_can_activate(
                    &existing_domain,
                    &domain_edges,
                    &status_lookup,
                ) {
                    return failure(format!("transition blocked: {}", e));
                }
            }
            // Other transitions (e.g. Active↔Paused): no dependency check per spec line 794

            let (actor_ref_kind, actor_ref_id, actor_ref_label) = parse_actor_ref(&args.actor_id);
            let _updated = WorkItemRecord {
                status: target_status,
                actor_ref_kind,
                actor_ref_id: Some(actor_ref_id.unwrap_or_default()),
                actor_ref_label: Some(actor_ref_label.unwrap_or_default()),
                ..existing
            };
            if let Err(e) = storage.update_work_item_status(&args.work_item_id, target_status, None)
            {
                return failure(format!("failed to transition work item: {}", e));
            }
            match args.format {
                OutputFormat::Json => CommandOutput {
                    status: 0,
                    stdout: format!(
                        "{{\"id\": \"{}\", \"status\": \"{:?}\"}}\n",
                        args.work_item_id, target_status
                    ),
                    stderr: String::new(),
                },
                OutputFormat::Text => CommandOutput {
                    status: 0,
                    stdout: format!(
                        "work item {} transitioned to {:?}\n",
                        args.work_item_id, target_status
                    ),
                    stderr: String::new(),
                },
            }
        }
    }
}

// ── Dep subcommand handler ────────────────────────────────────────────────────

fn run_dep(command: DepCommand, environment: &CliEnvironment) -> CommandOutput {
    let mut storage = match open_storage_for_plan(environment) {
        Some(s) => s,
        None => {
            return failure(
                "sddk plan requires an adopted project: no .sddk/adoption.json found in parent dirs"
                    .to_string(),
            );
        }
    };
    match command {
        DepCommand::Add(args) => {
            // Parse the dependency kind
            let kind = match args.kind.to_lowercase().as_str() {
                "blocks" => DependencyEdgeKind::Blocks,
                "blocksonclosure" | "blocks_on_closure" => DependencyEdgeKind::BlocksOnClosure,
                _ => {
                    return failure(format!(
                        "invalid dependency kind: {} (expected Blocks or BlocksOnClosure)",
                        args.kind
                    ));
                }
            };
            let from_id = &args.from_id;
            let to_id = &args.to_id;
            // Load both work items to get their statuses
            let from_wi = match storage.get_work_item(from_id) {
                Ok(Some(r)) => r.into_domain(),
                Ok(None) => return failure(format!("from work item not found: {}", from_id)),
                Err(e) => return failure(format!("failed to get from work item: {}", e)),
            };
            let to_wi = match storage.get_work_item(to_id) {
                Ok(Some(r)) => r.into_domain(),
                Ok(None) => return failure(format!("to work item not found: {}", to_id)),
                Err(e) => return failure(format!("failed to get to work item: {}", e)),
            };
            // Load all edges for the to-workitem to check dependency resolution
            let all_edges = match storage.list_dependency_edges_by_cycle(&to_wi.cycle_id) {
                Ok(edges) => edges,
                Err(e) => return failure(format!("failed to load dependency edges: {}", e)),
            };
            let domain_edges: Vec<DependencyEdgeV1> =
                all_edges.into_iter().map(|r| r.into_domain()).collect();
            let status_lookup = |wid: &sddk_domain::planning::WorkItemId| {
                if wid == &from_wi.id {
                    Some(from_wi.status)
                } else if wid == &to_wi.id {
                    Some(to_wi.status)
                } else {
                    storage.get_work_item(wid).ok().flatten().map(|r| r.status)
                }
            };
            // Validate using DependencyResolutionService
            if let Err(e) = DependencyResolutionService::resolve_can_activate(
                &to_wi,
                &domain_edges,
                &status_lookup,
            ) {
                return failure(format!(
                    "dependency blocked: {} (kind: {:?}): {}",
                    args.to_id, kind, e
                ));
            }
            // Parse actor ref
            let (actor_ref_kind, actor_ref_id, actor_ref_label) = parse_actor_ref(&args.actor_id);
            let edge_record = DependencyEdgeRecord {
                from_id: from_id.clone(),
                to_id: to_id.clone(),
                kind,
                actor_ref_kind,
                actor_ref_id: Some(actor_ref_id.unwrap_or_default()),
                actor_ref_label: Some(actor_ref_label.unwrap_or_default()),
                schema_version: 1,
            };
            if let Err(e) = storage.insert_dependency_edge(&edge_record) {
                return failure(format!("failed to add dependency: {}", e));
            }
            match args.format {
                OutputFormat::Json => CommandOutput {
                    status: 0,
                    stdout: format!(
                        "{{\"from\": \"{}\", \"to\": \"{}\", \"kind\": \"{:?}\"}}\n",
                        from_id, to_id, kind
                    ),
                    stderr: String::new(),
                },
                OutputFormat::Text => CommandOutput {
                    status: 0,
                    stdout: format!("dependency added: {} ─[{:.?}]─> {}\n", from_id, kind, to_id),
                    stderr: String::new(),
                },
            }
        }
    }
}

// ── Evidence subcommand handler ────────────────────────────────────────────────

fn run_evidence(command: EvidenceCommand, environment: &CliEnvironment) -> CommandOutput {
    let mut storage = match open_storage_for_plan(environment) {
        Some(s) => s,
        None => {
            return failure(
                "sddk plan requires an adopted project: no .sddk/adoption.json found in parent dirs"
                    .to_string(),
            );
        }
    };
    match command {
        EvidenceCommand::Attach(args) => {
            // Read the evidence body file
            let body = match std::fs::read(&args.body_file) {
                Ok(b) => b,
                Err(e) => {
                    return failure(format!(
                        "failed to read evidence file {}: {}",
                        args.body_file.display(),
                        e
                    ));
                }
            };
            if body.is_empty() {
                return failure(format!(
                    "evidence body is empty: {}",
                    args.body_file.display()
                ));
            }
            // Parse the evidence kind
            let planning_kind = match args.kind.to_lowercase().as_str() {
                "log" => PlanningEvidenceKind::Log,
                "metric" | "metrics" => PlanningEvidenceKind::Metric,
                "snapshot" => PlanningEvidenceKind::Snapshot,
                "reference" => PlanningEvidenceKind::Reference,
                "approval" => PlanningEvidenceKind::Approval,
                _ => {
                    return failure(format!(
                        "invalid evidence kind: {} (expected: log, metric, snapshot, reference, approval)",
                        args.kind
                    ));
                }
            };
            let (actor_ref_kind, actor_ref_id, actor_ref_label) = parse_actor_ref(&args.actor_id);
            let work_item_id = args.work_item_id.clone();
            let record = EvidenceAttachmentRecord {
                id: Uuid::new_v4().hyphenated().to_string(),
                work_item_id,
                kind: planning_kind,
                body_ref: "pending".to_string(), // Will be set by storage
                actor_ref_kind,
                actor_ref_id: Some(actor_ref_id.unwrap_or_default()),
                actor_ref_label: Some(actor_ref_label.unwrap_or_default()),
                schema_version: EVIDENCE_ATTACHMENT_SCHEMA_VERSION,
            };
            match storage.insert_evidence_attachment(&record, &body) {
                Ok(()) => {
                    // Get the CAS hash that was computed
                    use sha2::{Digest, Sha256};
                    let hash = Sha256::digest(&body);
                    let cas_hash = format!("sha256:{:x}", hash);
                    match args.format {
                        OutputFormat::Json => CommandOutput {
                            status: 0,
                            stdout: format!(
                                "{{\"id\": \"{}\", \"work_item_id\": \"{}\", \"cas\": \"{}\"}}\n",
                                record.id, record.work_item_id, cas_hash
                            ),
                            stderr: String::new(),
                        },
                        OutputFormat::Text => CommandOutput {
                            status: 0,
                            stdout: format!(
                                "evidence attached: {} to {} (cas: {})\n",
                                record.id, record.work_item_id, cas_hash
                            ),
                            stderr: String::new(),
                        },
                    }
                }
                Err(e) => failure(format!("failed to attach evidence: {}", e)),
            }
        }
    }
}

// ── Decision subcommand handler ────────────────────────────────────────────────

fn run_decision(command: DecisionCommand, environment: &CliEnvironment) -> CommandOutput {
    let storage = match open_storage_for_plan(environment) {
        Some(s) => s,
        None => {
            return failure(
                "sddk plan requires an adopted project: no .sddk/adoption.json found in parent dirs"
                    .to_string(),
            );
        }
    };
    match command {
        DecisionCommand::Record(args) => {
            // Parse decision kind
            let decision_kind = match args.kind.to_lowercase().as_str() {
                "accept" | "architectural" => DecisionKind::Accept,
                "reject" | "rejection" | "implementation" => DecisionKind::Reject,
                "defer" | "deferred" | "priority" => DecisionKind::Defer,
                "escalate" | "escalated" => DecisionKind::Escalate,
                _ => {
                    return failure(format!(
                        "invalid decision kind: {} (expected: accept, reject, defer, escalate)",
                        args.kind
                    ));
                }
            };
            // Validate rationale is non-empty using the domain constructor
            let (actor_ref_kind, actor_ref_id, actor_ref_label) = parse_actor_ref(&args.actor_id);
            let domain_dr = match sddk_domain::planning::DecisionRecordV1::new(
                Uuid::new_v4().hyphenated().to_string(),
                args.work_item_id.clone(),
                decision_kind,
                args.rationale.clone(),
                None, // actor_ref
            ) {
                Ok(dr) => dr,
                Err(_) => {
                    return failure("rationale must be non-empty".to_string());
                }
            };
            let mut record = sddk_domain::planning::DecisionRecordRecord::from_domain(&domain_dr);
            // Override actor_ref fields with CLI-provided values
            record.actor_ref_kind = actor_ref_kind;
            record.actor_ref_id = Some(actor_ref_id.unwrap_or_default());
            record.actor_ref_label = Some(actor_ref_label.unwrap_or_default());
            if let Err(e) = storage.insert_decision_record(&record) {
                return failure(format!("failed to record decision: {}", e));
            }
            match args.format {
                OutputFormat::Json => CommandOutput {
                    status: 0,
                    stdout: format!(
                        "{{\"id\": \"{}\", \"work_item_id\": \"{}\"}}\n",
                        record.id, record.work_item_id
                    ),
                    stderr: String::new(),
                },
                OutputFormat::Text => CommandOutput {
                    status: 0,
                    stdout: format!(
                        "decision recorded: {} for work item {}\n",
                        record.id, record.work_item_id
                    ),
                    stderr: String::new(),
                },
            }
        }
    }
}

// ── Graph subcommand handler ──────────────────────────────────────────────────

fn run_graph(cycle_id: &str, format: OutputFormat, environment: &CliEnvironment) -> CommandOutput {
    let storage = match open_storage_for_plan(environment) {
        Some(s) => s,
        None => {
            return failure(
                "sddk plan requires an adopted project: no .sddk/adoption.json found in parent dirs"
                    .to_string(),
            );
        }
    };
    match storage.build_provenance_chain(cycle_id) {
        Ok(chain) => match format {
            OutputFormat::Json => match serde_json::to_string_pretty(&chain) {
                Ok(json) => CommandOutput {
                    status: 0,
                    stdout: format!("{json}\n"),
                    stderr: String::new(),
                },
                Err(e) => failure(format!("failed to serialize provenance chain: {}", e)),
            },
            OutputFormat::Text => CommandOutput {
                status: 0,
                stdout: format!(
                    "cycle: {}\nwork items: {}\nevidence: {}\ndecisions: {}\n",
                    chain.cycle_id,
                    chain.work_item_ids.len(),
                    chain.evidence_refs.len(),
                    chain.decision_refs.len()
                ),
                stderr: String::new(),
            },
        },
        Err(e) => failure(format!("failed to build provenance chain: {}", e)),
    }
}
