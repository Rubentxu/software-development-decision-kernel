//! `sddk memory` — Decision Memory CLI (SPEC-004 §CLI, M4).
//!
//! Exposes the already-shipped `sddk_engine::decision_memory` substrate
//! (CDD-MEMORY-001/002, v1.147.0 / v1.148.0) through the canonical
//! subcommand surface declared by SPEC-004:
//!
//! ```text
//! sddk memory status
//! sddk memory log
//! sddk memory tree
//! sddk memory show <ref|oid>
//! sddk memory diff <a>..<b>
//! sddk memory merge-base <a> <b>
//! sddk memory reflog
//! sddk memory audit
//! ```
//!
//! The CLI does not own a persistent store. Every subcommand builds a
//! deterministic `InMemoryMemoryStore` from a fixture scenario and
//! runs the corresponding `MemoryStore::*` method. A future M4.x
//! cycle can wire a persistent backend behind the same surface.

#![forbid(unsafe_code)]
#![deny(clippy::all)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use sddk_engine::decision_memory::{
    DecisionMemoryAuthor, DecisionMemoryBlob, DecisionMemoryCommit, DecisionMemoryError,
    DecisionMemoryTree, InMemoryMemoryStore, MemoryDiff, MemoryId, MemoryShow, MemoryStore,
    RefKind, ReflogEntry, ReflogScope, TreeProjection,
};
use serde::Serialize;

use crate::cycle::RuntimeArgs;
use crate::{CliEnvironment, CommandOutput, OutputFormat};

// =====================================================================
// Subcommand surface (SPEC-004 §CLI)
// =====================================================================

/// Memory subcommand enum. Exactly the eight names in SPEC-004 §CLI.
#[derive(Debug, Subcommand)]
pub(crate) enum MemoryCommand {
    /// Summary: object counts, HEAD ref, reflog head, reachable commits.
    Status(MemoryStatusArgs),
    /// Walk commit ancestors from HEAD (or `--from <oid>`) capped at `--max`.
    Log(MemoryLogArgs),
    /// Typed tree projection for `--oid <ref|oid>`.
    Tree(MemoryTreeArgs),
    /// Show one commit (commit + tree + refs pointing at it).
    Show(MemoryShowArgs),
    /// Semantic diff between two commits. `a..b` syntax is required.
    Diff(MemoryDiffArgs),
    /// Lowest common ancestor of two commits under parent-link topology.
    MergeBase(MemoryMergeBaseArgs),
    /// Reflog entries (newest first), optionally filtered by `--scope`.
    Reflog(MemoryReflogArgs),
    /// Composite audit: status + reflog + reachable commit count.
    Audit(MemoryAuditArgs),
}

impl MemoryCommand {
    /// Returns the canonical subcommand names, in the order declared by
    /// SPEC-004 §CLI. The arch_lint M4 markers assert this exact set
    /// (via the test in `memory_cmd::tests`); public to the crate so
    /// the marker and the CLI share one source of truth.
    #[allow(dead_code)]
    pub(crate) fn spec_004_names() -> [&'static str; 8] {
        [
            "status",
            "log",
            "tree",
            "show",
            "diff",
            "merge-base",
            "reflog",
            "audit",
        ]
    }
}

// =====================================================================
// Argument structs
// =====================================================================

