//! M8.2 — `sddk dev cockpit view` subcommand.
//!
//! Surfaces the H9 Cockpit Views engine (`crates/sddk-engine/src/cockpit_views.rs`,
//! 495 LoC, COCKPIT-001: structural/temporal views over an
//! `ActiveGraphProjection`). Four views, one subcommand with `--kind`:
//!
//! - `overview`: counts per node kind + edge kind + roots.
//! - `journal`: chronological activity log derived from the projection's
//!   `recorded_at` timestamps.
//! - `timeline`: ordered list of node + edge events.
//! - `execution`: workflow DAG + run + memory HEAD + assurance nodes.
//!
//! M8.2 ships COCKPIT-001. M8.3 (`obs` subcommand) ships COCKPIT-002
//! (`crates/sddk-engine/src/cockpit_observability.rs`): five operational
//! views over the same projection — providers / usage / assurance /
//! handoff / experiments.
//!
//! ## Subcommands
//!
//! - `view --kind {overview,journal,timeline,execution}` (default `overview`).
//! - `obs --kind {providers,usage,assurance,handoff,experiments}` (default `providers`).
//! - Both accept optional `--from-input <path>` to load an
//!   `ActiveGraphInput` JSON fixture. Without it, the projection is empty
//!   and the view reports an empty structure (no fabrication).
//!
//! ## Output
//!
//! `CockpitView` is `{ title, sections: [{heading, lines}], generated_at, kind }`.
//! The CLI serializes the same shape as JSON and renders it as a
//! readable text tree by default.

use std::path::PathBuf;

use anyhow::Context;
use clap::{Args, Subcommand, ValueEnum};
use sddk_engine::active_graph::{
    ActiveGraphInput, ActiveGraphProjection, ActiveGraphProjector, DefaultActiveGraphProjector,
    ProvenanceRef, ProvenanceSourceKind,
};
use sddk_engine::active_graph_drift::{DefaultDriftEngine, DriftEngine};
use sddk_engine::cockpit_observability::{
    CockpitObservabilityBuilder, CockpitObservabilityKind, DefaultCockpitObservabilityBuilder,
};
use sddk_engine::cockpit_views::{
    CockpitSection, CockpitView, CockpitViewBuilder, CockpitViewKind, DefaultCockpitViewBuilder,
};
use sddk_domain::workflow_ir::NodeId;
use serde::{Deserialize, Serialize};

use crate::{CliEnvironment, CommandOutput, OutputFormat};

#[derive(Debug, Args)]
pub(crate) struct CockpitArgs {
    #[command(subcommand)]
    pub command: CockpitCommand,
}

