//! Ledger verification and event inspection commands.

use anyhow::Context;
use clap::{Args, Subcommand};
use sddk_domain::models::LedgerEvent;
use sddk_domain::ports::SnapshotPort;
use serde::Serialize;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat, RuntimeArgs, RuntimeContext, failure_envelope,
    render_result,
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
    /// Replay events from a named snapshot, optionally verifying chain hashes.
    Replay(ReplayArgs),
    /// Tail the ledger in real time (M9.5 live-mode streaming).
    ///
    /// Polls `list_events_after` in a loop and emits one event per line
    /// (NDJSON when `--format json|ndjson`, pretty text otherwise). Exits
    /// cleanly after `--max-events` events, on SIGINT (Ctrl-C), or after
    /// `--idle-timeout-ms` of no new events (default: never).
    Watch(LedgerWatchArgs),
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
    /// Maximum events to list (default 50; use 0 for all).
    ///
    /// The listing always declares how many events exist in total and whether
    /// anything was left out, in both output formats — a silent window over a
    /// 590-event ledger is what INC-DEBT-060 F63 is about.
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

#[derive(Debug, Clone, Args)]
pub(crate) struct ReplayArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Name of the snapshot to replay from.
    #[arg(long)]
    pub(crate) from_snapshot: String,
    /// Verify content and chain hashes during replay.
    #[arg(long)]
    pub(crate) verify_hashes: bool,
    /// Stream to replay (defaults to project stream).
    #[arg(long)]
    pub(crate) stream: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct LedgerWatchArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Start tailing after this ledger sequence. Default 0 (emit everything
    /// from the beginning). Use `--from-tail` instead to start after the
    /// current latest sequence (ignore existing events).
    #[arg(long, default_value_t = 0)]
    pub(crate) from_sequence: i64,
    /// Start tailing strictly after the current latest sequence. Useful for
    /// live-streaming new events without re-emitting historical ones.
    #[arg(long, conflicts_with = "from_sequence")]
    pub(crate) from_tail: bool,
    /// Restrict to events sharing one command frame.
    #[arg(long)]
    pub(crate) frame: Option<String>,
    /// Restrict to events for one cycle.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
    /// Poll interval in milliseconds.
    #[arg(long, default_value_t = 500)]
    pub(crate) interval_ms: u64,
    /// Cap on events emitted before exiting (0 = unlimited).
    #[arg(long, default_value_t = 0)]
    pub(crate) max_events: u64,
    /// Exit after this many milliseconds with no new events (0 = never).
    #[arg(long, default_value_t = 0)]
    pub(crate) idle_timeout_ms: u64,
    /// Output format: `text` (pretty per-line), `json` or `ndjson` (one
    /// JSON object per line, no pretty-printing).
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

pub(crate) fn run_ledger(command: LedgerCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        LedgerCommand::Verify(args) => run_ledger_verify(args, environment),
        LedgerCommand::VerifyChain(args) => run_verify_chain(args, environment),
        LedgerCommand::BackfillChain(args) => run_backfill_chain(args, environment),
        LedgerCommand::Events(args) => run_ledger_events(args, environment),
        LedgerCommand::Export(args) => run_ledger_export(args, environment),
        LedgerCommand::Replay(args) => run_replay(args, environment),
        LedgerCommand::Watch(args) => run_ledger_watch(args, environment),
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
    /// The stream verified, or the label describing the set verified when the
    /// caller named none.
    stream: String,
    /// How many streams the verdict covers, so a set is never reported as if it
    /// were one stream.
    streams: usize,
    event_count: usize,
    head_chain_hash: Option<String>,
    status: VerifyChainStatus,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum VerifyChainStatus {
    Pass,
    /// The named stream holds no events, so no chain was checked.
    Empty,
    Fail {
        error: String,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct BackfillChainOutput {
    /// The stream backfilled, or the label describing the set when the caller
    /// named none.
    stream: String,
    /// How many streams the count covers.
    streams: usize,
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

/// Envelope for `ledger events`. INC-DEBT-060, F63.
///
/// This was a bare `Vec<LedgerEventOutput>`, and that shape is the defect: an
/// array has nowhere to declare that it is a *window*. The command defaults to
/// the 50 newest events — measured on `p-63676b11dc0ef88f`: 50 of 590 events, 19
/// of 114 cycles named, and nothing on screen saying so — so anything reading
/// the payload as "the ledger" under-reads it by 540 events and cannot know.
///
/// The total costs nothing: `Storage::list_events` already loads every stream in
/// full (`canonical_events` walks them with `u32::MAX`) and the truncation
/// happens afterwards in memory. `total_events` is the length of the vector
/// *before* the `take`, which is information this command was already
/// discarding.
#[derive(Serialize)]
struct LedgerEventsOutput {
    /// The events actually shown, ascending by sequence.
    events: Vec<LedgerEventOutput>,
    /// How many events the ledger holds for this query, shown or not.
    total_events: usize,
    /// How many are in `events`. Redundant with `events.len()` on purpose: a
    /// consumer should not have to count an array to learn its own size.
    shown: usize,
    /// Whether anything was left out.
    truncated: bool,
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

/// Decides which streams a chain command covers, and how to name that set.
///
/// Both chain commands need this decision, and both once made it the same wrong
/// way independently: a `project:<id>` stream that no event has ever used. The
/// ledger is one file per project, so every stream it holds belongs to that
/// project and the honest default is the set itself.
fn resolve_streams(
    explicit: Option<&str>,
    available: Vec<String>,
    project_id: &str,
) -> (String, Vec<String>) {
    match explicit {
        Some(stream) => (stream.to_string(), vec![stream.to_string()]),
        None => (format!("all streams of {project_id}"), available),
    }
}

fn run_verify_chain(args: VerifyChainArgs, environment: &CliEnvironment) -> CommandOutput {
    use sddk_storage::event_store::SqliteEventStore;
    let format = args.format;
    let result = (|| -> anyhow::Result<VerifyChainOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let event_store = SqliteEventStore::open(context.paths.ledger.parent().unwrap())?;
        let project_id = context.identity.project_id.to_string();
        let (label, streams) = resolve_streams(
            args.stream.as_deref(),
            event_store.list_streams()?,
            &project_id,
        );
        verify_streams(&event_store, label, streams)
    })();
    render_result(result, format, verify_chain_text)
}

/// Verifies a resolved set of streams and reports one verdict for the set.
///
/// `label` is the name `resolve_streams` gave the set, passed in rather than
/// rebuilt: only the resolver knows whether the caller named one stream or took
/// the default. Recomputing it here answered an explicitly named stream with the
/// set's label, so the report named something other than what was examined.
///
/// A single named stream reports its own head. A set reports no head, because no
/// one event is the head of several streams and naming one would imply it is.
fn verify_streams(
    event_store: &sddk_storage::event_store::SqliteEventStore,
    label: String,
    streams: Vec<String>,
) -> anyhow::Result<VerifyChainOutput> {
    use sddk_domain::EventStore;
    let single = streams.len() == 1;
    let stream_count = streams.len();
    let mut event_count = 0usize;
    let mut failure: Option<String> = None;
    for stream in &streams {
        event_count += event_store.load_stream(stream, None, u32::MAX)?.len();
        if failure.is_none()
            && let Err(e) = event_store.verify_chain_integrity(stream)
        {
            failure = Some(format!("{stream}: {e}"));
        }
    }
    // Emptiness is judged by the events actually examined, not by how many names
    // the set holds: a name that resolves to no events is still nothing checked.
    let status = match (event_count, failure) {
        (0, _) => VerifyChainStatus::Empty,
        (_, Some(error)) => VerifyChainStatus::Fail { error },
        (_, None) => VerifyChainStatus::Pass,
    };
    let head_chain_hash = if single {
        event_store.head_chain_hash(&streams[0])?
    } else {
        None
    };
    Ok(VerifyChainOutput {
        stream: label,
        streams: stream_count,
        event_count,
        head_chain_hash,
        status,
    })
}

fn verify_chain_text(output: &VerifyChainOutput) -> String {
    let head = output.head_chain_hash.as_deref().unwrap_or("null");
    match &output.status {
        VerifyChainStatus::Pass => format!(
            "stream: {}\nstreams: {}\nevent_count: {}\nhead_chain_hash: {head}\nstatus: PASS\n",
            output.stream, output.streams, output.event_count
        ),
        VerifyChainStatus::Empty => format!(
            "stream: {}\nstreams: {}\nevent_count: {}\nhead_chain_hash: {head}\n\
             status: EMPTY\n\
             no events were verified, so this is not a statement that any chain is intact\n",
            output.stream, output.streams, output.event_count
        ),
        VerifyChainStatus::Fail { error } => format!(
            "stream: {}\nstreams: {}\nevent_count: {}\nhead_chain_hash: {head}\nstatus: FAIL\nerror: {}\n",
            output.stream, output.streams, output.event_count, error
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
        let project_id = context.identity.project_id.to_string();
        let (label, streams) = resolve_streams(
            args.stream.as_deref(),
            event_store.list_streams()?,
            &project_id,
        );
        let total = streams.len();
        let mut updated = 0usize;
        let mut failure: Option<String> = None;
        for stream in &streams {
            match event_store.backfill_chain_hash(stream) {
                Ok(n) => updated += n,
                Err(e) => {
                    if failure.is_none() {
                        failure = Some(format!("{stream}: {e}"));
                    }
                }
            }
        }
        let status = match failure {
            Some(error) => BackfillChainStatus::Fail { error },
            None => BackfillChainStatus::Success,
        };
        Ok(BackfillChainOutput {
            stream: label,
            streams: total,
            updated,
            status,
        })
    })();
    render_result(result, format, backfill_chain_text)
}

fn backfill_chain_text(output: &BackfillChainOutput) -> String {
    match &output.status {
        BackfillChainStatus::Success => format!(
            "stream: {}\nstreams: {}\nupdated: {}\nstatus: SUCCESS\n",
            output.stream, output.streams, output.updated
        ),
        BackfillChainStatus::Fail { error } => format!(
            "stream: {}\nstreams: {}\nupdated: {}\nstatus: FAIL\nerror: {}\n",
            output.stream, output.streams, output.updated, error
        ),
    }
}

fn run_ledger_events(args: LedgerEventsArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<LedgerEventsOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let all = match &args.frame {
            Some(frame) => context.storage.list_frame_events(frame)?,
            None => context.storage.list_events()?,
        };
        // Taken before the `take`, and that is the whole fix: the vector already
        // holds every event, so declaring what was left out costs no query and
        // no new storage API.
        let total_events = all.len();
        // `0` means all, matching `ledger export --limit 0` and
        // `ledger watch --max-events 0` in the same binary. It used to mean
        // *zero* here, which is the opposite convention one command away and
        // produced an empty listing with exit 0.
        let limit = if args.limit == 0 {
            usize::MAX
        } else {
            args.limit
        };
        let events: Vec<LedgerEventOutput> = all
            .into_iter()
            .rev()
            .take(limit)
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
            .collect();
        let shown = events.len();
        Ok(LedgerEventsOutput {
            truncated: shown < total_events,
            shown,
            total_events,
            events,
        })
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

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ReplayOutput {
    snapshot_name: String,
    stream: String,
    events_applied: u64,
    from_sequence: u64,
    to_sequence: u64,
    status: ReplayStatus,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
// allow(dead_code): owner=ledger; reason=reserved ReplayStatus expansion (Success+Fail match CLI exit codes); exit=when replay status is wired to CLI exit codes.
#[allow(dead_code)]
// Retained for future ReplayStatus expansion (Success + Fail states match CLI exit codes).
enum ReplayStatus {
    Success,
    Fail { error: String },
}

fn run_replay(args: ReplayArgs, environment: &CliEnvironment) -> CommandOutput {
    use sddk_domain::EventStore;
    use sddk_storage::event_store::SqliteEventStore;
    let format = args.format;
    let result = (|| -> anyhow::Result<ReplayOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let ledger_dir = context.paths.ledger.parent().unwrap();
        let event_store = SqliteEventStore::open(ledger_dir)?;

        // Load snapshot
        let snapshot: sddk_domain::replay::Snapshot = event_store
            .load_snapshot(&args.from_snapshot)
            .map_err(|e| anyhow::anyhow!("load_snapshot: {e}"))?
            .ok_or_else(|| anyhow::anyhow!("snapshot '{}' not found", args.from_snapshot))?;

        let stream = args.stream.unwrap_or_else(|| snapshot.stream_id.clone());

        if stream != snapshot.stream_id {
            anyhow::bail!(
                "stream '{}' does not match snapshot stream '{}'",
                stream,
                snapshot.stream_id
            );
        }

        // Verify chain if requested
        if args.verify_hashes {
            event_store
                .verify_stream_chain(&stream)
                .map_err(|e| anyhow::anyhow!("verify_stream_chain: {e}"))?;
            event_store
                .verify_chain_integrity(&stream)
                .map_err(|e| anyhow::anyhow!("verify_chain_integrity: {e}"))?;
        }

        // Load events after snapshot
        let events = event_store.load_stream(&stream, Some(snapshot.sequence), u32::MAX)?;
        let from_seq = snapshot.sequence + 1;
        let to_seq = events
            .last()
            .map(|e| e.sequence)
            .unwrap_or(snapshot.sequence);

        Ok(ReplayOutput {
            snapshot_name: snapshot.name,
            stream: stream.clone(),
            events_applied: events.len() as u64,
            from_sequence: from_seq,
            to_sequence: to_seq,
            status: ReplayStatus::Success,
        })
    })();
    render_result(result, format, replay_text)
}

fn replay_text(output: &ReplayOutput) -> String {
    match &output.status {
        ReplayStatus::Success => format!(
            "snapshot: {}\nstream: {}\nevents_applied: {}\nfrom_sequence: {}\nto_sequence: {}\nstatus: SUCCESS\n",
            output.snapshot_name,
            output.stream,
            output.events_applied,
            output.from_sequence,
            output.to_sequence
        ),
        ReplayStatus::Fail { error } => format!(
            "snapshot: {}\nstream: {}\nstatus: FAIL\nerror: {}\n",
            output.snapshot_name, output.stream, error
        ),
    }
}

fn ledger_verify_text(output: &LedgerVerifyOutput) -> String {
    format!(
        "event_count: {}\nlast_hash: {}\n",
        output.event_count,
        output.last_hash.as_deref().unwrap_or("null")
    )
}

fn ledger_events_text(output: &LedgerEventsOutput) -> String {
    let mut text = String::new();
    // The declaration comes first and comes in both cases on purpose. A count
    // that only appears when something was truncated cannot be read off a log
    // where nothing was truncated, and "nothing was truncated" is exactly the
    // claim a reader needs to be able to check.
    text.push_str(&format!(
        "events: {} of {} ({})\n",
        output.shown,
        output.total_events,
        if output.truncated {
            "truncated"
        } else {
            "complete"
        }
    ));
    // An empty ledger declares zero rather than printing a bare `no events`,
    // which reads as "there was nothing to look at" when what it should say is
    // "nothing was left out".
    for event in &output.events {
        text.push_str(&format!(
            "{} {} {} {} {}\n",
            event.sequence,
            event.event_type,
            event.event_id,
            event.frame_id,
            event.cycle_id.as_deref().unwrap_or("-")
        ));
    }
    text
}

/// Applies the `--cycle` and `--frame` narrowing that `ledger watch` uses, in
/// **one** place, called from two: the poll loop and the total it declares.
///
/// One function with two call sites is the whole point. Two copies of these two
/// `retain`s would be two rules declaring what "an event of this query" means —
/// free to drift, and nothing would notice. That is the defect this change
/// exists to close, reproduced one level down: `Storage::list_events_after`
/// walks every stream and then `.take(limit)`s, throwing the length away on
/// every poll from a `canonical_events()` that has already loaded the whole
/// ledger. So the count here is made of the *same* predicate as the emission,
/// not of a cheaper parallel query.
fn apply_watch_filters(events: &mut Vec<LedgerEvent>, cycle: Option<&str>, frame: Option<&str>) {
    if let Some(cycle) = cycle {
        events.retain(|ev| ev.cycle_id.as_deref() == Some(cycle));
    }
    if let Some(frame) = frame {
        events.retain(|ev| ev.frame_id == *frame);
    }
}

/// What `ledger watch` declares when it stops.
///
/// `emitted` is what this run showed. `total_events` is how many events the
/// ledger held for **this** query — the same start cursor and the same filters,
/// counted through the same [`apply_watch_filters`]. `pending` is **derived**,
/// never a third number written by hand: `saturating_sub` so a mid-run
/// deletion cannot panic, and the arithmetic closing is asserted by R3 in
/// `tests/ledger_watch_declaration.rs`.
///
/// The two renderers in `run_ledger_watch` read this one struct. A footer and
/// a JSON summary are two declarations of the same fact; writing their numbers
/// separately would make them free to disagree.
#[derive(Serialize)]
struct LedgerWatchSummary {
    emitted: u64,
    total_events: u64,
}

impl LedgerWatchSummary {
    fn pending(&self) -> u64 {
        self.total_events.saturating_sub(self.emitted)
    }
}

/// Tail the ledger in real time (M9.5 live-mode streaming).
///
/// Polls `Storage::list_events_after(last_seq, limit)` in a loop, writing
/// one event per line (NDJSON when `--format json`, pretty text when
/// `--format text`). Returns a [`CommandOutput`] containing the streamed
/// lines so unit tests can assert on them. In a real terminal session the
/// same lines flow to the process stdout via the captured `CommandOutput`
/// pipeline.
///
/// Exit conditions (in order):
///   * `--max-events` reached (0 = unlimited)
///   * `--idle-timeout-ms` elapsed with no new events (0 = never)
///   * SIGINT/EOF on stdin (handled by callers, not in-process)
fn run_ledger_watch(args: LedgerWatchArgs, environment: &CliEnvironment) -> CommandOutput {
    use std::fmt::Write as _;

    let mut stdout = String::new();
    let format = args.format;

    let result: anyhow::Result<LedgerWatchSummary> = (|| -> anyhow::Result<LedgerWatchSummary> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let mut after_sequence = args.from_sequence;
        let interval = std::time::Duration::from_millis(args.interval_ms.max(1));
        let max_events = args.max_events;
        let idle_timeout = std::time::Duration::from_millis(args.idle_timeout_ms);
        let started = std::time::Instant::now();
        let mut last_activity = std::time::Instant::now();
        let mut emitted: u64 = 0;

        // When `--from-tail` is requested, start strictly after the
        // current latest sequence so historical events are not re-emitted.
        if args.from_tail {
            let tail = context
                .storage
                .list_events()
                .context("loading current ledger tail for --from-tail")?;
            after_sequence = tail.last().map(|ev| ev.sequence).unwrap_or(0);
        }

        // The cursor this run *started* from, kept apart from the running
        // one: the total has to describe what this run could have emitted,
        // and using the cursor it finished on would answer a different
        // question — "how much is still there" instead of "how much was
        // there", which is a number that cannot be compared with `emitted`.
        let start_cursor = after_sequence;

        'poll: loop {
            // Bounded fetch: ask for a generous chunk per poll. SQLite
            // returns up to `limit` rows ordered by sequence ASC.
            let chunk_size: i64 = 256;
            let mut events = context
                .storage
                .list_events_after(after_sequence, chunk_size)
                .context("polling list_events_after")?;

            // Optional cycle/frame narrowing — `list_events_after` is
            // already narrow by sequence range, but cycle/frame filters
            // require a second pass.
            apply_watch_filters(&mut events, args.cycle.as_deref(), args.frame.as_deref());

            if events.is_empty() {
                if !idle_timeout.is_zero() && last_activity.elapsed() >= idle_timeout {
                    break;
                }
                std::thread::sleep(interval);
                // Defensive: avoid pathological infinite loop if a clock
                // skew or test harness pins time forward.
                if !idle_timeout.is_zero() && last_activity.elapsed() >= idle_timeout {
                    break;
                }
                continue;
            }

            for event in &events {
                let line = match format {
                    OutputFormat::Json => {
                        serde_json::to_string(event).context("serializing LedgerEvent to NDJSON")?
                    }
                    OutputFormat::Text => format!(
                        "{:>8}  {:<36}  {:<24}  cycle={}  frame={}",
                        event.sequence,
                        event.event_type,
                        event.event_id,
                        event.cycle_id.as_deref().unwrap_or("-"),
                        event.frame_id.as_str(),
                    ),
                };
                writeln!(stdout, "{line}").expect("writing to String never fails");
                after_sequence = event.sequence;
                emitted = emitted.saturating_add(1);
                if max_events > 0 && emitted >= max_events {
                    break 'poll;
                }
            }
            last_activity = std::time::Instant::now();
            // Safety bound: never loop forever in pathological cases
            // (e.g. test harness without an idle timeout). 5 minutes is
            // well beyond the longest expected run.
            if started.elapsed() > std::time::Duration::from_secs(300) {
                anyhow::bail!("ledger watch exceeded 5 minute safety bound");
            }
        }

        // Declaring the window. The cap is not a reason to stay silent
        // about it: "emitted 5" over a ledger of 591 is indistinguishable
        // from "that was all of them" unless the other 586 are named.
        // `i64::MAX` removes the cap for this one count, and the same
        // filter the loop used, so the number cannot disagree with the
        // events that were (or were not) emitted.
        let mut available = context
            .storage
            .list_events_after(start_cursor, i64::MAX)
            .context("counting the events this watch could have emitted")?;
        apply_watch_filters(&mut available, args.cycle.as_deref(), args.frame.as_deref());

        Ok(LedgerWatchSummary {
            emitted,
            total_events: available.len() as u64,
        })
    })();

    match result {
        Ok(summary) => {
            // For JSON mode, emit a one-line summary so callers can see
            // the loop ended cleanly. For text mode, write a footer line.
            // Both read the same struct, so the two cannot disagree.
            match format {
                OutputFormat::Json => {
                    let _ = writeln!(
                        stdout,
                        "{}",
                        serde_json::json!({
                            "__watch_complete": true,
                            "emitted": summary.emitted,
                            "total_events": summary.total_events,
                            "pending": summary.pending(),
                        })
                    );
                }
                OutputFormat::Text => {
                    let _ = writeln!(
                        stdout,
                        "[watch] emitted {} of {} ({} not emitted), exiting",
                        summary.emitted,
                        summary.total_events,
                        summary.pending()
                    );
                }
            }
            CommandOutput {
                status: 0,
                stdout,
                stderr: String::new(),
            }
        }
        Err(error) => failure_envelope(&error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // allow(dead_code): owner=ledger-tests; reason=shared test env() fixture; exit=remove if no test uses it.
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

#[cfg(test)]
mod chain_scope_tests {
    use sddk_domain::{ActorKind, ActorRef, EntityRef, EventEnvelopeV1, EventStore};
    use sddk_storage::event_store::SqliteEventStore;

    /// Appends one event to `stream` and returns the store holding it.
    fn store_with(stream: &str, project: &str, count: u64) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let mut store = SqliteEventStore::open(dir.path()).expect("a store");
        store
            .connection()
            .execute(
                "INSERT OR IGNORE INTO projects (project_id, display_name, scope, created_at)
                 VALUES (?1, ?1, '.', '2026-08-19T10:00:00Z')",
                rusqlite::params![project],
            )
            .expect("the project row exists");
        for sequence in 1..=count {
            let mut envelope = EventEnvelopeV1 {
                event_id: format!("evt-{stream}-{sequence}"),
                event_type: "cycle.phase.transitioned".into(),
                schema_version: 1,
                stream_id: stream.into(),
                sequence,
                project_id: project.into(),
                occurred_at: "2026-08-19T10:00:00Z".into(),
                recorded_at: "2026-08-19T10:00:00Z".into(),
                actor: ActorRef {
                    kind: ActorKind::System,
                    id: "scope-test".into(),
                    definition_hash: None,
                    policy_hash: None,
                    model: None,
                    role: None,
                },
                subjects: vec![EntityRef {
                    kind: "capability".into(),
                    id: "scope".into(),
                    version: None,
                    content_hash: None,
                }],
                payload: serde_json::Value::Null,
                evidence_refs: vec![],
                content_hash: String::new(),
                metadata: None,
                causation_id: None,
                correlation_id: None,
                cycle_id: Some("c-1".into()),
                frame_id: None,
                fork_id: None,
            };
            let computed = envelope.compute_content_hash();
            envelope.content_hash = computed;
            store.append(&envelope).expect("the event is appended");
        }
        drop(store);
        dir
    }

    fn open(dir: &tempfile::TempDir) -> SqliteEventStore {
        SqliteEventStore::open(dir.path()).expect("a store")
    }

    #[test]
    fn un_stream_vacio_no_se_declara_cadena_intacta() {
        // `verify_chain_integrity` answers Ok for zero events, which is true of
        // zero events and false as a claim about a ledger. The command must not
        // pass that truth along as a verdict.
        let dir = tempfile::tempdir().expect("a temporary directory");
        let store = SqliteEventStore::open(dir.path()).expect("a store");
        let output = super::verify_streams(
            &store,
            "cycle:p-none/never-ran".to_string(),
            vec!["cycle:p-none/never-ran".to_string()],
        )
        .expect("the verification runs");

        assert_eq!(
            output.event_count, 0,
            "the stream really is empty; that is the premise of this test"
        );
        assert!(
            !matches!(output.status, super::VerifyChainStatus::Pass),
            "a stream with no events cannot be reported as an intact chain"
        );
        assert!(
            matches!(output.status, super::VerifyChainStatus::Empty),
            "the absence must be named, not left to be read as a pass: {:?}",
            output.status
        );
    }

    #[test]
    fn el_por_defecto_verifica_los_streams_que_existen() {
        // The previous default named `project:<id>`, a stream no event has ever
        // used, so it selected nothing and reported the selection as a pass.
        let dir = store_with("cycle:p-demo/one", "p-demo", 2);
        let dir_two = store_with("cycle:p-demo/two", "p-demo", 3);
        // Both streams belong to one ledger, so mirror the second into the first.
        let store = open(&dir);
        for event in open(&dir_two)
            .load_stream("cycle:p-demo/two", None, u32::MAX)
            .unwrap()
        {
            let mut copy = event;
            copy.event_id = format!("copy-{}", copy.event_id);
            let mut writable = SqliteEventStore::open(dir.path()).unwrap();
            let computed = copy.compute_content_hash();
            copy.content_hash = computed;
            writable.append(&copy).expect("the event is appended");
        }

        let (label, streams) = super::resolve_streams(
            None,
            store.list_streams().expect("the streams are listed"),
            "p-demo",
        );
        let output =
            super::verify_streams(&store, label.clone(), streams).expect("the verification runs");
        assert_eq!(
            label, "all streams of p-demo",
            "the default names the set it covers"
        );
        assert_eq!(
            output.stream, label,
            "the verdict is reported under the name the resolver gave it"
        );
        assert_eq!(
            output.streams, 2,
            "both streams of the ledger must be covered, not one invented stream"
        );
        assert_eq!(
            output.event_count, 5,
            "every event of every stream is counted"
        );
        assert!(
            matches!(output.status, super::VerifyChainStatus::Pass),
            "an intact chain over real events is a pass: {:?}",
            output.status
        );
    }

    #[test]
    fn un_stream_nombrado_se_responde_con_su_nombre() {
        // The label must name what was asked for. The two tests above both go
        // through the default path, so nothing pinned the explicit one — and
        // `verify_streams` rebuilt the label with `resolve_streams(None, ..)`.
        // That answered `--stream cycle:p-demo/one` with "all streams of
        // p-demo": a set label for one named stream, which is the same class of
        // mistake this commit exists to remove. The report named something other
        // than what was examined.
        let dir = store_with("cycle:p-demo/one", "p-demo", 2);
        let store = open(&dir);

        let (label, streams) = super::resolve_streams(
            Some("cycle:p-demo/one"),
            store.list_streams().expect("the streams are listed"),
            "p-demo",
        );
        let output =
            super::verify_streams(&store, label.clone(), streams).expect("the verification runs");

        assert_eq!(
            output.stream, "cycle:p-demo/one",
            "an explicitly named stream must be echoed, not relabelled as the whole set"
        );
        assert_eq!(
            label, output.stream,
            "the verdict must carry the name the resolver produced"
        );
        assert_eq!(output.streams, 1, "one named stream is one stream");
        assert!(
            output.head_chain_hash.is_some(),
            "a single named stream has a head that may be reported"
        );
    }
}
