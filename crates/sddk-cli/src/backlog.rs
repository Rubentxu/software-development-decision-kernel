//! Backlog ledger CLI commands (cycle 2/4).
//!
//! Exposes the `BacklogStore` substrate shipped in cycle 1/4 through
//! four subcommands:
//!
//! - `sddk backlog capture` — register a new backlog item.
//! - `sddk backlog triage` — assign/re-assign the priority of an item.
//! - `sddk backlog list` — list live items.
//! - `sddk backlog show <item-id>` — show one item with its full event log.
//!
//! Promotes REQ-Backlog-Item-Capture and REQ-Backlog-Item-Triage-Priority
//! from "engine substrate accepted" to "fully implemented (engine + CLI)".
//!
//! `promote`, `discard`, and `render` are deferred to cycles 3/4.

use clap::{Args, Subcommand, ValueEnum};
use sddk_domain::backlog::{
    BacklogError, BacklogEventLogEntry, BacklogItemId, BacklogItemRow, BacklogPriority,
    generate_ulid, now_rfc3339,
};
use sddk_storage::{BacklogEvent, BacklogStore, SqliteBacklogStoreOwned};
use serde::Serialize;

use crate::cycle::RuntimeArgs;
use crate::{CliEnvironment, CommandOutput, OutputFormat, failure, render_result};

