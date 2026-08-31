//! Control plane local de telemetría: store SQLite central, ingest
//! cross-proyecto, agregación y estado (ADR-0009/ADR-0010, milestone
//! CP-2026-08). Sin componente MCP.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use serde::Serialize;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat,
    metrics::{self, MetricsWindow},
    render_result,
};
use sddk_domain::{ControlPlane, UatResultRow};
use sddk_storage::SqliteControlPlane;

/// SQLite store file of the control plane.
const CONTROL_PLANE_DB: &str = "control-plane.sqlite";
/// Dashboard output default path.
const DASHBOARD_HTML: &str = "dashboard.html";

/// Schema v1 of the control plane store (ADR-0009 §4).
// `dead_code` allow: pre-existing schema constant retained for future
// schema migrations; tracked for cleanup in phase2-hygiene-baseline.
#[allow(dead_code)]
const SCHEMA_V1: &str = r#"
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS projects (
    project_id   TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    scope        TEXT NOT NULL DEFAULT '.',
    remote_url   TEXT,
    first_seen   TEXT NOT NULL,
    last_seen    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS cycles (
    cycle_id                  TEXT PRIMARY KEY,
    project_id                TEXT NOT NULL REFERENCES projects(project_id),
    path                      TEXT NOT NULL DEFAULT 'unknown',
    context_quality           TEXT NOT NULL DEFAULT 'C2',
    phase_durations_sec       TEXT NOT NULL DEFAULT '{}',
    coherence_scores          TEXT NOT NULL DEFAULT '[]',
    correction_cycles         INTEGER NOT NULL DEFAULT 0,
    tokens_used               INTEGER NOT NULL DEFAULT 0,
    cost_estimate_usd         REAL NOT NULL DEFAULT 0.0,
    costs                     TEXT NOT NULL DEFAULT '{}',
    first_pass_success        INTEGER NOT NULL DEFAULT 0,
    verify_verdict            TEXT NOT NULL DEFAULT 'UNKNOWN',
    merged_to_main            INTEGER NOT NULL DEFAULT 0,
    tag_version               TEXT,
    lead_time_hours           REAL,
    teleological_coherence_pct REAL,
    recorded_at               TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS aggregates (
    window_days   INTEGER NOT NULL,
    computed_at   TEXT NOT NULL,
    payload_json  TEXT NOT NULL,
    PRIMARY KEY (window_days)
);

CREATE INDEX IF NOT EXISTS idx_cycles_project ON cycles(project_id);
CREATE INDEX IF NOT EXISTS idx_cycles_recorded ON cycles(recorded_at);

CREATE TABLE IF NOT EXISTS uat_results (
    project_id     TEXT NOT NULL REFERENCES projects(project_id),
    tag_version    TEXT NOT NULL,
    verdict        TEXT NOT NULL,           -- READY|READY_WITH_RISKS|NOT_READY
    coverage_pct   REAL NOT NULL DEFAULT 0,
    defects        INTEGER NOT NULL DEFAULT 0,
    session_count  INTEGER NOT NULL DEFAULT 0,
    uat_duration_minutes INTEGER NOT NULL DEFAULT 0,
    recorded_at    TEXT NOT NULL,
    PRIMARY KEY (project_id, tag_version)
);
"#;

#[derive(Debug, Subcommand)]
pub(crate) enum TelemetryCommand {
    /// Ingest telemetry from all adopted projects into the central store.
    Ingest(TelemetryIngestArgs),
    /// Compute cross-project aggregates over a window.
    Aggregate(TelemetryAggregateArgs),
    /// Show per-project coverage and data-gap summary.
    Status(TelemetryStatusArgs),
    /// Generate the self-contained HTML dashboard.
    Dashboard(TelemetryDashboardArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct TelemetryIngestArgs {
    /// Show the plan without writing.
    #[arg(long)]
    pub(crate) dry_run: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct TelemetryAggregateArgs {
    /// Aggregation window.
    #[arg(long, value_enum, default_value_t = MetricsWindow::SevenDays)]
    pub(crate) window: MetricsWindow,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct TelemetryStatusArgs {
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct TelemetryDashboardArgs {
    /// Output HTML path.
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Resolve the control plane directory under the XDG data root.
fn control_plane_dir(environment: &CliEnvironment) -> anyhow::Result<PathBuf> {
    let data_home = if let Some(dir) = &environment.sddk_data_dir {
        dir.clone()
    } else {
        match (&environment.data_home, &environment.home) {
            (Some(data), _) => data.clone(),
            (None, Some(home)) => home.join(".local/share"),
            (None, None) => dirs::data_dir().ok_or_else(|| {
                anyhow::anyhow!("no data root: set HOME, XDG_DATA_HOME or SDDK_DATA_DIR")
            })?,
        }
    };
    if !data_home.is_absolute() {
        anyhow::bail!("data root must be absolute: {data_home:?}");
    }
    Ok(data_home.join("sddk/control-plane"))
}

/// Whether the control plane store file exists (without creating it).
pub(crate) fn store_exists(environment: &CliEnvironment) -> bool {
    match control_plane_dir(environment) {
        Ok(dir) => dir.join(CONTROL_PLANE_DB).is_file(),
        Err(_) => false,
    }
}

/// Open (and initialize) the central SQLite store.
pub(crate) fn open_store(
    environment: &CliEnvironment,
    dry_run: bool,
) -> anyhow::Result<Box<dyn ControlPlane>> {
    let dir = control_plane_dir(environment)?;
    if dry_run {
        // Validate the dir is reachable without creating it.
        let parent = dir
            .parent()
            .ok_or_else(|| anyhow::anyhow!("control plane dir has no parent"))?;
        if !parent.exists() {
            anyhow::bail!("data root does not exist: {}", parent.display());
        }
        let plane = SqliteControlPlane::open_in_memory().map_err(anyhow::Error::from)?;
        return Ok(Box::new(plane));
    }
    let plane = SqliteControlPlane::open(&dir).map_err(anyhow::Error::from)?;
    Ok(Box::new(plane))
}

/// A project discovered under `~/.local/share/sddk/projects/<id>/`.
struct DiscoveredProject {
    project_id: String,
    display_name: String,
    scope: String,
    remote_url: Option<String>,
    dir: PathBuf,
}

/// Scan the projects root for adopted workspaces.
fn discover_projects(environment: &CliEnvironment) -> anyhow::Result<Vec<DiscoveredProject>> {
    let data_home = if let Some(dir) = &environment.sddk_data_dir {
        dir.clone()
    } else {
        match (&environment.data_home, &environment.home) {
            (Some(data), _) => data.clone(),
            (None, Some(home)) => home.join(".local/share"),
            (None, None) => return Ok(Vec::new()),
        }
    };
    let projects_root = data_home.join("sddk/projects");
    if !projects_root.exists() {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    for entry in std::fs::read_dir(&projects_root)? {
        let dir = entry?.path();
        if !dir.is_dir() {
            continue;
        }
        let project_id = dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_owned();
        // adoption.json lives at workspaces/*/adoption.json or at the project
        // root on older layouts; prefer the deepest receipts.
        let receipts: Vec<PathBuf> = walkdir_limited(&dir, 4)
            .into_iter()
            .filter(|path| {
                path.file_name()
                    .map(|n| n == "adoption.json")
                    .unwrap_or(false)
            })
            .collect();
        let Some(receipt_path) = receipts.first() else {
            continue;
        };
        let receipt = match read_adoption_receipt(receipt_path) {
            Ok(receipt) => receipt,
            Err(_) => continue,
        };
        found.push(DiscoveredProject {
            project_id: receipt.project_id.clone().unwrap_or(project_id.clone()),
            display_name: receipt.display_name.clone(),
            scope: receipt.scope.clone(),
            remote_url: receipt.remote_url.clone(),
            dir,
        });
    }
    Ok(found)
}

/// Adoption receipt subset read from a project.
struct AdoptionReceipt {
    project_id: Option<String>,
    display_name: String,
    scope: String,
    remote_url: Option<String>,
}

fn read_adoption_receipt(path: &Path) -> anyhow::Result<AdoptionReceipt> {
    let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    Ok(AdoptionReceipt {
        project_id: value
            .get("project_id")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        display_name: value
            .get("display_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_owned(),
        scope: value
            .get("scope")
            .and_then(|v| v.as_str())
            .unwrap_or(".")
            .to_owned(),
        remote_url: value
            .get("remote_url")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
    })
}

/// Shallow walk helper: yield files up to `max_depth` levels deep.
fn walkdir_limited(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    use std::collections::VecDeque;
    let mut out = Vec::new();
    let mut queue: VecDeque<(PathBuf, usize)> = VecDeque::new();
    queue.push_back((root.to_path_buf(), 0));
    while let Some((dir, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                queue.push_back((path, depth + 1));
            } else {
                out.push(path);
            }
        }
    }
    out
}

/// Metrics record rows read from a project's metrics.jsonl.
fn read_project_metrics(project: &DiscoveredProject) -> Vec<sddk_domain::MetricsRecord> {
    let jsonl = project.dir.join("metrics/metrics.jsonl");
    if !jsonl.exists() {
        return Vec::new();
    }
    let Ok(content) = std::fs::read_to_string(&jsonl) else {
        return Vec::new();
    };
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<sddk_domain::MetricsRecord>(line).ok())
        .collect()
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct IngestOutput {
    projects_seen: u32,
    cycles_ingested: u32,
    cycles_derived: u32,
    duplicates_skipped: u32,
    dry_run: bool,
}

fn run_telemetry_ingest(args: TelemetryIngestArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<IngestOutput> {
        let projects = discover_projects(environment)?;
        let mut plane = open_store(environment, args.dry_run)?;
        let now = now_rfc3339()?;
        let mut output = IngestOutput {
            projects_seen: 0,
            cycles_ingested: 0,
            cycles_derived: 0,
            duplicates_skipped: 0,
            dry_run: args.dry_run,
        };
        for project in &projects {
            output.projects_seen += 1;
            upsert_project(&mut *plane, project, &now)?;
            let records = read_project_metrics(project);
            let mut seen: BTreeMap<String, ()> = BTreeMap::new();
            for record in &records {
                if seen.insert(record.cycle_id.clone(), ()).is_some() {
                    output.duplicates_skipped += 1;
                    continue;
                }
                upsert_cycle(&mut *plane, project, record)?;
                output.cycles_ingested += 1;
            }
            // Derive records for cycles present in the ledger but absent from
            // metrics.jsonl (reuses the same derivation as metrics backfill).
            let state_home = std::env::var_os("XDG_STATE_HOME")
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state"))
                });
            let ledger_path = state_home.map(|sh| {
                sh.join("sddk/projects")
                    .join(&project.project_id)
                    .join("ledger.sqlite")
            });
            let derived = if let Some(path) = ledger_path {
                match crate::Storage::open_read_only(&path) {
                    Ok(storage) => derive_ledger_cycles(project, &records, &storage),
                    Err(_) => Vec::new(),
                }
            } else {
                Vec::new()
            };
            for (cycle_id, record) in &derived {
                if seen.insert(cycle_id.clone(), ()).is_some() {
                    continue;
                }
                upsert_cycle(&mut *plane, project, record)?;
                output.cycles_derived += 1;
            }
        }
        Ok(output)
    })();
    render_result(result, format, ingest_text)
}

fn ingest_text(output: &IngestOutput) -> String {
    format!(
        "projects_seen: {}\ncycles_ingested: {}\ncycles_derived: {}\nduplicates_skipped: {}\ndry_run: {}\n",
        output.projects_seen,
        output.cycles_ingested,
        output.cycles_derived,
        output.duplicates_skipped,
        output.dry_run
    )
}

fn upsert_project(
    plane: &mut dyn ControlPlane,
    project: &DiscoveredProject,
    now: &str,
) -> anyhow::Result<()> {
    plane
        .upsert_project(
            &project.project_id,
            &project.display_name,
            &project.scope,
            project.remote_url.as_deref(),
            now,
        )
        .map_err(anyhow::Error::from)?;
    Ok(())
}

fn upsert_cycle(
    plane: &mut dyn ControlPlane,
    project: &DiscoveredProject,
    record: &sddk_domain::MetricsRecord,
) -> anyhow::Result<()> {
    plane
        .upsert_cycle(&project.project_id, record)
        .map_err(anyhow::Error::from)?;
    Ok(())
}

/// Upsert a UAT aggregate into the control plane (ADR-012: the CP stores
/// only the numeric rollup; sessions/evidence stay in XDG artifacts).
// `dead_code` allow: retained as API surface for future use;
/// tracked for cleanup in phase2-hygiene-baseline.
#[allow(dead_code)]
pub(crate) fn upsert_uat_result(
    plane: &mut dyn ControlPlane,
    result: &UatResultRow,
) -> anyhow::Result<()> {
    plane
        .upsert_uat_result(result)
        .map_err(anyhow::Error::from)?;
    Ok(())
}

/// Load UAT aggregates for the readiness panel (ADR-013).
pub(crate) fn load_uat_results(plane: &dyn ControlPlane) -> anyhow::Result<Vec<UatResultRow>> {
    plane.load_uat_results().map_err(anyhow::Error::from)
}

/// Derive metrics records for cycles only present in the project ledger.
fn derive_ledger_cycles(
    _project: &DiscoveredProject,
    existing: &[sddk_domain::MetricsRecord],
    ledger: &dyn sddk_domain::Ledger,
) -> Vec<(String, sddk_domain::MetricsRecord)> {
    let existing_ids: std::collections::HashSet<&str> = existing
        .iter()
        .map(|record| record.cycle_id.as_str())
        .collect();
    let events = read_ledger_events(ledger);
    // Group events by cycle.
    let mut by_cycle: BTreeMap<String, Vec<sddk_domain::LedgerEvent>> = BTreeMap::new();
    for event in events {
        if let Some(cycle_id) = &event.cycle_id {
            by_cycle.entry(cycle_id.clone()).or_default().push(event);
        }
    }
    let mut out = Vec::new();
    for (cycle_id, events) in by_cycle {
        if existing_ids.contains(cycle_id.as_str()) {
            continue;
        }
        let derived = metrics::derive_from_events(&events);
        let now = now_rfc3339().unwrap_or_else(|_| "unknown".to_owned());
        let path = events
            .iter()
            .rev()
            .find_map(|event| {
                event
                    .state_after
                    .as_ref()
                    .and_then(|state| state.get("path"))
                    .and_then(|value| value.as_str())
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| "unknown".to_owned());
        out.push((
            cycle_id.clone(),
            sddk_domain::MetricsRecord {
                cycle_id,
                path,
                context_quality: "C2".to_owned(),
                phase_durations_sec: derived.phase_durations_sec,
                coherence_scores: Vec::new(),
                correction_cycles: derived.correction_cycles,
                tokens_used: 0,
                cost_estimate_usd: 0.0,
                first_pass_success: derived.first_pass_success,
                verify_verdict: derived.verify_verdict,
                merged_to_main: derived.merged_to_main,
                tag_version: derived.tag_version,
                lead_time_hours: derived.lead_time_hours,
                teleological_coherence_pct: None,
                costs: std::collections::HashMap::new(),
                recorded_at: now,
            },
        ));
    }
    out
}

/// Read all ledger events from a project ledger SQLite file.
fn read_ledger_events(ledger: &dyn sddk_domain::Ledger) -> Vec<sddk_domain::LedgerEvent> {
    ledger.load_all_ledger_events().unwrap_or_default()
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct AggregateOutput {
    window_days: u16,
    aggregate: sddk_domain::MetricsAggregate,
    tuning: sddk_domain::F3Tuning,
}

fn run_telemetry_aggregate(
    args: TelemetryAggregateArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<AggregateOutput> {
        let mut plane = open_store(environment, false)?;
        let records = load_cycles(&*plane)?;
        let aggregate = metrics::compute_aggregate(&records, args.window.days());
        let tuning = metrics::tuning_from_aggregate(&aggregate);
        let now = now_rfc3339()?;
        let payload = serde_json::to_string(&aggregate)?;
        plane
            .upsert_aggregate(args.window.days(), &now, &payload)
            .map_err(anyhow::Error::from)?;
        Ok(AggregateOutput {
            window_days: args.window.days(),
            aggregate,
            tuning,
        })
    })();
    render_result(result, format, aggregate_output_text)
}

fn aggregate_output_text(output: &AggregateOutput) -> String {
    format!(
        "{}\ntuning: {}",
        metrics::aggregate_text(&output.aggregate),
        metrics::tuning_text(&output.tuning)
    )
}

/// Load all cycle records from the central store.
pub(crate) fn load_cycles(
    plane: &dyn ControlPlane,
) -> anyhow::Result<Vec<sddk_domain::MetricsRecord>> {
    plane.load_cycles().map_err(anyhow::Error::from)
}

/// Status of one project in the control plane.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ProjectStatus {
    project_id: String,
    display_name: String,
    cycles: u32,
    cycles_with_cost: u32,
    cycles_with_coherence: u32,
    last_ingest: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct StatusOutput {
    projects: Vec<ProjectStatus>,
    total_cycles: u32,
    data_gaps: Vec<String>,
}

fn run_telemetry_status(args: TelemetryStatusArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<StatusOutput> {
        let dir = control_plane_dir(environment)?;
        let plane = SqliteControlPlane::open(&dir).map_err(anyhow::Error::from)?;
        let raw_status = plane.load_project_status()?;
        let mut projects = Vec::new();
        let mut total_cycles = 0u32;
        let mut gaps = Vec::new();
        for (
            project_id,
            display_name,
            cycles,
            cycles_with_cost,
            cycles_with_coherence,
            last_ingest,
        ) in raw_status
        {
            total_cycles += cycles;
            if cycles_with_cost == 0 && cycles > 0 {
                gaps.push(format!(
                    "{}: 0/{} cycles with cost data",
                    display_name, cycles
                ));
            }
            if cycles_with_coherence == 0 && cycles > 0 {
                gaps.push(format!(
                    "{}: 0/{} cycles with teleological coherence",
                    display_name, cycles
                ));
            }
            projects.push(ProjectStatus {
                project_id,
                display_name,
                cycles,
                cycles_with_cost,
                cycles_with_coherence,
                last_ingest,
            });
        }
        Ok(StatusOutput {
            projects,
            total_cycles,
            data_gaps: gaps,
        })
    })();
    render_result(result, format, status_text)
}

fn status_text(output: &StatusOutput) -> String {
    let mut text = format!("total_cycles: {}\n", output.total_cycles);
    for project in &output.projects {
        text.push_str(&format!(
            "- {} [{}] cycles={} cost={} coherence={} last={}\n",
            project.display_name,
            project.project_id,
            project.cycles,
            project.cycles_with_cost,
            project.cycles_with_coherence,
            project.last_ingest.as_deref().unwrap_or("n/a"),
        ));
    }
    if output.data_gaps.is_empty() {
        text.push_str("data_gaps: none\n");
    } else {
        text.push_str("data_gaps:\n");
        for gap in &output.data_gaps {
            text.push_str(&format!("- {gap}\n"));
        }
    }
    text
}

fn run_telemetry_dashboard(
    args: TelemetryDashboardArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<PathBuf> {
        let plane = open_store(environment, false)?;
        let records = load_cycles(&*plane)?;
        let uat_rows = load_uat_results(&*plane)?;
        let aggregate_7d = metrics::compute_aggregate(&records, 7);
        let aggregate_30d = metrics::compute_aggregate(&records, 30);
        let tuning = metrics::tuning_from_aggregate(&aggregate_30d);
        let html =
            render_dashboard_html(&records, &aggregate_7d, &aggregate_30d, &tuning, &uat_rows);
        let dir = control_plane_dir(environment)?;
        std::fs::create_dir_all(&dir)?;
        let path = args
            .output
            .clone()
            .unwrap_or_else(|| dir.join(DASHBOARD_HTML));
        std::fs::write(&path, html)?;
        Ok(path)
    })();
    render_result(result, format, |path| {
        format!("dashboard written: {}\n", path.display())
    })
}

/// Render the self-contained HTML dashboard (ADR-0010).
///
/// Datasets are embedded as inline JSON; there are NO external URLs, CDN
/// references or fetch calls. The output is deterministic for a given store.
fn render_dashboard_html(
    records: &[sddk_domain::MetricsRecord],
    aggregate_7d: &sddk_domain::MetricsAggregate,
    aggregate_30d: &sddk_domain::MetricsAggregate,
    tuning: &sddk_domain::F3Tuning,
    uat_results: &[UatResultRow],
) -> String {
    let cycles_json = serde_json::to_string_pretty(&records).unwrap_or_else(|_| "[]".into());
    let agg7_json = serde_json::to_string_pretty(aggregate_7d).unwrap_or_else(|_| "{}".into());
    let agg30_json = serde_json::to_string_pretty(aggregate_30d).unwrap_or_else(|_| "{}".into());
    let tuning_json = serde_json::to_string_pretty(tuning).unwrap_or_else(|_| "{}".into());
    let uat_json = serde_json::to_string_pretty(uat_results).unwrap_or_else(|_| "[]".into());
    let generated_at = now_rfc3339().unwrap_or_else(|_| "unknown".into());
    let (sample, first_pass, lead_time, cost, bottleneck) = aggregate_summary(aggregate_30d);
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>SDDK Control Plane</title>
<style>
  :root {{ --bg:#0f1115; --card:#181b21; --text:#e6e6e6; --muted:#9aa0a6; --accent:#4da3ff; --ok:#4caf50; --warn:#ffb74d; }}
  * {{ box-sizing:border-box; }}
  body {{ margin:0; font-family:ui-sans-serif,system-ui,-apple-system,Segoe UI,Roboto,sans-serif; background:var(--bg); color:var(--text); padding:2rem; }}
  h1 {{ font-size:1.4rem; margin:0 0 .25rem; }}
  h2 {{ font-size:1rem; margin:1.5rem 0 .5rem; color:var(--muted); text-transform:uppercase; letter-spacing:.05em; }}
  .sub {{ color:var(--muted); font-size:.8rem; margin-bottom:1rem; }}
  .kpis {{ display:grid; grid-template-columns:repeat(auto-fit,minmax(150px,1fr)); gap:.75rem; }}
  .kpi {{ background:var(--card); border:1px solid #23272e; border-radius:8px; padding:.9rem; }}
  .kpi .v {{ font-size:1.3rem; font-weight:600; }}
  .kpi .l {{ color:var(--muted); font-size:.72rem; text-transform:uppercase; letter-spacing:.05em; }}
  table {{ width:100%; border-collapse:collapse; background:var(--card); border-radius:8px; overflow:hidden; font-size:.8rem; }}
  th,td {{ padding:.45rem .6rem; text-align:left; border-bottom:1px solid #23272e; }}
  th {{ color:var(--muted); font-weight:500; text-transform:uppercase; font-size:.68rem; letter-spacing:.05em; }}
  tr:last-child td {{ border-bottom:none; }}
  .pass {{ color:var(--ok); }} .warn {{ color:var(--warn); }}
  .gap {{ color:var(--warn); font-size:.8rem; }}
  footer {{ margin-top:2rem; color:var(--muted); font-size:.7rem; }}
</style>
</head>
<body>
<h1>SDDK Control Plane</h1>
<div class="sub">generated {generated_at} &middot; local-first, no network</div>

<h2>KPIs (30d)</h2>
<div class="kpis">
  <div class="kpi"><div class="v">{sample}</div><div class="l">cycles</div></div>
  <div class="kpi"><div class="v">{first_pass}</div><div class="l">first pass</div></div>
  <div class="kpi"><div class="v">{lead_time}</div><div class="l">median lead time (h)</div></div>
  <div class="kpi"><div class="v">${cost}</div><div class="l">median cost</div></div>
  <div class="kpi"><div class="v">{bottleneck}</div><div class="l">bottleneck phase</div></div>
</div>

<h2>Trends</h2>
<table id="trends"></table>

<h2>Cycles</h2>
<table id="cycles"></table>

<h2>UAT readiness</h2>
<table id="uat"></table>

<h2>Data gaps</h2>
<div id="gaps" class="gap"></div>

<footer>sddk telemetry dashboard &middot; datasets embedded inline</footer>

<script>
const CYCLES = {cycles_json};
const AGG7 = {agg7_json};
const AGG30 = {agg30_json};
const TUNING = {tuning_json};
const UAT = {uat_json};

function fmtMoney(v) {{ return (v == null || v === 0) ? "n/a" : v.toFixed(2); }}
function fmtLead(v) {{ return (v == null) ? "n/a" : v.toFixed(2); }}

const trends = [
  ["window", "7d", "30d"],
  ["sample", AGG7.sample_size, AGG30.sample_size],
  ["first pass rate", (AGG7.first_pass_success_rate*100).toFixed(0)+"%", (AGG30.first_pass_success_rate*100).toFixed(0)+"%"],
  ["median lead time (h)", fmtLead(AGG7.median_lead_time_hours), fmtLead(AGG30.median_lead_time_hours)],
  ["median cost ($)", fmtMoney(AGG7.median_cost_usd), fmtMoney(AGG30.median_cost_usd)],
  ["bottleneck", AGG7.top_bottleneck_phase || "n/a", AGG30.top_bottleneck_phase || "n/a"],
];
document.getElementById("trends").innerHTML = trends.map(row =>
  "<tr>" + row.map((c,i) => i===0 ? "<th>"+c+"</th>" : "<td>"+c+"</td>").join("") + "</tr>"
).join("");

const cols = ["cycle_id","project_id","path","verify_verdict","first_pass_success","merged_to_main","tag_version","lead_time_hours","cost_estimate_usd","teleological_coherence_pct","tokens_used"];
const sorted = [...CYCLES].sort((a,b) => (b.recorded_at||"").localeCompare(a.recorded_at||""));
document.getElementById("cycles").innerHTML =
  "<tr>" + cols.map(c => "<th>"+c.replace(/_/g," ")+"</th>").join("") + "</tr>" +
  sorted.map(r => "<tr>" + cols.map(c => {{
    const v = r[c];
    let s = (v === null || v === undefined) ? "" : String(v);
    if (c === "cost_estimate_usd") s = fmtMoney(v);
    if (c === "lead_time_hours") s = fmtLead(v);
    if (c === "verify_verdict") s = v === "PASS" ? '<span class="pass">PASS</span>' : '<span class="warn">'+v+'</span>';
    return "<td>"+s+"</td>";
  }}).join("") + "</tr>").join("");

const gaps = [];
for (const r of CYCLES) {{
  if (!r.cost_estimate_usd) gaps.push(r.cycle_id + ": no cost data");
  if (r.teleological_coherence_pct == null) gaps.push(r.cycle_id + ": no coherence data");
}}
document.getElementById("gaps").innerHTML = gaps.length
  ? gaps.slice(0, 50).map(g => "<div>"+g+"</div>").join("")
  : "none";

const uatCols = ["project_id","tag_version","verdict","coverage_pct","defects","session_count","uat_duration_minutes"];
document.getElementById("uat").innerHTML = UAT.length === 0
  ? "<tr><td>no UAT results yet</td></tr>"
  : "<tr>" + uatCols.map(c => "<th>"+c.replace(/_/g," ")+"</th>").join("") + "</tr>" +
    UAT.map(r => "<tr>" + uatCols.map(c => {{
      const v = r[c];
      let s = (v === null || v === undefined) ? "" : String(v);
      if (c === "verdict") s = v === "READY" ? '<span class="pass">READY</span>' : '<span class="warn">'+v+'</span>';
      if (c === "coverage_pct") s = v + "%";
      return "<td>"+s+"</td>";
    }}).join("") + "</tr>").join("");
</script>
</body>
</html>
"#,
        generated_at = generated_at,
        sample = sample,
        first_pass = first_pass,
        lead_time = lead_time,
        cost = cost,
        bottleneck = bottleneck,
        cycles_json = cycles_json,
        agg7_json = agg7_json,
        agg30_json = agg30_json,
        tuning_json = tuning_json,
    )
}

/// Extract the headline numbers for the KPI row.
fn aggregate_summary(
    aggregate: &sddk_domain::MetricsAggregate,
) -> (u32, String, String, String, String) {
    (
        aggregate.sample_size,
        format!("{:.0}%", aggregate.first_pass_success_rate * 100.0),
        aggregate
            .median_lead_time_hours
            .map(|value| format!("{value:.2}"))
            .unwrap_or_else(|| "n/a".into()),
        aggregate
            .median_cost_usd
            .map(|value| format!("{value:.2}"))
            .unwrap_or_else(|| "n/a".into()),
        aggregate
            .top_bottleneck_phase
            .clone()
            .unwrap_or_else(|| "n/a".into()),
    )
}

fn now_rfc3339() -> anyhow::Result<String> {
    Ok(time::OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?)
}

pub(crate) fn run_telemetry(
    command: TelemetryCommand,
    environment: &CliEnvironment,
) -> CommandOutput {
    match command {
        TelemetryCommand::Ingest(args) => run_telemetry_ingest(args, environment),
        TelemetryCommand::Aggregate(args) => run_telemetry_aggregate(args, environment),
        TelemetryCommand::Status(args) => run_telemetry_status(args, environment),
        TelemetryCommand::Dashboard(args) => run_telemetry_dashboard(args, environment),
    }
}
