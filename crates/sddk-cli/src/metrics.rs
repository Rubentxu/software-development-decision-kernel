//! Metrics capture, aggregation, F3 tuning, and analytics commands.

use std::collections::HashMap;
use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};
use sddk_domain::{F3Tuning, MetricsAggregate, MetricsRecord};
use time::OffsetDateTime;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat, RuntimeArgs, RuntimeContext, render_result,
};

/// Window selector for metrics aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum MetricsWindow {
    #[value(name = "7d")]
    SevenDays,
    #[value(name = "30d")]
    ThirtyDays,
}

impl MetricsWindow {
    pub(crate) fn days(self) -> u16 {
        match self {
            Self::SevenDays => 7,
            Self::ThirtyDays => 30,
        }
    }
}

#[derive(Debug, Subcommand)]
pub(crate) enum MetricsCommand {
    /// Record metrics for a closed cycle (Levels A-E + L1-L6 costs).
    Record(Box<MetricsRecordArgs>),
    /// Compute rolling aggregates over a window.
    Aggregate(MetricsAggregateArgs),
    /// Emit the F3 self-tuning recommendation block.
    Tuning(MetricsTuningArgs),
    /// Re-derive records for cycles with poor fields from ledger events.
    Backfill(MetricsBackfillArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct MetricsBackfillArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct MetricsRecordArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier to record metrics for.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Verification verdict: PASS | PW | FAIL.
    #[arg(long, default_value = "PASS")]
    pub(crate) verdict: String,
    /// Whether the change was merged to main.
    #[arg(long)]
    pub(crate) merged: bool,
    /// First verification attempt passed.
    #[arg(long)]
    pub(crate) first_pass: bool,
    /// Correction cycles count.
    #[arg(long, default_value_t = 0)]
    pub(crate) corrections: u8,
    /// Context quality at triage (C0..C3).
    #[arg(long, default_value = "C2")]
    pub(crate) context_quality: String,
    /// Workflow path taken (b-direct | a-min | a-lite | a-full).
    #[arg(long)]
    pub(crate) path: Option<String>,
    /// Semantic version tag when released.
    #[arg(long)]
    pub(crate) tag: Option<String>,
    /// Estimated cost in USD.
    #[arg(long)]
    pub(crate) cost: Option<f64>,
    /// Estimated tokens used (for cost estimation when --cost is absent).
    #[arg(long)]
    pub(crate) tokens: Option<u64>,
    /// Model used (for cost estimation when --cost is absent).
    #[arg(long)]
    pub(crate) model: Option<String>,
    /// Teleological coherence percentage (Level E) for the cycle.
    #[arg(long)]
    pub(crate) coherence: Option<f64>,
    /// Loop costs as JSON, e.g. '{"L1": 0.4, "L2": 1.2}'.
    #[arg(long)]
    pub(crate) costs: Option<String>,
    /// Persist a context quality override for this cycle (C0..C3).
    #[arg(long)]
    pub(crate) set_context: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct MetricsAggregateArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Aggregation window.
    #[arg(long, value_enum, default_value_t = MetricsWindow::SevenDays)]
    pub(crate) window: MetricsWindow,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct MetricsTuningArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Aggregation window for the tuning signals.
    #[arg(long, value_enum, default_value_t = MetricsWindow::SevenDays)]
    pub(crate) window: MetricsWindow,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Metrics store file names.
const METRICS_JSONL: &str = "metrics.jsonl";
const AGGREGATE_JSON: &str = "aggregate.json";

/// Resolve the project metrics directory from a runtime context.
///
/// The metrics directory lives next to the artifacts directory under the
/// project data root: `<data>/sddk/projects/<project_id>/metrics`.
fn metrics_dir(context: &RuntimeContext) -> anyhow::Result<PathBuf> {
    let artifacts = &context.artifacts_path;
    let project_data = artifacts
        .parent()
        .ok_or_else(|| anyhow::anyhow!("artifacts path has no parent"))?;
    Ok(project_data.join("metrics"))
}

/// Append one metrics record to `metrics.jsonl`.
pub(crate) fn append_record(
    context: &RuntimeContext,
    record: &MetricsRecord,
) -> anyhow::Result<PathBuf> {
    let dir = metrics_dir(context)?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(METRICS_JSONL);
    let mut line = serde_json::to_string(record)?;
    line.push('\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    use std::io::Write;
    file.write_all(line.as_bytes())?;
    Ok(path)
}

/// Read all records from `metrics.jsonl`, skipping corrupt lines.
pub(crate) fn read_records(context: &RuntimeContext) -> anyhow::Result<Vec<MetricsRecord>> {
    let dir = metrics_dir(context)?;
    let path = dir.join(METRICS_JSONL);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path)?;
    let mut records = Vec::new();
    for (index, line) in content.lines().enumerate() {
        match serde_json::from_str::<MetricsRecord>(line) {
            Ok(record) => records.push(record),
            Err(_) => eprintln!("warning: skipping corrupt metrics line {}", index + 1),
        }
    }
    Ok(records)
}

/// Atomically replace the metrics JSONL with the given records.
fn write_jsonl(context: &RuntimeContext, records: &[MetricsRecord]) -> anyhow::Result<()> {
    let dir = metrics_dir(context)?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(METRICS_JSONL);
    let tmp = dir.join(format!("{METRICS_JSONL}.tmp"));
    let mut content = String::new();
    for record in records {
        content.push_str(&serde_json::to_string(record)?);
        content.push('\n');
    }
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

/// Context quality store file: maps cycle_id -> C0..C3.
const CONTEXT_JSON: &str = "context.json";

/// Persist a context quality override for a cycle.
fn write_context_quality(
    context: &RuntimeContext,
    cycle_id: &str,
    quality: &str,
) -> anyhow::Result<()> {
    let dir = metrics_dir(context)?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(CONTEXT_JSON);
    let mut map: std::collections::BTreeMap<String, String> = if path.exists() {
        serde_json::from_str(&std::fs::read_to_string(&path)?).unwrap_or_default()
    } else {
        std::collections::BTreeMap::new()
    };
    map.insert(cycle_id.to_owned(), quality.to_owned());
    std::fs::write(&path, serde_json::to_string_pretty(&map)?)?;
    Ok(())
}

/// Read the persisted context quality for a cycle, if any.
fn read_context_quality(context: &RuntimeContext, cycle_id: &str) -> Option<String> {
    let dir = metrics_dir(context).ok()?;
    let path = dir.join(CONTEXT_JSON);
    if !path.exists() {
        return None;
    }
    let map: std::collections::BTreeMap<String, String> =
        serde_json::from_str(&std::fs::read_to_string(&path).ok()?).ok()?;
    map.get(cycle_id).cloned()
}

/// Read the F3 `path_bias` recommendation from `tuning.md`, if present.
pub(crate) fn read_tuning_path_bias(context: &RuntimeContext) -> Option<String> {
    let path = metrics_dir(context).ok()?.join("tuning.md");
    let content = std::fs::read_to_string(path).ok()?;
    content
        .lines()
        .find_map(|line| line.strip_prefix("- path_bias: ").map(str::to_owned))
}

/// Idempotent: if a record for the cycle already exists, this is a no-op.
/// Best-effort: derivation never blocks; missing data defaults to explicit
/// sentinels (`UNKNOWN` verdict, false flags, 0 costs).
pub(crate) fn capture_cycle_metrics(
    context: &RuntimeContext,
    manifest: &sddk_domain::CycleManifest,
) -> anyhow::Result<()> {
    let existing = read_records(context)?;
    if existing
        .iter()
        .any(|record| record.cycle_id == manifest.cycle_id)
    {
        return Ok(());
    }

    let events = context.storage.list_cycle_events(&manifest.cycle_id)?;
    let derived = derive_from_events(&events);
    let tag_version = derived
        .tag_version
        .or_else(|| manifest.release.as_ref().and_then(|r| r.tag.clone()));
    let path = match manifest.path {
        sddk_domain::CyclePath::BDirect => "b-direct",
        sddk_domain::CyclePath::AMin => "a-min",
        sddk_domain::CyclePath::ALite => "a-lite",
        sddk_domain::CyclePath::AFull => "a-full",
    }
    .to_owned();

    let now = OffsetDateTime::now_utc();
    let recorded_at = now.format(&time::format_description::well_known::Rfc3339)?;
    let context_quality =
        read_context_quality(context, &manifest.cycle_id).unwrap_or_else(|| "C2".to_owned());

    let record = MetricsRecord {
        cycle_id: manifest.cycle_id.clone(),
        path,
        context_quality,
        phase_durations_sec: derived.phase_durations_sec,
        coherence_scores: Vec::new(),
        correction_cycles: derived.correction_cycles,
        tokens_used: 0,
        cost_estimate_usd: 0.0,
        first_pass_success: derived.first_pass_success,
        verify_verdict: derived.verify_verdict,
        merged_to_main: derived.merged_to_main,
        tag_version,
        lead_time_hours: derived.lead_time_hours,
        teleological_coherence_pct: None,
        costs: HashMap::new(),
        recorded_at,
    };
    append_record(context, &record)?;
    eprintln!(
        "metrics: auto-captured record for cycle {}",
        record.cycle_id
    );
    Ok(())
}

/// Fields derived from a cycle's ledger event history.
pub(crate) struct DerivedFields {
    pub(crate) phase_durations_sec: HashMap<String, u64>,
    pub(crate) verify_verdict: String,
    pub(crate) lead_time_hours: Option<f64>,
    pub(crate) tag_version: Option<String>,
    pub(crate) correction_cycles: u8,
    pub(crate) first_pass_success: bool,
    pub(crate) merged_to_main: bool,
}

/// Derive metrics fields from the cycle's ledger events.
///
/// Best-effort per field: a corrupt timestamp degrades one field, not the
/// record. Defaults match the pre-enrichment behavior.
pub(crate) fn derive_from_events(events: &[sddk_domain::LedgerEvent]) -> DerivedFields {
    let mut phase_durations_sec = HashMap::new();
    let mut phase_start: Option<(String, OffsetDateTime)> = None;
    let mut verify_verdict = "UNKNOWN".to_owned();
    let mut lead_time_hours: Option<f64> = None;
    let mut tag_version: Option<String> = None;
    let mut correction_cycles: u8 = 0;
    let mut merged_to_main = false;
    let mut first_ts: Option<OffsetDateTime> = None;
    let mut last_ts: Option<OffsetDateTime> = None;

    for event in events {
        let ts = match OffsetDateTime::parse(
            &event.occurred_at,
            &time::format_description::well_known::Rfc3339,
        ) {
            Ok(ts) => ts,
            Err(_) => continue,
        };
        if first_ts.is_none() {
            first_ts = Some(ts);
        }
        last_ts = Some(ts);

        // Phase duration accumulation: when phase changes, close the previous
        // phase with the time delta.
        let phase = event
            .state_after
            .as_ref()
            .and_then(|state| state.get("phase"))
            .and_then(|value| value.as_str())
            .map(str::to_owned);
        if let Some(current_phase) = phase {
            if let Some((start_phase, start_ts)) = &phase_start {
                if start_phase != &current_phase {
                    let seconds = (ts - *start_ts).whole_seconds().max(0) as u64;
                    *phase_durations_sec.entry(start_phase.clone()).or_insert(0) += seconds;
                    phase_start = Some((current_phase, ts));
                }
            } else {
                phase_start = Some((current_phase, ts));
            }
        }

        // Remediation detection for corrections + verdict.
        let status = event
            .state_after
            .as_ref()
            .and_then(|state| state.get("status"))
            .and_then(|value| value.as_str());
        // Explicit verdict in the event payload wins over status derivation.
        let payload_verdict = event
            .payload
            .get("verify_verdict")
            .or_else(|| event.payload.get("verdict"))
            .and_then(|value| value.as_str())
            .map(|value| value.to_uppercase());
        if let Some(verdict) = payload_verdict
            && verify_verdict == "UNKNOWN"
        {
            verify_verdict = verdict;
        }
        if status == Some("REMEDIATING") {
            correction_cycles = correction_cycles.saturating_add(1);
            verify_verdict = "FAIL".to_owned();
        } else if status == Some("RELEASED") {
            verify_verdict = "PASS".to_owned();
            merged_to_main = true;
        }

        // Release receipt tag extraction.
        if event.event_type == "cycle.transitioned"
            && status == Some("RELEASED")
            && let Some(state) = &event.state_after
            && let Some(artifacts) = state.get("artifacts")
            && let Some(receipt) = artifacts.get("release-receipt")
        {
            tag_version = receipt
                .get("path")
                .and_then(|value| value.as_str())
                .or_else(|| receipt.as_str())
                .map(str::to_owned);
        }
    }

    // Close the final open phase with the last timestamp.
    if let Some((start_phase, start_ts)) = &phase_start
        && let Some(last) = last_ts
    {
        let seconds = (last - *start_ts).whole_seconds().max(0) as u64;
        *phase_durations_sec.entry(start_phase.clone()).or_insert(0) += seconds;
    }

    if let (Some(first), Some(last)) = (first_ts, last_ts) {
        lead_time_hours = Some((last - first).whole_seconds() as f64 / 3600.0);
    }

    DerivedFields {
        phase_durations_sec,
        verify_verdict,
        lead_time_hours,
        tag_version,
        correction_cycles,
        first_pass_success: correction_cycles == 0,
        merged_to_main,
    }
}

/// Filter records to a window (by recorded_at).
pub(crate) fn window_records(records: Vec<MetricsRecord>, window_days: u16) -> Vec<MetricsRecord> {
    let cutoff = OffsetDateTime::now_utc() - time::Duration::days(window_days as i64);
    records
        .into_iter()
        .filter(|record| {
            OffsetDateTime::parse(
                &record.recorded_at,
                &time::format_description::well_known::Rfc3339,
            )
            .map(|when| when >= cutoff)
            .unwrap_or(true)
        })
        .collect()
}

/// Known per-token USD rates for cost estimation (approximate list prices).
const MODEL_RATES: [(&str, f64); 4] = [
    ("mini-m2.7", 0.50 / 1_000_000.0),
    ("deepseek-v4-pro", 1.20 / 1_000_000.0),
    ("glm-4.7", 0.80 / 1_000_000.0),
    ("deepseek-v4-flash", 0.30 / 1_000_000.0),
];

/// Estimate cost in USD from tokens and model, or fall back to a default rate.
fn estimate_cost(tokens: u64, model: Option<&str>) -> f64 {
    let rate = model
        .and_then(|name| {
            MODEL_RATES
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, r)| *r)
        })
        .unwrap_or(0.50 / 1_000_000.0);
    tokens as f64 * rate
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.total_cmp(b));
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        Some((values[middle - 1] + values[middle]) / 2.0)
    } else {
        Some(values[middle])
    }
}

