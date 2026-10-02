//! Self-contained HTML inspector for a vault.

use std::fmt::Write as _;

use serde::Serialize;
use thiserror::Error;

use crate::graph::GraphView;
use crate::index::{NodeKind, VaultIndex, VaultNode};

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
    // The page states its own scope before it shows any data. A projection that
    // does not say what it leaves out cannot answer "is this everything?", and
    // that question is the only reason a reader would want the omitted list.
    if let Some(note) = projection_note(index) {
        writeln!(html, "{note}").unwrap();
    }
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

/// Fields of [`VaultNode`] that the exported page deliberately does not carry.
///
/// This is the *only* list that decides the projection, and it is rendered into
/// the page, so a consumer can answer "is this everything?" without reading the
/// code. A projection that omits silently is the defect; a projection that says
/// what it omits is a decision.
const OMITTED_NODE_FIELDS: &[&str] = &[
    "body", // The document itself. Inlining every body makes a self-contained
           // page grow by the size of the vault, and a node inspector wants
           // the metadata, not the prose. Stated here rather than left as a
           // silent omission.
];

/// The node as the exported page sees it.
///
/// It used to be a hand-written `serde_json::json!` over a type that already
/// derives `Serialize`, and that is how `tags` dropped out while `status` stayed,
/// with no reason anyone could derive and nothing that said so.
///
/// **What deriving it from `VaultNode` buys, stated exactly, because the first
/// version of this comment claimed more and was falsified:** reading the fields
/// off the node rather than rebuilding a `Value` by hand removes the class where
/// a projection and its source **disagree about a value** — a field read from the
/// wrong place, an enum flattened to a string built locally.
///
/// **What it does not buy is compile-time enforcement, and the claim that it did
/// was measured false.** Adding a field to `VaultNode` does not break this build:
/// `From<&VaultNode> for NodeProjection` maps between two *different* types, so
/// neither side is exhaustive as far as the compiler is concerned. Verified by
/// mutation — `mutant_field: String` added to `VaultNode`, the parser updated to
/// satisfy it, and `cargo build` **succeeded**. The same is true of
/// `GraphExport::from(&GraphView)` above, whose comment claimed the opposite.
///
/// So the coupling is held by [`OMITTED_NODE_FIELDS`] plus the test that pins the
/// two field sets against each other, not by the type system. A note claiming
/// otherwise would be the same defect wearing the hat of a guarantee.
#[derive(Serialize)]
struct NodeProjection<'a> {
    id: &'a str,
    kind: NodeKind,
    path: &'a str,
    title: &'a str,
    status: Option<&'a str>,
    tags: &'a [String],
    wikilinks: &'a [String],
}

impl<'a> From<&'a VaultNode> for NodeProjection<'a> {
    fn from(node: &'a VaultNode) -> Self {
        Self {
            id: &node.id,
            kind: node.kind,
            path: &node.path,
            title: &node.title,
            status: node.status.as_deref(),
            // Travels with `status`, not without it: both are frontmatter
            // metadata read off the same block, and the table already has a
            // column for one of them. Two rules for one block of YAML is how
            // the asymmetry happened the first time.
            tags: &node.tags,
            wikilinks: &node.wikilinks,
        }
    }
}

fn export_node(node: &VaultNode) -> serde_json::Value {
    serde_json::to_value(NodeProjection::from(node)).expect("a projection of owned data")
}

/// The sentence the page carries about its own scope.
///
/// Both counts are **derived from serialisation**, not written down: the number
/// the page claims to carry is the number its own projection produces, and the
/// number it compares against is the number `VaultNode` produces. A constant
/// here would be the same defect one level down — a hand-written list of what
/// the type has — wearing a different hat.
fn projection_note(index: &VaultIndex) -> Option<String> {
    let first = index.nodes.first()?;
    let carried = serde_json::to_value(NodeProjection::from(first))
        .ok()?
        .as_object()
        .map(|o| o.len())?;
    let declared = serde_json::to_value(first)
        .ok()?
        .as_object()
        .map(|o| o.len())?;
    let omitted = OMITTED_NODE_FIELDS.join(", ");
    Some(format!(
        "<p><em>Scope:</em> each node below carries {carried} of {declared} fields of \
         <code>VaultNode</code>. Not carried: <code>{omitted}</code>. The page is a \
         projection, not a copy of the vault.</p>"
    ))
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
    /// Field-by-field, so the values come from the view rather than being
    /// rebuilt beside it.
    ///
    /// **This does not make the field list exhaustive at compile time, and an
    /// earlier version of this comment said it did.** The claim was falsified by
    /// mutation: adding a field to `GraphView` and to the struct's source does
    /// not break the build, because a `From` between two *different* types is
    /// exhaustive on neither side as far as the compiler is concerned. What
    /// actually holds the coupling here is `r3_embedded_graph_matches_vault_graph_
    /// field_by_field`, which pins the two serialisations against each other.
    /// A comment promising a guarantee the type system does not give is worse
    /// than no comment, because the next reader trusts it.
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
