//! Vault indexing, validation, search, graph, and export commands.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use sddk_gateway::CapabilityPolicy;
use sddk_vault::{Diagnostic, GraphView, SearchHit, Severity};
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
        let (index, diagnostics) = sddk_vault::index_vault(&args.vault)?;
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