/// Compute the rolling aggregate for a set of records.
pub(crate) fn compute_aggregate(records: &[MetricsRecord], window_days: u16) -> MetricsAggregate {
    let mut aggregate = MetricsAggregate::empty(window_days);
    aggregate.sample_size = records.len() as u32;
    if records.is_empty() {
        return aggregate;
    }
    let passes = records
        .iter()
        .filter(|record| record.first_pass_success)
        .count();
    aggregate.first_pass_success_rate = passes as f64 / records.len() as f64;

    let mut lead_times: Vec<f64> = records
        .iter()
        .filter_map(|record| record.lead_time_hours)
        .collect();
    aggregate.median_lead_time_hours = median(&mut lead_times);

    let mut costs: Vec<f64> = records
        .iter()
        .filter(|record| record.cost_estimate_usd > 0.0)
        .map(|record| record.cost_estimate_usd)
        .collect();
    aggregate.median_cost_usd = median(&mut costs);

    let mut phase_totals: HashMap<String, (u64, u32)> = HashMap::new();
    for record in records {
        for (phase, seconds) in &record.phase_durations_sec {
            let entry = phase_totals.entry(phase.clone()).or_insert((0, 0));
            entry.0 += seconds;
            entry.1 += 1;
        }
        *aggregate
            .path_distribution
            .entry(record.path.clone())
            .or_insert(0) += 1;
        *aggregate
            .verdict_distribution
            .entry(record.verify_verdict.clone())
            .or_insert(0) += 1;
    }
    aggregate.top_bottleneck_phase = phase_totals
        .into_iter()
        .filter(|(_, (_, count))| *count > 0)
        .max_by(|a, b| (a.1.0 as f64 / a.1.1 as f64).total_cmp(&(b.1.0 as f64 / b.1.1 as f64)))
        .map(|(phase, _)| phase);
    aggregate
}

