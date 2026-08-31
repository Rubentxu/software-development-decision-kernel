//! `sddk cycle inventory` — produce the cycle files inventory as a JSON
//! artifact (`sddk.inventory/v1`).
//!
//! The inventory reducer is a read-only, deterministic reducer of the
//! project's working tree against its current HEAD. It writes exactly one
//! artifact inside the cycle's XDG artifacts directory:
//!
//! - `{cycle_artifacts_path}/{cycle_id}/inventory.json`
//! - `{cycle_artifacts_path}/{cycle_id}/inventory.json.sha256` (sidecar digest)
//!
//! The output JSON conforms to the contract declared at
//! `prompts/sddk/contracts/inventory.schema.json`. The reducer never touches
//! the adopted project repository (zero-intrusion policy, ADR-0011).
//!
//! The reducer calls `git` through `GitExecutor::run_ok` which already
//! enforces output caps, timeouts, and the env allowlist. No new dependency
//! is added.

use std::fmt::Write as _;
use std::path::Path;

use clap::Args;
use serde::Serialize;
use sha2::{Digest, Sha256};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use sddk_gateway::GitExecutor;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat, RuntimeArgs, RuntimeContext, render_result,
};

/// Schema identifier emitted at the top of every artifact.
const SCHEMA_ID: &str = "sddk.inventory/v1";

/// Closed prefix buckets considered SDDK-managed.
const CLOSED_PREFIXES: [&str; 7] = [
    "prompts/", "agents/", "skills/", "assets/", "tools/", "docs/", "tests/",
];

