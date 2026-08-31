//! Stable relation diagnostics for a parsed vault.

use serde::Serialize;
use thiserror::Error;

use crate::index::VaultIndex;

/// Cycle attribution for diagnostics whose missing target identifies an existing
/// cycle node. Closed-set recoverable codes are declared verbatim in ADR-0078;
/// initial allow-list is `{VAULT003}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CycleScope {
    /// Project identifier.
    pub project_id: String,
    /// Cycle identifier.
    pub cycle_id: String,
}

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
    /// Populated only for VAULT003 broken-link diagnostics whose missing
    /// target names an existing cycle node (`cycles/<project_id>/cycle-<N>-<slug>.md`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<CycleScope>,
    /// Machine-readable error kind for scoped-cli diagnostics.
    /// Values: `InvalidScopeCycleId`, `RepairReceiptMissingOrInvalid`,
    /// `ReceiptEvidenceHashMismatch`, `RepairQueueMalformed`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_kind: Option<String>,
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

/// Attaches cycle scope to a VAULT003 diagnostic if the missing target
/// matches the canonical cycle node path pattern.
///
/// Returns `Some(CycleScope)` if `target` matches `project_id/cycle_id`
/// where the cycle node file would be at `cycles/<project_id>/cycle-<N>-<slug>.md`.
/// Otherwise returns `None`.
fn attach_scope(target: &str) -> Option<CycleScope> {
    // Target format: project_id/cycle_id (e.g. "p-52b95ef55999f9de/cycle-44-build-remediate-transition")
    // We need to check if this target corresponds to an existing cycle node.
    // The cycle node path convention is: cycles/<project_id>/cycle-<N>-<slug>.md
    // But we don't have filesystem access here — we just extract the scope from the target.
    // The actual validation that the node exists happens at a higher layer.
    let parts: Vec<&str> = target.split('/').collect();
    if parts.len() != 2 {
        return None;
    }
    let project_id = parts[0];
    let cycle_id = parts[1];
    // Basic validation: project_id should look like a project ID (p- followed by hex)
    if !project_id.starts_with("p-") || cycle_id.is_empty() {
        return None;
    }
    Some(CycleScope {
        project_id: project_id.to_string(),
        // cycle_id is the FULL target (project_id/cycle_id) to match the queue key format
        cycle_id: target.to_string(),
    })
}

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
                scope: None,
                error_kind: None,
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
                scope: None,
                error_kind: None,
            });
        }
        if let Some(previous) = seen_ids.insert(node.id.clone(), node.path.clone()) {
            diagnostics.push(Diagnostic {
                code: DUPLICATE_ID.into(),
                severity: Severity::Error,
                node: Some(node.id.clone()),
                message: format!("id {} used by {} and {}", node.id, previous, node.path),
                hint: "make node ids unique across the vault".into(),
                scope: None,
                error_kind: None,
            });
        }
        for target in &node.wikilinks {
            if index.get(target).is_none() {
                let scope = attach_scope(target);
                diagnostics.push(Diagnostic {
                    code: BROKEN_LINK.into(),
                    severity: Severity::Error,
                    node: Some(node.id.clone()),
                    message: format!("node {} links to missing target {target}", node.id),
                    hint: "create the target node or fix the wikilink".into(),
                    scope,
                    error_kind: None,
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

    #[test]
    fn vaul003_broken_link_attaches_cycle_scope() {
        let directory = tempfile::tempdir().unwrap();
        node(
            &directory.path().join("a.md").to_string_lossy(),
            "---\nid: A\ntype: term\n---\n# A\n\nLinks [[p-52b95ef55999f9de/cycle-44-build-remediate-transition]]\n",
        );
        let index = parse_vault(directory.path()).unwrap();
        let diagnostics = validate_index(&index);
        let broken = diagnostics
            .iter()
            .find(|d| d.code == "VAULT003")
            .expect("VAULT003 diagnostic must be present");
        assert!(
            broken.scope.is_some(),
            "VAULT003 for cycle-scoped target must have scope attached"
        );
        let scope = broken.scope.as_ref().unwrap();
        assert_eq!(scope.project_id, "p-52b95ef55999f9de");
        assert_eq!(
            scope.cycle_id,
            "p-52b95ef55999f9de/cycle-44-build-remediate-transition"
        );
    }

    #[test]
    fn vaul003_non_cycle_target_has_no_scope() {
        let directory = tempfile::tempdir().unwrap();
        node(
            &directory.path().join("a.md").to_string_lossy(),
            "---\nid: A\ntype: term\n---\n# A\n\nLinks [[GhostNode]]\n",
        );
        let index = parse_vault(directory.path()).unwrap();
        let diagnostics = validate_index(&index);
        let broken = diagnostics
            .iter()
            .find(|d| d.code == "VAULT003")
            .expect("VAULT003 diagnostic must be present");
        assert!(
            broken.scope.is_none(),
            "VAULT003 for non-cycle target must have no scope"
        );
    }

    #[test]
    fn vaul001_has_no_scope() {
        let directory = tempfile::tempdir().unwrap();
        node(
            &directory.path().join("a.md").to_string_lossy(),
            "---\nid: \"\"\ntype: term\n---\n# A\n",
        );
        let index = parse_vault(directory.path()).unwrap();
        let diagnostics = validate_index(&index);
        let missing = diagnostics
            .iter()
            .find(|d| d.code == "VAULT001")
            .expect("VAULT001 diagnostic must be present");
        assert!(missing.scope.is_none(), "VAULT001 must never have scope");
    }

    #[test]
    fn default_json_output_omits_null_scope() {
        // Verify skip_serializing_if = "Option::is_none" on Diagnostic.scope
        let directory = tempfile::tempdir().unwrap();
        node(
            &directory.path().join("a.md").to_string_lossy(),
            "---\nid: A\ntype: term\n---\n# A\n\nLinks [[Ghost]]\n",
        );
        let index = parse_vault(directory.path()).unwrap();
        let diagnostics = validate_index(&index);
        let json = serde_json::to_string(&diagnostics).expect("diagnostics must serialize to JSON");
        // If scope were present (even as null), "scope":null would appear
        assert!(
            !json.contains("scope"),
            "JSON output must not contain 'scope' key for absent scope: {}",
            json
        );
    }

    #[test]
    fn closed_set_guard_only_vaul003_has_scope() {
        let directory = tempfile::tempdir().unwrap();
        node(
            &directory.path().join("a.md").to_string_lossy(),
            "---\nid: A\ntype: term\n---\n# A\n\nLinks [[VAULT001-broken]] [[VAULT002-broken]] [[VAULT003-broken]]\n",
        );
        node(
            &directory.path().join("b.md").to_string_lossy(),
            "---\nid: \"\"\ntype: term\n---\n# B\n",
        );
        node(
            &directory.path().join("c.md").to_string_lossy(),
            "---\nid: A\ntype: term\n---\n# C\n",
        );
        let index = parse_vault(directory.path()).unwrap();
        let diagnostics = validate_index(&index);
        let with_scope: Vec<_> = diagnostics.iter().filter(|d| d.scope.is_some()).collect();
        // Only VAULT003 can have scope (closed-set guard)
        for d in &with_scope {
            assert_eq!(
                d.code, "VAULT003",
                "Only VAULT003 may have scope; {} does not",
                d.code
            );
        }
    }
}
