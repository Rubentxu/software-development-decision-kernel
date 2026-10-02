//! Self-contained HTML inspector for a vault.

use std::fmt::Write as _;

use serde::Serialize;
use thiserror::Error;

use crate::graph::GraphView;
use crate::index::{VaultIndex, VaultNode};

/// Errors emitted while exporting the HTML inspector.
#[derive(Debug, Error)]
pub enum HtmlExportError {
    /// Structured data could not be encoded into the page.
    #[error("html export serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Renders a single self-contained HTML file with nodes, links, and backlinks.
pub fn export_html(index: &VaultIndex, graph: &GraphView) -> Result<String, HtmlExportError> {
    let nodes_json =
        serde_json::to_string(&index.nodes.iter().map(export_node).collect::<Vec<_>>())?;
    let graph_json = serde_json::to_string(&GraphExport::from(graph))?;

    let mut html = String::new();
    writeln!(html, "<!DOCTYPE html>").unwrap();
    writeln!(html, "<html lang=\"en\"><head><meta charset=\"utf-8\">").unwrap();
    writeln!(
        html,
        "<title>SDDK Vault Inspector</title><style>body{{font-family:system-ui,sans-serif;margin:2rem}}table{{border-collapse:collapse;width:100%}}td,th{{border:1px solid #ccc;padding:.35rem;text-align:left}}code{{background:#f4f4f4;padding:0 .2rem}}</style>"
    )
    .unwrap();
    writeln!(html, "</head><body>").unwrap();
    writeln!(html, "<h1>SDDK Vault Inspector</h1>").unwrap();
    writeln!(
        html,
        "<p>{} nodes, {} links, cyclic: {}</p>",
        graph.node_count, graph.edge_count, graph.cyclic
    )
    .unwrap();
    writeln!(html, "<h2>Nodes</h2><table><thead><tr><th>Id</th><th>Kind</th><th>Title</th><th>Status</th><th>Links</th><th>Backlinks</th></tr></thead><tbody>")
        .unwrap();
    for node in &index.nodes {
        writeln!(
            html,
            "<tr><td><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            escape(&node.id),
            serde_json::to_string(&node.kind).unwrap_or_default(),
            escape(&node.title),
            escape(node.status.as_deref().unwrap_or("")),
            node.wikilinks
                .iter()
                .map(|link| format!("<code>{}</code>", escape(link)))
                .collect::<Vec<_>>()
                .join(" "),
            index
                .backlinks_of(&node.id)
                .iter()
                .map(|source| format!("<code>{}</code>", escape(source)))
                .collect::<Vec<_>>()
                .join(" "),
        )
        .unwrap();
    }
    writeln!(html, "</tbody></table>").unwrap();
    writeln!(
        html,
        "<script>window.__vault_nodes__={nodes_json};window.__vault_graph__={graph_json};</script>"
    )
    .unwrap();
    writeln!(html, "</body></html>").unwrap();
    Ok(html)
}

fn export_node(node: &VaultNode) -> serde_json::Value {
    serde_json::json!({
        "id": node.id,
        "kind": serde_json::to_value(node.kind).unwrap_or_default(),
        "title": node.title,
        "path": node.path,
        "status": node.status,
        "wikilinks": node.wikilinks,
    })
}

/// The graph as the exported page sees it.
///
/// This is a **second declaration of the same fact** that `vault graph` declares
/// in its own JSON, not a rendering of the first. It was a hand-written struct
/// with three fields, which is how the two drifted apart: with a vault of two
/// disjoint cycles the page reported a `sample_cycle` and never said how many
/// cycles there were, and reported a missing `topological_order` without saying
/// why — the same omission `vault graph` had just stopped making, in a surface
/// nobody was looking at.
///
/// The `skip_serializing_if` attributes are **load-bearing, not cosmetic**. They
/// mirror `GraphView`'s, because the two surfaces have to agree on which keys are
/// *present* and not only on their values: without them this struct emits
/// `"topological_order": null` where `GraphView` omits the key entirely, and a
/// consumer comparing the two documents sees a difference where there is none
/// in meaning.
#[derive(Serialize)]
struct GraphExport {
    cyclic: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    sample_cycle: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cycle_count: Option<u64>,
    multiple_cycles: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    topological_order_absent_because: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    topological_order: Option<Vec<String>>,
}

impl From<&GraphView> for GraphExport {
    /// Field-by-field, so adding a field to `GraphView` without deciding what the
    /// page should say about it is a compile error rather than a silent
    /// divergence. That inversion of control is the actual fix: the previous
    /// mapping was a struct literal at the call site, and a call site is not
    /// where a compiler looks.
    fn from(graph: &GraphView) -> Self {
        Self {
            cyclic: graph.cyclic,
            sample_cycle: graph.sample_cycle.clone(),
            cycle_count: graph.cycle_count,
            multiple_cycles: graph.multiple_cycles,
            topological_order_absent_because: graph.topological_order_absent_because.clone(),
            topological_order: graph.topological_order.clone(),
        }
    }
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::{graph_view, parse_vault};

    use super::export_html;

    fn node(file: &str, content: &str) {
        fs::create_dir_all(std::path::Path::new(file).parent().unwrap()).unwrap();
        fs::write(file, content).unwrap();
    }

    #[test]
    fn renders_self_contained_inspector() {
        let directory = tempfile::tempdir().unwrap();
        node(
            &directory.path().join("terms/TERM-A.md").to_string_lossy(),
            "---\nid: TERM-A\ntype: term\n---\n# A\n\n[[TERM-B]]\n",
        );
        node(
            &directory.path().join("terms/TERM-B.md").to_string_lossy(),
            "---\nid: TERM-B\ntype: term\n---\n# B\n",
        );
        let index = parse_vault(directory.path()).unwrap();
        let graph = graph_view(&index).unwrap();
        let html = export_html(&index, &graph).unwrap();
        assert!(html.contains("SDDK Vault Inspector"));
        assert!(html.contains("TERM-A"));
        assert!(html.contains("TERM-B"));
        assert!(html.contains("__vault_nodes__"));
        assert!(html.contains("</html>"));
        assert!(!html.contains("</script></script>"));
    }
}
