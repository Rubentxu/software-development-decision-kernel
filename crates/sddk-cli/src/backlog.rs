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
//! Promotes REQ-Backlog-Item-Promote-Discard and
//! REQ-Backlog-Roadmap-Projection from "engine substrate accepted" to
//! "fully implemented (engine + CLI)" (cycle 3/4).

use clap::{Args, Subcommand, ValueEnum};
use sddk_domain::backlog::{
    BacklogError, BacklogEventLogEntry, BacklogItemId, BacklogItemRow, BacklogPriority,
    BacklogRenderKind, generate_ulid, now_rfc3339,
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
    /// Promote a triaged item into a target cycle (terminal state).
    Promote(BacklogPromoteArgs),
    /// Discard a triaged item with a closed-set reason (terminal state).
    Discard(BacklogDiscardArgs),
    /// Render BACKLOG.md / ROADMAP.md as a ledger-derived projection.
    Render(BacklogRenderArgs),
}

/// Closed-set discard reasons (REQ-Backlog-Item-Promote-Discard).
#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum CliDiscardReason {
    Superseded,
    Wontfix,
    Duplicate,
}

impl From<CliDiscardReason> for String {
    fn from(r: CliDiscardReason) -> Self {
        match r {
            CliDiscardReason::Superseded => "superseded".to_string(),
            CliDiscardReason::Wontfix => "wontfix".to_string(),
            CliDiscardReason::Duplicate => "duplicate".to_string(),
        }
    }
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

#[derive(Debug, Clone, Args)]
pub(crate) struct BacklogPromoteArgs {
    /// Existing triaged item id.
    #[arg(long)]
    pub(crate) item_id: BacklogItemId,
    /// Target cycle id to adopt the item into.
    #[arg(long)]
    pub(crate) to_cycle: String,
    /// Actor responsible for the promotion.
    #[arg(long)]
    pub(crate) actor_ref: String,
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct BacklogDiscardArgs {
    /// Existing triaged item id.
    #[arg(long)]
    pub(crate) item_id: BacklogItemId,
    /// Closed-set reason (superseded|wontfix|duplicate).
    #[arg(long, value_enum)]
    pub(crate) reason: CliDiscardReason,
    /// Actor responsible for the discard.
    #[arg(long)]
    pub(crate) actor_ref: String,
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct BacklogRenderArgs {
    /// What to render (backlog or roadmap).
    #[arg(long, value_enum, default_value_t = CliRenderKind::Backlog)]
    pub(crate) kind: CliRenderKind,
    /// Output path (default: BACKLOG.md / ROADMAP.md in the workspace root).
    #[arg(long)]
    pub(crate) output: Option<String>,
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum CliRenderKind {
    Backlog,
    Roadmap,
}

impl From<CliRenderKind> for BacklogRenderKind {
    fn from(k: CliRenderKind) -> Self {
        match k {
            CliRenderKind::Backlog => Self::Backlog,
            CliRenderKind::Roadmap => Self::Roadmap,
        }
    }
}

pub(crate) fn run_backlog(command: BacklogCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        BacklogCommand::Capture(args) => run_backlog_capture(args, environment),
        BacklogCommand::Triage(args) => run_backlog_triage(args, environment),
        BacklogCommand::List(args) => run_backlog_list(args, environment),
        BacklogCommand::Show(args) => run_backlog_show(args, environment),
        BacklogCommand::Promote(args) => run_backlog_promote(args, environment),
        BacklogCommand::Discard(args) => run_backlog_discard(args, environment),
        BacklogCommand::Render(args) => run_backlog_render(args, environment),
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

/// Loads the current item row and enforces the terminal-state contract:
/// the item MUST exist and MUST NOT be already promoted or discarded
/// (REQ-Backlog-Item-Promote-Discard scenarios 1-2).
fn require_transitionable(
    store: &mut SqliteBacklogStoreOwned,
    item_id: &BacklogItemId,
) -> anyhow::Result<BacklogItemRow> {
    let row = store
        .item(item_id)
        .map_err(|e| anyhow::anyhow!("backlog item lookup failed: {e}"))?
        .ok_or_else(|| BacklogError::ItemNotFound(item_id.clone()))?;
    match row.current_status {
        sddk_domain::backlog::BacklogStatus::Promoted => {
            Err(BacklogError::AlreadyPromoted(item_id.clone()).into())
        }
        sddk_domain::backlog::BacklogStatus::Discarded => {
            Err(BacklogError::AlreadyDiscarded(item_id.clone()).into())
        }
        _ => Ok(row),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct PromoteOutput {
    item_id: BacklogItemId,
    event_id: i64,
    target_id: String,
    promoted_at: String,
}

fn promote_text(o: &PromoteOutput) -> String {
    format!(
        "item_id: {}\nevent_id: {}\ntarget_id: {}\npromoted_at: {}\n",
        o.item_id, o.event_id, o.target_id, o.promoted_at
    )
}

fn run_backlog_promote(args: BacklogPromoteArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<PromoteOutput> {
        if args.to_cycle.is_empty() {
            return Err(BacklogError::OriginEvidenceRequired.into());
        }
        let mut store = open_store(&args.runtime, environment)?;
        require_transitionable(&mut store, &args.item_id)?;
        let promoted_at = now_rfc3339();
        let event = BacklogEvent::Promoted {
            item_id: args.item_id.clone(),
            target_kind: "cycle".to_string(),
            target_id: args.to_cycle.clone(),
            promoted_at: promoted_at.clone(),
            actor_ref: Some(args.actor_ref),
        };
        let event_id = store
            .append_event(&event)
            .map_err(|e| anyhow::anyhow!("backlog promote failed: {e}"))?;
        Ok(PromoteOutput {
            item_id: args.item_id,
            event_id,
            target_id: args.to_cycle,
            promoted_at,
        })
    })();
    match result {
        Ok(out) => render_result(Ok(out), format, promote_text),
        Err(e) => failure(e.to_string()),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct DiscardOutput {
    item_id: BacklogItemId,
    event_id: i64,
    reason: String,
    discarded_at: String,
}

fn discard_text(o: &DiscardOutput) -> String {
    format!(
        "item_id: {}\nevent_id: {}\nreason: {}\ndiscarded_at: {}\n",
        o.item_id, o.event_id, o.reason, o.discarded_at
    )
}

fn run_backlog_discard(args: BacklogDiscardArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<DiscardOutput> {
        let mut store = open_store(&args.runtime, environment)?;
        require_transitionable(&mut store, &args.item_id)?;
        let reason: String = args.reason.into();
        let discarded_at = now_rfc3339();
        let event = BacklogEvent::Discarded {
            item_id: args.item_id.clone(),
            reason: reason.clone(),
            discarded_at: discarded_at.clone(),
            actor_ref: Some(args.actor_ref),
        };
        let event_id = store
            .append_event(&event)
            .map_err(|e| anyhow::anyhow!("backlog discard failed: {e}"))?;
        Ok(DiscardOutput {
            item_id: args.item_id,
            event_id,
            reason,
            discarded_at,
        })
    })();
    match result {
        Ok(out) => render_result(Ok(out), format, discard_text),
        Err(e) => failure(e.to_string()),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct RenderOutput {
    kind: String,
    output_path: String,
    rendered_items: usize,
    sha256: String,
}

fn render_text(o: &RenderOutput) -> String {
    format!(
        "kind: {}\noutput_path: {}\nrendered_items: {}\nsha256: {}\n",
        o.kind, o.output_path, o.rendered_items, o.sha256
    )
}

/// Deterministic markdown projection of the live backlog items
/// (REQ-Backlog-Roadmap-Projection): sorted by (priority, item_id),
/// one wikilink entry per live item, byte-identical for the same
/// ledger head.
fn render_projection(kind: BacklogRenderKind, items: &[BacklogItemRow]) -> String {
    let title = match kind {
        BacklogRenderKind::Backlog => "# BACKLOG",
        BacklogRenderKind::Roadmap => "# ROADMAP",
    };
    let mut s = String::new();
    s.push_str(title);
    s.push_str("\n\n> Ledger-derived projection. Hand-edits are overwritten on next render.\n\n");
    let mut sorted: Vec<&BacklogItemRow> = items.iter().collect();
    sorted.sort_by(|a, b| {
        a.current_priority
            .map(|p| p as u8)
            .cmp(&b.current_priority.map(|p| p as u8))
            .then_with(|| a.item_id.cmp(&b.item_id))
    });
    for r in sorted {
        let prio = r
            .current_priority
            .map(|p| format!("{:?}", p))
            .unwrap_or_else(|| "-".to_string());
        s.push_str(&format!(
            "- [[{}]] {} — priority {} · status {:?} · origin `{}`\n",
            r.item_id, r.summary, prio, r.current_status, r.origin_cycle_id
        ));
    }
    s
}

fn run_backlog_render(args: BacklogRenderArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<RenderOutput> {
        use sha2::{Digest, Sha256};
        let mut store = open_store(&args.runtime, environment)?;
        let items = store
            .live_items()
            .map_err(|e| anyhow::anyhow!("backlog render failed: {e}"))?;
        let kind: BacklogRenderKind = args.kind.into();
        let markdown = render_projection(kind, &items);
        let bytes = markdown.as_bytes();
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let sha256 = format!("sha256:{:x}", hasher.finalize());
        let output_path = args.output.clone().unwrap_or_else(|| match kind {
            BacklogRenderKind::Backlog => "BACKLOG.md".to_string(),
            BacklogRenderKind::Roadmap => "ROADMAP.md".to_string(),
        });
        std::fs::write(&output_path, bytes)
            .map_err(|e| anyhow::anyhow!("backlog render write failed: {e}"))?;
        Ok(RenderOutput {
            kind: format!("{:?}", kind).to_lowercase(),
            output_path,
            rendered_items: items.len(),
            sha256,
        })
    })();
    match result {
        Ok(out) => render_result(Ok(out), format, render_text),
        Err(e) => failure(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sddk_domain::backlog::BacklogStatus;

    fn row(id: &str, prio: Option<BacklogPriority>, status: BacklogStatus) -> BacklogItemRow {
        BacklogItemRow {
            item_id: id.to_string(),
            origin_cycle_id: "p-x/c1".to_string(),
            origin_phase: "explore".to_string(),
            summary: format!("summary of {id}"),
            current_priority: prio,
            current_status: status,
            captured_at: "2026-09-12T00:00:00Z".to_string(),
            emitted_event_count: 2,
        }
    }

    #[test]
    fn render_includes_live_items_and_excludes_none_here() {
        let items = vec![
            row("B-002", Some(BacklogPriority::P1), BacklogStatus::Triaged),
            row(
                "B-001",
                Some(BacklogPriority::P0),
                BacklogStatus::Registered,
            ),
        ];
        let md = render_projection(BacklogRenderKind::Backlog, &items);
        assert!(md.contains("# BACKLOG"));
        assert!(md.contains("[[B-001]]"));
        assert!(md.contains("[[B-002]]"));
        // priority sort: P0 (B-001) before P1 (B-002)
        let b1 = md.find("[[B-001]]").unwrap();
        let b2 = md.find("[[B-002]]").unwrap();
        assert!(b1 < b2);
    }

    #[test]
    fn render_is_deterministic() {
        let items = vec![row(
            "B-001",
            Some(BacklogPriority::P2),
            BacklogStatus::Triaged,
        )];
        let a = render_projection(BacklogRenderKind::Backlog, &items);
        let b = render_projection(BacklogRenderKind::Backlog, &items);
        assert_eq!(a, b);
    }

    #[test]
    fn roadmap_kind_renders_roadmap_title() {
        let items = vec![row(
            "B-001",
            Some(BacklogPriority::P1),
            BacklogStatus::Triaged,
        )];
        let md = render_projection(BacklogRenderKind::Roadmap, &items);
        assert!(md.starts_with("# ROADMAP"));
    }

    #[test]
    fn empty_items_render_header_only() {
        let md = render_projection(BacklogRenderKind::Backlog, &[]);
        assert!(md.contains("# BACKLOG"));
        assert!(!md.contains("[["));
    }
}
