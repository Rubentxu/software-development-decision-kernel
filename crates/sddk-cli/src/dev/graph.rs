//! M8.0 — `sddk dev graph list/edges/projection` subcommand.
//!
//! Surfaces the engine's `active_graph` typed projection in a CLI-inspectable
//! form. Two parallel goals:
//!
//! - **Visibility**: the M8 (H9 Active Graph & Cockpit) engine module
//!   `crates/sddk-engine/src/active_graph.rs` (588 LoC, 16+ tests) is shipped
//!   but was unreachable from the operator. This module is the CLI
//!   counterpart — `sddk dev graph list` shows every projected node and
//!   `sddk dev graph edges` shows every typed relation.
//! - **Honest scope**: when no `ActiveGraphInput` is available (no live
//!   cycle context, no `--from-input` JSON), the projection is empty and
//!   a clear stderr note explains *why* — never fabricated nodes.
//!
//! ## Subcommands
//!
//! - `list`: every node from `ActiveGraphProjection.nodes`, sorted by NodeId,
//!   with kind/label/recorded_at.
//! - `edges`: every typed relation from `ActiveGraphProjection.edges`,
//!   sorted by `(kind, source, target)`.
//! - `projection`: full `ActiveGraphProjection` as JSON (nodes + edges +
//!   roots + counts).
//!
//! ## Input sources
//!
//! - `--from-input <path>`: read an `ActiveGraphInput` JSON fixture.
//! - otherwise: empty `ActiveGraphInput` (still produces a deterministic
//!   empty projection; the `recorded_at` defaults to the current
//!   UTC-RFC3339 timestamp so the projection is reproducible within a
//!   second).
//!
//! The M8.0 surface does **not** auto-derive `ActiveGraphInput` from the
//! current cycle — that bridge requires either Decision Memory integration
//! (M4) or the work-graph builder (M8.2 / SDD-ADAPTIVE-002) plumbing,
//! both of which are scoped to later M8.x cycles. M8.0 ships the
//! inspectable surface first so the operator can confirm the engine
//! behavior end-to-end against fixtures.

use std::path::PathBuf;

use anyhow::Context;
use clap::{Args, Subcommand, ValueEnum};
use sddk_domain::workflow_ir::NodeId;
use sddk_engine::active_graph::{
    ActiveGraphEdge, ActiveGraphEdgeKind, ActiveGraphInput, ActiveGraphNode, ActiveGraphNodeKind,
    ActiveGraphProjection, ActiveGraphProjector, DefaultActiveGraphProjector,
};
use sddk_engine::why_queries::{
    DefaultWhyQueryEngine, WhyCausalStep, WhyQueryEngine, WhyQueryKind, WhyQueryResult,
};
use serde::{Deserialize, Serialize};

use crate::{CliEnvironment, CommandOutput, OutputFormat};

