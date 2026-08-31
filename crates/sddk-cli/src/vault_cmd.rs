//! Vault indexing, validation, search, graph, and export commands.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use sddk_gateway::CapabilityPolicy;
use sddk_vault::{
    ALLOW_LIST, Diagnostic, GraphView, RepairReceipt, SearchHit, Severity, load_repair_queue,
};
use serde::Serialize;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat, RuntimeArgs, RuntimeContext, render_result,
};

#[derive(Debug, Subcommand)]
pub(crate) enum VaultCommand {
    /// Parse, validate, and rebuild the FTS index from a vault.
    Index(VaultIndexArgs),
    /// Validate node ids, titles, and wikilinks.
    Validate(VaultIndexArgs),
    /// Search the FTS index.
    Search(VaultSearchArgs),
    /// Show graph facts (cycles, topological order).
    Graph(VaultIndexArgs),
    /// Export a self-contained HTML inspector.
    Export(VaultExportArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct VaultIndexArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Vault directory.
    #[arg(long)]
    pub(crate) vault: PathBuf,
    /// SQLite index database path.
    #[arg(long)]
    pub(crate) db: Option<PathBuf>,
    /// Repeatable scope cycles for scoped VAULT003 down-classification.
    /// Each value must match `project_id/cycle_id` form.
    #[arg(long, value_name = "SCOPE", value_delimiter = ',')]
    pub(crate) scope_cycles: Vec<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct VaultSearchArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// SQLite index database path.
    #[arg(long)]
    pub(crate) db: PathBuf,
    /// Full-text query.
    #[arg(long)]
    pub(crate) query: String,
    /// Maximum hits.
    #[arg(long, default_value_t = 20)]
    pub(crate) limit: usize,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct VaultExportArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Vault directory.
    #[arg(long)]
    pub(crate) vault: PathBuf,
    /// Output HTML file.
    #[arg(long)]
    pub(crate) output: PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

pub(crate) fn run_vault(command: VaultCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        VaultCommand::Index(args) => run_vault_index(args, environment, false),
        VaultCommand::Validate(args) => run_vault_index(args, environment, true),
        VaultCommand::Search(args) => run_vault_search(args, environment),
        VaultCommand::Graph(args) => run_vault_graph(args, environment),
        VaultCommand::Export(args) => run_vault_export(args, environment),
    }
}

/// Check that a vault capability is authorized under the workflow policy.
fn check_vault_capability(
    runtime: &RuntimeArgs,
    environment: &CliEnvironment,
    capability: &str,
) -> anyhow::Result<()> {
    let context = RuntimeContext::open(runtime, environment, false)?;
    let policy = CapabilityPolicy::from_workflow(context.engine.workflow());
    let decision = policy.authorize(capability, false);
    if !decision.allowed {
        anyhow::bail!(
            "error[GATEWAY_DENIED]: capability {} is denied by policy{}",
            capability,
            if decision.requires_approval {
                " (requires --approve)"
            } else {
                ""
            }
        );
    }
    Ok(())
}

/// Apply scoped down-classification to VAULT003 diagnostics that have a matching
/// repair receipt in the queue.
///
/// A diagnostic is down-classified from `Error` to `Warning` when:
/// - Its code is in the `ALLOW_LIST` (verbatim `{VAULT003}` per ADR-0078)
/// - Its `scope` matches one of the provided `scope_cycles` arguments
/// - A valid (non-expired) `RepairReceipt` exists in the queue for that scope
fn apply_scope_downgrade(
    diagnostics: &mut [Diagnostic],
    scope_cycles: &[String],
    queue: &std::collections::HashMap<String, RepairReceipt>,
) {
    if scope_cycles.is_empty() {
        return;
    }

    let now = time::OffsetDateTime::now_utc();

    for diagnostic in diagnostics.iter_mut() {
        // Only process allow-listed codes
        if !ALLOW_LIST.contains(&diagnostic.code.as_str()) {
            continue;
        }

        // Check if diagnostic has a matching scope
        let Some(ref scope) = diagnostic.scope else {
            continue;
        };

        // cycle_id is the full target (project_id/cycle_id) after attach_scope fix
        if !scope_cycles.contains(&scope.cycle_id) {
            continue;
        }

        // Find matching receipt
        // Key format: {cycle_id}/{code}/{node} — cycle_id is the full
        // project_id/cycle_id target string (fixed in attach_scope)
        let key = format!(
            "{}/{}/{}",
            scope.cycle_id,
            diagnostic.code,
            diagnostic.node.as_deref().unwrap_or("")
        );
        if let Some(receipt) = queue.get(&key) {
            // Check if receipt is valid (not expired)
            if receipt.valid_to > now {
                // Down-classify
                diagnostic.severity = Severity::Warning;
            }
        }
    }
}

/// Summary of a repair receipt for JSON output.
#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct RepairReceiptSummary {
    cycle_id: String,
    code: String,
    node: String,
    expired: bool,
}

