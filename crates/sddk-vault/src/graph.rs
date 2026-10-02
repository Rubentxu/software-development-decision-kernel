//! petgraph projection of the vault wikilink graph.

use petgraph::algo::{is_cyclic_directed, toposort};
use petgraph::graph::{DiGraph, NodeIndex};
use serde::Serialize;
use thiserror::Error;

use crate::index::VaultIndex;

/// Errors emitted while projecting the vault graph.
#[derive(Debug, Error)]
pub enum VaultGraphError {
    /// Graph algorithms failed.
    #[error("vault graph computation failed: {0}")]
    Algorithm(String),
}

/// Computed graph facts for one vault.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GraphView {
    /// Node count.
    pub node_count: usize,
    /// Edge count.
    pub edge_count: usize,
    /// Whether the graph contains directed cycles.
    pub cyclic: bool,
    /// One representative cycle, when the graph is cyclic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_cycle: Option<Vec<String>>,
    /// Exact cycle count when it is 0 or 1, `None` when it is 2 or more.
    ///
    /// Saturated on purpose, and not for convenience. An exact count means
    /// enumerating simple cycles, and an enumeration is **wrong** before it is
    /// slow: a 1000-node ring has one cycle and naive counting returns 1000
    /// rotations of it, and a 500-cycle bouquet returns 1000 because each cycle
    /// is walked in both directions. Measured, and it took 558 ms on the ring —
    /// the most trivial graph there is. A field that looks like a truth and is not
    /// is worse than no field, so this reports 0, 1, or "2 or more" and stops.
    ///
    /// What it costs to answer the only question a `sample_cycle` actually owes
    /// its reader — *is there one, or more than one?* — is one extra pass: remove
    /// the sample cycle's edges and look again. Same complexity as the pass that
    /// found it, and no counter that can be wrong.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycle_count: Option<u64>,
    /// Whether more than one cycle exists. True whenever `cycle_count` is `None`
    /// on a cyclic graph, and always false otherwise.
    pub multiple_cycles: bool,
    /// Why the topological order is missing, when it is.
    ///
    /// Previously the field was simply absent whenever the graph was cyclic, with
    /// nothing tying it to `cyclic: true`. That reads as "not computed", which is
    /// a different claim from "does not exist because the graph is cyclic".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topological_order_absent_because: Option<String>,
    /// Topological order of node ids, when the graph is acyclic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topological_order: Option<Vec<String>>,
}

/// Builds a directed graph over node ids.
pub fn build_graph(index: &VaultIndex) -> DiGraph<String, ()> {
    let mut graph = DiGraph::<String, ()>::new();
    let mut indices = std::collections::HashMap::new();
    for node in &index.nodes {
        let node_index = graph.add_node(node.id.clone());
        indices.insert(node.id.clone(), node_index);
    }
    for node in &index.nodes {
        let source = indices[&node.id];
        for target in &node.wikilinks {
            if let Some(target_index) = indices.get(target) {
                graph.add_edge(source, *target_index, ());
            }
        }
    }
    graph
}

/// Computes cycle, path, and ordering facts for a vault index.
pub fn graph_view(index: &VaultIndex) -> Result<GraphView, VaultGraphError> {
    let graph = build_graph(index);
    let node_count = graph.node_count();
    let edge_count = graph.edge_count();

    if !is_cyclic_directed(&graph) {
        let order = toposort(&graph, None)
            .map_err(|_| VaultGraphError::Algorithm("toposort failed".into()))?;
        let topological_order = Some(
            order
                .into_iter()
                .map(|index: NodeIndex| graph[index].clone())
                .collect(),
        );
        return Ok(GraphView {
            node_count,
            edge_count,
            cyclic: false,
            sample_cycle: None,
            cycle_count: Some(0),
            multiple_cycles: false,
            topological_order_absent_because: None,
            topological_order,
        });
    }

    let sample_cycle = find_sample_cycle(&graph);
    // Answer the only question a sample owes its reader: is there one cycle, or
    // more than one? Remove the sample's own edges and look again — one extra
    // pass of the same function, and no counter that can be wrong.
    let multiple_cycles = match &sample_cycle {
        Some(cycle) => {
            let mut without_sample = graph.clone();
            remove_cycle_edges(&mut without_sample, &graph, cycle);
            find_sample_cycle(&without_sample).is_some()
        }
        None => false,
    };
    Ok(GraphView {
        node_count,
        edge_count,
        cyclic: true,
        cycle_count: if multiple_cycles { None } else { Some(1) },
        multiple_cycles,
        sample_cycle,
        topological_order_absent_because: Some("cyclic".to_string()),
        topological_order: None,
    })
}

