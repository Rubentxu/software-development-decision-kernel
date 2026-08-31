//! Reactive knowledge graph commands: query, why, rebuild (SPEC-004 §9).

use clap::{Args, Subcommand};
use sddk_domain::{GraphState, GraphStore, PatternQuery, PatternStep};
use sddk_storage::graph_store::SqliteGraphStore;
use serde::Serialize;

use crate::cycle::RuntimeArgs;
use crate::{CliEnvironment, CommandOutput, OutputFormat, render_result};

#[derive(Debug, Subcommand)]
pub(crate) enum GraphCommand {
    /// Run a deterministic pattern query over the graph.
    Query(GraphQueryArgs),
    /// Show the provenance of a node (`kind:id`) or relation.
    Why(GraphWhyArgs),
    /// Rebuild the graph projection from the event ledger.
    Rebuild(GraphRebuildArgs),
    /// Show staleness state and causal path for an entity.
    WhyStale(GraphWhyStaleArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct GraphWhyStaleArgs {
    /// Entity key (`kind:id`), e.g. `requirement:R1`.
    #[arg(long)]
    pub(crate) entity: String,
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct GraphQueryArgs {
    /// Pattern expression, e.g. `capability -> approval.capability.granted -> actor`.
    #[arg(long)]
    pub(crate) pattern: String,
    /// Fixed start node (`kind:id`).
    #[arg(long)]
    pub(crate) start: Option<String>,
    /// Start node type predicate (e.g. `capability`).
    #[arg(long)]
    pub(crate) start_type: Option<String>,
    /// Maximum traversal depth (0 = unbounded).
    #[arg(long, default_value_t = 0)]
    pub(crate) max_depth: u32,
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct GraphWhyArgs {
    /// Entity key (`kind:id`), e.g. `capability:git.commit`.
    #[arg(long)]
    pub(crate) entity: String,
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct GraphRebuildArgs {
    /// Event stream to project (defaults to `project:<project_id>`).
    #[arg(long)]
    pub(crate) stream: Option<String>,
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

pub(crate) fn run_graph(command: GraphCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        GraphCommand::Query(args) => run_graph_query(args, environment),
        GraphCommand::Why(args) => run_graph_why(args, environment),
        GraphCommand::Rebuild(args) => run_graph_rebuild(args, environment),
        GraphCommand::WhyStale(args) => run_graph_why_stale(args, environment),
    }
}

fn run_graph_why_stale(args: GraphWhyStaleArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<WhyStaleOutput> {
        let state = crate::stale_cmd::load_graph_state(&args.runtime, environment)?;
        let staleness = crate::stale_cmd::derive_for_entity(&state, &args.entity);
        Ok(WhyStaleOutput {
            entity: args.entity.clone(),
            state: crate::stale_cmd::serde_state_name(staleness.state),
            causal_path: staleness.causal_path,
            verified_by: staleness.verified_by,
        })
    })();
    match result {
        Ok(output) => render_result(Ok(output), format, why_stale_text),
        Err(error) => crate::failure(error.to_string()),
    }
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct WhyStaleOutput {
    entity: String,
    state: String,
    causal_path: Vec<String>,
    verified_by: Option<String>,
}

fn why_stale_text(output: &WhyStaleOutput) -> String {
    let path = if output.causal_path.is_empty() {
        "-".to_string()
    } else {
        output.causal_path.join(" -> ")
    };
    let verified = output.verified_by.as_deref().unwrap_or("-");
    format!(
        "entity: {}\nstate: {}\nverified_by: {}\ncausal_path: {}\n",
        output.entity, output.state, verified, path
    )
}

fn open_graph_store(
    args: &RuntimeArgs,
    environment: &CliEnvironment,
) -> anyhow::Result<(SqliteGraphStore, String)> {
    let context = crate::cycle::RuntimeContext::open(args, environment, false)?;
    let ledger_dir = context
        .paths
        .ledger
        .parent()
        .ok_or_else(|| anyhow::anyhow!("ledger path has no parent"))?
        .to_path_buf();
    let stream = format!("project:{}", context.identity.project_id);
    let store = SqliteGraphStore::open(&ledger_dir)?;
    Ok((store, stream))
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct GraphQueryOutput {
    matches: Vec<Vec<String>>,
}

fn run_graph_query(args: GraphQueryArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<GraphQueryOutput> {
        let (store, stream) = open_graph_store(&args.runtime, environment)?;
        let state = store.load_state()?.ok_or_else(|| {
            anyhow::anyhow!("graph not built — run `sddk graph rebuild` first (stream {stream})")
        })?;
        let query = parse_pattern(&args.pattern, &args.start, &args.start_type, args.max_depth)?;
        let matches = query.execute(&state);
        Ok(GraphQueryOutput { matches })
    })();
    match result {
        Ok(output) => render_result(Ok(output), format, graph_query_text),
        Err(error) => crate::failure(error.to_string()),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct GraphWhyOutput {
    entity: String,
    found: bool,
    node: Option<sddk_domain::GraphNode>,
    relations: Vec<sddk_domain::GraphEdge>,
}

fn run_graph_why(args: GraphWhyArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<GraphWhyOutput> {
        let (store, _stream) = open_graph_store(&args.runtime, environment)?;
        let state = store.load_state()?.unwrap_or_default();
        let node = state.nodes.get(&args.entity).cloned();
        let relations: Vec<sddk_domain::GraphEdge> = state
            .edges
            .iter()
            .filter(|e| e.from == args.entity || e.to == args.entity)
            .cloned()
            .collect();
        Ok(GraphWhyOutput {
            entity: args.entity.clone(),
            found: node.is_some(),
            node,
            relations,
        })
    })();
    match result {
        Ok(output) => render_result(Ok(output), format, graph_why_text),
        Err(error) => crate::failure(error.to_string()),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct GraphRebuildOutput {
    stream: String,
    nodes: usize,
    edges: usize,
    last_event_sequence: u64,
}

fn run_graph_rebuild(args: GraphRebuildArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<GraphRebuildOutput> {
        let (mut store, default_stream) = open_graph_store(&args.runtime, environment)?;
        let stream = args.stream.clone().unwrap_or(default_stream.clone());
        let state: GraphState = store.rebuild_from_ledger(&stream)?;
        Ok(GraphRebuildOutput {
            stream,
            nodes: state.nodes.len(),
            edges: state.edges.len(),
            last_event_sequence: state.last_event_sequence,
        })
    })();
    match result {
        Ok(output) => render_result(Ok(output), format, graph_rebuild_text),
        Err(error) => crate::failure(error.to_string()),
    }
}

/// Parses a simple pattern expression:
/// `Type -> relation -> Type [-> relation -> Type ...]`
/// where each segment may be prefixed with `!` for NOT EXISTS.
fn parse_pattern(
    pattern: &str,
    start: &Option<String>,
    start_type: &Option<String>,
    max_depth: u32,
) -> anyhow::Result<PatternQuery> {
    let segments: Vec<&str> = pattern.split("->").map(|s| s.trim()).collect();
    if segments.len() < 2 {
        anyhow::bail!("invalid pattern: expected at least 'Type -> relation' (got '{pattern}')");
    }
    let mut steps = Vec::new();
    let mut idx = 1;
    while idx < segments.len() {
        let relation = segments[idx];
        let not_exists = relation.starts_with('!');
        let relation = relation.trim_start_matches('!').trim();
        let node_type = segments
            .get(idx + 1)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty());
        steps.push(PatternStep {
            node_type: node_type.map(|s| s.to_string()),
            relation: relation.to_string(),
            not_exists,
        });
        idx += 2;
    }
    let mut query = PatternQuery {
        start: start.clone(),
        start_type: start_type.clone(),
        steps,
        max_depth,
    };
    // If no explicit start_type, derive from the first segment.
    if query.start_type.is_none() && query.start.is_none() {
        let first = segments[0].trim();
        if !first.is_empty() && !first.contains(':') {
            query.start_type = Some(first.to_string());
        }
    }
    Ok(query)
}

fn graph_query_text(output: &GraphQueryOutput) -> String {
    if output.matches.is_empty() {
        return "no matches\n".to_string();
    }
    let mut text = String::new();
    for path in &output.matches {
        text.push_str(&format!("{}\n", path.join(" -> ")));
    }
    text
}

fn graph_why_text(output: &GraphWhyOutput) -> String {
    let mut text = format!("entity: {}\nfound: {}\n", output.entity, output.found);
    if let Some(node) = &output.node {
        text.push_str(&format!(
            "kind: {}\nid: {}\ncreated_by: {}\ncontent_hash: {}\noccurred_at: {}\n",
            node.kind, node.id, node.created_by, node.content_hash, node.occurred_at
        ));
    }
    if !output.relations.is_empty() {
        text.push_str("relations:\n");
        for edge in &output.relations {
            text.push_str(&format!(
                "  {} --{}--> {} (event {})\n",
                edge.from, edge.relation, edge.to, edge.event_id
            ));
        }
    }
    text
}

fn graph_rebuild_text(output: &GraphRebuildOutput) -> String {
    format!(
        "stream: {}\nnodes: {}\nedges: {}\nlast_event_sequence: {}\n",
        output.stream, output.nodes, output.edges, output.last_event_sequence
    )
}