/// Map an aggregate to F3 tuning recommendations (advisory).
pub(crate) fn tuning_from_aggregate(aggregate: &MetricsAggregate) -> F3Tuning {
    let mut tuning = F3Tuning::default();
    if aggregate.sample_size >= 3 {
        if aggregate.first_pass_success_rate > 0.85 {
            tuning.path_bias = Some("A-min".to_owned());
        } else if aggregate.first_pass_success_rate >= 0.6 {
            // Middle band: keep A-lite, add verification lens to close the gap.
            tuning.path_bias = Some("A-lite".to_owned());
            tuning.recommended_lens.push("test-quality".to_owned());
        } else {
            tuning.recommended_deepen.push("spec".to_owned());
            tuning.recommended_deepen.push("verify".to_owned());
        }
    }
    match aggregate.top_bottleneck_phase.as_deref() {
        Some("apply") => tuning.recommended_lens.push("test-quality".to_owned()),
        Some("release") => tuning.recommended_skip.push("manual-merge".to_owned()),
        _ => {}
    }
    tuning
}

/// Write the aggregate to `aggregate.json`.
fn write_aggregate(
    context: &RuntimeContext,
    aggregate: &MetricsAggregate,
) -> anyhow::Result<PathBuf> {
    let dir = metrics_dir(context)?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(AGGREGATE_JSON);
    let content = serde_json::to_string_pretty(aggregate)?;
    std::fs::write(&path, content)?;
    Ok(path)
}

