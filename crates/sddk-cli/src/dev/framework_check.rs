//! Framework link report types and asset sync helpers.
//! Agent registration moved to `editor_adapters/` (ADR-0019).

use crate::dev::common::walk_dir;
use std::path::Path;

// ── Link report types (shared with link.rs) ────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct LinkReport {
    pub editor: String,
    pub agents_linked: usize,
    pub skills_linked: usize,
    pub prompts_linked: usize,
    pub workflows_linked: usize,
    pub stale_replaced: usize,
    pub pruned: usize,
    pub agents_registered: usize,
    pub agents_updated_stale: usize,
    pub agents_skipped_existing: usize,
    pub agents_skipped_unresolved: usize,
    pub errors: Vec<String>,
}

pub(super) fn link_report_text(report: &LinkReport) -> String {
    format!(
        "editor: {}\nagents: {}\nskills: {}\nprompts: {}\nworkflows: {}\nstale_replaced: {}\npruned: {}\nregistered: {}\nupdated_stale: {}\nskipped_existing: {}\nskipped_unresolved: {}\nerrors: {}\n",
        report.editor,
        report.agents_linked,
        report.skills_linked,
        report.prompts_linked,
        report.workflows_linked,
        report.stale_replaced,
        report.pruned,
        report.agents_registered,
        report.agents_updated_stale,
        report.agents_skipped_existing,
        report.agents_skipped_unresolved,
        report.errors.len()
    )
}

// ── Asset sync ────────────────────────────────────────────────────────────────

/// Sync the framework assets tree from source into target (idempotent).
pub(super) fn sync_assets(source: &Path, target: &Path) -> anyhow::Result<usize> {
    let mut copied = 0usize;
    std::fs::create_dir_all(target)?;
    for entry in walk_dir(source) {
        let relative = entry
            .strip_prefix(source)
            .unwrap_or(entry.as_path())
            .to_path_buf();
        let destination = target.join(&relative);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let needs_copy = match (std::fs::read(&entry), std::fs::read(&destination)) {
            (Ok(src), Ok(dst)) => src != dst,
            _ => true,
        };
        if needs_copy {
            std::fs::copy(&entry, &destination)?;
        }
        copied += 1;
    }
    Ok(copied)
}
