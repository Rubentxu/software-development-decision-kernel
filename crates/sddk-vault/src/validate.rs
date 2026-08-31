//! Stable relation diagnostics for a parsed vault.

use serde::Serialize;
use thiserror::Error;

use crate::index::VaultIndex;

/// Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Breaks the vault contract.
    Error,
    /// Non-fatal inconsistency.
    Warning,
}

/// One stable, structured vault diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    /// Stable machine-readable code.
    pub code: String,
    /// Severity.
    pub severity: Severity,
    /// Offending node id, when node-scoped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    /// Human-readable problem.
    pub message: String,
    /// Suggested remediation.
    pub hint: String,
}

/// Errors emitted while validating an index.
#[derive(Debug, Error)]
pub enum VaultDiagnosticError {
    /// Diagnostics could not be encoded.
    #[error("vault diagnostics serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

const MISSING_ID: &str = "VAULT001";
const DUPLICATE_ID: &str = "VAULT002";
const BROKEN_LINK: &str = "VAULT003";
const EMPTY_TITLE: &str = "VAULT004";

/// Validates node ids, titles, and wikilink targets deterministically.
pub fn validate_index(index: &VaultIndex) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen_ids = std::collections::HashMap::new();

    for node in &index.nodes {
        if node.id.trim().is_empty() {
            diagnostics.push(Diagnostic {
                code: MISSING_ID.into(),
                severity: Severity::Error,
                node: None,
                message: format!("node {} has no id", node.path),
                hint: "declare an `id` in the frontmatter or rename the file to its id".into(),
            });
            continue;
        }
        if node.title.trim().is_empty() {
            diagnostics.push(Diagnostic {
                code: EMPTY_TITLE.into(),
                severity: Severity::Warning,
                node: Some(node.id.clone()),
                message: "node has an empty title".into(),
                hint: "add a `title` frontmatter field or an `# H1` heading".into(),
            });
        }
        if let Some(previous) = seen_ids.insert(node.id.clone(), node.path.clone()) {
            diagnostics.push(Diagnostic {
                code: DUPLICATE_ID.into(),
                severity: Severity::Error,
                node: Some(node.id.clone()),
                message: format!("id {} used by {} and {}", node.id, previous, node.path),
                hint: "make node ids unique across the vault".into(),
            });
        }
        for target in &node.wikilinks {
            if index.get(target).is_none() {
                diagnostics.push(Diagnostic {
                    code: BROKEN_LINK.into(),
                    severity: Severity::Error,
                    node: Some(node.id.clone()),
                    message: format!("node {} links to missing target {target}", node.id),
                    hint: "create the target node or fix the wikilink".into(),
                });
            }
        }
    }

    diagnostics.sort_by(|left, right| {
        (left.severity, &left.code, &left.node, &left.message).cmp(&(
            right.severity,
            &right.code,
            &right.node,
            &right.message,
        ))
    });
    diagnostics.dedup();
    diagnostics
}

/// Counts diagnostics by severity.
pub fn summary(diagnostics: &[Diagnostic]) -> (usize, usize) {
    (
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .count(),
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Warning)
            .count(),
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::parser::parse_vault;

    use super::{Severity, validate_index};

    fn node(file: &str, content: &str) {
        fs::create_dir_all(std::path::Path::new(file).parent().unwrap()).unwrap();
        fs::write(file, content).unwrap();
    }

    #[test]
    fn reports_missing_duplicate_and_broken() {
        let directory = tempfile::tempdir().unwrap();
        node(
            &directory.path().join("a.md").to_string_lossy(),
            "---\nid: A\ntype: term\n---\n# A\n\nLinks [[Ghost]] and [[B]]\n",
        );
        node(
            &directory.path().join("missing.md").to_string_lossy(),
            "---\nid: \"\"\ntype: term\n---\n# Missing\n",
        );
        node(
            &directory.path().join("b.md").to_string_lossy(),
            "---\nid: A\ntype: term\n---\n# B\n",
        );

        let index = parse_vault(directory.path()).unwrap();
        let diagnostics = validate_index(&index);
        assert!(diagnostics.iter().any(|d| d.code == "VAULT001"));
        assert!(diagnostics.iter().any(|d| d.code == "VAULT002"));
        assert!(diagnostics.iter().any(|d| d.code == "VAULT003"));
        let errors = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        assert_eq!(errors, 4);
    }

    #[test]
    fn valid_vault_has_no_error_diagnostics() {
        let directory = tempfile::tempdir().unwrap();
        node(
            &directory.path().join("terms/TERM-A.md").to_string_lossy(),
            "---\nid: TERM-A\ntype: term\n---\n# A\n\nSee [[TERM-B]]\n",
        );
        node(
            &directory.path().join("terms/TERM-B.md").to_string_lossy(),
            "---\nid: TERM-B\ntype: term\n---\n# B\n",
        );
        let index = parse_vault(directory.path()).unwrap();
        let diagnostics = validate_index(&index);
        assert!(diagnostics.is_empty());
    }
}
