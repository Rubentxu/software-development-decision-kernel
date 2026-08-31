//! Knowledge vault parsing, validation, graph, and search.
//!
//! The vault is a directory of Markdown nodes with YAML frontmatter and
//! `[[wikilinks]]`. The crate parses nodes, validates relations, builds a
//! `petgraph` projection, maintains a rebuildable SQLite FTS5 index, and
//! exports a self-contained HTML inspector.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(missing_docs)]

mod export;
mod graph;
mod index;
mod parser;
mod repair;
mod search;
mod validate;

pub use export::{HtmlExportError, export_html};
pub use graph::{GraphView, VaultGraphError, build_graph, graph_view};
pub use index::{NodeKind, VaultIndex, VaultNode};
pub use parser::{VaultError, parse_vault};
pub use repair::{
    ALLOW_LIST, RepairAction, RepairQueueError, RepairReceipt, RepairReceiptError,
    append_repair_receipt, load_repair_queue, verify_receipt_evidence,
};
pub use search::{
    SearchHit, SearchIndexError, SyncSummary, index_has_rows, open_index, rebuild_search_index,
    search_index, sync_search_index,
};
pub use validate::{
    CycleScope, Diagnostic, Severity, VaultDiagnosticError, summary, validate_index,
};

/// Indexes a vault directory and validates every relation.
pub fn index_vault(
    directory: &std::path::Path,
) -> Result<(VaultIndex, Vec<Diagnostic>), VaultError> {
    let index = parse_vault(directory)?;
    let diagnostics = validate_index(&index);
    Ok((index, diagnostics))
}
