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
            topological_order,
        });
    }

    let sample_cycle = find_sample_cycle(&graph);
    Ok(GraphView {
        node_count,
        edge_count,
        cyclic: true,
        sample_cycle,
        topological_order: None,
    })
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