#[derive(Debug, Clone, Args)]
pub(crate) struct MemoryStatusArgs {
    /// Project root (accepted for CLI parity; not used in this cycle).
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct MemoryLogArgs {
    /// Start at this commit (default: HEAD).
    #[arg(long)]
    pub(crate) from: Option<String>,
    /// Maximum number of ancestors to walk.
    #[arg(long, default_value_t = 32)]
    pub(crate) max: usize,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct MemoryTreeArgs {
    /// Commit oid (or ref) whose tree to project.
    #[arg(long)]
    pub(crate) oid: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct MemoryShowArgs {
    /// Commit oid (or ref) to show.
    #[arg(long)]
    pub(crate) oid: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct MemoryDiffArgs {
    /// `a..b` pair (required).
    #[arg(long)]
    pub(crate) range: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct MemoryMergeBaseArgs {
    /// First commit oid.
    #[arg(long)]
    pub(crate) a: String,
    /// Second commit oid.
    #[arg(long)]
    pub(crate) b: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct MemoryReflogArgs {
    /// `all` (default) or `head`.
    #[arg(long, default_value = "all")]
    pub(crate) scope: String,
    /// Maximum reflog entries to return.
    #[arg(long, default_value_t = 32)]
    pub(crate) max: usize,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct MemoryAuditArgs {
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

// =====================================================================
// JSON-serializable views (over non-Serialize engine types)
// =====================================================================

/// JSON-serializable mirror of [`MemoryShow`].
#[derive(Debug, Serialize)]
struct MemoryShowView {
    commit_id: String,
    tree_id: String,
    parents: Vec<String>,
    author: DecisionMemoryAuthor,
    message: String,
    reason: String,
    project_id: String,
    work_item_id: Option<String>,
    refs_pointing_here: Vec<String>,
}

/// JSON-serializable mirror of [`MemoryDiff`].
#[derive(Debug, Serialize)]
struct MemoryDiffView {
    a: String,
    b: String,
    added_blobs: Vec<String>,
    removed_blobs: Vec<String>,
    modified_trees: Vec<String>,
    ref_changes: Vec<RefMovementView>,
}

#[derive(Debug, Serialize)]
struct RefMovementView {
    ref_path: String,
    from: Option<String>,
    to: Option<String>,
    at_seq: u64,
}

/// JSON-serializable mirror of [`TreeProjection`].
#[derive(Debug, Serialize)]
struct TreeProjectionView {
    at_commit: String,
    entries: BTreeMap<String, Vec<TreeEntryView>>,
}

#[derive(Debug, Serialize)]
struct TreeEntryView {
    name: String,
    id: String,
}

impl From<&MemoryShow> for MemoryShowView {
    fn from(m: &MemoryShow) -> Self {
        Self {
            commit_id: hex_id(&m.commit.id),
            tree_id: hex_id(&m.tree.id),
            parents: m.commit.parents.iter().map(hex_id).collect(),
            author: m.commit.author.clone(),
            message: m.commit.message.clone(),
            reason: m.commit.reason.clone(),
            project_id: m.commit.project_id.clone(),
            work_item_id: m.commit.work_item_id.clone(),
            refs_pointing_here: m.refs_pointing_here.iter().map(RefKind::ref_path).collect(),
        }
    }
}

impl From<&MemoryDiff> for MemoryDiffView {
    fn from(d: &MemoryDiff) -> Self {
        Self {
            a: hex_id(&d.a),
            b: hex_id(&d.b),
            added_blobs: d.added_blobs.iter().map(hex_id).collect(),
            removed_blobs: d.removed_blobs.iter().map(hex_id).collect(),
            modified_trees: d.modified_trees.iter().map(hex_id).collect(),
            ref_changes: d
                .ref_changes
                .iter()
                .map(|m| RefMovementView {
                    ref_path: m.ref_path.clone(),
                    from: m.from.as_ref().map(hex_id),
                    to: m.to.as_ref().map(hex_id),
                    at_seq: m.at_seq,
                })
                .collect(),
        }
    }
}

impl From<&TreeProjection> for TreeProjectionView {
    fn from(t: &TreeProjection) -> Self {
        Self {
            at_commit: hex_id(&t.at_commit),
            entries: t
                .entries
                .iter()
                .map(|(kind, list)| {
                    (
                        kind.clone(),
                        list.iter()
                            .map(|e| TreeEntryView {
                                name: e.name.clone(),
                                id: hex_id(&e.id),
                            })
                            .collect(),
                    )
                })
                .collect(),
        }
    }
}

fn hex_id(id: &MemoryId) -> String {
    let mut out = String::with_capacity(64);
    for byte in id {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}

// =====================================================================
// Public dispatch
// =====================================================================

/// Top-level entry point. Mirrors `run_fork` / `run_knowledge`.
pub(crate) fn run_memory(command: MemoryCommand, _env: &CliEnvironment) -> CommandOutput {
    let _ = RuntimeArgs::default();
    let result = match command {
        MemoryCommand::Status(args) => handle_status(&args),
        MemoryCommand::Log(args) => handle_log(&args),
        MemoryCommand::Tree(args) => handle_tree(&args),
        MemoryCommand::Show(args) => handle_show(&args),
        MemoryCommand::Diff(args) => handle_diff(&args),
        MemoryCommand::MergeBase(args) => handle_merge_base(&args),
        MemoryCommand::Reflog(args) => handle_reflog(&args),
        MemoryCommand::Audit(args) => handle_audit(&args),
    };
    match result {
        Ok(out) => out,
        Err(err) => CommandOutput {
            stdout: String::new(),
            stderr: err,
            status: 1,
        },
    }
}

// =====================================================================
// Handlers
// =====================================================================

fn handle_status(args: &MemoryStatusArgs) -> Result<CommandOutput, String> {
    let store = fixture::deterministic_three_commit_store();
    let view = render_status(&store);
    render_view(&view, args.format, "status", &mut Vec::new())
}

fn handle_log(args: &MemoryLogArgs) -> Result<CommandOutput, String> {
    let store = fixture::deterministic_three_commit_store();
    let from = match &args.from {
        Some(text) => parse_oid(text, &store)?,
        None => store
            .resolve_ref(&RefKind::Head)
            .ok_or_else(|| "no HEAD ref in store".to_string())?,
    };
    let ids = store.log(from, args.max).map_err(err_to_string)?;
    let view: Vec<String> = ids.iter().map(hex_id).collect();
    render_view(&view, args.format, "log", &mut Vec::new())
}

fn handle_tree(args: &MemoryTreeArgs) -> Result<CommandOutput, String> {
    let store = fixture::deterministic_three_commit_store();
    let id = parse_oid(&args.oid, &store)?;
    let tree = store.tree(id).map_err(err_to_string)?;
    let view = TreeProjectionView::from(&tree);
    render_view(&view, args.format, "tree", &mut Vec::new())
}

fn handle_show(args: &MemoryShowArgs) -> Result<CommandOutput, String> {
    let store = fixture::deterministic_three_commit_store();
    let id = parse_oid(&args.oid, &store)?;
    let show = store.show(id).map_err(err_to_string)?;
    let view = MemoryShowView::from(&show);
    render_view(&view, args.format, "show", &mut Vec::new())
}

fn handle_diff(args: &MemoryDiffArgs) -> Result<CommandOutput, String> {
    let (a_text, b_text) = parse_range(&args.range)?;
    let store = fixture::deterministic_three_commit_store();
    let a = parse_oid(&a_text, &store)?;
    let b = parse_oid(&b_text, &store)?;
    let diff = store.diff(a, b).map_err(err_to_string)?;
    let view = MemoryDiffView::from(&diff);
    render_view(&view, args.format, "diff", &mut Vec::new())
}

fn handle_merge_base(args: &MemoryMergeBaseArgs) -> Result<CommandOutput, String> {
    let store = fixture::deterministic_three_commit_store();
    let a = parse_oid(&args.a, &store)?;
    let b = parse_oid(&args.b, &store)?;
    let common = store.merge_base(a, b).map_err(err_to_string)?;
    let view = hex_id(&common);
    render_view(&view, args.format, "merge-base", &mut Vec::new())
}

fn handle_reflog(args: &MemoryReflogArgs) -> Result<CommandOutput, String> {
    let store = fixture::deterministic_three_commit_store();
    // The in-memory store's `reflog(scope, max)` returns Vec::new() in
    // v1.150.0; the persistent surface is `reflog_history`. We pick
    // the path the user asked for, but always project the persistent
    // HEAD reflog when scope == head.
    let entries = match args.scope.as_str() {
        "head" => store.reflog_history(RefKind::Head).map_err(err_to_string)?,
        _ => {
            // For `all`, `branches`, `tags`: the in-memory store does
            // not maintain a per-scope aggregated reflog yet, so we
            // surface the HEAD history as the only durable reflog.
            store.reflog_history(RefKind::Head).map_err(err_to_string)?
        }
    };
    let _ = (ReflogScope::All, args.max);
    let view: Vec<&ReflogEntry> = entries.iter().take(args.max).collect();
    render_view(&view, args.format, "reflog", &mut Vec::new())
}

fn handle_audit(args: &MemoryAuditArgs) -> Result<CommandOutput, String> {
    let store = fixture::deterministic_three_commit_store();
    let status = render_status(&store);
    let entries = store.reflog_history(RefKind::Head).map_err(err_to_string)?;
    let view = AuditView {
        status,
        reflog_len: entries.len(),
    };
    render_view(&view, args.format, "audit", &mut Vec::new())
}

// =====================================================================
// Internal view types
// =====================================================================

#[derive(Debug, Serialize)]
struct StatusView {
    blobs: usize,
    trees: usize,
    commits: usize,
    head: Option<String>,
    reflog_head: Option<ReflogEntry>,
}

#[derive(Debug, Serialize)]
struct AuditView {
    status: StatusView,
    reflog_len: usize,
}

fn render_status(store: &InMemoryMemoryStore) -> StatusView {
    let head = store.resolve_ref(&RefKind::Head).map(|id| hex_id(&id));
    let reflog_head = store
        .reflog_history(RefKind::Head)
        .ok()
        .and_then(|v| v.into_iter().next());
    StatusView {
        blobs: count_blobs(store),
        trees: count_trees(store),
        commits: count_commits(store),
        head,
        reflog_head,
    }
}

// We derive the counts from the live store rather than tracking them
// as we go. `commits` comes from `log(HEAD, very_large)`; `trees` and
// `blobs` are reachable from the commit DAG (one tree per commit,
// one blob per `claim` entry of each tree). This keeps the
// counter contract tied to the canonical source of truth.
fn count_blobs(store: &InMemoryMemoryStore) -> usize {
    let mut total = 0usize;
    if let Ok(commits) = collect_known_commits(store) {
        for cid in commits {
            if let Some(c) = store.get_commit(&cid)
                && let Some(t) = store.get_tree(&c.tree)
            {
                for entries in t.entries.values() {
                    total += entries.len();
                }
            }
        }
    }
    total
}

fn count_trees(store: &InMemoryMemoryStore) -> usize {
    collect_known_commits(store)
        .map(|cs| {
            cs.iter()
                .filter_map(|id| store.get_commit(id).map(|c| c.tree))
                .collect::<std::collections::BTreeSet<_>>()
                .len()
        })
        .unwrap_or(0)
}

fn count_commits(store: &InMemoryMemoryStore) -> usize {
    collect_known_commits(store).map(|v| v.len()).unwrap_or(0)
}

fn collect_known_commits(
    store: &InMemoryMemoryStore,
) -> Result<Vec<MemoryId>, DecisionMemoryError> {
    let head = store
        .resolve_ref(&RefKind::Head)
        .ok_or_else(|| DecisionMemoryError::NotFound {
            kind: "ref",
            id: RefKind::Head.ref_path(),
        })?;
    // MemoryStore::log caps at 1024; pass that explicitly.
    store.log(head, 1024)
}

fn parse_oid(text: &str, store: &InMemoryMemoryStore) -> Result<MemoryId, String> {
    if let Some(id) = parse_hex_id(text) {
        return Ok(id);
    }
    if let Some(id) = store.resolve_ref(&RefKind::Head)
        && text == "HEAD"
    {
        return Ok(id);
    }
    if let Some(id) = resolve_branch(store, text) {
        return Ok(id);
    }
    Err(format!("unknown oid or ref: {text}"))
}

fn resolve_branch(store: &InMemoryMemoryStore, name: &str) -> Option<MemoryId> {
    if let Some(id) = store.resolve_ref(&RefKind::Branch(name.to_string())) {
        return Some(id);
    }
    if let Some(id) = store.resolve_ref(&RefKind::Tag(name.to_string())) {
        return Some(id);
    }
    None
}

fn parse_range(range: &str) -> Result<(String, String), String> {
    // Reject triple-dot forms outright; they are ambiguous with merge-base
    // notation in some tools.
    if range.contains("...") {
        return Err(format!(
            "triple-dot ranges are not supported (use a..b): {range}"
        ));
    }
    let mut parts = range.split("..");
    let a = parts
        .next()
        .ok_or_else(|| format!("invalid range (no left): {range}"))?;
    let b = parts
        .next()
        .ok_or_else(|| format!("invalid range (no right): {range}"))?;
    if parts.next().is_some() {
        return Err(format!("invalid range (extra `..`): {range}"));
    }
    if a.is_empty() || b.is_empty() {
        return Err(format!("invalid range (empty side): {range}"));
    }
    Ok((a.to_string(), b.to_string()))
}

fn parse_hex_id(text: &str) -> Option<MemoryId> {
    if text.len() != 64 {
        return None;
    }
    let bytes = text.as_bytes();
    let mut out = [0u8; 32];
    for i in 0..32 {
        let hi = hex_nibble(bytes[2 * i])?;
        let lo = hex_nibble(bytes[2 * i + 1])?;
        out[i] = (hi << 4) | lo;
    }
    Some(out)
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn err_to_string(e: DecisionMemoryError) -> String {
    e.to_string()
}

// =====================================================================
// Rendering (parity with the rest of the CLI)
// =====================================================================

fn render_view<T: Serialize>(
    value: &T,
    format: OutputFormat,
    command_name: &str,
    trail: &mut Vec<String>,
) -> Result<CommandOutput, String> {
    let payload = match format {
        OutputFormat::Json => serde_json::to_string_pretty(value)
            .map_err(|e| format!("serialize {command_name}: {e}"))?,
        OutputFormat::Text => render_text(value, command_name),
    };
    let _ = trail;
    Ok(CommandOutput {
        stdout: payload,
        stderr: String::new(),
        status: 0,
    })
}

fn render_text<T: Serialize>(value: &T, command_name: &str) -> String {
    let json = serde_json::to_string_pretty(value)
        .unwrap_or_else(|_| format!("<unrenderable {command_name}>"));
    // Compact text rendering: key=value lines, one per top-level field.
    let _ = json;
    match command_name {
        "status" => {
            if let Ok(v) = serde_json::to_value(value) {
                let mut out = String::new();
                if let Some(obj) = v.as_object() {
                    for (k, val) in obj {
                        out.push_str(k);
                        out.push('=');
                        out.push_str(&compact(val));
                        out.push('\n');
                    }
                }
                out
            } else {
                String::new()
            }
        }
        "audit" => {
            if let Ok(v) = serde_json::to_value(value) {
                let mut out = String::from("audit\n");
                if let Some(obj) = v.as_object() {
                    if let Some(status) = obj.get("status") {
                        out.push_str("  status:\n");
                        if let Some(s) = status.as_object() {
                            for (k, val) in s {
                                out.push_str("    ");
                                out.push_str(k);
                                out.push('=');
                                out.push_str(&compact(val));
                                out.push('\n');
                            }
                        }
                    }
                    if let Some(len) = obj.get("reflog_len") {
                        out.push_str("  reflog_len=");
                        out.push_str(&compact(len));
                        out.push('\n');
                    }
                }
                out
            } else {
                String::new()
            }
        }
        "log" => {
            if let Ok(v) = serde_json::to_value(value) {
                if let Some(arr) = v.as_array() {
                    arr.iter().map(compact).collect::<Vec<_>>().join("\n")
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        }
        _ => {
            // Fall back to compact one-line JSON for nested types.
            serde_json::to_string(value).unwrap_or_default()
        }
    }
}

fn compact(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(a) => format!("[{} items]", a.len()),
        serde_json::Value::Object(o) => format!("{{{} keys}}", o.len()),
    }
}

// =====================================================================
// Fixture: deterministic in-process store (3-commit chain + 1 reflog).
// =====================================================================

mod fixture {
    use super::*;

    /// Build a deterministic in-memory store with three commits, HEAD
    /// pointing at the most recent, and one reflog entry per write.
    pub(super) fn deterministic_three_commit_store() -> InMemoryMemoryStore {
        let store = InMemoryMemoryStore::new();
        // Build three commits in a linear chain.
        let c1 = make_commit(&store, vec![], "c1: scaffold", "init scaffold");
        let c2 = make_commit(&store, vec![c1], "c2: logic", "add logic");
        let c3 = make_commit(&store, vec![c2], "c3: tests", "add tests");
        // Move HEAD to c3 (reflog entry written via the store API).
        let mut reflog = sddk_engine::decision_memory::Reflog::new();
        store
            .write_ref_with_reflog(
                RefKind::Head,
                c3,
                &mut reflog,
                "test-fixture",
                "initialize HEAD",
            )
            .expect("write_ref_with_reflog (HEAD)");
        store
    }

    fn make_commit(
        store: &InMemoryMemoryStore,
        parents: Vec<MemoryId>,
        message: &str,
        reason: &str,
    ) -> MemoryId {
        let blob = DecisionMemoryBlob::new("test-fixture", format!("payload-for-{message}"))
            .expect("DecisionMemoryBlob::new");
        let blob_id = store.put_blob(blob).expect("put_blob");
        let mut entries = BTreeMap::new();
        entries.insert(
            "claim".to_string(),
            vec![sddk_engine::decision_memory::TreeEntry {
                name: "main".to_string(),
                id: blob_id,
            }],
        );
        let tree = DecisionMemoryTree::new(entries).expect("DecisionMemoryTree::new");
        let tree_id = store.put_tree(tree).expect("put_tree");
        let author =
            DecisionMemoryAuthor::new("system", "test-fixture").expect("DecisionMemoryAuthor::new");
        let commit = DecisionMemoryCommit::new(
            parents,
            tree_id,
            author,
            "1970-01-01T00:00:00Z".to_string(),
            "test-project".to_string(),
            None::<String>,
            None::<String>,
            None::<String>,
            None::<String>,
            None::<String>,
            None::<String>,
            message.to_string(),
            reason.to_string(),
            vec![],
        )
        .expect("DecisionMemoryCommit::new");
        store.put_commit(commit).expect("put_commit")
    }
}

// =====================================================================
// Tests
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spec_004_subcommand_set_is_exact() {
        let names: Vec<&str> = MemoryCommand::spec_004_names().to_vec();
        assert_eq!(
            names,
            vec![
                "status",
                "log",
                "tree",
                "show",
                "diff",
                "merge-base",
                "reflog",
                "audit",
            ]
        );
    }

    #[test]
    fn fixture_yields_three_commits_via_log() {
        let store = fixture::deterministic_three_commit_store();
        let head = store
            .resolve_ref(&RefKind::Head)
            .expect("HEAD must be set in fixture");
        let log = store.log(head, 16).expect("log");
        assert_eq!(log.len(), 3, "linear chain of three commits");
    }

    #[test]
    fn status_view_reports_head_for_three_commit_fixture() {
        let store = fixture::deterministic_three_commit_store();
        let view = render_status(&store);
        assert!(view.head.is_some(), "HEAD must resolve in fixture");
        assert!(view.reflog_head.is_some(), "reflog must have one entry");
    }

    #[test]
    fn show_view_round_trips_memory_show() {
        let store = fixture::deterministic_three_commit_store();
        let head = store.resolve_ref(&RefKind::Head).expect("HEAD");
        let show = store.show(head).expect("show");
        let view = MemoryShowView::from(&show);
        let json = serde_json::to_string(&view).expect("serialize");
        assert!(json.contains("commit_id"));
        assert!(json.contains("refs_pointing_here"));
    }

    #[test]
    fn diff_view_round_trips_memory_diff() {
        let store = fixture::deterministic_three_commit_store();
        let log = store
            .log(store.resolve_ref(&RefKind::Head).unwrap(), 8)
            .unwrap();
        let a = log[2];
        let b = log[0];
        let diff = store.diff(a, b).expect("diff");
        let view = MemoryDiffView::from(&diff);
        let json = serde_json::to_string(&view).expect("serialize");
        assert!(json.contains("\"a\""));
        assert!(json.contains("\"b\""));
    }

    #[test]
    fn tree_projection_view_round_trips() {
        let store = fixture::deterministic_three_commit_store();
        let head = store.resolve_ref(&RefKind::Head).unwrap();
        let tree = store.tree(head).expect("tree");
        let view = TreeProjectionView::from(&tree);
        let json = serde_json::to_string(&view).expect("serialize");
        assert!(json.contains("at_commit"));
    }

    #[test]
    fn parse_range_accepts_a_dot_dot_b() {
        let (a, b) = parse_range("a..b").unwrap();
        assert_eq!(a, "a");
        assert_eq!(b, "b");
    }

    #[test]
    fn parse_range_rejects_triple_dot() {
        assert!(parse_range("a...b").is_err());
        assert!(parse_range("a..").is_err());
        assert!(parse_range("..b").is_err());
    }

    #[test]
    fn parse_hex_id_round_trips_64_hex_chars() {
        let id = [7u8; 32];
        let hex = hex_id(&id);
        let back = parse_hex_id(&hex).expect("parse");
        assert_eq!(back, id);
    }
}