/// Read the persisted aggregate if present.
fn read_aggregate(context: &RuntimeContext) -> anyhow::Result<Option<MetricsAggregate>> {
    let dir = metrics_dir(context)?;
    let path = dir.join(AGGREGATE_JSON);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&content).ok())
}

pub(crate) fn run_metrics(command: MetricsCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        MetricsCommand::Record(args) => run_metrics_record(*args, environment),
        MetricsCommand::Aggregate(args) => run_metrics_aggregate(args, environment),
        MetricsCommand::Tuning(args) => run_metrics_tuning(args, environment),
        MetricsCommand::Backfill(args) => run_metrics_backfill(args, environment),
    }
}

/// A record is considered poor when enrichment would add signal.
fn record_is_poor(record: &MetricsRecord) -> bool {
    record.verify_verdict == "UNKNOWN"
        || record.tag_version.is_none()
        || record.phase_durations_sec.is_empty()
        || record.lead_time_hours.is_none()
        || !record.merged_to_main
}

fn run_metrics_backfill(args: MetricsBackfillArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<Vec<MetricsRecord>> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let events = context.storage.list_events()?;
        // Group events by cycle; find cycles whose final state is CLOSED.
        let mut cycles: std::collections::BTreeMap<String, Vec<sddk_domain::LedgerEvent>> =
            std::collections::BTreeMap::new();
        for event in &events {
            if let Some(cycle_id) = &event.cycle_id {
                cycles
                    .entry(cycle_id.clone())
                    .or_default()
                    .push(event.clone());
            }
        }

        let existing = read_records(&context)?;
        // Deduplicate: keep at most one record per cycle (the richest).
        let mut best_per_cycle: std::collections::BTreeMap<String, MetricsRecord> =
            std::collections::BTreeMap::new();
        for record in existing {
            let insert = match best_per_cycle.get(&record.cycle_id) {
                None => true,
                Some(current) => {
                    // Rich wins over poor; for equal richness keep the latest.
                    !record_is_poor(&record) && record_is_poor(current)
                }
            };
            if insert {
                best_per_cycle.insert(record.cycle_id.clone(), record);
            }
        }
        let existing: Vec<MetricsRecord> = best_per_cycle.into_values().collect();
        // Persist the deduplicated set so duplicates are removed even when no
        // cycle needs backfilling below.
        write_jsonl(&context, &existing)?;
        let mut backfilled = Vec::new();
        let mut rewritten = false;
        for (cycle_id, cycle_events) in &cycles {
            // Determine final status from the last state_after.
            let closed = cycle_events.iter().rev().find_map(|event| {
                event
                    .state_after
                    .as_ref()
                    .and_then(|state| state.get("status"))
                    .and_then(|value| value.as_str())
                    .map(|status| status == "CLOSED")
            });
            if closed != Some(true) {
                continue;
            }
            let already_enriched = existing
                .iter()
                .any(|record| record.cycle_id == *cycle_id && !record_is_poor(record));
            if already_enriched {
                continue;
            }
            let derived = derive_from_events(cycle_events);
            let now = OffsetDateTime::now_utc();
            let recorded_at = now.format(&time::format_description::well_known::Rfc3339)?;
            let path = match cycle_events.iter().rev().find_map(|event| {
                event
                    .state_after
                    .as_ref()
                    .and_then(|state| state.get("path"))
                    .and_then(|value| value.as_str())
                    .map(str::to_owned)
            }) {
                Some(path) => path,
                None => "unknown".to_owned(),
            };
            let context_quality =
                read_context_quality(&context, cycle_id).unwrap_or_else(|| "C2".to_owned());
            let record = MetricsRecord {
                cycle_id: cycle_id.clone(),
                path,
                context_quality,
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
                costs: HashMap::new(),
                recorded_at,
            };
            // Replace: drop any existing (poor) records for this cycle, then append the enriched one.
            if !rewritten {
                let kept: Vec<MetricsRecord> = existing
                    .iter()
                    .filter(|record| record.cycle_id != *cycle_id)
                    .cloned()
                    .collect();
                write_jsonl(&context, &kept)?;
                rewritten = true;
            } else {
                // Subsequent cycles: drop their records too before appending.
                let current = read_records(&context)?;
                let kept: Vec<MetricsRecord> = current
                    .into_iter()
                    .filter(|record| record.cycle_id != *cycle_id)
                    .collect();
                write_jsonl(&context, &kept)?;
            }
            append_record(&context, &record)?;
            backfilled.push(record);
        }
        eprintln!("metrics: backfilled {} records", backfilled.len());
        Ok(backfilled)
    })();
    render_result(result, format, backfill_text)
}

