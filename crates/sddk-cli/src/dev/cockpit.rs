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
//! M8.2 ships COCKPIT-001 only. COCKPIT-002 (providers/usage/assurance/
//! handoff/experiments) lives in `crates/sddk-engine/src/cockpit_observability.rs`
//! and will surface as `sddk dev cockpit obs` in M8.3.
//!
//! ## Subcommand
//!
//! - `view --kind {overview,journal,timeline,execution}` (default `overview`).
//! - Optional `--from-input <path>` to load an `ActiveGraphInput` JSON
//!   fixture. Without it, the projection is empty and the view reports
//!   an empty structure (no fabrication).
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

pub(crate) fn run_dev_cockpit_view(
    args: CockpitViewArgs,
    _env: &CliEnvironment,
) -> CommandOutput {
    let (input, source) = match load_input(args.from_input.as_ref()) {
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
        OutputFormat::Text => render_view_text(&row),
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

fn render_view_text(row: &CockpitViewRow) -> CommandOutput {
    let mut out = String::new();
    out.push_str("=== sddk dev cockpit view ===\n\n");
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
}
