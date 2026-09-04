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

use std::path::PathBuf;

use clap::{Args, Subcommand};

use crate::{
    CliEnvironment, CommandOutput, OutputFormat,
    cycle::{self, CyclePathArg, CycleStartArgs, RuntimeArgs},
    failure,
};

/// Deprecation warning emitted when using the legacy `sddk plan <name>` form.
const DEPRECATION_WARNING: &str =
    "sddk plan <name> is deprecated and will be removed in v1.87.0; \
     use 'sddk cycle start --name <name>' instead";

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
pub(crate) fn run_plan(
    command: PlanCommand,
    _environment: &CliEnvironment,
) -> CommandOutput {
    match command {
        PlanCommand::WorkItem { command } => run_workitem(command),
        PlanCommand::Dep { command } => run_dep(command),
        PlanCommand::Evidence { command } => run_evidence(command),
        PlanCommand::Decision { command } => run_decision(command),
        PlanCommand::Graph { cycle_id, format } => run_graph(&cycle_id, format),
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

fn run_workitem(command: WorkItemCommand) -> CommandOutput {
    match command {
        WorkItemCommand::Create(args) => {
            failure(format!(
                "workitem create: PLN-LEDGER-002 CLI storage wiring pending (cycle_id={}, title={})",
                args.cycle_id, args.title
            ))
        }
        WorkItemCommand::Show(args) => {
            failure(format!(
                "workitem show: PLN-LEDGER-002 CLI storage wiring pending (work_item_id={})",
                args.work_item_id
            ))
        }
        WorkItemCommand::List(args) => {
            failure(format!(
                "workitem list: PLN-LEDGER-002 CLI storage wiring pending (cycle_id={})",
                args.cycle_id
            ))
        }
        WorkItemCommand::Transition(args) => {
            failure(format!(
                "workitem transition: PLN-LEDGER-002 CLI storage wiring pending (work_item_id={}, to={})",
                args.work_item_id, args.to
            ))
        }
    }
}

// ── Dep subcommand handler ────────────────────────────────────────────────────

fn run_dep(command: DepCommand) -> CommandOutput {
    match command {
        DepCommand::Add(args) => {
            failure(format!(
                "dep add: PLN-LEDGER-002 CLI storage wiring pending (from_id={}, to_id={}, kind={})",
                args.from_id, args.to_id, args.kind
            ))
        }
    }
}

// ── Evidence subcommand handler ────────────────────────────────────────────────

fn run_evidence(command: EvidenceCommand) -> CommandOutput {
    match command {
        EvidenceCommand::Attach(args) => {
            failure(format!(
                "evidence attach: PLN-LEDGER-002 CLI storage wiring pending (work_item_id={}, kind={})",
                args.work_item_id, args.kind
            ))
        }
    }
}

// ── Decision subcommand handler ────────────────────────────────────────────────

fn run_decision(command: DecisionCommand) -> CommandOutput {
    match command {
        DecisionCommand::Record(args) => {
            failure(format!(
                "decision record: PLN-LEDGER-002 CLI storage wiring pending (work_item_id={}, kind={})",
                args.work_item_id, args.kind
            ))
        }
    }
}

// ── Graph subcommand handler ──────────────────────────────────────────────────

fn run_graph(cycle_id: &str, format: OutputFormat) -> CommandOutput {
    failure(format!(
        "graph: PLN-LEDGER-002 CLI storage wiring pending (cycle_id={})",
        cycle_id
    ))
}