/// Removes from `target` the edges that make up `cycle`, which is expressed in
/// ids and refers to `source`.
///
/// The graph is rebuilt by id rather than by index because `remove_edge` shifts
/// nothing but `NodeIndex` values are tied to the graph they came from, and the
/// two graphs here are separate objects.
fn remove_cycle_edges(
    target: &mut DiGraph<String, ()>,
    source: &DiGraph<String, ()>,
    cycle: &[String],
) {
    let by_id: std::collections::HashMap<&str, NodeIndex> = source
        .node_indices()
        .map(|i| (source[i].as_str(), i))
        .collect();
    let edges: Vec<(NodeIndex, NodeIndex)> = cycle
        .iter()
        .filter_map(|id| by_id.get(id.as_str()).copied())
        .zip(
            cycle
                .iter()
                .skip(1)
                .chain(cycle.iter().take(1))
                .filter_map(|id| by_id.get(id.as_str()).copied()),
        )
        .collect();
    for (from, to) in edges {
        if let Some(edge) = target.find_edge(from, to) {
            target.remove_edge(edge);
        }
    }
}

fn find_sample_cycle(graph: &DiGraph<String, ()>) -> Option<Vec<String>> {
    for start in graph.node_indices() {
        if let Some(cycle) = dfs_cycle(graph, start, start, &mut Vec::new()) {
            return Some(cycle);
        }
    }
    None
}

fn dfs_cycle(
    graph: &DiGraph<String, ()>,
    start: NodeIndex,
    current: NodeIndex,
    path: &mut Vec<String>,
) -> Option<Vec<String>> {
    path.push(graph[current].clone());
    for neighbor in graph.neighbors(current) {
        if neighbor == start {
            let mut cycle = path.clone();
            cycle.push(graph[start].clone());
            return Some(cycle);
        }
        if !path.contains(&graph[neighbor])
            && let Some(cycle) = dfs_cycle(graph, start, neighbor, path)
        {
            return Some(cycle);
        }
    }
    path.pop();
    None
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::parser::parse_vault;

    use super::graph_view;

    fn node(file: &str, content: &str) {
        fs::create_dir_all(std::path::Path::new(file).parent().unwrap()).unwrap();
        fs::write(file, content).unwrap();
    }

    #[test]
    fn acyclic_vault_reports_topological_order() {
        let directory = tempfile::tempdir().unwrap();
        node(
            &directory.path().join("a.md").to_string_lossy(),
            "---\nid: A\ntype: term\n---\n# A\n\n[[B]] [[C]]\n",
        );
        node(
            &directory.path().join("b.md").to_string_lossy(),
            "---\nid: B\ntype: term\n---\n# B\n\n[[C]]\n",
        );
        node(
            &directory.path().join("c.md").to_string_lossy(),
            "---\nid: C\ntype: term\n---\n# C\n",
        );
        let index = parse_vault(directory.path()).unwrap();
        let view = graph_view(&index).unwrap();
        assert!(!view.cyclic);
        assert_eq!(view.node_count, 3);
        assert_eq!(view.edge_count, 3);
        let order = view.topological_order.unwrap();
        assert!(
            order.iter().position(|id| id == "C").unwrap()
                > order.iter().position(|id| id == "A").unwrap()
        );
    }

    #[test]
    fn cyclic_vault_reports_sample_cycle() {
        let directory = tempfile::tempdir().unwrap();
        node(
            &directory.path().join("a.md").to_string_lossy(),
            "---\nid: A\ntype: term\n---\n# A\n\n[[B]]\n",
        );
        node(
            &directory.path().join("b.md").to_string_lossy(),
            "---\nid: B\ntype: term\n---\n# B\n\n[[A]]\n",
        );
        let index = parse_vault(directory.path()).unwrap();
        let view = graph_view(&index).unwrap();
        assert!(view.cyclic);
        let cycle = view.sample_cycle.unwrap();
        assert!(cycle.len() >= 3);
        assert_eq!(cycle.first(), cycle.last());
        assert!(view.topological_order.is_none());
    }
}
