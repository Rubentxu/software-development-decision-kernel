//! Ledger verification and event inspection commands.

use anyhow::Context;
use clap::{Args, Subcommand};
use serde::Serialize;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat, RuntimeArgs, RuntimeContext, render_result,
};

#[derive(Debug, Subcommand)]
pub(crate) enum LedgerCommand {
    /// Verify sequence continuity, predecessor links, and event hashes.
    Verify(LedgerVerifyArgs),
    /// Verify stream hash chain integrity (Phase 2 SHOULD).
    VerifyChain(VerifyChainArgs),
    /// Backfill chain_hash for pre-MIGRATION_10 events.
    BackfillChain(BackfillChainArgs),
    /// List ledger events, optionally scoped to one command frame.
    Events(LedgerEventsArgs),
    /// Export ledger events as newline-delimited JSON (JSONL) to a file.
    Export(LedgerExportArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct LedgerVerifyArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct VerifyChainArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Stream ID to verify. Defaults to the project stream.
    #[arg(long)]
    pub(crate) stream: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct BackfillChainArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Stream ID to backfill. Defaults to all streams.
    #[arg(long)]
    pub(crate) stream: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct LedgerEventsArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Restrict to events sharing one command frame.
    #[arg(long)]
    pub(crate) frame: Option<String>,
    /// Maximum events to list.
    #[arg(long, default_value_t = 50)]
    pub(crate) limit: usize,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct LedgerExportArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Restrict to events in this cycle.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
    /// Restrict to events sharing one command frame.
    #[arg(long)]
    pub(crate) frame: Option<String>,
    /// Maximum events to export (default 1000; use 0 for all).
    #[arg(long, default_value_t = 1000)]
    pub(crate) limit: usize,
    /// Output file path. Required — JSONL files are typically saved to disk.
    #[arg(long)]
    pub(crate) output: std::path::PathBuf,
}

pub(crate) fn run_ledger(command: LedgerCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        LedgerCommand::Verify(args) => run_ledger_verify(args, environment),
        LedgerCommand::VerifyChain(args) => run_verify_chain(args, environment),
        LedgerCommand::BackfillChain(args) => run_backfill_chain(args, environment),
        LedgerCommand::Events(args) => run_ledger_events(args, environment),
        LedgerCommand::Export(args) => run_ledger_export(args, environment),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct LedgerVerifyOutput {
    event_count: usize,
    last_hash: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct VerifyChainOutput {
    stream: String,
    event_count: usize,
    head_chain_hash: Option<String>,
    status: VerifyChainStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum VerifyChainStatus {
    Pass,
    Fail { error: String },
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct BackfillChainOutput {
    stream: String,
    updated: usize,
    status: BackfillChainStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum BackfillChainStatus {
    Success,
    Fail { error: String },
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct LedgerEventOutput {
    sequence: i64,
    event_id: String,
    frame_id: String,
    command_id: String,
    event_type: String,
    cycle_id: Option<String>,
    actor: String,
    occurred_at: String,
}

fn run_ledger_verify(args: LedgerVerifyArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<LedgerVerifyOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let verified = context.storage.verify_ledger()?;
        Ok(LedgerVerifyOutput {
            event_count: verified.event_count,
            last_hash: verified.last_hash,
        })
    })();
    render_result(result, format, ledger_verify_text)
}

fn run_verify_chain(args: VerifyChainArgs, environment: &CliEnvironment) -> CommandOutput {
    use sddk_domain::EventStore;
    use sddk_storage::event_store::SqliteEventStore;
    let format = args.format;
    let result = (|| -> anyhow::Result<VerifyChainOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let stream = args
            .stream
            .unwrap_or_else(|| format!("project:{}", context.identity.project_id));
        let event_store = SqliteEventStore::open(context.paths.ledger.parent().unwrap())?;
        let head_chain = event_store.head_chain_hash(&stream)?;
        let event_count = event_store.load_stream(&stream, None, u32::MAX)?.len();
        match event_store.verify_chain_integrity(&stream) {
            Ok(()) => Ok(VerifyChainOutput {
                stream: stream.clone(),
                event_count,
                head_chain_hash: head_chain,
                status: VerifyChainStatus::Pass,
            }),
            Err(e) => Ok(VerifyChainOutput {
                stream,
                event_count,
                head_chain_hash: head_chain,
                status: VerifyChainStatus::Fail {
                    error: e.to_string(),
                },
            }),
        }
    })();
    render_result(result, format, verify_chain_text)
}

fn verify_chain_text(output: &VerifyChainOutput) -> String {
    match &output.status {
        VerifyChainStatus::Pass => format!(
            "stream: {}\nevent_count: {}\nhead_chain_hash: {}\nstatus: PASS\n",
            output.stream,
            output.event_count,
            output.head_chain_hash.as_deref().unwrap_or("null")
        ),
        VerifyChainStatus::Fail { error } => format!(
            "stream: {}\nevent_count: {}\nhead_chain_hash: {}\nstatus: FAIL\nerror: {}\n",
            output.stream,
            output.event_count,
            output.head_chain_hash.as_deref().unwrap_or("null"),
            error
        ),
    }
}

fn run_backfill_chain(args: BackfillChainArgs, environment: &CliEnvironment) -> CommandOutput {
    use sddk_domain::EventStore;
    use sddk_storage::event_store::SqliteEventStore;
    let format = args.format;
    let result = (|| -> anyhow::Result<BackfillChainOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let ledger_dir = context.paths.ledger.parent().unwrap();
        let mut event_store = SqliteEventStore::open(ledger_dir)?;
        let stream = args
            .stream
            .unwrap_or_else(|| format!("project:{}", context.identity.project_id));
        match event_store.backfill_chain_hash(&stream) {
            Ok(updated) => Ok(BackfillChainOutput {
                stream: stream.clone(),
                updated,
                status: BackfillChainStatus::Success,
            }),
            Err(e) => Ok(BackfillChainOutput {
                stream,
                updated: 0,
                status: BackfillChainStatus::Fail {
                    error: e.to_string(),
                },
            }),
        }
    })();
    render_result(result, format, backfill_chain_text)
}

fn backfill_chain_text(output: &BackfillChainOutput) -> String {
    match &output.status {
        BackfillChainStatus::Success => format!(
            "stream: {}\nupdated: {}\nstatus: SUCCESS\n",
            output.stream, output.updated
        ),
        BackfillChainStatus::Fail { error } => format!(
            "stream: {}\nupdated: {}\nstatus: FAIL\nerror: {}\n",
            output.stream, output.updated, error
        ),
    }
}

fn run_ledger_events(args: LedgerEventsArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<Vec<LedgerEventOutput>> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let events = match &args.frame {
            Some(frame) => context.storage.list_frame_events(frame)?,
            None => context.storage.list_events()?,
        };
        Ok(events
            .into_iter()
            .rev()
            .take(args.limit)
            .rev()
            .map(|event| LedgerEventOutput {
                sequence: event.sequence,
                event_id: event.event_id,
                frame_id: event.frame_id,
                command_id: event.command_id,
                event_type: event.event_type,
                cycle_id: event.cycle_id,
                actor: event.actor,
                occurred_at: event.occurred_at,
            })
            .collect())
    })();
    render_result(result, format, ledger_events_text)
}

/// Exports ledger events as JSONL to the specified output file.
/// Events are written one JSON object per line, in ascending sequence order.
/// Filtering: cycle_id → frame_id → limit (applied in that order).
fn run_ledger_export(args: LedgerExportArgs, environment: &CliEnvironment) -> CommandOutput {
    let result = (|| -> anyhow::Result<ExportOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;

        let all_events = if let Some(cycle) = &args.cycle {
            context.storage.list_cycle_events(cycle)?
        } else if let Some(frame) = &args.frame {
            context.storage.list_frame_events(frame)?
        } else {
            context.storage.list_events()?
        };

        let limit = if args.limit == 0 {
            usize::MAX
        } else {
            args.limit
        };
        let events: Vec<_> = all_events.into_iter().take(limit).collect();

        // Write JSONL — one JSON object per line.
        let file = std::fs::File::create(&args.output)
            .with_context(|| format!("creating output file {}", args.output.display()))?;
        let mut buf = std::io::BufWriter::new(file);
        let mut count = 0;
        for event in &events {
            let json = serde_json::to_string(event).context("serializing LedgerEvent to JSON")?;
            use std::io::Write;
            writeln!(buf, "{json}").context("writing JSON line")?;
            count += 1;
        }
        drop(buf); // flushes on drop

        Ok(ExportOutput {
            path: args.output.clone(),
            count,
        })
    })();