fn backfill_text(records: &Vec<MetricsRecord>) -> String {
    let mut text = format!("backfilled: {}\n", records.len());
    for record in records {
        text.push_str(&format!(
            "- {} verdict={} tag={}\n",
            record.cycle_id,
            record.verify_verdict,
            record.tag_version.as_deref().unwrap_or("None")
        ));
    }
    text
}

fn run_metrics_record(args: MetricsRecordArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<MetricsRecord> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        if let Some(quality) = &args.set_context {
            write_context_quality(&context, &args.cycle, quality)?;
        }
        let now = OffsetDateTime::now_utc();
        let recorded_at = now.format(&time::format_description::well_known::Rfc3339)?;
        let costs = match &args.costs {
            Some(raw) => serde_json::from_str::<HashMap<String, f64>>(raw)
                .map_err(|error| anyhow::anyhow!("invalid --costs JSON: {error}"))?,
            None => HashMap::new(),
        };
        let record = MetricsRecord {
            cycle_id: args.cycle.clone(),
            path: args.path.clone().unwrap_or_else(|| "unknown".to_owned()),
            context_quality: args.context_quality.clone(),
            phase_durations_sec: HashMap::new(),
            coherence_scores: Vec::new(),
            correction_cycles: args.corrections,
            tokens_used: args.tokens.unwrap_or(0),
            cost_estimate_usd: args
                .cost
                .unwrap_or_else(|| estimate_cost(args.tokens.unwrap_or(0), args.model.as_deref())),
            first_pass_success: args.first_pass,
            verify_verdict: args.verdict.clone(),
            merged_to_main: args.merged,
            tag_version: args.tag.clone(),
            lead_time_hours: None,
            teleological_coherence_pct: args.coherence,
            costs,
            recorded_at,
        };
        let path = upsert_record(&context, &record)?;
        eprintln!("metrics appended: {}", path.display());
        Ok(record)
    })();
    render_result(result, format, metrics_record_text)
}