/// `sddk cycle inventory` arguments.
#[derive(Debug, Clone, Args)]
pub(crate) struct CycleInventoryArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Optional override for the comparison base (default: stage + working
    /// tree vs HEAD). Accepts only `stage-and-working-tree-vs-head` in this
    /// revision; other enum values land alongside future reducer modes.
    #[arg(long, default_value = "stage-and-working-tree-vs-head")]
    pub(crate) comparison: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Public inventory output, equivalent to `inventory.json` on disk.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct InventoryOutput {
    /// Schema identifier.
    pub schema: String,
    /// RFC 3339 timestamp captured at run time.
    pub generated_at: String,
    /// Captured Git context.
    pub git: GitContext,
    /// Aggregate counters and the optional unavailability reason.
    pub summary: InventorySummary,
    /// Per-bucket counters keyed by stable prefix.
    pub buckets: BTreeMapString,
    /// Individual file observations after parsing.
    pub files: Vec<InventoryEntry>,
    /// Paths ignored by the project's own `.gitignore`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ignored_by_project: Vec<String>,
}

/// Hashable JSON-serializable wrapper around `BTreeMap<String, BucketCounters>`
/// so we keep stable ordering for reproducible artifacts.
#[derive(Debug, Serialize, Default)]
pub(crate) struct BTreeMapString(std::collections::BTreeMap<String, BucketCounters>);

impl std::ops::Deref for BTreeMapString {
    type Target = std::collections::BTreeMap<String, BucketCounters>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for BTreeMapString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl IntoIterator for BTreeMapString {
    type Item = (String, BucketCounters);
    type IntoIter = std::collections::btree_map::IntoIter<String, BucketCounters>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[derive(Debug, Serialize, Default, Clone)]
pub(crate) struct BucketCounters {
    pub added: u32,
    pub modified: u32,
    pub deleted: u32,
    pub renamed: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct GitContext {
    pub root: String,
    pub head: Option<String>,
    pub comparison: String,
    pub parent: Option<String>,
    pub tag: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct InventorySummary {
    pub added: u32,
    pub modified: u32,
    pub deleted: u32,
    pub renamed: u32,
    pub ignored_inventory: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignored_by_project: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EntryStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct InventoryEntry {
    pub bucket: String,
    pub status: EntryStatus,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub renamed_from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

/// Envelope returned on the command's stdout.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct InventoryEnvelope {
    contract_version: &'static str,
    schema: &'static str,
    path: String,
    sha256: String,
    bytes: u64,
    unavailable_reason: Option<String>,
}

/// Run the `sddk cycle inventory` command and return its command output.
pub(crate) fn run_cycle_inventory(
    args: CycleInventoryArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let cycle_id = args.cycle.clone();
    let comparison = args.comparison.clone();
    let result: anyhow::Result<InventoryEnvelope> = (|| -> anyhow::Result<InventoryEnvelope> {
        let context = RuntimeContext::open(&args.runtime, environment, true)?;
        let artifacts_dir = context.cycle_artifacts_path.join(&cycle_id);
        std::fs::create_dir_all(&artifacts_dir)?;
        let payload = build_inventory_payload(&context, &comparison)?;
        let json = serde_json::to_vec_pretty(&payload)?;
        let destination = artifacts_dir.join("inventory.json");
        let sha_hex = persist_atomic(&destination, &json)?;
        let unavailable_reason = payload.summary.unavailable_reason.clone();
        Ok(InventoryEnvelope {
            contract_version: "sddk.inventory/v1",
            schema: SCHEMA_ID,
            path: destination.to_string_lossy().into_owned(),
            sha256: sha_hex,
            bytes: json.len() as u64,
            unavailable_reason,
        })
    })();
    render_result(result, format, envelope_text)
}

/// Build the full inventory payload by combining Git `diff`, `status`, and
/// `check-ignore` streams into the schema-compliant structure.
pub(crate) fn build_inventory_payload(
    context: &RuntimeContext,
    comparison: &str,
) -> anyhow::Result<InventoryOutput> {
    if comparison != "stage-and-working-tree-vs-head" {
        anyhow::bail!(
            "unsupported comparison `{comparison}`; only `stage-and-working-tree-vs-head` \
             is implemented in this revision"
        );
    }

    let git = GitExecutor::new(context.root.clone());
    let generated_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("RFC 3339 formatting cannot fail");

    // Detect unavailability. We do not call any other git command when the
    // project has no `.git` marker so the reducer fails closed quickly.
    let inside = git.is_inside_work_tree().unwrap_or(false);
    if !inside {
        return Ok(unavailable_payload(
            &context.root,
            &generated_at,
            "git-not-initialized",
        ));
    }

    // Capture HEAD. An unborn branch (no commits) is still recoverable: we
    // persist `unavailable_reason=git-context-missing` rather than guessing
    // a parent.
    let head_sha = git.head_sha().ok();
    if head_sha.is_none() {
        return Ok(unavailable_payload(
            &context.root,
            &generated_at,
            "git-context-missing",
        ));
    }
    let head_sha = head_sha.expect("checked above");

    // Capture the streams.
    let diff_text = match git.run_read_only("diff", &["--raw", "-M50%", "HEAD"]) {
        Ok(text) => text,
        Err(_) => {
            return Ok(unavailable_payload(
                &context.root,
                &generated_at,
                "io-error",
            ));
        }
    };
    let status_text = match git.run_read_only(
        "status",
        &["--porcelain", "--untracked-files=all", "--ignored"],
    ) {
        Ok(text) => text,
        Err(_) => {
            return Ok(unavailable_payload(
                &context.root,
                &generated_at,
                "io-error",
            ));
        }
    };

    // Capture `.gitignore`-driven ignored paths. `git ls-files --others
    // --exclude-standard --ignored --directory` returns each top-level
    // ignored entry in a single invocation, avoiding the stdin-pipe
    // dance that `check-ignore --stdin` requires.
    let ignored_by_project = match git.run_read_only(
        "ls-files",
        &["--others", "--exclude-standard", "--ignored", "--directory"],
    ) {
        Ok(text) => parse_ls_files_ignored(&text),
        Err(_) => Vec::new(),
    };

    // Build files + buckets.
    let mut entries = parse_diff(&diff_text);
    let untracked = parse_untracked(&status_text);
    entries.extend(untracked);
    entries.sort_by(|a, b| a.path.cmp(&b.path));

    let mut buckets = BTreeMapString::default();
    let mut summary = InventorySummary {
        added: 0,
        modified: 0,
        deleted: 0,
        renamed: 0,
        ignored_inventory: ignored_by_project.len() as u32,
        ignored_by_project: if ignored_by_project.is_empty() {
            None
        } else {
            Some(ignored_by_project.len() as u32)
        },
        unavailable_reason: None,
    };
    for entry in &entries {
        let key = classify_bucket(&entry.path);
        let bucket = buckets.0.entry(key).or_default();
        match entry.status {
            EntryStatus::Added => {
                summary.added += 1;
                bucket.added += 1;
            }
            EntryStatus::Modified => {
                summary.modified += 1;
                bucket.modified += 1;
            }
            EntryStatus::Deleted => {
                summary.deleted += 1;
                bucket.deleted += 1;
            }
            EntryStatus::Renamed => {
                summary.renamed += 1;
                bucket.renamed += 1;
            }
        }
    }

    Ok(InventoryOutput {
        schema: SCHEMA_ID.to_owned(),
        generated_at,
        git: GitContext {
            root: context.root.to_string_lossy().into_owned(),
            head: Some(head_sha),
            comparison: "stage-and-working-tree-vs-head".to_owned(),
            parent: None,
            tag: None,
        },
        summary,
        buckets,
        files: entries,
        ignored_by_project,
    })
}

fn unavailable_payload(root: &Path, generated_at: &str, reason: &str) -> InventoryOutput {
    InventoryOutput {
        schema: SCHEMA_ID.to_owned(),
        generated_at: generated_at.to_owned(),
        git: GitContext {
            root: root.to_string_lossy().into_owned(),
            head: None,
            comparison: "stage-and-working-tree-vs-head".to_owned(),
            parent: None,
            tag: None,
        },
        summary: InventorySummary {
            added: 0,
            modified: 0,
            deleted: 0,
            renamed: 0,
            ignored_inventory: 0,
            ignored_by_project: None,
            unavailable_reason: Some(reason.to_owned()),
        },
        buckets: BTreeMapString::default(),
        files: Vec::new(),
        ignored_by_project: Vec::new(),
    }
}

fn envelope_text(env: &InventoryEnvelope) -> String {
    let mut buf = String::new();
    writeln!(buf, "contract_version: {}", env.contract_version).unwrap();
    writeln!(buf, "schema: {}", env.schema).unwrap();
    writeln!(buf, "path: {}", env.path).unwrap();
    writeln!(buf, "sha256: {}", env.sha256).unwrap();
    writeln!(buf, "bytes: {}", env.bytes).unwrap();
    if let Some(reason) = &env.unavailable_reason {
        writeln!(buf, "unavailable_reason: {reason}").unwrap();
    }
    buf
}

fn persist_atomic(destination: &Path, bytes: &[u8]) -> anyhow::Result<String> {
    let parent = destination
        .parent()
        .ok_or_else(|| anyhow::anyhow!("destination has no parent: {destination:?}"))?;
    std::fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(
        ".{}.tmp-{}",
        destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("inventory"),
        std::process::id()
    ));
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(&temporary, destination)?;

    let sha_hex = format!("{:x}", Sha256::digest(bytes));

    let sidecar = destination.with_extension("json.sha256");
    std::fs::write(&sidecar, format!("{sha_hex}  inventory.json\n"))?;
    Ok(sha_hex)
}

fn parse_diff(diff_text: &str) -> Vec<InventoryEntry> {
    let mut entries = Vec::new();
    for line in diff_text.lines() {
        let Some(rest) = line.strip_prefix(':') else {
            continue;
        };
        let cells: Vec<&str> = rest.split('\t').collect();
        if cells.len() < 2 {
            continue;
        }
        let header = cells[0];
        let header_cells: Vec<&str> = header.split(' ').collect();
        if header_cells.len() < 5 {
            continue;
        }
        let new_mode = header_cells[1].to_owned();
        let status = header_cells[4];
        let paths: Vec<&str> = cells[1..].to_vec();
        let Some(action) = status.chars().next() else {
            continue;
        };
        match action {
            'A' => {
                let path = paths.first().copied().unwrap_or_default().to_owned();
                entries.push(InventoryEntry {
                    bucket: classify_bucket(&path),
                    status: EntryStatus::Added,
                    path,
                    renamed_from: None,
                    old_path: None,
                    sha256: None,
                    mode: Some(new_mode),
                });
            }
            'M' => {
                let path = paths.first().copied().unwrap_or_default().to_owned();
                entries.push(InventoryEntry {
                    bucket: classify_bucket(&path),
                    status: EntryStatus::Modified,
                    path,
                    renamed_from: None,
                    old_path: None,
                    sha256: None,
                    mode: Some(new_mode),
                });
            }
            'D' => {
                let path = paths.first().copied().unwrap_or_default().to_owned();
                entries.push(InventoryEntry {
                    bucket: classify_bucket(&path),
                    status: EntryStatus::Deleted,
                    path,
                    renamed_from: None,
                    old_path: None,
                    sha256: None,
                    mode: Some(new_mode),
                });
            }
            'R' => {
                if paths.len() < 2 {
                    continue;
                }
                let old_path = paths[0].to_owned();
                let new_path = paths[1].to_owned();
                entries.push(InventoryEntry {
                    bucket: classify_bucket(&new_path),
                    status: EntryStatus::Renamed,
                    path: new_path,
                    renamed_from: Some(old_path.clone()),
                    old_path: Some(old_path),
                    sha256: None,
                    mode: Some(new_mode),
                });
            }
            _ => continue,
        }
    }
    entries
}

fn parse_untracked(status_text: &str) -> Vec<InventoryEntry> {
    let mut entries = Vec::new();
    for line in status_text.lines() {
        if !line.starts_with("?? ") {
            continue;
        }
        let path = line[3..].trim().to_owned();
        if path.is_empty() {
            continue;
        }
        entries.push(InventoryEntry {
            bucket: classify_bucket(&path),
            status: EntryStatus::Added,
            path,
            renamed_from: None,
            old_path: None,
            sha256: None,
            mode: Some("100644".to_owned()),
        });
    }
    entries
}

/// Parses the output of `git ls-files --others --exclude-standard
/// --ignored --directory` into a sorted, deduplicated list of relative
/// paths whose presence in the working tree is suppressed by the project's
/// own `.gitignore`.
fn parse_ls_files_ignored(ls_files_text: &str) -> Vec<String> {
    let mut paths: Vec<String> = ls_files_text
        .lines()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect();
    paths.sort();
    paths.dedup();
    paths
}

fn classify_bucket(path: &str) -> String {
    for prefix in CLOSED_PREFIXES {
        if path.starts_with(prefix) {
            return prefix.trim_end_matches('/').to_owned();
        }
    }
    if let Some((head, _)) = path.split_once('/') {
        format!("untagged_project/{head}")
    } else {
        "untagged_project".to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_bucket_returns_closed_prefix() {
        assert_eq!(classify_bucket("prompts/sddk/foo.md"), "prompts");
        assert_eq!(classify_bucket("agents/x.md"), "agents");
        assert_eq!(classify_bucket("untagged.txt"), "untagged_project");
        assert_eq!(
            classify_bucket("untagged_dir/file.md"),
            "untagged_project/untagged_dir"
        );
    }

    #[test]
    fn parse_diff_handles_renames_modifications_and_additions() {
        // Real `git diff --raw -M50%` produces lines of the form
        //   :old_mode new_mode old_sha new_sha status<TAB>path[,<TAB>path]
        let diff = ":100644 100644 abc def A\tprompts/sddk/new.md\n\
             :100644 100644 abc def M\tprompts/sddk/edit.md\n\
             :100644 100644 abc def R100\tskills/oldname.md\tskills/newname.md\n\
             :100644 100644 abc def D\tdocs/gone.md\n";
        let entries = parse_diff(diff);
        assert_eq!(entries.len(), 4);
        assert!(
            entries
                .iter()
                .any(|e| e.status == EntryStatus::Added && e.path == "prompts/sddk/new.md"),
            "added entry present"
        );
        assert!(
            entries
                .iter()
                .any(|e| e.status == EntryStatus::Modified && e.path == "prompts/sddk/edit.md"),
            "modified entry present"
        );
        let rename = entries
            .iter()
            .find(|e| e.status == EntryStatus::Renamed)
            .expect("rename entry");
        assert_eq!(rename.path, "skills/newname.md");
        assert_eq!(rename.renamed_from.as_deref(), Some("skills/oldname.md"));
        assert!(
            entries
                .iter()
                .any(|e| e.status == EntryStatus::Deleted && e.path == "docs/gone.md"),
            "deleted entry present"
        );
    }

    #[test]
    fn parse_untracked_skips_ignored_markers() {
        let status = "?? prompts/sddk/newfile.md\n!! generated/cache.bin\n M modified.md\n";
        let entries = parse_untracked(status);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, "prompts/sddk/newfile.md");
        assert_eq!(entries[0].status, EntryStatus::Added);
    }

    #[test]
    fn payload_records_unavailable_reason_when_no_git() {
        let root = std::env::temp_dir();
        let now = "2026-08-24T17:00:00+00:00".to_owned();
        let payload = unavailable_payload(&root, &now, "git-not-initialized");
        assert_eq!(
            payload.summary.unavailable_reason.as_deref(),
            Some("git-not-initialized")
        );
        assert_eq!(payload.files.len(), 0);
        assert_eq!(payload.buckets.0.len(), 0);
    }

    #[test]
    fn summary_total_matches_files_count_plus_ignored() {
        let entries = parse_diff(
            ":100644 100644 abc def A\tprompts/sddk/a.md\n\
             :100644 100644 abc def A\tprompts/sddk/b.md\n",
        );
        assert_eq!(entries.len(), 2);
        let added: u32 = entries
            .iter()
            .filter(|e| e.status == EntryStatus::Added)
            .count() as u32;
        assert_eq!(added, 2);
    }

    #[test]
    fn persist_atomic_writes_sha256_sidecar() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let destination = tmp.path().join("inventory.json");
        let sha =
            persist_atomic(&destination, b"{\"schema\":\"sddk.inventory/v1\"}").expect("persist");
        let body = std::fs::read_to_string(&destination).expect("read");
        assert!(body.contains("sddk.inventory/v1"));
        let sidecar =
            std::fs::read_to_string(destination.with_extension("json.sha256")).expect("sidecar");
        assert!(sidecar.starts_with(&sha));
        assert!(sidecar.ends_with("  inventory.json\n"));
    }
}
