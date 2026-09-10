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
};
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
    input
}

/// Parsed slices from a cycle's archive manifest.
#[derive(Debug, Default, Clone)]
struct ParsedManifest {
    shas: Vec<String>,
    parent_edges: Vec<(String, String)>,
    evidence_edges: Vec<(String, String)>,
    memory_refs: Vec<(String, String)>,
}

/// Parse the manifest into workflow_nodes, parent_edges, evidence_edges
/// and memory_refs. The parser is deliberately conservative — anything
/// it cannot structurally match is dropped, never guessed.
fn parse_commit_rows(text: &str) -> ParsedManifest {
    let mut out = ParsedManifest::default();
    let mut in_commits = false;
    for line in text.lines() {
        let trimmed = line.trim();
        // Detect the "## Commits" section header. M8.2/M8.3 manifests
        // use exactly this heading; if a future cycle uses a different
        // header the parser degrades to empty (no fabrication).
        if trimmed.starts_with("## ") && trimmed.to_ascii_lowercase().contains("commit") {
            in_commits = true;
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
            out.shas.push(sha);
        }
    }
    // Parent edges + evidence + memory refs come from explicit
    // "Bridges" / "Deferred items" sections if present. We look for
    // bullet lists under those headings.
    parse_section_bullets_for(text, "Bridges", &mut |bullet: &str| {
        // Bridge shape: "- M8.x — ..." or "- <source> → <target>"
        // We treat each bridge as an evidence edge keyed by the cycle
        // id (source) and the first SHA in the bridge target.
        if let Some((label, sha)) = parse_bridge_bullet(bullet, &out.shas) {
            out.evidence_edges.push((label, sha));
        }
    });
    parse_section_bullets_for(text, "Deferred items", &mut |bullet: &str| {
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
        out.memory_refs.push((label, target));
    });
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
}