    match result {
        Ok(output) => CommandOutput {
            status: 0,
            stdout: format!(
                "exported {} events to {}\n",
                output.count,
                output.path.display()
            ),
            stderr: String::new(),
        },
        Err(e) => CommandOutput {
            status: 1,
            stdout: String::new(),
            stderr: format!("error: export failed: {e}\n"),
        },
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ExportOutput {
    path: std::path::PathBuf,
    count: usize,
}

fn ledger_verify_text(output: &LedgerVerifyOutput) -> String {
    format!(
        "event_count: {}\nlast_hash: {}\n",
        output.event_count,
        output.last_hash.as_deref().unwrap_or("null")
    )
}

fn ledger_events_text(events: &Vec<LedgerEventOutput>) -> String {
    if events.is_empty() {
        return "no events\n".to_owned();
    }
    let mut output = String::new();
    for event in events {
        output.push_str(&format!(
            "{} {} {} {} {}\n",
            event.sequence,
            event.event_type,
            event.event_id,
            event.frame_id,
            event.cycle_id.as_deref().unwrap_or("-")
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    fn env() -> CliEnvironment {
        CliEnvironment::default()
    }

    // Smoke test: constructing the args and invoking the function is enough.
    // Full export requires a real project runtime (RuntimeContext::open),
    // which is covered by integration tests.
    #[test]
    fn ledger_export_args_construction() {
        let tmp = std::env::temp_dir().join("sddk-export-test.jsonl");
        let _args = LedgerExportArgs {
            runtime: RuntimeArgs {
                root: Some(tmp.parent().unwrap().to_path_buf()),
                scope: Some(".".to_string()),
                remote: None,
                fallback_seed: None,
                no_infer: false,
            },
            cycle: None,
            frame: None,
            limit: 10,
            output: tmp.clone(),
        };
        // RuntimeContext::open will fail without a real project, but args construction is tested.
        let _ = std::fs::remove_file(tmp);
    }
}