impl From<&RepairReceipt> for RepairReceiptSummary {
    fn from(receipt: &RepairReceipt) -> Self {
        RepairReceiptSummary {
            cycle_id: receipt.cycle_id.clone(),
            code: receipt.code.clone(),
            node: receipt.node.clone(),
            expired: receipt.valid_to < time::OffsetDateTime::now_utc(),
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct IndexOutput {
    nodes: usize,
    backlinks: usize,
    errors: usize,
    warnings: usize,
    inserted: usize,
    updated: usize,
    deleted: usize,
    diagnostics: Vec<Diagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repair_queue: Option<Vec<RepairReceiptSummary>>,
}

fn run_vault_index(
    args: VaultIndexArgs,
    environment: &CliEnvironment,
    validate_only: bool,
) -> CommandOutput {
    let format = args.format;
    let capability = if validate_only {
        "vault.validate"
    } else {
        "vault.index"
    };
    let result = (|| -> anyhow::Result<IndexOutput> {
        check_vault_capability(&args.runtime, environment, capability)?;
        let (index, mut diagnostics) = sddk_vault::index_vault(&args.vault)?;
        let backlinks: usize = index.backlinks.values().map(Vec::len).sum();
        let mut inserted = 0;
        let mut updated = 0;
        let mut deleted = 0;
        if !validate_only {
            let db = match args.db {
                Some(db) => db,
                None => args
                    .vault
                    .parent()
                    .map(|parent| parent.join("vault-index.sqlite"))
                    .unwrap_or_else(|| PathBuf::from("vault-index.sqlite")),
            };
            let connection = sddk_vault::open_index(&db)?;
            let summary = sddk_vault::sync_search_index(&connection, &index)?;
            inserted = summary.inserted;
            updated = summary.updated;
            deleted = summary.deleted;
        }

        // Load repair queue and apply scoped down-classification
        let repair_queue_path = args.vault.join("repair-queue.yaml");
        let queue = if repair_queue_path.exists() {
            load_repair_queue(&repair_queue_path).unwrap_or_default()
        } else {
            std::collections::HashMap::new()
        };

        // Apply scope downgrades if --scope-cycles is provided
        apply_scope_downgrade(&mut diagnostics, &args.scope_cycles, &queue);

        // Build repair queue summary for JSON output
        let repair_queue_summary: Option<Vec<RepairReceiptSummary>> = if queue.is_empty() {
            None
        } else {
            Some(queue.values().map(RepairReceiptSummary::from).collect())
        };

        let (errors, warnings) = sddk_vault::summary(&diagnostics);
        Ok(IndexOutput {
            nodes: index.nodes.len(),
            backlinks,
            errors,
            warnings,
            inserted,
            updated,
            deleted,
            diagnostics,
            repair_queue: repair_queue_summary,
        })
    })();
    match result {
        Ok(output) => {
            let mut command = render_result(Ok(output.clone()), format, index_text);
            if validate_only && output.errors > 0 {
                command.status = 1;
            }
            command
        }
        Err(error) => crate::failure(error.to_string()),
    }
}

fn run_vault_search(args: VaultSearchArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<Vec<SearchHit>> {
        check_vault_capability(&args.runtime, environment, "vault.search")?;
        let connection = sddk_vault::open_index(&args.db)?;
        Ok(sddk_vault::search_index(
            &connection,
            &args.query,
            args.limit,
        )?)
    })();
    render_result(result, format, search_text)
}

fn run_vault_graph(args: VaultIndexArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<GraphView> {
        check_vault_capability(&args.runtime, environment, "vault.graph")?;
        let index = sddk_vault::parse_vault(&args.vault)?;
        Ok(sddk_vault::graph_view(&index)?)
    })();
    render_result(result, format, graph_text)
}

fn run_vault_export(args: VaultExportArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<String> {
        check_vault_capability(&args.runtime, environment, "vault.export")?;
        let (index, _) = sddk_vault::index_vault(&args.vault)?;
        let graph = sddk_vault::graph_view(&index)?;
        let html = sddk_vault::export_html(&index, &graph)?;
        std::fs::write(&args.output, &html)?;
        Ok(args.output.to_string_lossy().into_owned())
    })();
    render_result(result, format, |path| format!("wrote {path}\n"))
}

fn index_text(output: &IndexOutput) -> String {
    let mut text = format!(
        "nodes: {}\nbacklinks: {}\nerrors: {}\nwarnings: {}\ninserted: {}\nupdated: {}\ndeleted: {}\n",
        output.nodes,
        output.backlinks,
        output.errors,
        output.warnings,
        output.inserted,
        output.updated,
        output.deleted
    );
    for diagnostic in &output.diagnostics {
        text.push_str(&format!(
            "{}[{}] {}: {}\n  help: {}\n",
            match diagnostic.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            },
            diagnostic.code,
            diagnostic.node.as_deref().unwrap_or("-"),
            diagnostic.message,
            diagnostic.hint
        ));
    }
    text
}

fn search_text(hits: &Vec<SearchHit>) -> String {
    if hits.is_empty() {
        return "no hits\n".to_owned();
    }
    let mut text = String::new();
    for hit in hits {
        text.push_str(&format!("{} {} {}\n", hit.id, hit.kind, hit.path));
    }
    text
}

fn graph_text(view: &GraphView) -> String {
    let mut text = format!(
        "node_count: {}\nedge_count: {}\ncyclic: {}\n",
        view.node_count, view.edge_count, view.cyclic
    );
    if let Some(cycle) = &view.sample_cycle {
        text.push_str(&format!("sample_cycle: {}\n", cycle.join(" -> ")));
    }
    if let Some(order) = &view.topological_order {
        text.push_str(&format!("topological_order: {}\n", order.join(", ")));
    }
    text
}