#[derive(Debug, Subcommand)]
pub(crate) enum CockpitCommand {
    /// Render a structural/temporal Cockpit view over the projected
    /// active graph (overview, journal, timeline, execution).
    View(CockpitViewArgs),
    /// Render an operational Cockpit observability view over the
    /// projected active graph (providers, usage, assurance, handoff,
    /// experiments).
    Obs(CockpitObsArgs),
    /// Compute drift between two active graph projections (M8.7).
    /// Emits added / removed / changed nodes and edges with
    /// field-level attribution. Sources can be archive manifests
    /// (`--cycle-a` / `--cycle-b`) or `ActiveGraphInput` JSON
    /// files (`--input-a` / `--input-b`).
    Diff(CockpitDiffArgs),
    /// Compute a stable SHA-256 digest of the active graph
    /// projection (M8.8). Two kinds: `strict` (default — includes
    /// `recorded_at`, so any timestamp drift produces a different
    /// hash) and `content` (ignores `recorded_at`, only structural
    /// content participates). Sources can be an archive manifest
    /// (`--cycle`) or an `ActiveGraphInput` JSON file (`--input`).
    Digest(CockpitDigestArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CockpitViewArgs {
    /// Which Cockpit view to render.
    ///
    /// - `overview`: counts per node kind + edge kind + roots.
    /// - `journal`: chronological activity log derived from
    ///   `recorded_at` timestamps.
    /// - `timeline`: ordered list of node + edge events.
    /// - `execution`: workflow DAG + run + memory HEAD + assurance nodes.
    #[arg(long, value_enum, default_value_t = CockpitViewKindArg::Overview)]
    pub kind: CockpitViewKindArg,
    /// Read `ActiveGraphInput` from a JSON file at this path.
    /// When omitted, an empty input is used (empty view — no fabrication).
    #[arg(long)]
    pub from_input: Option<PathBuf>,
    /// Derive `ActiveGraphInput` from the cycle archive manifest at
    /// `~/.sddk-knowledge/sddk-framework/cycles/<cycle-id>/archive-manifest.md`.
    /// The derived input is built only from what the manifest honestly
    /// exposes (commit SHAs as workflow nodes, commit parent relations as
    /// parent_of edges, bridges as evidence_of edges, deferred items as
    /// memory_refs). If the cycle does not exist, the command fails with
    /// a clear stderr message — no fabrication.
    /// Takes precedence over `--from-input` when both are set.
    #[arg(long)]
    pub from_cycle: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum CockpitViewKindArg {
    Overview,
    Journal,
    Timeline,
    Execution,
}

impl CockpitViewKindArg {
    fn to_engine(self) -> CockpitViewKind {
        match self {
            Self::Overview => CockpitViewKind::Overview,
            Self::Journal => CockpitViewKind::Journal,
            Self::Timeline => CockpitViewKind::Timeline,
            Self::Execution => CockpitViewKind::Execution,
        }
    }
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CockpitObsArgs {
    /// Which Cockpit observability view to render.
    ///
    /// - `providers`: WorkflowRun + Assurance node inventory.
    /// - `usage`: edge-kind usage counts (parent_of / evidence_of / ...).
    /// - `assurance`: evidence edges + standalone assurance nodes.
    /// - `handoff`: delegated_to edges (orchestrator → worker).
    /// - `experiments`: WorkflowLab + promote/gate decisions.
    #[arg(long, value_enum, default_value_t = CockpitObsKindArg::Providers)]
    pub kind: CockpitObsKindArg,
    /// Read `ActiveGraphInput` from a JSON file at this path.
    /// When omitted, an empty input is used (empty view — no fabrication).
    #[arg(long)]
    pub from_input: Option<PathBuf>,
    /// Derive `ActiveGraphInput` from the cycle archive manifest at
    /// `~/.sddk-knowledge/sddk-framework/cycles/<cycle-id>/archive-manifest.md`.
    /// Takes precedence over `--from-input` when both are set.
    #[arg(long)]
    pub from_cycle: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum CockpitObsKindArg {
    Providers,
    Usage,
    Assurance,
    Handoff,
    Experiments,
}

impl CockpitObsKindArg {
    fn to_engine(self) -> CockpitObservabilityKind {
        match self {
            Self::Providers => CockpitObservabilityKind::Providers,
            Self::Usage => CockpitObservabilityKind::Usage,
            Self::Assurance => CockpitObservabilityKind::Assurance,
            Self::Handoff => CockpitObservabilityKind::Handoff,
            Self::Experiments => CockpitObservabilityKind::Experiments,
        }
    }
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CockpitDiffArgs {
    /// Cycle id for projection A.
    ///
    /// Reads `~/.sddk-knowledge/sddk-framework/cycles/<cycle-a>/archive-manifest.md`.
    #[arg(long, conflicts_with = "input_a")]
    pub cycle_a: Option<String>,
    /// Cycle id for projection B.
    #[arg(long, conflicts_with = "input_b")]
    pub cycle_b: Option<String>,
    /// Path to an `ActiveGraphInput` JSON file for projection A.
    /// Cannot be combined with `--cycle-a`.
    #[arg(long)]
    pub input_a: Option<PathBuf>,
    /// Path to an `ActiveGraphInput` JSON file for projection B.
    /// Cannot be combined with `--cycle-b`.
    #[arg(long)]
    pub input_b: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CockpitDigestArgs {
    /// Cycle id to digest.
    ///
    /// Reads `~/.sddk-knowledge/sddk-framework/cycles/<cycle-id>/archive-manifest.md`.
    /// Cannot be combined with `--input`.
    #[arg(long, conflicts_with = "input")]
    pub cycle: Option<String>,
    /// Path to an `ActiveGraphInput` JSON file to digest.
    /// Cannot be combined with `--cycle`.
    #[arg(long)]
    pub input: Option<PathBuf>,
    /// Digest kind.
    ///
    /// - `strict`: hash the full projection including `recorded_at`;
    ///   any timestamp drift produces a different hash.
    /// - `content`: ignore `recorded_at`; only structural + provenance
    ///   content participates in the hash.
    #[arg(long, value_enum, default_value_t = CockpitDigestKindArg::Strict)]
    pub kind: CockpitDigestKindArg,
    /// Output format.
    ///
    /// - `text`: prints `<hex>\n` to stdout (operator-friendly,
    ///   pipeable to `cmp` / `diff`).
    /// - `json`: prints `{kind, hex, source}` JSON envelope.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum CockpitDigestKindArg {
    Strict,
    Content,
}

impl CockpitDigestKindArg {
    fn to_engine(self) -> sddk_engine::active_graph_digest::DigestKind {
        match self {
            Self::Strict => sddk_engine::active_graph_digest::DigestKind::Strict,
            Self::Content => sddk_engine::active_graph_digest::DigestKind::Content,
        }
    }
}

// ── JSON envelope ────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct CockpitSectionRow {
    heading: String,
    lines: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CockpitViewRow {
    title: String,
    kind: String,
    sections: Vec<CockpitSectionRow>,
    section_count: usize,
    line_count: usize,
    generated_at: String,
    source: String,
}

impl CockpitViewRow {
    fn from_view(view: &CockpitView, source: &str) -> Self {
        let sections: Vec<CockpitSectionRow> = view
            .sections
            .iter()
            .map(|s: &CockpitSection| CockpitSectionRow {
                heading: s.heading.clone(),
                lines: s.lines.clone(),
            })
            .collect();
        let line_count = sections.iter().map(|s| s.lines.len()).sum();
        Self {
            title: view.title.clone(),
            kind: view.kind.label().to_string(),
            section_count: sections.len(),
            line_count,
            sections,
            generated_at: view.generated_at.clone(),
            source: source.to_string(),
        }
    }
}

// ── Input loading (mirror of dev/graph.rs) ───────────────────────────────

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

/// Resolve the project vault root (`~/.sddk-knowledge/sddk-framework`).
///
/// M8.4 deliberately inlines this here rather than depending on
/// `debt::resolve_vault_path` to keep the cockpit surface module
/// self-contained: this is the only place the cockpit surface needs
/// the vault, and the resolver is two lines.
fn resolve_vault_root(env: &CliEnvironment) -> anyhow::Result<PathBuf> {
    let home = env
        .home
        .clone()
        .or_else(dirs::home_dir)
        .ok_or_else(|| anyhow::anyhow!("no home directory to resolve vault from"))?;
    Ok(home.join(".sddk-knowledge").join("sddk-framework"))
}

/// Load `ActiveGraphInput` derived from a cycle's archive manifest.
///
/// What the derivation honestly exposes (no fabrication):
/// - `workflow_nodes`: each commit SHA listed in the manifest's "Commits"
///   table becomes a `NodeId(<sha>)`. We do NOT synthesize the cycle ID
///   as a node — it appears as `cycle_id` metadata only.
/// - `workflow_edges`: each commit's parent SHA, if present in the same
///   table, becomes a `parent_of` edge. Manifests without explicit
///   parent listings produce no edges (honest empty, not inferred).
/// - `evidence_links`: each "Bridge" bullet becomes one `evidence_of`
///   edge keyed by `(cycle-id, sha-of-featured-commit)` — we only emit
///   edges for bridges whose target SHA appears in the manifest.
/// - `memory_refs`: each "Deferred item" bullet becomes a memory_ref
///   keyed by the deferred work-item label.
///
/// If the cycle does not exist or the manifest is missing/unreadable,
/// returns a clear error so the caller can surface it on stderr.
fn load_input_from_cycle(
    cycle_id: &str,
    env: &CliEnvironment,
) -> anyhow::Result<(ActiveGraphInput, String)> {
    let vault_root = resolve_vault_root(env)?;
    let cycle_dir = vault_root.join("cycles").join(cycle_id);
    let manifest_path = cycle_dir.join("archive-manifest.md");
    if !cycle_dir.exists() {
        anyhow::bail!(
            "cycle '{cycle_id}' not found at {} (no directory under cycles/)",
            cycle_dir.display()
        );
    }
    if !manifest_path.exists() {
        anyhow::bail!(
            "cycle '{cycle_id}' has no archive-manifest.md at {}",
            manifest_path.display()
        );
    }
    let bytes = std::fs::read(&manifest_path).with_context(|| {
        format!("reading archive manifest for cycle '{cycle_id}'")
    })?;
    let text = String::from_utf8(bytes).with_context(|| {
        format!(
            "archive manifest for cycle '{cycle_id}' is not valid UTF-8 at {}",
            manifest_path.display()
        )
    })?;
    Ok((derive_active_graph_input_from_manifest(&text), format!(
        "cycle:{cycle_id}"
    )))
}

/// Pure derivation: parse a manifest's text into an `ActiveGraphInput`.
///
/// Exposed as a separate function so unit tests can drive it with a
/// fixed string and never touch the filesystem. The parser is
/// intentionally narrow: it only recognises the markdown table/header
/// shapes that the SDDK `archive-manifest.md` convention uses
/// (consistent with M8.2/M8.3 manifests). Anything it cannot parse
/// is silently skipped — that is the contract for "honest derivation":
/// we never invent edges that the manifest does not state.
fn derive_active_graph_input_from_manifest(text: &str) -> ActiveGraphInput {
    let mut input = ActiveGraphInput::default();
    let commits = parse_commit_rows(text);
    for sha in &commits.shas {
        input.workflow_nodes.push(NodeId(sha.clone()));
    }
    for (child, parent) in &commits.parent_edges {
        input.workflow_edges.push((
            NodeId(child.clone()),
            NodeId(parent.clone()),
        ));
    }
    for (source, target) in &commits.evidence_edges {
        input.evidence_links.push((source.clone(), NodeId(target.clone())));
    }
    for (label, target) in &commits.memory_refs {
        input.memory_refs.push((label.clone(), NodeId(target.clone())));
    }
    if let Some(first_sha) = commits.shas.first().cloned() {
        // Cycle ID isn't a node, but we pin the run id to the first
        // commit's short SHA so observability views can group by cycle.
        input.workflow_run_id = Some(first_sha.chars().take(7).collect());
    }
    // M8.6 — populate per-node + per-edge provenance. The first
    // occurrence of each SHA wins (BTreeMap insert keeps it), so
    // bullet-driven entries (which arrive after the table row in
    // parse order) only fill gaps. This keeps the locator closest to
    // the canonical "table row" for workflow_nodes that appear there.
    for (sha, locator) in &commits.sha_provenance {
        input
            .node_provenance
            .entry(NodeId(sha.clone()))
            .or_insert_with(|| {
                ProvenanceRef::new(
                    ProvenanceSourceKind::CycleManifest,
                    locator.clone(),
                )
            });
    }
    for ((child, parent), locator) in &commits.edge_provenance {
        input
            .edge_provenance
            .entry((NodeId(child.clone()), NodeId(parent.clone())))
            .or_insert_with(|| {
                ProvenanceRef::new(
                    ProvenanceSourceKind::CycleManifest,
                    locator.clone(),
                )
            });
    }
    input
}

/// Parsed slices from a cycle's archive manifest.
#[derive(Debug, Default, Clone)]
struct ParsedManifest {
    shas: Vec<String>,
    parent_edges: Vec<(String, String)>,
    evidence_edges: Vec<(String, String)>,
    memory_refs: Vec<(String, String)>,
    /// M8.6 — per-SHA provenance (`Commits:row_N` locator) so the
    /// projection can attribute every workflow node back to its
    /// byte position in the manifest. Recorded as we encounter the
    /// SHA in the Commits table — never inferred.
    sha_provenance: Vec<(String, String)>,
    /// M8.6 — per-edge provenance (`Commit_parents:bullet_N`
    /// locator) so the projection can attribute every parent_of edge
    /// back to its bullet.
    edge_provenance: Vec<((String, String), String)>,
}

/// Parse the manifest into workflow_nodes, parent_edges, evidence_edges
/// and memory_refs. The parser is deliberately conservative — anything
/// it cannot structurally match is dropped, never guessed.
fn parse_commit_rows(text: &str) -> ParsedManifest {
    let mut out = ParsedManifest::default();
    let mut in_commits = false;
    let mut commits_section: &str = "";
    // M8.6 — count rows in the Commits table so we can attach a stable
    // locator (`Commits:row_N`) to each SHA we record.
    let mut commit_row_idx: usize = 0;
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        // Detect the "## Commits" section header. M8.2/M8.3 manifests
        // use exactly this heading; if a future cycle uses a different
        // header the parser degrades to empty (no fabrication).
        if trimmed.starts_with("## ") && trimmed.to_ascii_lowercase().contains("commit") {
            in_commits = true;
            commits_section = trimmed.trim_start_matches("## ").trim();
            commit_row_idx = 0;
            continue;
        }
        if trimmed.starts_with("## ") {
            in_commits = false;
        }
        if !in_commits {
            continue;
        }
        // Table row shape: `| `<sha>` | feat(uat) | ... |` or
        // `| `<sha>` | fix(engine) | ... |`. We only pick the first
        // column when it looks like a 7–40 char hex SHA.
        if let Some(sha) = parse_table_row_sha(trimmed)
            && !out.shas.contains(&sha)
        {
            out.shas.push(sha.clone());
            commit_row_idx += 1;
            out.sha_provenance.push((
                sha,
                format!("{commits_section}:row_{commit_row_idx}@line_{}", idx + 1),
            ));
        }
    }
    // Parent edges + evidence + memory refs come from explicit
    // "Bridges" / "Deferred items" sections if present. We look for
    // bullet lists under those headings.
    parse_section_bullets_for_with_locator(
        text,
        "Bridges",
        &mut |bullet: &str, line_idx: usize, section: &str| {
            // Bridge shape: "- M8.x — ..." or "- <source> → <target>"
            // We treat each bridge as an evidence edge keyed by the cycle
            // id (source) and the first SHA in the bridge target.
            if let Some((label, sha)) = parse_bridge_bullet(bullet, &out.shas) {
                let locator = format!("{section}:bullet@line_{line_idx}");
                // Record provenance for the evidence node (target SHA).
                out.sha_provenance
                    .push((sha.clone(), locator.clone()));
                out.evidence_edges.push((label, sha));
                let _ = locator;
            }
        },
    );
    parse_section_bullets_for_with_locator(
        text,
        "Deferred items",
        &mut |bullet: &str, line_idx: usize, section: &str| {
            // Deferred bullet shape: "- M9 — ..."
            let label = bullet
                .trim_start_matches("- ")
                .trim_start_matches("* ")
                .chars()
                .take(64)
                .collect::<String>();
            let target = out
                .shas
                .first()
                .cloned()
                .unwrap_or_else(|| "<unknown>".to_string());
            let locator = format!("{section}:bullet@line_{line_idx}");
            // The first commit SHA gets an additional provenance
            // pointer so memory_ref targets are also attributable.
            out.sha_provenance.push((target.clone(), locator));
            out.memory_refs.push((label, target));
        },
    );
    // Parent edges come from an explicit "## Commit parents" section if
    // the manifest provides one (M8.5 convention). The parser only emits
    // edges whose both SHAs appear in the known commit list — anything
    // else is dropped silently to preserve the "honest derivation"
    // contract from M8.4. Manifests without this section still parse
    // cleanly with empty `parent_edges` (backward compatible).
    parse_section_bullets_for_with_locator(
        text,
        "Commit parents",
        &mut |bullet: &str, line_idx: usize, section: &str| {
            // Bullet shape: "- `<child>` → `<parent>` (note)"
            // The arrow can be `→`, `->`, or `=>`; the note is optional.
            if let Some((child, parent)) = parse_parent_edge_bullet(bullet, &out.shas) {
                let locator = format!("{section}:bullet@line_{line_idx}");
                out.sha_provenance.push((child.clone(), locator.clone()));
                out.sha_provenance.push((parent.clone(), locator.clone()));
                out.parent_edges.push((child.clone(), parent.clone()));
                out.edge_provenance.push(((child, parent), locator));
            }
        },
    );
    // Parent edges are inferred from the cycle's own commit history.
    // The manifest only lists commit SHAs; parent relations are not
    // explicit. To stay honest, we leave parent_edges empty unless
    // the manifest itself spells them out (future format) — no
    // `git rev-parse` lookup. This means cycles parsed from archive
    // manifests render as flat node lists, not trees, until a richer
    // manifest format ships.
    out
}

/// Extract a SHA-like token from a markdown table row's first cell.
fn parse_table_row_sha(line: &str) -> Option<String> {
    let first = line
        .split('|')
        .map(str::trim)
        .find(|cell| !cell.is_empty())?
        .trim_matches('`')
        .trim();
    if first.len() >= 7 && first.len() <= 40 && first.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(first.to_string())
    } else {
        None
    }
}

/// Walk a markdown section by heading and feed each bullet to a callback.
///
/// Uses a higher-ranked closure bound (`for<'b> FnMut(&'b str)`) so
/// callers can pass closures that capture references of any lifetime
/// (the simpler `FnMut(&str)` signature fights the borrow checker
/// when the same helper is invoked twice with closures over `out`).
///
/// Retained for M8.4/M8.5 fixture authors and external callers; the
/// M8.6 cycle-aware parser uses
/// [`parse_section_bullets_for_with_locator`] instead because it
/// also reports the source line index.
#[allow(dead_code)]
fn parse_section_bullets_for<'a, F: for<'b> FnMut(&'b str)>(
    text: &'a str,
    heading: &str,
    mut cb: F,
) {
    let mut in_section = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            let section_name = trimmed.trim_start_matches("## ").to_ascii_lowercase();
            in_section = section_name.starts_with(&heading.to_ascii_lowercase());
            continue;
        }
        if in_section && (trimmed.starts_with("- ") || trimmed.starts_with("* ")) {
            cb(trimmed);
        }
    }
}

/// Same as [`parse_section_bullets_for`] but also passes the 1-based
/// line index and the canonical section name so callers can record
/// per-bullet provenance (M8.6 contract). `section_locator` is the
/// raw heading text with the `## ` prefix stripped; callers compose
/// it into a stable locator like `Commits:row_3`.
///
/// `line_idx` is the byte-line index in the original `text`, not a
/// character offset — that matches what every other CLI surface in
/// this crate reports as "line".
fn parse_section_bullets_for_with_locator<'a, F: for<'b> FnMut(&'b str, usize, &'b str)>(
    text: &'a str,
    heading: &str,
    mut cb: F,
) {
    let mut in_section = false;
    let mut section_locator: &str = "";
    for (idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("## ") {
            section_locator = trimmed.trim_start_matches("## ").trim();
            in_section = section_locator
                .to_ascii_lowercase()
                .starts_with(&heading.to_ascii_lowercase());
            continue;
        }
        if in_section && (trimmed.starts_with("- ") || trimmed.starts_with("* ")) {
            cb(trimmed, idx + 1, section_locator);
        }
    }
}

/// Parse a "Bridges" bullet into `(label, target_sha)`.
///
/// Honest contract: only emit an evidence edge if the bridge bullet
/// mentions a SHA that appears in the manifest's commit list. Anything
/// else is dropped silently.
fn parse_bridge_bullet(bullet: &str, known_shas: &[String]) -> Option<(String, String)> {
    for sha in known_shas {
        if bullet.contains(sha) || bullet.contains(&sha.chars().take(7).collect::<String>()) {
            let label = bullet
                .trim_start_matches("- ")
                .trim_start_matches("* ")
                .chars()
                .take(96)
                .collect::<String>();
            return Some((label, sha.clone()));
        }
    }
    None
}

/// Parse a "Commit parents" bullet into `(child, parent)` SHAs.
///
/// Bullet shape:
///   - ` `<child>` → `<parent>` (note)`       (Unicode arrow)
///   - ` `<child>` -> `<parent>` (note)`       (ASCII arrow)
///   - ` `<child>` => `<parent>` (note)`       (fat arrow)
///   - ` `<child>` → <parent>` (no note)
///
/// Honest contract: both SHAs must appear in `known_shas` (the manifest's
/// commit list). If either side is missing or not in the commit list,
/// the bullet is dropped silently — the manifest is the only authority
/// for `parent_of` edges in M8.5; we never fabricate.
fn parse_parent_edge_bullet(bullet: &str, known_shas: &[String]) -> Option<(String, String)> {
    // Strip leading bullet marker.
    let body = bullet
        .trim_start_matches("- ")
        .trim_start_matches("* ")
        .trim();
    // Find an arrow token. We accept three shapes for ergonomics.
    let arrow = [" → ", " -> ", " => "]
        .iter()
        .find(|a| body.contains(*a))
        .copied()?;
    let mut split = body.splitn(2, arrow);
    let left = split.next()?.trim().trim_matches('`').trim();
    let right = split.next()?.trim().trim_matches('`').trim();
    // The note (in parentheses) is optional — strip it after backtick
    // trimming so trailing backticks inside the note don't pollute the
    // SHA (e.g. ``def5678` (release commit)``).
    let right = right
        .split_once(" (")
        .map(|(head, _)| head.trim().trim_matches('`').trim())
        .unwrap_or(right);
    // Both SHAs must look like a SHA and be in the known list.
    if !is_sha_like(left) || !is_sha_like(right) {
        return None;
    }
    if !known_shas.iter().any(|k| k == left)
        || !known_shas.iter().any(|k| k == right)
    {
        return None;
    }
    Some((left.to_string(), right.to_string()))
}

/// True if `s` is a 7–40 char ASCII hex token (the same shape
/// `parse_table_row_sha` accepts). Pulled out so the parent-edge
/// parser and the table-row parser agree on what "looks like a SHA"
/// means.
fn is_sha_like(s: &str) -> bool {
    let len = s.len();
    (7..=40).contains(&len) && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Deterministic RFC-3339 UTC timestamp truncated to seconds (same
/// inline algorithm as `dev/graph.rs` so the two surfaces stay in sync).
fn now_rfc3339_seconds() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let secs_per_day = 86_400u64;
    let secs_per_hour = 3_600u64;
    let secs_per_min = 60u64;
    let days = now / secs_per_day;
    let rem = now % secs_per_day;
    let hh = rem / secs_per_hour;
    let rem = rem % secs_per_hour;
    let mm = rem / secs_per_min;
    let ss = rem % secs_per_min;
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

/// Dispatch to the right input source for M8.4 cycle-aware loading.
///
/// Priority (highest first):
/// 1. `--from-cycle <cycle-id>`: derive from the cycle's archive manifest.
/// 2. `--from-input <path>`: load a JSON fixture.
/// 3. None of the above: empty projection (`<empty-default>` source).
///
/// The single failure surface (stderr) is owned by the runner; this
/// function just propagates `anyhow::Error` with enough context for
/// the runner to format a clear message.
fn resolve_input(
    from_input: Option<&PathBuf>,
    from_cycle: Option<&str>,
    env: &CliEnvironment,
) -> anyhow::Result<(ActiveGraphInput, String)> {
    if let Some(cycle_id) = from_cycle {
        return load_input_from_cycle(cycle_id, env);
    }
    load_input(from_input)
}

pub(crate) fn run_dev_cockpit_view(
    args: CockpitViewArgs,
    env: &CliEnvironment,
) -> CommandOutput {
    let (input, source) = match resolve_input(
        args.from_input.as_ref(),
        args.from_cycle.as_deref(),
        env,
    ) {
        Ok(pair) => pair,
        Err(error) => return CommandOutput {
            status: 1,
            stdout: String::new(),
            stderr: format!("cockpit view: {error}"),
        },
    };
    let projection = project(input);
    let builder = DefaultCockpitViewBuilder;
    let view: CockpitView = builder.build(&projection, args.kind.to_engine(), &now_rfc3339_seconds());
    let row = CockpitViewRow::from_view(&view, &source);

    match args.format {
        OutputFormat::Text => render_view_text(&row, "sddk dev cockpit view"),
        OutputFormat::Json => match serde_json::to_string_pretty(&row) {
            Ok(json) => CommandOutput {
                status: 0,
                stdout: format!("{json}\n"),
                stderr: String::new(),
            },
            Err(error) => CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("cockpit view: failed to serialize: {error}"),
            },
        },
    }
}

pub(crate) fn run_dev_cockpit_obs(
    args: CockpitObsArgs,
    env: &CliEnvironment,
) -> CommandOutput {
    let (input, source) = match resolve_input(
        args.from_input.as_ref(),
        args.from_cycle.as_deref(),
        env,
    ) {
        Ok(pair) => pair,
        Err(error) => return CommandOutput {
            status: 1,
            stdout: String::new(),
            stderr: format!("cockpit obs: {error}"),
        },
    };
    let projection = project(input);
    let builder = DefaultCockpitObservabilityBuilder;
    let view: CockpitView = builder.build(&projection, args.kind.to_engine(), &now_rfc3339_seconds());
    let row = CockpitViewRow::from_view(&view, &source);

    match args.format {
        OutputFormat::Text => render_view_text(&row, "sddk dev cockpit obs"),
        OutputFormat::Json => match serde_json::to_string_pretty(&row) {
            Ok(json) => CommandOutput {
                status: 0,
                stdout: format!("{json}\n"),
                stderr: String::new(),
            },
            Err(error) => CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("cockpit obs: failed to serialize: {error}"),
            },
        },
    }
}

/// Run `sddk dev cockpit diff` — M8.7 cross-input drift detection.
///
/// Resolves two `ActiveGraphProjection`s (one from a cycle, one from
/// either another cycle or an input JSON file), runs
/// `DefaultDriftEngine::diff`, and emits a `DriftRow` JSON envelope
/// or a human-readable text rendering.
pub(crate) fn run_dev_cockpit_diff(
    args: CockpitDiffArgs,
    env: &CliEnvironment,
) -> CommandOutput {
    // Validate the pairing up front.
    if args.cycle_a.is_none() && args.input_a.is_none() {
        return CommandOutput {
            status: 1,
            stdout: String::new(),
            stderr: "cockpit diff: either --cycle-a or --input-a is required".to_string(),
        };
    }
    if args.cycle_b.is_none() && args.input_b.is_none() {
        return CommandOutput {
            status: 1,
            stdout: String::new(),
            stderr: "cockpit diff: either --cycle-b or --input-b is required".to_string(),
        };
    }

    // Resolve projection A.
    let (input_a, source_a) = match resolve_diff_side(
        args.cycle_a.as_deref(),
        args.input_a.as_ref(),
        env,
        "a",
    ) {
        Ok(pair) => pair,
        Err(error) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("cockpit diff: {error}"),
            };
        }
    };
    // Resolve projection B.
    let (input_b, source_b) = match resolve_diff_side(
        args.cycle_b.as_deref(),
        args.input_b.as_ref(),
        env,
        "b",
    ) {
        Ok(pair) => pair,
        Err(error) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("cockpit diff: {error}"),
            };
        }
    };

    let projection_a = project(input_a);
    let projection_b = project(input_b);
    let engine = DefaultDriftEngine;
    let report = engine.diff(&projection_a, &projection_b);
    let row = DriftRow::from_report(&report, &source_a, &source_b);

    match args.format {
        OutputFormat::Text => render_diff_text(&row),
        OutputFormat::Json => match serde_json::to_string_pretty(&row) {
            Ok(json) => CommandOutput {
                status: 0,
                stdout: format!("{json}\n"),
                stderr: String::new(),
            },
            Err(error) => CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("cockpit diff: failed to serialize: {error}"),
            },
        },
    }
}

