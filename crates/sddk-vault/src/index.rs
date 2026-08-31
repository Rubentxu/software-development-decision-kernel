//! Markdown node types and the parsed vault index.

use std::collections::HashMap;

use serde::Serialize;

/// Kind of a vault node, derived from frontmatter `type` or the parent folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// Milestone node.
    Milestone,
    /// Architecture decision record.
    Adr,
    /// Requirement or specification node.
    Spec,
    /// Cycle record.
    Cycle,
    /// Incidence record.
    Incidence,
    /// Term definition.
    Term,
    /// Map of content (index) node.
    Moc,
    /// Anything else.
    Other,
}

impl NodeKind {
    /// Maps a frontmatter `type` value to a kind.
    pub fn from_type(value: &str) -> Self {
        match value {
            "milestone" => NodeKind::Milestone,
            "adr" | "decision" => NodeKind::Adr,
            "requirement" | "spec" | "specification" => NodeKind::Spec,
            "cycle" => NodeKind::Cycle,
            "incidence" | "incident" => NodeKind::Incidence,
            "term" | "glossary" => NodeKind::Term,
            "moc" | "map_of_content" => NodeKind::Moc,
            _ => NodeKind::Other,
        }
    }

    /// Maps a folder name to a kind.
    pub fn from_folder(folder: &str) -> Self {
        match folder {
            "milestones" => NodeKind::Milestone,
            "adrs" => NodeKind::Adr,
            "specs" => NodeKind::Spec,
            "cycles" => NodeKind::Cycle,
            "incidences" => NodeKind::Incidence,
            "terms" => NodeKind::Term,
            _ => NodeKind::Other,
        }
    }
}

/// One parsed vault node.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct VaultNode {
    /// Node identifier from frontmatter `id` or the file stem.
    pub id: String,
    /// Node kind.
    pub kind: NodeKind,
    /// Repository-relative Markdown path with forward slashes.
    pub path: String,
    /// Display title.
    pub title: String,
    /// Frontmatter status, when declared.
    pub status: Option<String>,
    /// Frontmatter tags.
    pub tags: Vec<String>,
    /// Markdown body without frontmatter.
    pub body: String,
    /// Wikilink targets in declaration order.
    pub wikilinks: Vec<String>,
}

/// Parsed vault with resolved backlinks.
#[derive(Debug, Clone, PartialEq)]
pub struct VaultIndex {
    /// Nodes in discovery order.
    pub nodes: Vec<VaultNode>,
    /// Node id to node index.
    pub by_id: HashMap<String, usize>,
    /// Target id to list of source node ids.
    pub backlinks: HashMap<String, Vec<String>>,
}

impl VaultIndex {
    /// Looks up a node by identifier.
    pub fn get(&self, id: &str) -> Option<&VaultNode> {
        self.by_id.get(id).map(|index| &self.nodes[*index])
    }

    /// Returns the ids that link to `id`.
    pub fn backlinks_of(&self, id: &str) -> Vec<&str> {
        self.backlinks
            .get(id)
            .map(|sources| sources.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }
}