/// Idempotent upsert: replace any existing record for the same `cycle_id`.
///
/// The manual `metrics record` wins over the auto-captured record: a cycle
/// closed by the CLI gets its first (derived) record, then an agent may
/// enrich it with tokens/model/cost/coherence without duplicating rows.
fn upsert_record(context: &RuntimeContext, record: &MetricsRecord) -> anyhow::Result<PathBuf> {
    let existing = read_records(context)?;
    let retained: Vec<MetricsRecord> = existing
        .into_iter()
        .filter(|current| current.cycle_id != record.cycle_id)
        .collect();
    write_jsonl(context, &retained)?;
    append_record(context, record)
}

fn run_metrics_aggregate(
    args: MetricsAggregateArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<MetricsAggregate> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let records = read_records(&context)?;
        let records = window_records(records, args.window.days());
        let aggregate = compute_aggregate(&records, args.window.days());
        let path = write_aggregate(&context, &aggregate)?;
        eprintln!("aggregate written: {}", path.display());
        Ok(aggregate)
    })();
    render_result(result, format, aggregate_text)
}

fn run_metrics_tuning(args: MetricsTuningArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<F3Tuning> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let aggregate = match read_aggregate(&context)? {
            Some(aggregate) => aggregate,
            None => {
                let records = read_records(&context)?;
                let records = window_records(records, args.window.days());
                compute_aggregate(&records, args.window.days())
            }
        };
        let tuning = tuning_from_aggregate(&aggregate);
        // Persist the tuning block for the next cycle's launch plan.
        let dir = metrics_dir(&context)?;
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("tuning.md"), tuning_markdown(&tuning))?;
        Ok(tuning)
    })();
    render_result(result, format, tuning_text)
}

