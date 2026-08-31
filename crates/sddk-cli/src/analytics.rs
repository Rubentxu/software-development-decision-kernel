//! Analytics commands: report, trends, bottleneck, and research packets.

use std::collections::HashMap;

use clap::{Args, Subcommand};
use sddk_domain::{MetricsAggregate, MetricsRecord};
use serde::Serialize;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat, RuntimeArgs, RuntimeContext, render_result,
};

use crate::metrics::{
    MetricsWindow, compute_aggregate, read_records, tuning_from_aggregate, window_records,
};

#[derive(Debug, Subcommand)]
pub(crate) enum AnalyticsCommand {
    /// Show the current rolling aggregate report.
    Report(AnalyticsWindowArgs),
    /// Show per-window trends (7d vs 30d).
    Trends(AnalyticsWindowArgs),
    /// Show the top bottleneck phase and its cost impact.
    Bottleneck(AnalyticsWindowArgs),
    /// Emit a structured research packet for the self-research agents.
    Research(AnalyticsResearchArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct AnalyticsWindowArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Aggregation window.
    #[arg(long, value_enum, default_value_t = MetricsWindow::SevenDays)]
    pub(crate) window: MetricsWindow,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Arguments for the research command (cross-project control plane support).
#[derive(Debug, Clone, Args)]
pub(crate) struct AnalyticsResearchArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Aggregation window.
    #[arg(long, value_enum, default_value_t = MetricsWindow::SevenDays)]
    pub(crate) window: MetricsWindow,
    /// Include ALL projects from the control plane store (ADR-0009) instead of
    /// only the current project.
    #[arg(long)]
    pub(crate) all_projects: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct TrendsOutput {
    window_7d: MetricsAggregate,
    window_30d: MetricsAggregate,
}

pub(crate) fn run_analytics(
    command: AnalyticsCommand,
    environment: &CliEnvironment,
) -> CommandOutput {
    match command {
        AnalyticsCommand::Report(args) => run_analytics_report(args, environment),
        AnalyticsCommand::Trends(args) => run_analytics_trends(args, environment),
        AnalyticsCommand::Bottleneck(args) => run_analytics_bottleneck(args, environment),
        AnalyticsCommand::Research(args) => run_analytics_research(args, environment),
    }
}

/// Per-cycle summary inside a research packet.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct CycleSummary {
    cycle_id: String,
    path: String,
    verdict: String,
    merged: bool,
    lead_time_hours: Option<f64>,
    tag_version: Option<String>,
    phase_durations_sec: HashMap<String, u64>,
}

impl From<&MetricsRecord> for CycleSummary {
    fn from(record: &MetricsRecord) -> Self {
        Self {
            cycle_id: record.cycle_id.clone(),
            path: record.path.clone(),
            verdict: record.verify_verdict.clone(),
            merged: record.merged_to_main,
            lead_time_hours: record.lead_time_hours,
            tag_version: record.tag_version.clone(),
            phase_durations_sec: record.phase_durations_sec.clone(),
        }
    }
}

/// Structured research packet: the input contract for the self-research agents.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ResearchPacket {
    generated_at: String,
    window_days: u16,
    aggregate: MetricsAggregate,
    cycles: Vec<CycleSummary>,
    /// Per-project summary when the packet is cross-project (`--all-projects`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    projects: Vec<ProjectSummary>,
    signals: Vec<String>,
}

/// Per-project summary inside a cross-project research packet (ADR-0009).
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ProjectSummary {
    project_id: String,
    cycles: usize,
    first_pass_success_rate: f64,
    median_lead_time_hours: Option<f64>,
    median_cost_usd: Option<f64>,
    top_bottleneck_phase: Option<String>,
}

fn run_analytics_research(
    args: AnalyticsResearchArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<ResearchPacket> {
        let records: Vec<MetricsRecord> = if args.all_projects {
            // Cross-project: load from the control plane store when available.
            if crate::telemetry::store_exists(environment) {
                let plane = crate::telemetry::open_store(environment, false)?;
                crate::telemetry::load_cycles(&*plane)?
            } else {
                anyhow::bail!("control plane store not found; run `sddk telemetry ingest` first")
            }
        } else {
            let context = RuntimeContext::open(&args.runtime, environment, false)?;
            read_records(&context)?
        };
        let records = window_records(records, args.window.days());
        let aggregate = compute_aggregate(&records, args.window.days());
        let tuning = tuning_from_aggregate(&aggregate);
        let mut signals = Vec::new();
        if let Some(bias) = &tuning.path_bias {
            signals.push(format!("path_bias: {bias}"));
        }
        for lens in &tuning.recommended_lens {
            signals.push(format!("lens: {lens}"));
        }
        for skip in &tuning.recommended_skip {
            signals.push(format!("skip: {skip}"));
        }
        for deepen in &tuning.recommended_deepen {
            signals.push(format!("deepen: {deepen}"));
        }
        let projects = if args.all_projects {
            summarize_projects(&records)
        } else {
            Vec::new()
        };
        let generated_at = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)?;
        Ok(ResearchPacket {
            generated_at,
            window_days: args.window.days(),
            aggregate,
            cycles: records.iter().map(CycleSummary::from).collect(),
            projects,
            signals,
        })
    })();
    render_result(result, format, research_text)
}

