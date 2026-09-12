// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// semantic_kind.rs — T-01 (M3 arch-spec-005)
//
// Closed-set `NodeKind` and `RelationKind` enums. Core families/relations
// are typed; extensions are explicitly namespaced to avoid free strings.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The 18 core node families listed in arch-spec-005.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum CoreNodeKind {
    Project,
    Goal,
    WorkItem,
    Cycle,
    Workflow,
    PlanRevision,
    Run,
    Action,
    Decision,
    Alternative,
    Assumption,
    Evidence,
    Risk,
    Debt,
    Contribution,
    Synthesis,
    Dissent,
    Artifact,
}

impl CoreNodeKind {
    pub const ALL: [CoreNodeKind; 18] = [
        CoreNodeKind::Project,
        CoreNodeKind::Goal,
        CoreNodeKind::WorkItem,
        CoreNodeKind::Cycle,
        CoreNodeKind::Workflow,
        CoreNodeKind::PlanRevision,
        CoreNodeKind::Run,
        CoreNodeKind::Action,
        CoreNodeKind::Decision,
        CoreNodeKind::Alternative,
        CoreNodeKind::Assumption,
        CoreNodeKind::Evidence,
        CoreNodeKind::Risk,
        CoreNodeKind::Debt,
        CoreNodeKind::Contribution,
        CoreNodeKind::Synthesis,
        CoreNodeKind::Dissent,
        CoreNodeKind::Artifact,
    ];

    pub fn domain_tag(&self) -> &'static str {
        match self {
            CoreNodeKind::Project => "project",
            CoreNodeKind::Goal => "goal",
            CoreNodeKind::WorkItem => "work_item",
            CoreNodeKind::Cycle => "cycle",
            CoreNodeKind::Workflow => "workflow",
            CoreNodeKind::PlanRevision => "plan_revision",
            CoreNodeKind::Run => "run",
            CoreNodeKind::Action => "action",
            CoreNodeKind::Decision => "decision",
            CoreNodeKind::Alternative => "alternative",
            CoreNodeKind::Assumption => "assumption",
            CoreNodeKind::Evidence => "evidence",
            CoreNodeKind::Risk => "risk",
            CoreNodeKind::Debt => "debt",
            CoreNodeKind::Contribution => "contribution",
            CoreNodeKind::Synthesis => "synthesis",
            CoreNodeKind::Dissent => "dissent",
            CoreNodeKind::Artifact => "artifact",
        }
    }
}

/// The 14 core relation kinds listed in arch-spec-005 plus the two
/// evidence-model relations added in v1.168.35: `Verifies` (an Evidence
/// confirms a Decision, Assumption, or Contribution) and `ObservedFor`
/// (an Evidence was collected specifically for a Risk, Goal, or Run).
/// Per ADR-0100, these replace the legacy `PlanningEvidenceKind` enum
/// (which was a closed-set discriminator instead of a typed graph edge).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum CoreRelationKind {
    DependsOn,
    CausedBy,
    Supports,
    Contradicts,
    Justifies,
    SelectedOver,
    Supersedes,
    ProducedBy,
    ConsumedBy,
    Gates,
    References,
    Affects,
    /// Evidence → Decision/Assumption/Contribution. The evidence confirms
    /// the target holds (e.g. a UAT scenario verifies an acceptance criterion).
    Verifies,
    /// Evidence → Risk/Goal/Run. The evidence was collected specifically
    /// to inform or guard the target (e.g. a regression run observed a
    /// risk mitigation).
    ObservedFor,
}

impl CoreRelationKind {
    pub const ALL: [CoreRelationKind; 14] = [
        CoreRelationKind::DependsOn,
        CoreRelationKind::CausedBy,
        CoreRelationKind::Supports,
        CoreRelationKind::Contradicts,
        CoreRelationKind::Justifies,
        CoreRelationKind::SelectedOver,
        CoreRelationKind::Supersedes,
        CoreRelationKind::ProducedBy,
        CoreRelationKind::ConsumedBy,
        CoreRelationKind::Gates,
        CoreRelationKind::References,
        CoreRelationKind::Affects,
        CoreRelationKind::Verifies,
        CoreRelationKind::ObservedFor,
    ];