/// Render the F3 tuning block as markdown for `tuning.md`.
fn tuning_markdown(tuning: &F3Tuning) -> String {
    let mut text = String::from("# F3 Tuning (from aggregate)\n\n");
    if let Some(path_bias) = &tuning.path_bias {
        text.push_str(&format!("- path_bias: {path_bias}\n"));
    }
    if let Some(threshold) = tuning.circuit_threshold {
        text.push_str(&format!("- circuit_threshold: {threshold}\n"));
    }
    if let Some(attempts) = tuning.per_task_max_attempts {
        text.push_str(&format!("- per_task_max_attempts: {attempts}\n"));
    }
    for phase in &tuning.recommended_skip {
        text.push_str(&format!("- recommended_skip: {phase}\n"));
    }
    for phase in &tuning.recommended_deepen {
        text.push_str(&format!("- recommended_deepen: {phase}\n"));
    }
    for lens in &tuning.recommended_lens {
        text.push_str(&format!("- recommended_lens: {lens}\n"));
    }
    text
}

fn metrics_record_text(record: &MetricsRecord) -> String {
    format!(
        "cycle: {}\nverdict: {}\nfirst_pass: {}\nmerged: {}\ncorrections: {}\ncost: {}\n",
        record.cycle_id,
        record.verify_verdict,
        record.first_pass_success,
        record.merged_to_main,
        record.correction_cycles,
        record.cost_estimate_usd
    )
}

pub(crate) fn aggregate_text(aggregate: &MetricsAggregate) -> String {
    format!(
        "window: {}d\nsample: {}\nfirst_pass_rate: {:.2}\nmedian_lead_time_hours: {}\nmedian_cost_usd: {}\ntop_bottleneck_phase: {}\npaths: {:?}\nverdicts: {:?}\n",
        aggregate.window_days,
        aggregate.sample_size,
        aggregate.first_pass_success_rate,
        aggregate
            .median_lead_time_hours
            .map(|value| format!("{value:.2}"))
            .unwrap_or_else(|| "n/a".to_owned()),
        aggregate
            .median_cost_usd
            .map(|value| format!("{value:.2}"))
            .unwrap_or_else(|| "n/a".to_owned()),
        aggregate.top_bottleneck_phase.as_deref().unwrap_or("n/a"),
        aggregate.path_distribution,
        aggregate.verdict_distribution
    )
}

pub(crate) fn tuning_text(tuning: &F3Tuning) -> String {
    let mut text = String::new();
    if let Some(path_bias) = &tuning.path_bias {
        text.push_str(&format!("path_bias: {path_bias}\n"));
    }
    if let Some(threshold) = tuning.circuit_threshold {
        text.push_str(&format!("circuit_threshold: {threshold}\n"));
    }
    if let Some(attempts) = tuning.per_task_max_attempts {
        text.push_str(&format!("per_task_max_attempts: {attempts}\n"));
    }
    for phase in &tuning.recommended_skip {
        text.push_str(&format!("recommend_skip: {phase}\n"));
    }
    for phase in &tuning.recommended_deepen {
        text.push_str(&format!("recommend_deepen: {phase}\n"));
    }
    for lens in &tuning.recommended_lens {
        text.push_str(&format!("recommend_lens: {lens}\n"));
    }
    if text.is_empty() {
        text.push_str("no tuning recommendations (sample too small or steady state)\n");
    }
    text
}