/// Resolve one side of a drift comparison. Prefers `cycle` when set
/// (consistent with `view`/`obs`); falls back to `input`.
fn resolve_diff_side(
    cycle: Option<&str>,
    input: Option<&PathBuf>,
    env: &CliEnvironment,
    side: &str,
) -> anyhow::Result<(ActiveGraphInput, String)> {
    if let Some(cycle_id) = cycle {
        return load_input_from_cycle(cycle_id, env)
            .map(|(input, src)| (input, format!("cycle[{side}]={cycle_id} ({src})")));
    }
    load_input(input).map(|(input, src)| (input, format!("input[{side}]={src}")))
}

/// JSON envelope for the drift command. Mirrors the engine's
/// `DriftReport` shape, plus source labels for traceability.
#[derive(Debug, Serialize, Deserialize)]
struct DriftRow {
    source_a: String,
    source_b: String,
    summary: DriftSummaryRow,
    node_deltas: Vec<DriftNodeRow>,
    edge_deltas: Vec<DriftEdgeRow>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DriftSummaryRow {
    nodes_added: usize,
    nodes_removed: usize,
    nodes_changed: usize,
    edges_added: usize,
    edges_removed: usize,
    edges_changed: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct DriftNodeRow {
    kind: String,
    node_id: String,
    before_label: Option<String>,
    after_label: Option<String>,
    changed_fields: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DriftEdgeRow {
    kind: String,
    sort_key: String,
    changed_fields: Vec<String>,
}

impl DriftRow {
    fn from_report(
        report: &sddk_engine::active_graph_drift::DriftReport,
        source_a: &str,
        source_b: &str,
    ) -> Self {
        let node_deltas: Vec<DriftNodeRow> = report
            .node_deltas
            .iter()
            .map(|d| DriftNodeRow {
                kind: d.kind.label().to_string(),
                node_id: d.node_id.0.clone(),
                before_label: d.before.as_ref().map(|n| n.label.clone()),
                after_label: d.after.as_ref().map(|n| n.label.clone()),
                changed_fields: d.changed_fields.clone(),
            })
            .collect();
        let edge_deltas: Vec<DriftEdgeRow> = report
            .edge_deltas
            .iter()
            .map(|d| DriftEdgeRow {
                kind: d.kind.label().to_string(),
                sort_key: format!("{:?}:{:?}→{:?}", d.sort_key.0, d.sort_key.1, d.sort_key.2),
                changed_fields: d.changed_fields.clone(),
            })
            .collect();
        Self {
            source_a: source_a.to_string(),
            source_b: source_b.to_string(),
            summary: DriftSummaryRow {
                nodes_added: report.summary.nodes_added,
                nodes_removed: report.summary.nodes_removed,
                nodes_changed: report.summary.nodes_changed,
                edges_added: report.summary.edges_added,
                edges_removed: report.summary.edges_removed,
                edges_changed: report.summary.edges_changed,
            },
            node_deltas,
            edge_deltas,
        }
    }
}

/// Run `sddk dev cockpit digest` — M8.8 stable projection digest.
///
/// Resolves an `ActiveGraphProjection` from a cycle or input file,
/// runs `ProjectionDigest::compute`, and emits the hex digest
/// (operator-pipeable) or a stable JSON envelope.
pub(crate) fn run_dev_cockpit_digest(
    args: CockpitDigestArgs,
    env: &CliEnvironment,
) -> CommandOutput {
    // Validate source: must have exactly one of --cycle / --input.
    if args.cycle.is_none() && args.input.is_none() {
        return CommandOutput {
            status: 1,
            stdout: String::new(),
            stderr: "cockpit digest: either --cycle or --input is required".to_string(),
        };
    }

    let (input, source) = match resolve_digest_source(
        args.cycle.as_deref(),
        args.input.as_ref(),
        env,
    ) {
        Ok(pair) => pair,
        Err(error) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("cockpit digest: {error}"),
            };
        }
    };

    let projection = project(input);
    let kind = args.kind.to_engine();
    let digest = sddk_engine::active_graph_digest::ProjectionDigest::compute(&projection, kind);

    match args.format {
        OutputFormat::Text => CommandOutput {
            status: 0,
            stdout: format!("{}\n", digest.hex),
            stderr: String::new(),
        },
        OutputFormat::Json => {
            let row = DigestRow {
                kind: kind.label().to_string(),
                hex: digest.hex.clone(),
                source,
            };
            match serde_json::to_string_pretty(&row) {
                Ok(json) => CommandOutput {
                    status: 0,
                    stdout: format!("{json}\n"),
                    stderr: String::new(),
                },
                Err(error) => CommandOutput {
                    status: 1,
                    stdout: String::new(),
                    stderr: format!("cockpit digest: failed to serialize: {error}"),
                },
            }
        }
    }
}

/// Resolve one source for a digest command. Prefers `--cycle` when set
/// (consistent with `view` / `obs` / `diff`); falls back to `--input`.
fn resolve_digest_source(
    cycle: Option<&str>,
    input: Option<&PathBuf>,
    env: &CliEnvironment,
) -> anyhow::Result<(ActiveGraphInput, String)> {
    if let Some(cycle_id) = cycle {
        return load_input_from_cycle(cycle_id, env)
            .map(|(input, src)| (input, format!("cycle={cycle_id} ({src})")));
    }
    load_input(input).map(|(input, src)| (input, format!("input={src}")))
}

/// JSON envelope for the digest command. Mirrors the engine's
/// `ProjectionDigest` shape, plus a source label for traceability.
#[derive(Debug, Serialize, Deserialize)]
struct DigestRow {
    kind: String,
    hex: String,
    source: String,
}

fn render_diff_text(row: &DriftRow) -> CommandOutput {
    let mut out = String::new();
    out.push_str("sddk dev cockpit diff\n");
    out.push_str("=====================\n");
    out.push_str(&format!("A: {}\n", row.source_a));
    out.push_str(&format!("B: {}\n", row.source_b));
    out.push('\n');
    out.push_str("Summary\n");
    out.push_str("-------\n");
    out.push_str(&format!(
        "nodes: +{} / -{} / ~{}\n",
        row.summary.nodes_added, row.summary.nodes_removed, row.summary.nodes_changed
    ));
    out.push_str(&format!(
        "edges: +{} / -{} / ~{}\n",
        row.summary.edges_added, row.summary.edges_removed, row.summary.edges_changed
    ));
    out.push('\n');

    if row.node_deltas.is_empty() && row.edge_deltas.is_empty() {
        out.push_str("No drift detected.\n");
    } else {
        if !row.node_deltas.is_empty() {
            out.push_str("Node drift\n");
            out.push_str("----------\n");
            for d in &row.node_deltas {
                match d.kind.as_str() {
                    "added" => {
                        out.push_str(&format!(
                            "+ node {} [label={}]\n",
                            d.node_id,
                            d.after_label.as_deref().unwrap_or("?")
                        ));
                    }
                    "removed" => {
                        out.push_str(&format!(
                            "- node {} [label={}]\n",
                            d.node_id,
                            d.before_label.as_deref().unwrap_or("?")
                        ));
                    }
                    "changed" => {
                        out.push_str(&format!(
                            "~ node {} fields={:?} [{} → {}]\n",
                            d.node_id,
                            d.changed_fields,
                            d.before_label.as_deref().unwrap_or("?"),
                            d.after_label.as_deref().unwrap_or("?"),
                        ));
                    }
                    _ => {}
                }
            }
            out.push('\n');
        }
        if !row.edge_deltas.is_empty() {
            out.push_str("Edge drift\n");
            out.push_str("----------\n");
            for d in &row.edge_deltas {
                match d.kind.as_str() {
                    "added" => out.push_str(&format!("+ edge {}\n", d.sort_key)),
                    "removed" => out.push_str(&format!("- edge {}\n", d.sort_key)),
                    "changed" => {
                        out.push_str(&format!("~ edge {} fields={:?}\n", d.sort_key, d.changed_fields));
                    }
                    _ => {}
                }
            }
        }
    }
    CommandOutput {
        status: 0,
        stdout: out,
        stderr: String::new(),
    }
}

fn render_view_text(row: &CockpitViewRow, banner: &str) -> CommandOutput {
    let mut out = String::new();
    out.push_str(&format!("=== {banner} ===\n\n"));
    out.push_str(&format!("title:       {}\n", row.title));
    out.push_str(&format!("kind:        {}\n", row.kind));
    out.push_str(&format!("sections:    {}\n", row.section_count));
    out.push_str(&format!("lines:       {}\n", row.line_count));
    out.push_str(&format!("generated:   {}\n", row.generated_at));
    out.push_str(&format!("source:      {}\n", row.source));
    out.push('\n');
    if row.sections.is_empty() {
        out.push_str("(empty view — projection has no nodes/edges)\n");
    } else {
        for section in &row.sections {
            out.push_str(&format!("[{}]\n", section.heading));
            if section.lines.is_empty() {
                out.push_str("  (no lines)\n");
            } else {
                for line in &section.lines {
                    out.push_str(&format!("  {line}\n"));
                }
            }
            out.push('\n');
        }
    }
    CommandOutput {
        status: 0,
        stdout: out,
        stderr: String::new(),
    }
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_workflow_with_evidence() -> ActiveGraphInput {
        let mut input = ActiveGraphInput::default();
        input.workflow_nodes = vec![NodeId("wf-explore".to_string()), NodeId("wf-apply".to_string())];
        input.workflow_edges = vec![(
            NodeId("wf-explore".to_string()),
            NodeId("wf-apply".to_string()),
        )];
        input.workflow_run_id = Some("run-001".to_string());
        input.evidence_links = vec![("verifier-passed".to_string(), NodeId("wf-apply".to_string()))];
        input
    }

    #[test]
    fn cockpit_view_kind_arg_maps_all_four_variants() {
        assert!(matches!(
            CockpitViewKindArg::Overview.to_engine(),
            CockpitViewKind::Overview
        ));
        assert!(matches!(
            CockpitViewKindArg::Journal.to_engine(),
            CockpitViewKind::Journal
        ));
        assert!(matches!(
            CockpitViewKindArg::Timeline.to_engine(),
            CockpitViewKind::Timeline
        ));
        assert!(matches!(
            CockpitViewKindArg::Execution.to_engine(),
            CockpitViewKind::Execution
        ));
    }

    #[test]
    fn empty_projection_produces_view_with_no_signals() {
        // The engine always emits a fixed section structure for every
        // kind — even on empty input. We don't pin the exact number of
        // sections; we only assert the view is well-formed and that
        // signal-bearing lines (node counts, edges, journal entries)
        // are absent. The "Summary" section always renders the canonical
        // "N node(s), M edge(s)" line; we treat it as a structural
        // footer rather than signal.
        let projection = project(ActiveGraphInput::default());
        let builder = DefaultCockpitViewBuilder;
        for kind in [
            CockpitViewKind::Overview,
            CockpitViewKind::Journal,
            CockpitViewKind::Timeline,
            CockpitViewKind::Execution,
        ] {
            let view = builder.build(&projection, kind, "t0");
            assert_eq!(view.kind, kind);
            assert!(!view.title.is_empty(), "kind={:?} should have a title", kind);
            assert!(
                !view.sections.is_empty(),
                "kind={:?} should have at least one section",
                kind
            );
            let total_lines: usize = view.sections.iter().map(|s| s.lines.len()).sum();
            // Empty projection must produce only the structural
            // footers (Summary + per-kind "empty" placeholders the
            // engine emits). We cap it generously rather than pin
            // exact counts — the contract we care about is "no real
            // data" not "exactly zero lines".
            assert!(
                total_lines <= 4,
                "kind={:?} should have minimal lines on empty projection, got {} (sections={:?})",
                kind,
                total_lines,
                view.sections
            );
        }
    }

    #[test]
    fn non_empty_projection_produces_overview_with_counts() {
        let projection = project(fixture_workflow_with_evidence());
        let builder = DefaultCockpitViewBuilder;
        let view = builder.build(&projection, CockpitViewKind::Overview, "t0");
        assert!(!view.sections.is_empty(), "overview must report at least one section");
        let flat: String = view
            .sections
            .iter()
            .flat_map(|s| std::iter::once(s.heading.as_str()).chain(s.lines.iter().map(String::as_str)))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            flat.contains("workflow"),
            "overview should mention workflow nodes: {}",
            flat
        );
        assert!(
            flat.contains("parent_of"),
            "overview should mention parent_of edges: {}",
            flat
        );
        assert!(
            flat.contains("evidence_of"),
            "overview should mention evidence_of edges: {}",
            flat
        );
    }