    pub fn domain_tag(&self) -> &'static str {
        match self {
            CoreRelationKind::DependsOn => "depends_on",
            CoreRelationKind::CausedBy => "caused_by",
            CoreRelationKind::Supports => "supports",
            CoreRelationKind::Contradicts => "contradicts",
            CoreRelationKind::Justifies => "justifies",
            CoreRelationKind::SelectedOver => "selected_over",
            CoreRelationKind::Supersedes => "supersedes",
            CoreRelationKind::ProducedBy => "produced_by",
            CoreRelationKind::ConsumedBy => "consumed_by",
            CoreRelationKind::Gates => "gates",
            CoreRelationKind::References => "references",
            CoreRelationKind::Affects => "affects",
            CoreRelationKind::Verifies => "verifies",
            CoreRelationKind::ObservedFor => "observed_for",
        }
    }
}

/// Namespaced kind for extensions: "namespace.kind". Strings are NOT freely
/// accepted; the parser rejects malformed inputs.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct NamespacedKind(pub String);

impl NamespacedKind {
    pub fn new(s: &str) -> Result<Self, SemanticKindError> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return Err(SemanticKindError::Empty);
        }
        let parts: Vec<&str> = trimmed.split('.').collect();
        // Accept either "kind" or "namespace.kind" (two-segment). Reject
        // three or more segments or empty segments.
        if parts.len() > 2 || parts.iter().any(|p| p.is_empty()) {
            return Err(SemanticKindError::Malformed {
                input: s.to_string(),
                expected: "kind | namespace.kind (lowercase ASCII, digits, '_')".into(),
            });
        }
        for segment in &parts {
            for c in segment.chars() {
                if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '_' {
                    return Err(SemanticKindError::Malformed {
                        input: s.to_string(),
                        expected: "kind: lowercase ASCII, digits, '_'".into(),
                    });
                }
            }
        }
        Ok(Self(s.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum NodeKind {
    Core(CoreNodeKind),
    Extension(NamespacedKind),
}

impl NodeKind {
    pub fn parse(s: &str) -> Result<Self, SemanticKindError> {
        if let Some(core) = core_node_from_tag(s) {
            return Ok(NodeKind::Core(core));
        }
        Ok(NodeKind::Extension(NamespacedKind::new(s)?))
    }

    pub fn domain_tag(&self) -> String {
        match self {
            NodeKind::Core(c) => c.domain_tag().to_string(),
            NodeKind::Extension(n) => n.as_str().to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum RelationKind {
    Core(CoreRelationKind),
    Extension(NamespacedKind),
}

impl RelationKind {
    pub fn parse(s: &str) -> Result<Self, SemanticKindError> {
        if let Some(core) = core_relation_from_tag(s) {
            return Ok(RelationKind::Core(core));
        }
        Ok(RelationKind::Extension(NamespacedKind::new(s)?))
    }

    pub fn domain_tag(&self) -> String {
        match self {
            RelationKind::Core(c) => c.domain_tag().to_string(),
            RelationKind::Extension(n) => n.as_str().to_string(),
        }
    }
}

/// Closed-set parser: rejects unknown core labels. Returns `None` for any
/// string not matching a known core kind, signalling the caller to try the
/// namespaced parser instead.
fn core_node_from_tag(s: &str) -> Option<CoreNodeKind> {
    CoreNodeKind::ALL
        .iter()
        .find(|c| c.domain_tag() == s)
        .copied()
}

fn core_relation_from_tag(s: &str) -> Option<CoreRelationKind> {
    CoreRelationKind::ALL
        .iter()
        .find(|c| c.domain_tag() == s)
        .copied()
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum SemanticKindError {
    #[error("namespaced kind input is empty")]
    Empty,
    #[error("malformed namespaced kind '{input}': expected {expected}")]
    Malformed { input: String, expected: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_node_kinds_have_distinct_tags() {
        let tags: Vec<&str> = CoreNodeKind::ALL.iter().map(|c| c.domain_tag()).collect();
        let unique: std::collections::BTreeSet<&str> = tags.iter().copied().collect();
        assert_eq!(unique.len(), tags.len(), "core node tags must be distinct");
    }

    #[test]
    fn core_relation_kinds_have_distinct_tags() {
        let tags: Vec<&str> = CoreRelationKind::ALL
            .iter()
            .map(|c| c.domain_tag())
            .collect();
        let unique: std::collections::BTreeSet<&str> = tags.iter().copied().collect();
        assert_eq!(
            unique.len(),
            tags.len(),
            "core relation tags must be distinct"
        );
    }

    #[test]
    fn node_kind_rejects_unknown_core_label() {
        // Free string "ghost" is not a core tag; must parse as Extension.
        let kind = NodeKind::parse("ghost").expect("should not error; falls to Extension");
        match kind {
            NodeKind::Extension(n) => assert_eq!(n.as_str(), "ghost"),
            _ => panic!("expected Extension"),
        }
    }

    #[test]
    fn relation_kind_rejects_unknown_core_label() {
        let kind = RelationKind::parse("ghost").expect("falls to Extension");
        assert!(matches!(kind, RelationKind::Extension(_)));
    }

    #[test]
    fn node_kind_extension_namespaced_is_explicit() {
        let kind = NodeKind::parse("pack_uat.test_suite").unwrap();
        match kind {
            NodeKind::Extension(n) => {
                assert_eq!(n.as_str(), "pack_uat.test_suite");
            }
            _ => panic!("expected Extension"),
        }
    }

    #[test]
    fn namespaced_kind_rejects_malformed_input() {
        // Three or more segments rejected
        assert!(NamespacedKind::new("a.b.c").is_err());
        // Empty namespace
        assert!(NamespacedKind::new(".kind").is_err());
        // Empty kind
        assert!(NamespacedKind::new("ns.").is_err());
        // Uppercase rejected
        assert!(NamespacedKind::new("Ns.kind").is_err());
        // Hyphen rejected
        assert!(NamespacedKind::new("ns-kind.foo").is_err());
        // Empty input
        assert!(NamespacedKind::new("").is_err());
        // Single-segment with valid chars is accepted as a namespaced kind.
        assert!(NamespacedKind::new("simple_kind").is_ok());
        // Two-segment with valid chars is accepted.
        assert!(NamespacedKind::new("ns.kind").is_ok());
    }

    #[test]
    fn parse_recognises_known_core() {
        let kind = NodeKind::parse("decision").unwrap();
        assert_eq!(kind, NodeKind::Core(CoreNodeKind::Decision));
    }

    #[test]
    fn parse_recognises_known_relation_core() {
        let kind = RelationKind::parse("depends_on").unwrap();
        assert_eq!(kind, RelationKind::Core(CoreRelationKind::DependsOn));
    }

    #[test]
    fn parse_recognises_evidence_relations() {
        // Verifies + ObservedFor (added v1.168.35 per ADR-0100): the
        // universal Evidence model uses these two relations to attach
        // EvidenceRef to Decisions/Assumptions/Contributions (Verifies)
        // and Risks/Goals/Runs (ObservedFor). They MUST round-trip
        // through the closed-set parser as Core variants, not fall
        // through to the NamespacedKind extension.
        let verifies = RelationKind::parse("verifies").unwrap();
        assert_eq!(
            verifies,
            RelationKind::Core(CoreRelationKind::Verifies),
            "verifies must parse as core relation, not extension"
        );
        let observed = RelationKind::parse("observed_for").unwrap();
        assert_eq!(
            observed,
            RelationKind::Core(CoreRelationKind::ObservedFor),
            "observed_for must parse as core relation, not extension"
        );
    }

    #[test]
    fn core_relation_kinds_have_14_entries_after_evidence_relations() {
        // Pin the count so a future addition must update both this test
        // AND the doc comment line 81. Without this pin, adding a relation
        // silently is too easy.
        assert_eq!(
            CoreRelationKind::ALL.len(),
            14,
            "expected 14 core relation kinds (12 original + Verifies + ObservedFor)"
        );
    }

    #[test]
    fn evidence_relations_have_canonical_tags() {
        // Domain tags must be lowercase snake_case and unique across the
        // ALL array. Verifies → "verifies"; ObservedFor → "observed_for".
        assert_eq!(CoreRelationKind::Verifies.domain_tag(), "verifies");
        assert_eq!(CoreRelationKind::ObservedFor.domain_tag(), "observed_for");
        // No collision with the 12 original tags (would panic in
        // core_relation_kinds_have_distinct_tags anyway, but be explicit).
        let tags: Vec<&str> = CoreRelationKind::ALL
            .iter()
            .map(|c| c.domain_tag())
            .collect();
        let unique: std::collections::BTreeSet<&str> = tags.iter().copied().collect();
        assert_eq!(
            unique.len(),
            tags.len(),
            "evidence relation tags must be distinct from existing 12"
        );
    }
}