#[derive(Debug, Args)]
pub(crate) struct GraphArgs {
    #[command(subcommand)]
    pub command: GraphCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum GraphCommand {
    /// List every node in the projected active graph (kind + label + recorded_at).
    List(GraphListArgs),
    /// List every typed relation (kind + source → target) in the projected active graph.
    Edges(GraphEdgesArgs),
    /// Emit the full `ActiveGraphProjection` (nodes + edges + roots + counts) as JSON.
    Projection(GraphProjectionArgs),
    /// Run a `why`-family causal query against the projected active graph:
    /// returns the inbound causal closure, summary, and `truncated` flag.
    Why(GraphWhyArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct GraphListArgs {
    /// Filter to a single node kind (workflow, workflow_run, decision_memory, …).
    #[arg(long, value_enum)]
    pub kind: Option<GraphNodeKindArg>,
    /// Read `ActiveGraphInput` from a JSON file at this path.
    /// When omitted, an empty input is used (empty projection).
    #[arg(long)]
    pub from_input: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct GraphEdgesArgs {
    /// Filter to a single edge kind (parent_of, delegated_to, evidence_of,
    /// gates, promotes, references).
    #[arg(long, value_enum)]
    pub kind: Option<GraphEdgeKindArg>,
    /// Read `ActiveGraphInput` from a JSON file at this path.
    #[arg(long)]
    pub from_input: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct GraphProjectionArgs {
    /// Read `ActiveGraphInput` from a JSON file at this path.
    #[arg(long)]
    pub from_input: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct GraphWhyArgs {
    /// Target node id to explain (e.g. `wf-1`, `run:m7-cycle-001`).
    #[arg(long)]
    pub target: String,
    /// Which why-verb to run.
    ///
    /// - `why`: walk the full inbound causal closure (BFS up to
    ///   `WHY_MAX_DEPTH` hops, sorted canonically).
    /// - `debt-why`: filter inbound to `gates` + `evidence_of` edges only.
    /// - `decision-why`: filter inbound to `promotes` + `references` +
    ///   `evidence_of` edges only.
    #[arg(long, value_enum, default_value_t = WhyKindArg::Why)]
    pub kind: WhyKindArg,
    /// Read `ActiveGraphInput` from a JSON file at this path.
    #[arg(long)]
    pub from_input: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum WhyKindArg {
    Why,
    DebtWhy,
    DecisionWhy,
}

impl WhyKindArg {
    fn to_engine(self) -> WhyQueryKind {
        match self {
            Self::Why => WhyQueryKind::Why,
            Self::DebtWhy => WhyQueryKind::DebtWhy,
            Self::DecisionWhy => WhyQueryKind::DecisionWhy,
        }
    }
}

// ── Value enums (clap-compatible subset of engine kinds) ─────────────────

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum GraphNodeKindArg {
    Workflow,
    WorkflowRun,
    DecisionMemory,
    DecisionMemoryRef,
    Assurance,
    Uat,
    WorkflowLab,
    LabPromotion,
    Runbook,
    HumanDecision,
}

impl GraphNodeKindArg {
    fn matches(self, kind: ActiveGraphNodeKind) -> bool {
        matches!(
            (self, kind),
            (Self::Workflow, ActiveGraphNodeKind::Workflow)
                | (Self::WorkflowRun, ActiveGraphNodeKind::WorkflowRun)
                | (Self::DecisionMemory, ActiveGraphNodeKind::DecisionMemory)
                | (
                    Self::DecisionMemoryRef,
                    ActiveGraphNodeKind::DecisionMemoryRef
                )
                | (Self::Assurance, ActiveGraphNodeKind::Assurance)
                | (Self::Uat, ActiveGraphNodeKind::Uat)
                | (Self::WorkflowLab, ActiveGraphNodeKind::WorkflowLab)
                | (Self::LabPromotion, ActiveGraphNodeKind::LabPromotion)
                | (Self::Runbook, ActiveGraphNodeKind::Runbook)
                | (Self::HumanDecision, ActiveGraphNodeKind::HumanDecision)
        )
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum GraphEdgeKindArg {
    ParentOf,
    DelegatedTo,
    EvidenceOf,
    Gates,
    Promotes,
    References,
}

impl GraphEdgeKindArg {
    fn matches(self, kind: ActiveGraphEdgeKind) -> bool {
        matches!(
            (self, kind),
            (Self::ParentOf, ActiveGraphEdgeKind::ParentOf)
                | (Self::DelegatedTo, ActiveGraphEdgeKind::DelegatedTo)
                | (Self::EvidenceOf, ActiveGraphEdgeKind::EvidenceOf)
                | (Self::Gates, ActiveGraphEdgeKind::Gates)
                | (Self::Promotes, ActiveGraphEdgeKind::Promotes)
                | (Self::References, ActiveGraphEdgeKind::References)
        )
    }
}

// ── Output rows ──────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct NodeRow {
    id: String,
    kind: String,
    label: String,
    recorded_at: String,
}

impl NodeRow {
    fn from_node(node: &ActiveGraphNode) -> Self {
        Self {
            id: node.id.0.clone(),
            kind: node.kind.label().to_string(),
            label: node.label.clone(),
            recorded_at: node.recorded_at.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct EdgeRow {
    kind: String,
    source: String,
    target: String,
}

impl EdgeRow {
    fn from_edge(edge: &ActiveGraphEdge) -> Self {
        Self {
            kind: edge.kind.label().to_string(),
            source: edge.source.0.clone(),
            target: edge.target.0.clone(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ProjectionReport {
    nodes: Vec<NodeRow>,
    edges: Vec<EdgeRow>,
    roots: Vec<String>,
    node_count: usize,
    edge_count: usize,
    recorded_at: String,
    source: String,
}

// ── Input loading ────────────────────────────────────────────────────────

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
struct InputFixture {
    #[serde(default)]
    workflow_nodes: Vec<String>,
    #[serde(default)]
    workflow_edges: Vec<[String; 2]>,
    #[serde(default)]
    workflow_run_id: Option<String>,
    #[serde(default)]
    memory_head: Option<String>,
    #[serde(default)]
    memory_refs: Vec<[String; 2]>,
    #[serde(default)]
    assurance_labels: Vec<String>,
    #[serde(default)]
    uat_labels: Vec<String>,
    #[serde(default)]
    workflow_lab_labels: Vec<String>,
    #[serde(default)]
    lab_promotions: Vec<(String, String, bool)>,
    #[serde(default)]
    runbook_labels: Vec<String>,
    #[serde(default)]
    human_decision_labels: Vec<String>,
    #[serde(default)]
    delegations: Vec<[String; 2]>,
    #[serde(default)]
    evidence_links: Vec<[String; 2]>,
}

impl InputFixture {
    fn into_input(self) -> ActiveGraphInput {
        // ActiveGraphInput is `#[non_exhaustive]`; we cannot use struct
        // expression syntax outside the defining crate, so we mutate from
        // `Default::default()` instead. This keeps the field set aligned
        // with whatever the engine adds (a new field becomes `Default`
        // and we never reach into the private layout).
        let mut input = ActiveGraphInput::default();
        input.workflow_nodes = self.workflow_nodes.into_iter().map(NodeId).collect();
        input.workflow_edges = self
            .workflow_edges
            .into_iter()
            .map(|[a, b]| (NodeId(a), NodeId(b)))
            .collect();
        input.workflow_run_id = self.workflow_run_id;
        input.memory_head = self.memory_head;
        input.memory_refs = self
            .memory_refs
            .into_iter()
            .map(|[a, b]| (a, NodeId(b)))
            .collect();
        input.assurance_labels = self.assurance_labels;
        input.uat_labels = self.uat_labels;
        input.workflow_lab_labels = self.workflow_lab_labels;
        input.lab_promotions = self
            .lab_promotions
            .into_iter()
            .map(|(a, b, c)| (a, NodeId(b), c))
            .collect();
        input.runbook_labels = self.runbook_labels;
        input.human_decision_labels = self.human_decision_labels;
        input.delegations = self
            .delegations
            .into_iter()
            .map(|[a, b]| (NodeId(a), NodeId(b)))
            .collect();
        input.evidence_links = self
            .evidence_links
            .into_iter()
            .map(|[a, b]| (a, NodeId(b)))
            .collect();
        input
    }
}

/// Load an `ActiveGraphInput` from `--from-input` or return an empty one.
///
/// Returns `(input, source_description)`:
/// - source_description is `"<path>"` if loaded from file,
///   `"<empty-default>"` otherwise.
fn load_input(from_input: Option<&PathBuf>) -> anyhow::Result<(ActiveGraphInput, String)> {
    let Some(path) = from_input else {
        return Ok((ActiveGraphInput::default(), "<empty-default>".to_string()));
    };
    let bytes = std::fs::read(path)
        .with_context(|| format!("reading ActiveGraphInput from {}", path.display()))?;
    let fixture: InputFixture = serde_json::from_slice(&bytes).with_context(|| {
        format!(
            "parsing ActiveGraphInput JSON at {} (expected object with optional workflow_nodes/edges/memory_head/etc.)",
            path.display()
        )
    })?;
    Ok((fixture.into_input(), path.display().to_string()))
}

/// Deterministic recorded_at: now-UTC truncated to seconds, RFC-3339.
fn now_rfc3339_seconds() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Minimal RFC-3339 UTC formatter (yyyy-mm-ddThh:mm:ssZ) without
    // pulling in chrono. Same shape engine fixtures use.
    let secs_per_day = 86_400u64;
    let secs_per_hour = 3_600u64;
    let secs_per_min = 60u64;
    let days = now / secs_per_day;
    let rem = now % secs_per_day;
    let hh = rem / secs_per_hour;
    let rem = rem % secs_per_hour;
    let mm = rem / secs_per_min;
    let ss = rem % secs_per_min;

    // Days → date. Algorithm from Howard Hinnant's `civil_from_days`.
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let year = if m <= 2 { y + 1 } else { y };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, m, d, hh, mm, ss
    )
}

fn project(input: ActiveGraphInput) -> ActiveGraphProjection {
    DefaultActiveGraphProjector.project(&input, &now_rfc3339_seconds())
}

// ── Runners ──────────────────────────────────────────────────────────────

pub(crate) fn run_dev_graph_list(args: GraphListArgs, _env: &CliEnvironment) -> CommandOutput {
    let (input, source) = match load_input(args.from_input.as_ref()) {
        Ok(pair) => pair,
        Err(error) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("graph list: {error}"),
            };
        }
    };
    let projection = project(input);
    let nodes: Vec<NodeRow> = projection
        .nodes
        .values()
        .filter(|node| match args.kind {
            None => true,
            Some(k) => k.matches(node.kind),
        })
        .map(NodeRow::from_node)
        .collect();

    match args.format {
        OutputFormat::Text => {
            let mut out = String::new();
            out.push_str("=== sddk dev graph list ===\n\n");
            out.push_str(&format!(
                "node_count: {} (after filter: {})\nroots:      {}\nsource:     {}\nrecorded:   {}\n\n",
                projection.node_count,
                nodes.len(),
                projection.roots.len(),
                source,
                projection
                    .nodes
                    .values()
                    .next()
                    .map(|n| n.recorded_at.as_str())
                    .unwrap_or(""),
            ));
            out.push_str("kind                  id                                       label\n");
            out.push_str("───────────────────── ──────────────────────────────────────── ─────────────────────────\n");
            for row in &nodes {
                out.push_str(&format!("{:<21} {:<41} {}\n", row.kind, row.id, row.label));
            }
            if nodes.is_empty() {
                out.push_str(
                    "(no nodes projected — empty input or no matches for --kind filter)\n",
                );
            }
            CommandOutput {
                status: 0,
                stdout: out,
                stderr: String::new(),
            }
        }
        OutputFormat::Json => match serde_json::to_string_pretty(&nodes) {
            Ok(json) => CommandOutput {
                status: 0,
                stdout: format!("{json}\n"),
                stderr: String::new(),
            },
            Err(error) => CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("graph list: failed to serialize: {error}"),
            },
        },
    }
}

pub(crate) fn run_dev_graph_edges(args: GraphEdgesArgs, _env: &CliEnvironment) -> CommandOutput {
    let (input, source) = match load_input(args.from_input.as_ref()) {
        Ok(pair) => pair,
        Err(error) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("graph edges: {error}"),
            };
        }
    };
    let projection = project(input);
    let edges: Vec<EdgeRow> = projection
        .edges
        .iter()
        .filter(|edge| match args.kind {
            None => true,
            Some(k) => k.matches(edge.kind),
        })
        .map(EdgeRow::from_edge)
        .collect();

    match args.format {
        OutputFormat::Text => {
            let mut out = String::new();
            out.push_str("=== sddk dev graph edges ===\n\n");
            out.push_str(&format!(
                "edge_count: {} (after filter: {})\nsource:     {}\n\n",
                projection.edge_count,
                edges.len(),
                source,
            ));
            out.push_str("kind            source                                     → target\n");
            out.push_str("─────────────── ─────────────────────────────────────────── ───────────────────────────────────────────\n");
            for row in &edges {
                out.push_str(&format!(
                    "{:<15} {:<44} → {}\n",
                    row.kind, row.source, row.target
                ));
            }
            if edges.is_empty() {
                out.push_str(
                    "(no edges projected — empty input or no matches for --kind filter)\n",
                );
            }
            CommandOutput {
                status: 0,
                stdout: out,
                stderr: String::new(),
            }
        }
        OutputFormat::Json => match serde_json::to_string_pretty(&edges) {
            Ok(json) => CommandOutput {
                status: 0,
                stdout: format!("{json}\n"),
                stderr: String::new(),
            },
            Err(error) => CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("graph edges: failed to serialize: {error}"),
            },
        },
    }
}

pub(crate) fn run_dev_graph_projection(
    args: GraphProjectionArgs,
    _env: &CliEnvironment,
) -> CommandOutput {
    let (input, source) = match load_input(args.from_input.as_ref()) {
        Ok(pair) => pair,
        Err(error) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("graph projection: {error}"),
            };
        }
    };
    let projection = project(input);
    let report = ProjectionReport {
        nodes: projection.nodes.values().map(NodeRow::from_node).collect(),
        edges: projection.edges.iter().map(EdgeRow::from_edge).collect(),
        roots: projection.roots.iter().map(|n| n.0.clone()).collect(),
        node_count: projection.node_count,
        edge_count: projection.edge_count,
        recorded_at: projection
            .nodes
            .values()
            .next()
            .map(|n| n.recorded_at.clone())
            .unwrap_or_default(),
        source,
    };

    match args.format {
        OutputFormat::Text => {
            // Text format mirrors the JSON shape in human-readable form so
            // operators get an audit-friendly view without jq.
            let mut out = String::new();
            out.push_str("=== sddk dev graph projection ===\n\n");
            out.push_str(&format!("node_count: {}\n", report.node_count));
            out.push_str(&format!("edge_count: {}\n", report.edge_count));
            out.push_str(&format!("roots:      {}\n", report.roots.len()));
            out.push_str(&format!("recorded:   {}\n", report.recorded_at));
            out.push_str(&format!("source:     {}\n\n", report.source));
            out.push_str("nodes:\n");
            for n in &report.nodes {
                out.push_str(&format!("  - {} [{}] {}\n", n.id, n.kind, n.label));
            }
            out.push_str("\nedges:\n");
            for e in &report.edges {
                out.push_str(&format!("  - {} {} → {}\n", e.kind, e.source, e.target));
            }
            out.push_str("\nroots:\n");
            for r in &report.roots {
                out.push_str(&format!("  - {r}\n"));
            }
            CommandOutput {
                status: 0,
                stdout: out,
                stderr: String::new(),
            }
        }
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => CommandOutput {
                status: 0,
                stdout: format!("{json}\n"),
                stderr: String::new(),
            },
            Err(error) => CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("graph projection: failed to serialize: {error}"),
            },
        },
    }
}

// ── Why queries ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct WhyStepRow {
    kind: String,
    /// For a `node` step this is the node id; for an `edge` step this is
    /// `source→target` (for human display only — prefer `source`/`target`).
    id: String,
    /// Set for `node` steps: the closed-set node kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    node_kind: Option<String>,
    /// Set for `edge` steps: the closed-set edge kind.
    #[serde(skip_serializing_if = "Option::is_none")]
    edge_kind: Option<String>,
    /// Set for `edge` steps.
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    /// Set for `edge` steps.
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
}

impl WhyStepRow {
    fn from_step(step: &WhyCausalStep) -> Self {
        match step {
            WhyCausalStep::Node { id, kind, .. } => Self {
                kind: "node".to_string(),
                id: id.0.clone(),
                node_kind: Some(kind.label().to_string()),
                edge_kind: None,
                source: None,
                target: None,
            },
            WhyCausalStep::Edge {
                kind,
                source,
                target,
            } => Self {
                kind: "edge".to_string(),
                id: format!("{}→{}", source.0, target.0),
                node_kind: None,
                edge_kind: Some(kind.label().to_string()),
                source: Some(source.0.clone()),
                target: Some(target.0.clone()),
            },
            // `WhyCausalStep` is `#[non_exhaustive]`; future variants
            // (e.g. Truncated marker) will land here. The fallback
            // preserves round-tripping via the textual `id` only.
            _ => Self {
                kind: "unknown".to_string(),
                id: format!("{step:?}"),
                node_kind: None,
                edge_kind: None,
                source: None,
                target: None,
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct WhyReport {
    kind: String,
    target: String,
    matched: bool,
    truncated: bool,
    summary: String,
    causal_path: Vec<WhyStepRow>,
    step_count: usize,
    generated_at: String,
    source: String,
}

pub(crate) fn run_dev_graph_why(args: GraphWhyArgs, _env: &CliEnvironment) -> CommandOutput {
    let (input, source) = match load_input(args.from_input.as_ref()) {
        Ok(pair) => pair,
        Err(error) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("graph why: {error}"),
            };
        }
    };
    let projection = project(input);
    let target = NodeId(args.target.clone());
    let engine = DefaultWhyQueryEngine;
    let result: WhyQueryResult = engine.query(
        &projection,
        &target,
        args.kind.to_engine(),
        &now_rfc3339_seconds(),
    );
    let step_rows: Vec<WhyStepRow> = result
        .causal_path
        .iter()
        .map(WhyStepRow::from_step)
        .collect();
    let step_count = step_rows.len();

    match args.format {
        OutputFormat::Text => render_why_text(&result, &source, step_rows),
        OutputFormat::Json => render_why_json(&result, &source, step_rows, step_count),
    }
}

fn render_why_text(
    result: &WhyQueryResult,
    source: &str,
    step_rows: Vec<WhyStepRow>,
) -> CommandOutput {
    let mut out = String::new();
    out.push_str("=== sddk dev graph why ===\n\n");
    out.push_str(&format!("kind:        {}\n", result.kind.label()));
    out.push_str(&format!("target:      {}\n", result.target.0));
    out.push_str(&format!("matched:     {}\n", result.matched));
    out.push_str(&format!("truncated:   {}\n", result.truncated));
    out.push_str(&format!("generated:   {}\n", result.generated_at));
    out.push_str(&format!("source:      {source}\n"));
    out.push_str(&format!("step_count:  {}\n", result.causal_path.len()));
    out.push('\n');
    if !result.matched {
        out.push_str(
            "(target not found in projection — run `sddk dev graph list` to inspect nodes)\n",
        );
    } else if result.causal_path.is_empty() {
        out.push_str("(no causal predecessors within kind scope — target is a root)\n");
    } else {
        out.push_str("causal_path:\n");
        for step in &step_rows {
            match step.kind.as_str() {
                "node" => {
                    out.push_str(&format!("  node    {}\n", step.id));
                }
                "edge" => {
                    out.push_str(&format!(
                        "  edge    {} --{}--> {}\n",
                        step.source.as_deref().unwrap_or("?"),
                        step.edge_kind.as_deref().unwrap_or("?"),
                        step.target.as_deref().unwrap_or("?"),
                    ));
                }
                _ => {
                    out.push_str(&format!("  ?       {}\n", step.id));
                }
            }
        }
    }
    out.push('\n');
    out.push_str(&format!("summary: {}\n", result.summary));
    CommandOutput {
        status: 0,
        stdout: out,
        stderr: String::new(),
    }
}

fn render_why_json(
    result: &WhyQueryResult,
    source: &str,
    step_rows: Vec<WhyStepRow>,
    step_count: usize,
) -> CommandOutput {
    let report = WhyReport {
        kind: result.kind.label().to_string(),
        target: result.target.0.clone(),
        matched: result.matched,
        truncated: result.truncated,
        summary: result.summary.clone(),
        causal_path: step_rows,
        step_count,
        generated_at: result.generated_at.clone(),
        source: source.to_string(),
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => CommandOutput {
            status: 0,
            stdout: format!("{json}\n"),
            stderr: String::new(),
        },
        Err(error) => CommandOutput {
            status: 1,
            stdout: String::new(),
            stderr: format!("graph why: failed to serialize: {error}"),
        },
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_two_nodes() -> ActiveGraphInput {
        let mut input = ActiveGraphInput::default();
        input.workflow_nodes = vec![
            NodeId("wf-root".to_string()),
            NodeId("wf-child".to_string()),
        ];
        input.workflow_edges = vec![(
            NodeId("wf-root".to_string()),
            NodeId("wf-child".to_string()),
        )];
        input
    }

    #[test]
    fn list_returns_empty_for_default_input() {
        let projection = project(ActiveGraphInput::default());
        assert_eq!(projection.node_count, 0);
        assert_eq!(projection.edge_count, 0);
        assert!(projection.nodes.is_empty());
        assert!(projection.edges.is_empty());
        assert!(projection.roots.is_empty());
    }

    #[test]
    fn list_includes_workflow_nodes_and_parent_edge() {
        let projection = project(fixture_two_nodes());
        assert_eq!(projection.node_count, 2);
        assert_eq!(projection.edge_count, 1);
        let kinds: Vec<_> = projection
            .nodes
            .values()
            .map(|n| n.kind)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect();
        assert_eq!(kinds, vec![ActiveGraphNodeKind::Workflow]);
        assert_eq!(projection.edges[0].kind, ActiveGraphEdgeKind::ParentOf);
        assert!(projection.roots.contains(&NodeId("wf-root".to_string())));
        assert!(projection.roots.contains(&NodeId("wf-child".to_string())));
    }

    #[test]
    fn load_input_from_json_fixture_returns_equivalent_input() {
        let json = r#"{
            "workflow_nodes": ["a", "b"],
            "workflow_edges": [["a", "b"]],
            "workflow_run_id": "run-1",
            "memory_head": "head-1",
            "uat_labels": ["scn-1"]
        }"#;
        let dir = std::env::temp_dir().join(format!(
            "sddk-graph-fixture-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fixture.json");
        std::fs::write(&path, json).unwrap();

        let (input, source) = load_input(Some(&path)).unwrap();
        assert_eq!(source, path.display().to_string());
        assert_eq!(input.workflow_nodes.len(), 2);
        assert_eq!(input.workflow_edges.len(), 1);
        assert_eq!(input.workflow_run_id.as_deref(), Some("run-1"));
        assert_eq!(input.memory_head.as_deref(), Some("head-1"));
        assert_eq!(input.uat_labels, vec!["scn-1".to_string()]);

        let projection = project(input);
        // 2 workflow + 1 run + 1 memory + 1 uat = 5 nodes total.
        assert_eq!(projection.node_count, 5);
    }

    #[test]
    fn load_input_falls_back_to_empty_when_no_path_given() {
        let (input, source) = load_input(None).unwrap();
        assert!(input.workflow_nodes.is_empty());
        assert_eq!(source, "<empty-default>");
    }

    #[test]
    fn load_input_errors_when_json_is_invalid() {
        let dir = std::env::temp_dir().join(format!(
            "sddk-graph-bad-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("bad.json");
        std::fs::write(&path, b"not-valid-json").unwrap();

        let result = load_input(Some(&path));
        assert!(result.is_err());
    }

    #[test]
    fn projection_report_round_trips_via_serde() {
        let input = fixture_two_nodes();
        let projection = project(input);
        let report = ProjectionReport {
            nodes: projection.nodes.values().map(NodeRow::from_node).collect(),
            edges: projection.edges.iter().map(EdgeRow::from_edge).collect(),
            roots: projection.roots.iter().map(|n| n.0.clone()).collect(),
            node_count: projection.node_count,
            edge_count: projection.edge_count,
            recorded_at: "1970-01-01T00:00:00Z".to_string(),
            source: "<empty-default>".to_string(),
        };
        let json = serde_json::to_string(&report).unwrap();
        let back: ProjectionReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.node_count, 2);
        assert_eq!(back.edge_count, 1);
        assert_eq!(back.nodes.len(), 2);
        assert_eq!(back.edges.len(), 1);
        assert_eq!(back.roots.len(), 2);
    }

    #[test]
    fn node_kind_filter_matches_canonical_subtypes() {
        assert!(GraphNodeKindArg::Workflow.matches(ActiveGraphNodeKind::Workflow));
        assert!(GraphNodeKindArg::WorkflowRun.matches(ActiveGraphNodeKind::WorkflowRun));
        assert!(GraphNodeKindArg::DecisionMemory.matches(ActiveGraphNodeKind::DecisionMemory));
        assert!(!GraphNodeKindArg::Workflow.matches(ActiveGraphNodeKind::Uat));
        assert!(!GraphNodeKindArg::Uat.matches(ActiveGraphNodeKind::Assurance));
    }

    #[test]
    fn edge_kind_filter_matches_canonical_subtypes() {
        assert!(GraphEdgeKindArg::ParentOf.matches(ActiveGraphEdgeKind::ParentOf));
        assert!(GraphEdgeKindArg::DelegatedTo.matches(ActiveGraphEdgeKind::DelegatedTo));
        assert!(!GraphEdgeKindArg::Promotes.matches(ActiveGraphEdgeKind::Gates));
        assert!(!GraphEdgeKindArg::References.matches(ActiveGraphEdgeKind::EvidenceOf));
    }

    #[test]
    fn now_rfc3339_is_well_formed_utc() {
        let s = now_rfc3339_seconds();
        // Format: YYYY-MM-DDTHH:MM:SSZ
        assert_eq!(s.len(), 20, "expected 20-char RFC-3339 UTC, got {s:?}");
        let parts: Vec<&str> = s.split('T').collect();
        assert_eq!(parts.len(), 2);
        let date_parts: Vec<&str> = parts[0].split('-').collect();
        assert_eq!(date_parts.len(), 3);
        assert_eq!(parts[1].len(), 9); // HH:MM:SSZ
        assert!(parts[1].ends_with('Z'));
    }

    // ── M8.1 why-query tests ─────────────────────────────────────────

    fn fixture_with_evidence_and_promotion() -> ActiveGraphInput {
        let mut input = ActiveGraphInput::default();
        input.workflow_nodes = vec![
            NodeId("wf-apply".to_string()),
            NodeId("wf-spec".to_string()),
        ];
        input.workflow_edges = vec![(
            NodeId("wf-spec".to_string()),
            NodeId("wf-apply".to_string()),
        )];
        input.evidence_links = vec![(
            "verifier-passed".to_string(),
            NodeId("wf-apply".to_string()),
        )];
        input.lab_promotions = vec![("gate-1".to_string(), NodeId("wf-apply".to_string()), true)];
        input
    }

    #[test]
    fn why_kind_arg_to_engine_maps_all_three_variants() {
        assert!(matches!(
            WhyKindArg::Why.to_engine(),
            sddk_engine::why_queries::WhyQueryKind::Why
        ));
        assert!(matches!(
            WhyKindArg::DebtWhy.to_engine(),
            sddk_engine::why_queries::WhyQueryKind::DebtWhy
        ));
        assert!(matches!(
            WhyKindArg::DecisionWhy.to_engine(),
            sddk_engine::why_queries::WhyQueryKind::DecisionWhy
        ));
    }

    #[test]
    fn why_query_engine_walks_full_inbound_closure() {
        let projection = project(fixture_with_evidence_and_promotion());
        let engine = DefaultWhyQueryEngine;
        let result = engine.query(
            &projection,
            &NodeId("wf-apply".to_string()),
            sddk_engine::why_queries::WhyQueryKind::Why,
            "t0",
        );
        assert!(result.matched);
        assert!(!result.truncated);
        assert!(!result.causal_path.is_empty());
        // `why` walks all inbound edges: parent_of + evidence_of + promotes
        let edge_kinds: std::collections::BTreeSet<String> = result
            .causal_path
            .iter()
            .filter_map(|step| match step {
                WhyCausalStep::Edge { kind, .. } => Some(kind.label().to_string()),
                _ => None,
            })
            .collect();
        assert!(edge_kinds.contains("parent_of"));
        assert!(edge_kinds.contains("evidence_of"));
        assert!(edge_kinds.contains("promotes"));
    }

    #[test]
    fn why_query_engine_debt_why_filters_to_evidence_and_gates_only() {
        let projection = project(fixture_with_evidence_and_promotion());
        let engine = DefaultWhyQueryEngine;
        let result = engine.query(
            &projection,
            &NodeId("wf-apply".to_string()),
            sddk_engine::why_queries::WhyQueryKind::DebtWhy,
            "t0",
        );
        let edge_kinds: std::collections::BTreeSet<String> = result
            .causal_path
            .iter()
            .filter_map(|step| match step {
                WhyCausalStep::Edge { kind, .. } => Some(kind.label().to_string()),
                _ => None,
            })
            .collect();
        assert!(edge_kinds.contains("evidence_of"));
        assert!(!edge_kinds.contains("parent_of"));
        assert!(!edge_kinds.contains("promotes"));
    }

    #[test]
    fn why_query_engine_decision_why_includes_evidence_references_promotes() {
        let projection = project(fixture_with_evidence_and_promotion());
        let engine = DefaultWhyQueryEngine;
        let result = engine.query(
            &projection,
            &NodeId("wf-apply".to_string()),
            sddk_engine::why_queries::WhyQueryKind::DecisionWhy,
            "t0",
        );
        let edge_kinds: std::collections::BTreeSet<String> = result
            .causal_path
            .iter()
            .filter_map(|step| match step {
                WhyCausalStep::Edge { kind, .. } => Some(kind.label().to_string()),
                _ => None,
            })
            .collect();
        assert!(edge_kinds.contains("evidence_of"));
        assert!(edge_kinds.contains("promotes"));
        assert!(!edge_kinds.contains("parent_of"));
    }

    #[test]
    fn why_query_returns_unknown_target_for_missing_node() {
        let projection = project(ActiveGraphInput::default());
        let engine = DefaultWhyQueryEngine;
        let result = engine.query(
            &projection,
            &NodeId("ghost".to_string()),
            sddk_engine::why_queries::WhyQueryKind::Why,
            "t0",
        );
        assert!(!result.matched);
        assert!(result.causal_path.is_empty());
        assert_eq!(result.summary, "unknown target");
    }

    #[test]
    fn why_step_row_serializes_node_with_node_kind_and_skips_edge_fields() {
        let step = WhyCausalStep::Node {
            id: NodeId("wf-1".to_string()),
            kind: ActiveGraphNodeKind::Workflow,
            label: "workflow:wf-1".to_string(),
        };
        let row = WhyStepRow::from_step(&step);
        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("\"kind\":\"node\""));
        assert!(json.contains("\"node_kind\":\"workflow\""));
        assert!(!json.contains("\"edge_kind\""));
        assert!(!json.contains("\"source\""));
        assert!(!json.contains("\"target\""));
    }

    #[test]
    fn why_step_row_serializes_edge_with_source_and_target_skips_node_kind() {
        let step = WhyCausalStep::Edge {
            kind: ActiveGraphEdgeKind::ParentOf,
            source: NodeId("a".to_string()),
            target: NodeId("b".to_string()),
        };
        let row = WhyStepRow::from_step(&step);
        let json = serde_json::to_string(&row).unwrap();
        assert!(json.contains("\"kind\":\"edge\""));
        assert!(json.contains("\"edge_kind\":\"parent_of\""));
        assert!(json.contains("\"source\":\"a\""));
        assert!(json.contains("\"target\":\"b\""));
        assert!(!json.contains("\"node_kind\""));
    }

    #[test]
    fn why_report_round_trips_via_serde() {
        let report = WhyReport {
            kind: "why".to_string(),
            target: "wf-1".to_string(),
            matched: true,
            truncated: false,
            summary: "why wf-1: 2 node(s), 1 edge(s)".to_string(),
            causal_path: vec![WhyStepRow {
                kind: "node".to_string(),
                id: "wf-1".to_string(),
                node_kind: Some("workflow".to_string()),
                edge_kind: None,
                source: None,
                target: None,
            }],
            step_count: 1,
            generated_at: "2026-09-10T00:00:00Z".to_string(),
            source: "<empty-default>".to_string(),
        };
        let json = serde_json::to_string(&report).unwrap();
        let back: WhyReport = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind, "why");
        assert_eq!(back.target, "wf-1");
        assert!(back.matched);
        assert_eq!(back.step_count, 1);
    }
}