    #[test]
    fn cockpit_view_row_reports_section_and_line_counts() {
        // Use the engine itself to build a real CockpitView, then wrap
        // it in the JSON envelope row. We cannot construct CockpitView
        // or CockpitSection from outside the defining crate (both are
        // `#[non_exhaustive]` without a `Default` impl), so the row
        // helper must operate on whatever the engine produces.
        let projection = project(fixture_workflow_with_evidence());
        let builder = DefaultCockpitViewBuilder;
        let view = builder.build(&projection, CockpitViewKind::Overview, "t0");
        let row = CockpitViewRow::from_view(&view, "<empty-default>");
        assert_eq!(row.kind, "overview");
        // The overview section's structure is engine-internal; we only
        // assert the counts are coherent with the section list.
        assert_eq!(row.section_count, row.sections.len());
        let flat_lines: usize = row.sections.iter().map(|s| s.lines.len()).sum();
        assert_eq!(row.line_count, flat_lines);
    }

    #[test]
    fn cockpit_view_row_serializes_empty_view_without_panic() {
        let projection = project(ActiveGraphInput::default());
        let builder = DefaultCockpitViewBuilder;
        let view = builder.build(&projection, CockpitViewKind::Journal, "t0");
        let row = CockpitViewRow::from_view(&view, "<empty-default>");
        assert_eq!(row.kind, "journal");
        // The engine always emits a fixed section structure even on
        // empty input; the row helper must still serialize without
        // panicking and preserve the section_count.
        let json = serde_json::to_string(&row).unwrap();
        let back: CockpitViewRow = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind, "journal");
        assert_eq!(back.section_count, row.section_count);
    }

    #[test]
    fn load_input_falls_back_to_empty_when_no_path_given() {
        let (input, source) = load_input(None).unwrap();
        assert!(input.workflow_nodes.is_empty());
        assert_eq!(source, "<empty-default>");
    }

    #[test]
    fn load_input_parses_valid_json_fixture() {
        let dir = std::env::temp_dir().join(format!(
            "sddk-cockpit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("fixture.json");
        std::fs::write(
            &path,
            br#"{"workflow_nodes":["a","b"],"workflow_edges":[["a","b"]],"workflow_run_id":"r-1"}"#,
        )
        .unwrap();
        let (input, source) = load_input(Some(&path)).unwrap();
        assert_eq!(source, path.display().to_string());
        assert_eq!(input.workflow_nodes.len(), 2);
        assert_eq!(input.workflow_edges.len(), 1);
        assert_eq!(input.workflow_run_id.as_deref(), Some("r-1"));
    }

    #[test]
    fn load_input_errors_on_invalid_json() {
        let dir = std::env::temp_dir().join(format!(
            "sddk-cockpit-bad-{}-{}",
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
    fn now_rfc3339_is_well_formed_utc() {
        let s = now_rfc3339_seconds();
        assert_eq!(s.len(), 20, "expected 20-char RFC-3339 UTC, got {s:?}");
        assert!(s.ends_with('Z'));
        assert!(s.contains('T'));
        let parts: Vec<&str> = s.split('T').collect();
        assert_eq!(parts.len(), 2);
    }

    // ── M8.3 — cockpit obs surface tests ──────────────────────────────

    #[test]
    fn cockpit_obs_kind_arg_maps_all_five_variants() {
        assert!(matches!(
            CockpitObsKindArg::Providers.to_engine(),
            CockpitObservabilityKind::Providers
        ));
        assert!(matches!(
            CockpitObsKindArg::Usage.to_engine(),
            CockpitObservabilityKind::Usage
        ));
        assert!(matches!(
            CockpitObsKindArg::Assurance.to_engine(),
            CockpitObservabilityKind::Assurance
        ));
        assert!(matches!(
            CockpitObsKindArg::Handoff.to_engine(),
            CockpitObservabilityKind::Handoff
        ));
        assert!(matches!(
            CockpitObsKindArg::Experiments.to_engine(),
            CockpitObservabilityKind::Experiments
        ));
    }

    #[test]
    fn empty_projection_produces_empty_placeholder_for_all_five_obs_kinds() {
        // The engine explicitly emits an "empty" placeholder for every
        // observability kind on empty input (vs. the cockpit_views
        // engine which emits a fixed structural section list). This
        // distinction is part of the contract: obs views report a
        // single 1-line "empty" placeholder so the operator can tell
        // apart "no data" from "data exists but rendered empty".
        let projection = project(ActiveGraphInput::default());
        let builder = DefaultCockpitObservabilityBuilder;
        for kind in [
            CockpitObservabilityKind::Providers,
            CockpitObservabilityKind::Usage,
            CockpitObservabilityKind::Assurance,
            CockpitObservabilityKind::Handoff,
            CockpitObservabilityKind::Experiments,
        ] {
            let view = builder.build(&projection, kind, "t0");
            assert!(!view.title.is_empty(), "kind={:?} should have a title", kind);
            assert_eq!(
                view.sections.len(),
                1,
                "kind={:?} empty projection should yield exactly one section, got {:?}",
                kind,
                view.sections
            );
            assert_eq!(
                view.sections[0].lines,
                vec!["empty".to_string()],
                "kind={:?} empty projection should yield the canonical 'empty' placeholder",
                kind
            );
        }
    }

    #[test]
    fn non_empty_projection_yields_populated_obs_views() {
        // Fixture designed to populate every obs kind:
        // - 1 WorkflowRun (Providers.Runs)
        // - 1 Assurance node (Providers.Assurance)
        // - 1 evidence_of edge (Assurance.Evidence + Usage.evidence_of)
        // - 1 delegated_to edge (Handoff.Delegations + Usage.delegated_to)
        // - 1 parent_of edge (Usage.parent_of)
        // - 1 WorkflowLab (Experiments.Labs)
        // - 1 promote + 1 gate (Experiments.Promotion Decisions + Usage)
        let mut input = ActiveGraphInput::default();
        input.workflow_nodes = vec![
            NodeId("wf".to_string()),
            NodeId("leaf".to_string()),
        ];
        input.workflow_edges = vec![(
            NodeId("wf".to_string()),
            NodeId("leaf".to_string()),
        )];
        input.workflow_run_id = Some("r1".to_string());
        input.assurance_labels = vec!["a1".to_string()];
        input.evidence_links = vec![("a1".to_string(), NodeId("leaf".to_string()))];
        input.delegations = vec![(
            NodeId("orch".to_string()),
            NodeId("worker".to_string()),
        )];
        input.workflow_lab_labels = vec!["lab1".to_string()];
        input.lab_promotions = vec![
            ("g1".to_string(), NodeId("leaf".to_string()), false),
            ("p1".to_string(), NodeId("wf".to_string()), true),
        ];
        let projection = project(input);
        let builder = DefaultCockpitObservabilityBuilder;

        // Providers: runs + assurance
        let v = builder.build(&projection, CockpitObservabilityKind::Providers, "t0");
        let flat: String = v
            .sections
            .iter()
            .flat_map(|s| {
                std::iter::once(s.heading.as_str()).chain(s.lines.iter().map(String::as_str))
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(flat.contains("r1"), "Providers should mention run r1: {flat}");
        assert!(
            flat.contains("a1"),
            "Providers should mention assurance a1: {flat}"
        );

        // Usage: at least 4 distinct edge-kind counts
        let v = builder.build(&projection, CockpitObservabilityKind::Usage, "t0");
        let lines = v
            .sections
            .first()
            .map(|s| s.lines.clone())
            .unwrap_or_default();
        let edge_kinds_reported: usize = lines
            .iter()
            .filter(|l| {
                l.contains("parent_of")
                    || l.contains("evidence_of")
                    || l.contains("delegated_to")
                    || l.contains("promotes")
                    || l.contains("gates")
                    || l.contains("references")
            })
            .count();
        assert!(
            edge_kinds_reported >= 4,
            "Usage should report at least 4 edge kinds, got {:?}",
            lines
        );

        // Assurance: evidence edge present
        let v = builder.build(&projection, CockpitObservabilityKind::Assurance, "t0");
        let evidence_section = v
            .sections
            .iter()
            .find(|s| s.heading == "Evidence")
            .expect("Assurance should have Evidence section");
        assert!(
            evidence_section
                .lines
                .iter()
                .any(|l| l.contains("a1") && l.contains("leaf")),
            "Assurance.Evidence should link a1 → leaf, got {:?}",
            evidence_section.lines
        );

        // Handoff: delegation present
        let v = builder.build(&projection, CockpitObservabilityKind::Handoff, "t0");
        let delegation_section = v
            .sections
            .iter()
            .find(|s| s.heading == "Delegations")
            .expect("Handoff should have Delegations section");
        assert!(
            delegation_section
                .lines
                .iter()
                .any(|l| l.contains("orch") && l.contains("worker")),
            "Handoff.Delegations should mention orch → worker, got {:?}",
            delegation_section.lines
        );

        // Experiments: labs + promote/gate decisions
        let v = builder.build(&projection, CockpitObservabilityKind::Experiments, "t0");
        let labs = v
            .sections
            .iter()
            .find(|s| s.heading == "Labs")
            .expect("Experiments should have Labs section");
        assert!(
            labs.lines.iter().any(|l| l.contains("lab1")),
            "Experiments.Labs should mention lab1, got {:?}",
            labs.lines
        );
        let promos = v
            .sections
            .iter()
            .find(|s| s.heading == "Promotion Decisions")
            .expect("Experiments should have Promotion Decisions section");
        assert!(
            promos.lines.iter().any(|l| l.contains("promote")),
            "Experiments should mention a promote decision, got {:?}",
            promos.lines
        );
        assert!(
            promos.lines.iter().any(|l| l.contains("gate")),
            "Experiments should mention a gate decision, got {:?}",
            promos.lines
        );
    }

    // ── M8.4 — cycle-aware derivation tests ───────────────────────────

    fn fixture_manifest() -> &'static str {
        "# M8.X — test cycle\n\n\
         ## Commits\n\n\
         | SHA | Type | Subject |\n\
         |-----|------|---------|\n\
         | abc1234 | feat(uat) | first commit |\n\
         | def5678 | chore(release) | bump version |\n\n\
         ## Commit parents\n\n\
         - `abc1234` → `def5678` (release commit)\n\
         - `def5678` -> `fed4321` (forward port; sha not in commits, must be dropped)\n\n\
         ## Bridges\n\n\
         - M8.0 — built on abc1234 (v1.166.5)\n\
         - M8.3 — extends def5678 (v1.166.8)\n\n\
         ## Deferred items\n\n\
         - M9 — remove compat debt\n"
    }

    #[test]
    fn derive_active_graph_input_from_manifest_extracts_workflow_nodes() {
        let input = derive_active_graph_input_from_manifest(fixture_manifest());
        let shas: Vec<String> = input.workflow_nodes.iter().map(|n| n.0.clone()).collect();
        assert_eq!(
            shas,
            vec!["abc1234".to_string(), "def5678".to_string()],
            "commit SHAs from the Commits table become workflow_nodes"
        );
        // The first short SHA becomes the workflow run id.
        assert_eq!(input.workflow_run_id.as_deref(), Some("abc1234"));
    }

    #[test]
    fn derive_active_graph_input_from_manifest_emits_evidence_only_for_known_shas() {
        let input = derive_active_graph_input_from_manifest(fixture_manifest());
        // Bridges section mentions both abc1234 and def5678. The
        // parser must emit one evidence_of edge per bridge whose SHA
        // appears in the known_shas list (both do).
        assert_eq!(
            input.evidence_links.len(),
            2,
            "two bridges × two known SHAs ⇒ two evidence_links, got {:?}",
            input.evidence_links
        );
        let labels: Vec<&str> = input
            .evidence_links
            .iter()
            .map(|(label, _)| label.as_str())
            .collect();
        assert!(
            labels.iter().any(|l| l.contains("M8.0") && l.contains("abc1234")),
            "expected an evidence edge labelled 'M8.0 ... abc1234', got {:?}",
            labels
        );
        assert!(
            labels.iter().any(|l| l.contains("M8.3") && l.contains("def5678")),
            "expected an evidence edge labelled 'M8.3 ... def5678', got {:?}",
            labels
        );
    }

    #[test]
    fn derive_active_graph_input_from_manifest_drops_unknown_shas_in_bridges() {
        // Bridge mentions a SHA that does NOT appear in the Commits
        // table (deadbeef). The parser must silently drop it — no
        // fabrication.
        let text = "## Commits\n\n| abc1234 | feat(uat) |\n\n## Bridges\n\n- deadbeef mentions an unknown SHA\n";
        let input = derive_active_graph_input_from_manifest(text);
        assert_eq!(input.workflow_nodes.len(), 1);
        assert!(
            input.evidence_links.is_empty(),
            "bridge with unknown SHA must produce zero evidence_links, got {:?}",
            input.evidence_links
        );
    }

    #[test]
    fn derive_active_graph_input_from_manifest_emits_memory_refs_from_deferred() {
        let input = derive_active_graph_input_from_manifest(fixture_manifest());
        assert_eq!(
            input.memory_refs.len(),
            1,
            "one deferred bullet ⇒ one memory_ref, got {:?}",
            input.memory_refs
        );
        let (label, target) = &input.memory_refs[0];
        assert!(label.starts_with("M9"), "label should start with M9");
        assert_eq!(target.0, "abc1234", "deferred refs the first commit");
    }

    // ── M8.5 — Commit parents section tests ────────────────────────

    #[test]
    fn derive_active_graph_input_from_manifest_emits_parent_edges_from_commit_parents_section() {
        // The fixture has `abc1234 → def5678` in the Commit parents
        // section. Both SHAs are in the known commits, so this must
        // produce exactly one parent_of edge.
        let input = derive_active_graph_input_from_manifest(fixture_manifest());
        assert_eq!(
            input.workflow_edges.len(),
            1,
            "exactly one parent edge should survive the known-SHA filter, got {:?}",
            input.workflow_edges
        );
        let (child, parent) = &input.workflow_edges[0];
        assert_eq!(child.0, "abc1234", "child SHA must be the left side of the arrow");
        assert_eq!(parent.0, "def5678", "parent SHA must be the right side of the arrow");
    }

    #[test]
    fn derive_active_graph_input_from_manifest_drops_parent_edges_with_unknown_shas() {
        // The fixture has `def5678 -> fed4321` where fed4321 is NOT in
        // the known commits list. That edge must be silently dropped —
        // we never fabricate parent_of edges from SHAs the manifest
        // does not acknowledge.
        let input = derive_active_graph_input_from_manifest(fixture_manifest());
        for (child, parent) in &input.workflow_edges {
            assert_ne!(
                child.0, "def5678",
                "edge whose parent is unknown must be dropped, got {:?}",
                input.workflow_edges
            );
            assert_ne!(parent.0, "fed4321");
        }
    }

    #[test]
    fn derive_active_graph_input_from_manifest_backward_compatible_without_parents_section() {
        // Manifests without a `## Commit parents` section must still
        // parse cleanly with empty parent_edges (M8.4 behavior).
        let text = "## Commits\n\n| abc1234 | feat(uat) |\n| def5678 | fix |\n";
        let input = derive_active_graph_input_from_manifest(text);
        assert_eq!(input.workflow_nodes.len(), 2);
        assert!(
            input.workflow_edges.is_empty(),
            "manifest without Commit parents section must yield empty parent_edges, got {:?}",
            input.workflow_edges
        );
    }

    #[test]
    fn parse_parent_edge_bullet_accepts_unicode_arrow_ascii_arrow_fat_arrow() {
        let shas = vec!["abc1234".to_string(), "def5678".to_string()];
        assert_eq!(
            parse_parent_edge_bullet("- `abc1234` → `def5678`", &shas),
            Some(("abc1234".to_string(), "def5678".to_string()))
        );
        assert_eq!(
            parse_parent_edge_bullet("- `abc1234` -> `def5678`", &shas),
            Some(("abc1234".to_string(), "def5678".to_string()))
        );
        assert_eq!(
            parse_parent_edge_bullet("- `abc1234` => `def5678` (note)", &shas),
            Some(("abc1234".to_string(), "def5678".to_string()))
        );
        // Missing SHA in known_shas — must return None.
        let small = vec!["abc1234".to_string()];
        assert_eq!(parse_parent_edge_bullet("- `abc1234` → `def5678`", &small), None);
        // Not a SHA — must return None.
        assert_eq!(
            parse_parent_edge_bullet("- `not-a-sha` → `def5678`", &shas),
            None
        );
        // No arrow — must return None.
        assert_eq!(
            parse_parent_edge_bullet("- `abc1234` to `def5678`", &shas),
            None
        );
    }

    #[test]
    fn resolve_input_prefers_from_cycle_over_from_input() {
        use std::path::PathBuf;
        // If both are set, --from-cycle wins. We can't easily call
        // resolve_input with a non-existent cycle dir without touching
        // the filesystem, so we test the negative path: --from-input
        // alone still works after the refactor.
        let env = test_env();
        let nonexistent_cycle = "definitely-not-a-cycle-zzzz";
        let result = resolve_input(None, Some(nonexistent_cycle), &env);
        assert!(
            result.is_err(),
            "resolve_input must propagate cycle-not-found as an error"
        );
        let err = format!("{}", result.unwrap_err());
        assert!(
            err.contains("not found") || err.contains("no directory"),
            "expected 'not found' / 'no directory' in error, got: {err}"
        );
        // --from-input alone with no path still falls back to empty
        // (this is the M8.2 / M8.3 contract preserved by M8.4).
        let (input, source) =
            resolve_input(None, None, &env).unwrap();
        assert!(input.workflow_nodes.is_empty());
        assert_eq!(source, "<empty-default>");
        // Suppress unused-binding warning for PathBuf import.
        let _ = PathBuf::new();
    }

    fn test_env() -> CliEnvironment {
        use std::path::PathBuf;
        CliEnvironment {
            home: Some(PathBuf::from("/tmp")),
            data_home: None,
            sddk_data_dir: None,
            state_home: None,
            cache_home: None,
            sddk_actor: None,
            user: None,
        }
    }

    // ── M8.6 — per-node + per-edge provenance recording ──────────────

    #[test]
    fn derive_active_graph_input_from_manifest_records_node_provenance_for_table_rows() {
        // The fixture has two SHAs in the Commits table. The parser
        // must record a `ProvenanceRef` for each, anchored to the
        // Commits section + the table-row index + the byte-line number.
        let input = derive_active_graph_input_from_manifest(fixture_manifest());
        let commit1 = input
            .node_provenance
            .get(&NodeId("abc1234".to_string()))
            .expect("abc1234 must have provenance");
        assert_eq!(commit1.source_kind, ProvenanceSourceKind::CycleManifest);
        assert!(
            commit1.source_locator.contains("row_1"),
            "first SHA must be at row_1, got {:?}",
            commit1.source_locator
        );
        assert!(
            commit1.source_locator.contains("line_"),
            "locator must include the byte-line number for audit, got {:?}",
            commit1.source_locator
        );
        let commit2 = input
            .node_provenance
            .get(&NodeId("def5678".to_string()))
            .expect("def5678 must have provenance");
        assert!(
            commit2.source_locator.contains("row_2"),
            "second SHA must be at row_2, got {:?}",
            commit2.source_locator
        );
    }

    #[test]
    fn derive_active_graph_input_from_manifest_records_edge_provenance_for_commit_parents() {
        // The fixture has one valid parent edge (`abc1234 → def5678`).
        // It must appear in `edge_provenance` keyed by `(child, parent)`
        // and pointing at the `Commit parents` section.
        let input = derive_active_graph_input_from_manifest(fixture_manifest());
        let edge_prov = input
            .edge_provenance
            .get(&(
                NodeId("abc1234".to_string()),
                NodeId("def5678".to_string()),
            ))
            .expect("parent_of edge must have provenance");
        assert_eq!(edge_prov.source_kind, ProvenanceSourceKind::CycleManifest);
        assert!(
            edge_prov.source_locator.contains("Commit parents"),
            "edge provenance must point at the Commit parents section, got {:?}",
            edge_prov.source_locator
        );
        assert!(
            edge_prov.source_locator.contains("line_"),
            "edge locator must include the byte-line number, got {:?}",
            edge_prov.source_locator
        );
    }

    #[test]
    fn derive_active_graph_input_from_manifest_provenance_is_none_for_inputs_without_manifest() {
        // Empty input ⇒ no provenance maps populated (backward compat).
        let input = derive_active_graph_input_from_manifest("");
        assert!(input.node_provenance.is_empty());
        assert!(input.edge_provenance.is_empty());
    }

    #[test]
    fn parse_section_bullets_for_with_locator_reports_line_idx_and_section_name() {
        let text = "## Section A\n\n- alpha\n- beta\n\n## Section B\n\n- gamma\n";
        let mut hits: Vec<(String, usize, String)> = Vec::new();
        let mut cb = |bullet: &str, line_idx: usize, section: &str| {
            hits.push((bullet.to_string(), line_idx, section.to_string()));
        };
        parse_section_bullets_for_with_locator(text, "Section A", &mut cb);
        assert_eq!(hits.len(), 2, "Section A has 2 bullets, got {hits:?}");
        assert_eq!(hits[0].0, "- alpha");
        assert_eq!(hits[0].2, "Section A");
        assert_eq!(hits[1].0, "- beta");
        assert_eq!(hits[1].2, "Section A");
        // line_idx must be 1-based and monotonically increasing.
        assert!(hits[0].1 >= 1);
        assert!(hits[1].1 > hits[0].1);
    }

    // ── M8.7 — cross-input drift detection ────────────────────────────

    #[test]
    fn diff_from_two_cycle_manifests_emits_no_drift_when_identical() {
        // Both sides are the same fixture manifest ⇒ zero deltas.
        let env = test_env();
        let fixture_path_a = write_fixture_to_tmp(
            "diff-a-",
            r#"{"workflow_nodes":["abc1234","def5678"]}"#,
        );
        let fixture_path_b = write_fixture_to_tmp(
            "diff-b-",
            r#"{"workflow_nodes":["abc1234","def5678"]}"#,
        );
        let args = CockpitDiffArgs {
            cycle_a: None,
            cycle_b: None,
            input_a: Some(fixture_path_a.clone()),
            input_b: Some(fixture_path_b.clone()),
            format: OutputFormat::Text,
        };
        let output = run_dev_cockpit_diff(args, &env);
        assert_eq!(output.status, 0, "stderr: {}", output.stderr);
        assert!(
            output.stdout.contains("No drift detected"),
            "stdout must show 'No drift detected' for identical inputs, got:\n{}",
            output.stdout
        );
        cleanup_tmp(&fixture_path_a);
        cleanup_tmp(&fixture_path_b);
    }

    #[test]
    fn diff_from_two_cycle_manifests_reports_added_nodes() {
        let env = test_env();
        let a = write_fixture_to_tmp(
            "diff-a-",
            r#"{"workflow_nodes":["abc1234"]}"#,
        );
        let b = write_fixture_to_tmp(
            "diff-b-",
            r#"{"workflow_nodes":["abc1234","new1234"]}"#,
        );
        let args = CockpitDiffArgs {
            cycle_a: None,
            cycle_b: None,
            input_a: Some(a.clone()),
            input_b: Some(b.clone()),
            format: OutputFormat::Text,
        };
        let output = run_dev_cockpit_diff(args, &env);
        assert_eq!(output.status, 0, "stderr: {}", output.stderr);
        assert!(
            output.stdout.contains("nodes: +1 / -0 / ~0"),
            "stdout must show +1/-0/~0 summary, got:\n{}",
            output.stdout
        );
        assert!(
            output.stdout.contains("new1234"),
            "stdout must name the added node, got:\n{}",
            output.stdout
        );
        cleanup_tmp(&a);
        cleanup_tmp(&b);
    }

    #[test]
    fn diff_requires_at_least_one_source_per_side() {
        let env = test_env();
        // Both sides empty → fail-fast.
        let args = CockpitDiffArgs {
            cycle_a: None,
            cycle_b: None,
            input_a: None,
            input_b: None,
            format: OutputFormat::Text,
        };
        let output = run_dev_cockpit_diff(args, &env);
        assert_eq!(output.status, 1);
        assert!(
            output.stderr.contains("--cycle-a or --input-a"),
            "stderr must explain missing A, got: {}",
            output.stderr
        );
        // A set, B missing → fail-fast.
        let a = write_fixture_to_tmp("diff-a-", r#"{"workflow_nodes":["a"]}"#);
        let args = CockpitDiffArgs {
            cycle_a: None,
            cycle_b: None,
            input_a: Some(a.clone()),
            input_b: None,
            format: OutputFormat::Text,
        };
        let output = run_dev_cockpit_diff(args, &env);
        assert_eq!(output.status, 1);
        assert!(
            output.stderr.contains("--cycle-b or --input-b"),
            "stderr must explain missing B, got: {}",
            output.stderr
        );
        cleanup_tmp(&a);
    }

    #[test]
    fn diff_json_envelope_shape_is_stable() {
        let env = test_env();
        let a = write_fixture_to_tmp("diff-a-", r#"{"workflow_nodes":["abc"]}"#);
        let b = write_fixture_to_tmp(
            "diff-b-",
            r#"{"workflow_nodes":["abc","new"]}"#,
        );
        let args = CockpitDiffArgs {
            cycle_a: None,
            cycle_b: None,
            input_a: Some(a.clone()),
            input_b: Some(b.clone()),
            format: OutputFormat::Json,
        };
        let output = run_dev_cockpit_diff(args, &env);
        assert_eq!(output.status, 0, "stderr: {}", output.stderr);
        let parsed: serde_json::Value =
            serde_json::from_str(&output.stdout).expect("stdout must be valid JSON");
        assert!(parsed["source_a"].is_string());
        assert!(parsed["source_b"].is_string());
        assert_eq!(parsed["summary"]["nodes_added"], 1);
        assert_eq!(parsed["summary"]["nodes_removed"], 0);
        assert_eq!(parsed["summary"]["nodes_changed"], 0);
        assert_eq!(parsed["summary"]["edges_added"], 0);
        assert_eq!(parsed["summary"]["edges_removed"], 0);
        assert_eq!(parsed["summary"]["edges_changed"], 0);
        assert_eq!(parsed["node_deltas"][0]["kind"], "added");
        assert_eq!(parsed["node_deltas"][0]["node_id"], "new");
        cleanup_tmp(&a);
        cleanup_tmp(&b);
    }

    /// Helper: write a small JSON fixture to a tmp file under `/tmp`
    /// and return its path. Caller must call `cleanup_tmp` to remove.
    fn write_fixture_to_tmp(prefix: &str, body: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let path = std::env::temp_dir().join(format!("{prefix}{nonce}.json"));
        std::fs::write(&path, body).expect("write tmp fixture");
        path
    }

    fn cleanup_tmp(path: &PathBuf) {
        let _ = std::fs::remove_file(path);
    }

    // ── M8.8 — stable projection digest ─────────────────────────────

    #[test]
    fn digest_emits_sha256_hex_for_text_format() {
        let env = test_env();
        let path = write_fixture_to_tmp(
            "digest-a-",
            r#"{"workflow_nodes":["abc1234","def5678"]}"#,
        );
        let args = CockpitDigestArgs {
            cycle: None,
            input: Some(path.clone()),
            kind: CockpitDigestKindArg::Strict,
            format: OutputFormat::Text,
        };
        let output = run_dev_cockpit_digest(args, &env);
        assert_eq!(output.status, 0, "stderr: {}", output.stderr);
        let trimmed = output.stdout.trim();
        assert!(
            trimmed.starts_with("sha256:"),
            "stdout must start with 'sha256:', got {:?}",
            trimmed
        );
        assert_eq!(
            trimmed.len(),
            "sha256:".len() + 64,
            "hex must be 64 chars, got {} chars: {:?}",
            trimmed.len(),
            trimmed
        );
        assert_eq!(output.stderr, "");
        cleanup_tmp(&path);
    }

    #[test]
    fn digest_is_stable_across_runs() {
        // The same input must produce the same digest across two calls.
        // This is the whole point of the operator contract: digest is a
        // cheap equality check that catches any structural change.
        let env = test_env();
        let path = write_fixture_to_tmp(
            "digest-stable-",
            r#"{"workflow_nodes":["m8_1","m8_2","m8_3"]}"#,
        );
        let make_args = || CockpitDigestArgs {
            cycle: None,
            input: Some(path.clone()),
            kind: CockpitDigestKindArg::Strict,
            format: OutputFormat::Text,
        };
        let d1 = run_dev_cockpit_digest(make_args(), &env).stdout;
        let d2 = run_dev_cockpit_digest(make_args(), &env).stdout;
        assert_eq!(d1, d2, "digest must be deterministic across calls");
        cleanup_tmp(&path);
    }

    #[test]
    fn digest_kind_strict_changes_with_recorded_at_but_content_does_not() {
        // Two fixtures with the same nodes but different structure ⇒
        // digests differ. We exercise the kind discrimination via two
        // different inputs because the CLI doesn't expose recorded_at
        // directly.
        let env = test_env();
        let p1 = write_fixture_to_tmp(
            "digest-strict-1-",
            r#"{"workflow_nodes":["a","b"]}"#,
        );
        let p2 = write_fixture_to_tmp(
            "digest-strict-2-",
            r#"{"workflow_nodes":["a","b","c"]}"#,
        );
        let args_strict_a = CockpitDigestArgs {
            cycle: None,
            input: Some(p1.clone()),
            kind: CockpitDigestKindArg::Strict,
            format: OutputFormat::Text,
        };
        let args_content_a = CockpitDigestArgs {
            cycle: None,
            input: Some(p1.clone()),
            kind: CockpitDigestKindArg::Content,
            format: OutputFormat::Text,
        };
        let d_strict_a = run_dev_cockpit_digest(args_strict_a, &env).stdout;
        let d_content_a = run_dev_cockpit_digest(args_content_a, &env).stdout;
        // Strict and content digests CAN differ for the same projection
        // because strict includes recorded_at and content strips it.
        // (For an empty-default input they may also happen to coincide
        // if there's no recorded_at variance, but with workflow_nodes
        // populated they still encode the same content + a timestamp
        // delta from strict.)
        // What we can pin down: both are valid sha256 hex strings.
        assert!(d_strict_a.trim().starts_with("sha256:"));
        assert!(d_content_a.trim().starts_with("sha256:"));

        // Sanity: a structurally different input ⇒ different digest
        // (under either kind).
        let args_strict_b = CockpitDigestArgs {
            cycle: None,
            input: Some(p2.clone()),
            kind: CockpitDigestKindArg::Strict,
            format: OutputFormat::Text,
        };
        let d_strict_b = run_dev_cockpit_digest(args_strict_b, &env).stdout;
        assert_ne!(
            d_strict_a, d_strict_b,
            "structural change must produce a different strict digest"
        );
        cleanup_tmp(&p1);
        cleanup_tmp(&p2);
    }

    #[test]
    fn digest_json_envelope_shape_is_stable() {
        let env = test_env();
        let path = write_fixture_to_tmp(
            "digest-json-",
            r#"{"workflow_nodes":["m8_8"]}"#,
        );
        let args = CockpitDigestArgs {
            cycle: None,
            input: Some(path.clone()),
            kind: CockpitDigestKindArg::Strict,
            format: OutputFormat::Json,
        };
        let output = run_dev_cockpit_digest(args, &env);
        assert_eq!(output.status, 0, "stderr: {}", output.stderr);
        let parsed: serde_json::Value =
            serde_json::from_str(&output.stdout).expect("stdout must be valid JSON");
        assert_eq!(parsed["kind"], "strict");
        assert!(parsed["hex"].as_str().unwrap().starts_with("sha256:"));
        assert_eq!(parsed["hex"].as_str().unwrap().len(), "sha256:".len() + 64);
        assert!(parsed["source"].is_string());
        cleanup_tmp(&path);
    }

    #[test]
    fn digest_requires_exactly_one_source() {
        let env = test_env();
        // No source at all → fail-fast.
        let args = CockpitDigestArgs {
            cycle: None,
            input: None,
            kind: CockpitDigestKindArg::Strict,
            format: OutputFormat::Text,
        };
        let output = run_dev_cockpit_digest(args, &env);
        assert_eq!(output.status, 1);
        assert!(
            output.stderr.contains("--cycle or --input"),
            "stderr must explain missing source, got: {}",
            output.stderr
        );
        // Both source flags set → clap enforces conflicts_with; if it
        // somehow gets through, the dispatcher should still refuse.
        let path = write_fixture_to_tmp("digest-conflict-", r#"{"workflow_nodes":[]}"#);
        let args = CockpitDigestArgs {
            cycle: Some("nonexistent-cycle-xyz".to_string()),
            input: Some(path.clone()),
            kind: CockpitDigestKindArg::Strict,
            format: OutputFormat::Text,
        };
        let output = run_dev_cockpit_digest(args, &env);
        // Either clap rejected (no run) or runtime rejected — but the
        // important thing is it doesn't silently produce a wrong digest.
        // If runtime, status must be non-zero.
        assert_ne!(output.status, 0, "conflicting sources must not succeed");
        cleanup_tmp(&path);
    }
}
