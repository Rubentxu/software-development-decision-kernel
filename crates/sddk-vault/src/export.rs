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
    let graph_json = serde_json::to_string(&GraphExport {
        cyclic: graph.cyclic,
        sample_cycle: graph.sample_cycle.clone(),
        topological_order: graph.topological_order.clone(),
    })?;

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

#[derive(Serialize)]
struct GraphExport {
    cyclic: bool,
    sample_cycle: Option<Vec<String>>,
    topological_order: Option<Vec<String>>,
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