#[derive(Debug, Subcommand)]
pub(crate) enum BacklogCommand {
    /// Register a new backlog item from a cycle.
    Capture(BacklogCaptureArgs),
    /// Update priority of an existing backlog item.
    Triage(BacklogTriageArgs),
    /// List live backlog items (Registered + Triaged only).
    List(BacklogListArgs),
    /// Show a single item with its full event log.
    Show(BacklogShowArgs),
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum CliBacklogPriority {
    P0,
    P1,
    P2,
    P3,
}

impl From<CliBacklogPriority> for BacklogPriority {
    fn from(p: CliBacklogPriority) -> Self {
        match p {
            CliBacklogPriority::P0 => Self::P0,
            CliBacklogPriority::P1 => Self::P1,
            CliBacklogPriority::P2 => Self::P2,
            CliBacklogPriority::P3 => Self::P3,
        }
    }
}

#[derive(Debug, Clone, Args)]
pub(crate) struct BacklogCaptureArgs {
    /// Origin cycle id (e.g. `p-63676b11dc0ef88f/cycle-1`).
    #[arg(long)]
    pub(crate) origin_cycle_id: String,
    /// Origin phase (e.g. `specify`, `design`, `verify`).
    #[arg(long)]
    pub(crate) origin_phase: String,
    /// One-line summary of the idea.
    #[arg(long)]
    pub(crate) summary: String,
    /// Comma-separated list of origin artifacts.
    #[arg(long, value_delimiter = ',')]
    pub(crate) origin_artifacts: Vec<String>,
    /// Actor responsible for the capture.
    #[arg(long)]
    pub(crate) actor_ref: String,
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct BacklogTriageArgs {
    /// Existing item id.
    #[arg(long)]
    pub(crate) item_id: BacklogItemId,
    /// New priority (P0|P1|P2|P3).
    #[arg(long, value_enum)]
    pub(crate) priority: CliBacklogPriority,
    /// Monotonic priority version (default 1).
    #[arg(long, default_value_t = 1)]
    pub(crate) priority_version: u32,
    /// RFC 3339 timestamp; defaults to now.
    #[arg(long)]
    pub(crate) valid_from: Option<String>,
    /// Actor responsible for the triage.
    #[arg(long)]
    pub(crate) actor_ref: String,
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct BacklogListArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct BacklogShowArgs {
    pub(crate) item_id: BacklogItemId,
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

pub(crate) fn run_backlog(command: BacklogCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        BacklogCommand::Capture(args) => run_backlog_capture(args, environment),
        BacklogCommand::Triage(args) => run_backlog_triage(args, environment),
        BacklogCommand::List(args) => run_backlog_list(args, environment),
        BacklogCommand::Show(args) => run_backlog_show(args, environment),
    }
}

fn open_store(
    args: &RuntimeArgs,
    environment: &CliEnvironment,
) -> anyhow::Result<SqliteBacklogStoreOwned> {
    let context = crate::cycle::RuntimeContext::open(args, environment, false)?;
    let ledger_dir = context
        .paths
        .ledger
        .parent()
        .ok_or_else(|| anyhow::anyhow!("ledger path has no parent"))?;
    SqliteBacklogStoreOwned::open(ledger_dir)
        .map_err(|e| anyhow::anyhow!("backlog store open failed: {e}"))
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct CaptureOutput {
    item_id: BacklogItemId,
    event_id: i64,
}

fn capture_text(o: &CaptureOutput) -> String {
    format!("item_id: {}\nevent_id: {}\n", o.item_id, o.event_id)
}

fn run_backlog_capture(args: BacklogCaptureArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<CaptureOutput> {
        if args.origin_cycle_id.is_empty() || args.origin_phase.is_empty() {
            return Err(BacklogError::OriginEvidenceRequired.into());
        }
        if args.summary.is_empty() {
            return Err(BacklogError::EmptySummary.into());
        }
        let mut store = open_store(&args.runtime, environment)?;
        let item_id = format!("bl-{}", generate_ulid());
        let event = BacklogEvent::Registered {
            item_id: item_id.clone(),
            origin_cycle_id: args.origin_cycle_id,
            origin_phase: args.origin_phase,
            origin_artifacts: args.origin_artifacts,
            summary: args.summary,
            captured_at: now_rfc3339(),
            actor_ref: Some(args.actor_ref),
        };
        let event_id = store
            .append_event(&event)
            .map_err(|e| anyhow::anyhow!("backlog capture failed: {e}"))?;
        Ok(CaptureOutput { item_id, event_id })
    })();
    match result {
        Ok(out) => render_result(Ok(out), format, capture_text),
        Err(e) => failure(e.to_string()),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct TriageOutput {
    item_id: BacklogItemId,
    event_id: i64,
    priority: String,
    priority_version: u32,
    valid_from: String,
}

fn triage_text(o: &TriageOutput) -> String {
    format!(
        "item_id: {}\nevent_id: {}\npriority: {}\npriority_version: {}\nvalid_from: {}\n",
        o.item_id, o.event_id, o.priority, o.priority_version, o.valid_from
    )
}

fn run_backlog_triage(args: BacklogTriageArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<TriageOutput> {
        let mut store = open_store(&args.runtime, environment)?;
        if store
            .item(&args.item_id)
            .map_err(|e| anyhow::anyhow!("backlog item lookup failed: {e}"))?
            .is_none()
        {
            return Err(BacklogError::ItemNotFound(args.item_id.clone()).into());
        }
        let priority: BacklogPriority = args.priority.into();
        let valid_from = args.valid_from.clone().unwrap_or_else(now_rfc3339);
        let event = BacklogEvent::Triaged {
            item_id: args.item_id.clone(),
            priority,
            priority_version: args.priority_version,
            valid_from: valid_from.clone(),
            valid_to: "9999-12-31T23:59:59Z".to_string(),
            actor_ref: Some(args.actor_ref.clone()),
        };
        let event_id = store
            .append_event(&event)
            .map_err(|e| anyhow::anyhow!("backlog triage failed: {e}"))?;
        Ok(TriageOutput {
            item_id: args.item_id,
            event_id,
            priority: format!("{:?}", priority),
            priority_version: args.priority_version,
            valid_from,
        })
    })();
    match result {
        Ok(out) => render_result(Ok(out), format, triage_text),
        Err(e) => failure(e.to_string()),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct ListOutput {
    items: Vec<BacklogItemRow>,
}

fn list_text(o: &ListOutput) -> String {
    if o.items.is_empty() {
        return "(no live backlog items)\n".to_string();
    }
    let mut s = String::new();
    s.push_str(
        "item_id | origin_cycle_id | origin_phase | priority | status | captured_at | events\n",
    );
    s.push_str("--- | --- | --- | --- | --- | --- | ---\n");
    for r in &o.items {
        let prio = r
            .current_priority
            .map(|p| format!("{:?}", p))
            .unwrap_or_else(|| "-".to_string());
        s.push_str(&format!(
            "{} | {} | {} | {} | {:?} | {} | {}\n",
            r.item_id,
            r.origin_cycle_id,
            r.origin_phase,
            prio,
            r.current_status,
            r.captured_at,
            r.emitted_event_count
        ));
    }
    s
}

fn run_backlog_list(args: BacklogListArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<ListOutput> {
        let mut store = open_store(&args.runtime, environment)?;
        let items = store
            .live_items()
            .map_err(|e| anyhow::anyhow!("backlog list failed: {e}"))?;
        Ok(ListOutput { items })
    })();
    match result {
        Ok(out) => render_result(Ok(out), format, list_text),
        Err(e) => failure(e.to_string()),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct ShowOutput {
    item: Option<BacklogItemRow>,
    events: Vec<BacklogEventLogEntry>,
}

fn show_text(o: &ShowOutput) -> String {
    match &o.item {
        None => format!(
            "item not found: no events returned ({} event log rows)\n",
            o.events.len()
        ),
        Some(row) => {
            let mut s = String::new();
            s.push_str("=== item ===\n");
            s.push_str(&format!("item_id: {}\n", row.item_id));
            s.push_str(&format!("origin_cycle_id: {}\n", row.origin_cycle_id));
            s.push_str(&format!("origin_phase: {}\n", row.origin_phase));
            s.push_str(&format!("summary: {}\n", row.summary));
            s.push_str(&format!(
                "current_priority: {}\n",
                row.current_priority
                    .map(|p| format!("{:?}", p))
                    .unwrap_or_else(|| "-".to_string())
            ));
            s.push_str(&format!("current_status: {:?}\n", row.current_status));
            s.push_str(&format!("captured_at: {}\n", row.captured_at));
            s.push_str(&format!(
                "emitted_event_count: {}\n",
                row.emitted_event_count
            ));
            s.push_str("\n=== event log ===\n");
            for (i, ev) in o.events.iter().enumerate() {
                s.push_str(&format!(
                    "event #{}: type={} v{} at={} actor={}\n  payload: {}\n",
                    i + 1,
                    ev.event_type,
                    ev.schema_version,
                    ev.emitted_at,
                    if ev.actor_ref.is_empty() {
                        "-"
                    } else {
                        &ev.actor_ref
                    },
                    ev.payload
                ));
            }
            s
        }
    }
}

fn run_backlog_show(args: BacklogShowArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<ShowOutput> {
        let mut store = open_store(&args.runtime, environment)?;
        let item = store
            .item(&args.item_id)
            .map_err(|e| anyhow::anyhow!("backlog show failed: {e}"))?;
        let events = store
            .events(&args.item_id)
            .map_err(|e| anyhow::anyhow!("backlog show events failed: {e}"))?;
        if item.is_none() && events.is_empty() {
            return Err(BacklogError::ItemNotFound(args.item_id).into());
        }
        Ok(ShowOutput { item, events })
    })();
    match result {
        Ok(out) => render_result(Ok(out), format, show_text),
        Err(e) => failure(e.to_string()),
    }
}