/// Group records by project (derived from the cycle_id prefix `p-<hex>/`).
fn summarize_projects(records: &[MetricsRecord]) -> Vec<ProjectSummary> {
    let mut grouped: std::collections::BTreeMap<String, Vec<&MetricsRecord>> =
        std::collections::BTreeMap::new();
    for record in records {
        let project_id = record
            .cycle_id
            .split('/')
            .next()
            .unwrap_or("unknown")
            .to_owned();
        grouped.entry(project_id).or_default().push(record);
    }
    grouped
        .into_iter()
        .map(|(project_id, group)| {
            let owned: Vec<MetricsRecord> = group.into_iter().cloned().collect();
            let aggregate = compute_aggregate(&owned, 7);
            ProjectSummary {
                project_id,
                cycles: owned.len(),
                first_pass_success_rate: aggregate.first_pass_success_rate,
                median_lead_time_hours: aggregate.median_lead_time_hours,
                median_cost_usd: aggregate.median_cost_usd,
                top_bottleneck_phase: aggregate.top_bottleneck_phase,
            }
        })
        .collect()
}

fn research_text(packet: &ResearchPacket) -> String {
    let mut text = format!(
        "research packet ({}d, {} cycles)\n",
        packet.window_days,
        packet.cycles.len()
    );
    text.push_str(&format!(
        "first_pass_rate: {:.2}\n",
        packet.aggregate.first_pass_success_rate
    ));
    if let Some(bottleneck) = &packet.aggregate.top_bottleneck_phase {
        text.push_str(&format!("bottleneck: {bottleneck}\n"));
    }
    if !packet.projects.is_empty() {
        text.push_str("projects:\n");
        for project in &packet.projects {
            text.push_str(&format!(
                "- {} cycles={} first_pass={:.2} lead_time={} cost={} bottleneck={}\n",
                project.project_id,
                project.cycles,
                project.first_pass_success_rate,
                project
                    .median_lead_time_hours
                    .map(|value| format!("{value:.2}"))
                    .unwrap_or_else(|| "n/a".into()),
                project
                    .median_cost_usd
                    .map(|value| format!("{value:.2}"))
                    .unwrap_or_else(|| "n/a".into()),
                project.top_bottleneck_phase.as_deref().unwrap_or("n/a"),
            ));
        }
    }
    if !packet.signals.is_empty() {
        text.push_str("signals:\n");
        for signal in &packet.signals {
            text.push_str(&format!("- {signal}\n"));
        }
    }
    text.push_str("cycles:\n");
    for cycle in &packet.cycles {
        text.push_str(&format!(
            "- {} verdict={} merged={}\n",
            cycle.cycle_id, cycle.verdict, cycle.merged
        ));
    }
    text
}

fn aggregate_for(
    context: &RuntimeContext,
    window: MetricsWindow,
) -> anyhow::Result<MetricsAggregate> {
    let records = read_records(context)?;
    let records = window_records(records, window.days());
    Ok(compute_aggregate(&records, window.days()))
}

fn run_analytics_report(args: AnalyticsWindowArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<MetricsAggregate> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        aggregate_for(&context, args.window)
    })();
    render_result(result, format, crate::metrics::aggregate_text)
}

fn run_analytics_trends(args: AnalyticsWindowArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<TrendsOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let window_7d = aggregate_for(&context, MetricsWindow::SevenDays)?;
        let window_30d = aggregate_for(&context, MetricsWindow::ThirtyDays)?;
        Ok(TrendsOutput {
            window_7d,
            window_30d,
        })
    })();
    render_result(result, format, trends_text)
}

fn run_analytics_bottleneck(
    args: AnalyticsWindowArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<MetricsAggregate> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        aggregate_for(&context, args.window)
    })();
    render_result(result, format, bottleneck_text)
}

fn trends_text(output: &TrendsOutput) -> String {
    format!(
        "trends 7d -> 30d:\n  first_pass_rate: {:.2} -> {:.2}\n  median_lead_time_hours: {} -> {}\n  sample: {} -> {}\n",
        output.window_7d.first_pass_success_rate,
        output.window_30d.first_pass_success_rate,
        output
            .window_7d
            .median_lead_time_hours
            .map(|value| format!("{value:.2}"))
            .unwrap_or_else(|| "n/a".to_owned()),
        output
            .window_30d
            .median_lead_time_hours
            .map(|value| format!("{value:.2}"))
            .unwrap_or_else(|| "n/a".to_owned()),
        output.window_7d.sample_size,
        output.window_30d.sample_size,
    )
}

fn bottleneck_text(output: &MetricsAggregate) -> String {
    let mut text = format!("sample: {}\n", output.sample_size);
    if let Some(bottleneck) = &output.top_bottleneck_phase {
        text.push_str(&format!("top_bottleneck_phase: {bottleneck}\n"));
    } else {
        text.push_str("top_bottleneck_phase: n/a (no phase durations recorded)\n");
    }
    text
}
