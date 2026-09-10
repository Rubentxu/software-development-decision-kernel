//! Cockpit observability views.
//!
//! Five read-only views on top of [`ActiveGraphProjection`] that surface
//! operational evidence: providers, usage, assurance, handoffs,
//! experiments. Output is plain-data (`CockpitView` from
//! `cockpit_views.rs`) and deterministic; the caller supplies
//! `recorded_at`.
//!
//! Pattern: P-a (struct + trait + plain-data evaluator).

use crate::active_graph::{ActiveGraphEdgeKind, ActiveGraphProjection};
use crate::cockpit_views::{CockpitSection, CockpitView};
use std::collections::BTreeMap;

// ── Audit guard constant ────────────────────────────────────────────────

/// Closed-set size of [`CockpitObservabilityKind`]. Bump when a variant
/// is added.
pub const COCKPIT_OBSERVABILITY_KIND_COUNT: usize = 5;

// ── Kind ─────────────────────────────────────────────────────────────────

/// Closed-set taxonomy of Cockpit observability views.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CockpitObservabilityKind {
    Providers,
    Usage,
    Assurance,
    Handoff,
    Experiments,
}

impl CockpitObservabilityKind {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Providers => "providers",
            Self::Usage => "usage",
            Self::Assurance => "assurance",
            Self::Handoff => "handoff",
            Self::Experiments => "experiments",
        }
    }
}

// ── Trait + default builder ──────────────────────────────────────────────

/// Builds observability views from an active-graph projection.
pub trait CockpitObservabilityBuilder {
    fn build(
        &self,
        projection: &ActiveGraphProjection,
        kind: CockpitObservabilityKind,
        recorded_at: &str,
    ) -> CockpitView;
}

/// Default plain-data builder.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DefaultCockpitObservabilityBuilder;

impl CockpitObservabilityBuilder for DefaultCockpitObservabilityBuilder {
    fn build(
        &self,
        projection: &ActiveGraphProjection,
        kind: CockpitObservabilityKind,
        recorded_at: &str,
    ) -> CockpitView {
        match kind {
            CockpitObservabilityKind::Providers => build_providers(projection, recorded_at),
            CockpitObservabilityKind::Usage => build_usage(projection, recorded_at),
            CockpitObservabilityKind::Assurance => build_assurance(projection, recorded_at),
            CockpitObservabilityKind::Handoff => build_handoff(projection, recorded_at),
            CockpitObservabilityKind::Experiments => build_experiments(projection, recorded_at),
        }
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

fn sorted(mut lines: Vec<String>) -> Vec<String> {
    lines.sort();
    lines
}

fn empty_view(title: &str, kind: CockpitObservabilityKind, recorded_at: &str) -> CockpitView {
    CockpitView {
        title: title.to_string(),
        sections: vec![CockpitSection {
            heading: title.to_string(),
            lines: vec!["empty".to_string()],
        }],
        generated_at: recorded_at.to_string(),
        kind: cockpit_view_kind_for(kind),
    }
}

// Map observability kinds to a representative CockpitViewKind for the
// `kind` field (so downstream renders don't need to know about two
// separate enums). For now we reuse CockpitViewKind::Overview as a
// neutral umbrella for observability views.
fn cockpit_view_kind_for(_k: CockpitObservabilityKind) -> crate::cockpit_views::CockpitViewKind {
    crate::cockpit_views::CockpitViewKind::Overview
}

fn build_providers(p: &ActiveGraphProjection, recorded_at: &str) -> CockpitView {
    let mut runs: Vec<String> = Vec::new();
    let mut assurance: Vec<String> = Vec::new();
    for node in p.nodes.values() {
        match node.kind {
            crate::active_graph::ActiveGraphNodeKind::WorkflowRun => {
                runs.push(node.id.0.clone());
            }
            crate::active_graph::ActiveGraphNodeKind::Assurance => {
                assurance.push(node.id.0.clone());
            }
            _ => {}
        }
    }
    if runs.is_empty() && assurance.is_empty() {
        return empty_view(
            "Providers",
            CockpitObservabilityKind::Providers,
            recorded_at,
        );
    }
    CockpitView {
        title: "Providers".to_string(),
        sections: vec![
            CockpitSection {
                heading: "Runs".to_string(),
                lines: sorted(runs),
            },
            CockpitSection {
                heading: "Assurance".to_string(),
                lines: sorted(assurance),
            },
        ],
        generated_at: recorded_at.to_string(),
        kind: cockpit_view_kind_for(CockpitObservabilityKind::Providers),
    }
}

fn build_usage(p: &ActiveGraphProjection, recorded_at: &str) -> CockpitView {
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for edge in &p.edges {
        let label = match edge.kind {
            ActiveGraphEdgeKind::DelegatedTo => "delegated_to",
            ActiveGraphEdgeKind::References => "references",
            ActiveGraphEdgeKind::EvidenceOf => "evidence_of",
            ActiveGraphEdgeKind::ParentOf => "parent_of",
            ActiveGraphEdgeKind::Gates => "gates",
            ActiveGraphEdgeKind::Promotes => "promotes",
        };
        *counts.entry(label).or_insert(0) += 1;
    }
    let lines = sorted(
        counts
            .iter()
            .map(|(k, v)| format!("{k}: {v}"))
            .collect::<Vec<_>>(),
    );
    if lines.is_empty() {
        return empty_view("Usage", CockpitObservabilityKind::Usage, recorded_at);
    }
    CockpitView {
        title: "Usage".to_string(),
        sections: vec![CockpitSection {
            heading: "Usage".to_string(),
            lines,
        }],
        generated_at: recorded_at.to_string(),
        kind: cockpit_view_kind_for(CockpitObservabilityKind::Usage),
    }
}

fn build_assurance(p: &ActiveGraphProjection, recorded_at: &str) -> CockpitView {
    let mut evidence_lines: Vec<String> = Vec::new();
    let mut standalone: Vec<String> = Vec::new();
    let mut evidence_source_ids: std::collections::BTreeSet<_> = std::collections::BTreeSet::new();
    for edge in &p.edges {
        if matches!(edge.kind, ActiveGraphEdgeKind::EvidenceOf) {
            evidence_source_ids.insert(edge.source.clone());
            evidence_lines.push(format!("{} → {}", edge.source.0, edge.target.0));
        }
    }
    for node in p.nodes.values() {
        if matches!(
            node.kind,
            crate::active_graph::ActiveGraphNodeKind::Assurance
        ) && !evidence_source_ids.contains(&node.id)
        {
            standalone.push(node.id.0.clone());
        }
    }
    if evidence_lines.is_empty() && standalone.is_empty() {
        return empty_view(
            "Assurance",
            CockpitObservabilityKind::Assurance,
            recorded_at,
        );
    }
    CockpitView {
        title: "Assurance".to_string(),
        sections: vec![
            CockpitSection {
                heading: "Evidence".to_string(),
                lines: sorted(evidence_lines),
            },
            CockpitSection {
                heading: "Standalone".to_string(),
                lines: sorted(standalone),
            },
        ],
        generated_at: recorded_at.to_string(),
        kind: cockpit_view_kind_for(CockpitObservabilityKind::Assurance),
    }
}

fn build_handoff(p: &ActiveGraphProjection, recorded_at: &str) -> CockpitView {
    let mut lines: Vec<String> = Vec::new();
    for edge in &p.edges {
        if matches!(edge.kind, ActiveGraphEdgeKind::DelegatedTo) {
            lines.push(format!("{} → {}", edge.source.0, edge.target.0));
        }
    }
    if lines.is_empty() {
        return empty_view("Handoff", CockpitObservabilityKind::Handoff, recorded_at);
    }
    CockpitView {
        title: "Handoff".to_string(),
        sections: vec![CockpitSection {
            heading: "Delegations".to_string(),
            lines: sorted(lines),
        }],
        generated_at: recorded_at.to_string(),
        kind: cockpit_view_kind_for(CockpitObservabilityKind::Handoff),
    }
}

fn build_experiments(p: &ActiveGraphProjection, recorded_at: &str) -> CockpitView {
    let mut lab_lines: Vec<String> = Vec::new();
    let mut promote_lines: Vec<String> = Vec::new();
    for node in p.nodes.values() {
        if matches!(
            node.kind,
            crate::active_graph::ActiveGraphNodeKind::WorkflowLab
        ) {
            lab_lines.push(node.id.0.clone());
        }
    }
    for edge in &p.edges {
        match edge.kind {
            ActiveGraphEdgeKind::Promotes => {
                promote_lines.push(format!("{} → {} promote", edge.source.0, edge.target.0))
            }
            ActiveGraphEdgeKind::Gates => {
                promote_lines.push(format!("{} → {} gate", edge.source.0, edge.target.0))
            }
            _ => {}
        }
    }
    if lab_lines.is_empty() && promote_lines.is_empty() {
        return empty_view(
            "Experiments",
            CockpitObservabilityKind::Experiments,
            recorded_at,
        );
    }
    CockpitView {
        title: "Experiments".to_string(),
        sections: vec![
            CockpitSection {
                heading: "Labs".to_string(),
                lines: sorted(lab_lines),
            },
            CockpitSection {
                heading: "Promotion Decisions".to_string(),
                lines: sorted(promote_lines),
            },
        ],
        generated_at: recorded_at.to_string(),
        kind: cockpit_view_kind_for(CockpitObservabilityKind::Experiments),
    }
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::collapsible_if)]
mod tests {
    use super::*;
    use crate::active_graph::{
        ActiveGraphInput, ActiveGraphProjector, DefaultActiveGraphProjector,
    };

    use sddk_domain::workflow_ir::NodeId;

    fn projector() -> DefaultActiveGraphProjector {
        DefaultActiveGraphProjector
    }
    fn builder() -> DefaultCockpitObservabilityBuilder {
        DefaultCockpitObservabilityBuilder
    }
    fn sample() -> ActiveGraphProjection {
        let input = ActiveGraphInput {
            workflow_nodes: vec![NodeId("wf".to_string()), NodeId("leaf".to_string())],
            workflow_edges: vec![(NodeId("wf".to_string()), NodeId("leaf".to_string()))],
            workflow_run_id: Some("r1".to_string()),
            memory_head: None,
            memory_refs: vec![("ref1".to_string(), NodeId("wf".to_string()))],
            assurance_labels: vec!["a1".to_string()],
            evidence_links: vec![("a1".to_string(), NodeId("leaf".to_string()))],
            lab_promotions: vec![
                ("g1".to_string(), NodeId("leaf".to_string()), false), // gate
                ("p1".to_string(), NodeId("wf".to_string()), true),    // promote
            ],
            workflow_lab_labels: vec!["lab1".to_string()],
            uat_labels: vec![],
            runbook_labels: vec![],
            human_decision_labels: vec![],
            delegations: vec![(NodeId("orch".to_string()), NodeId("worker".to_string()))],
            node_provenance: BTreeMap::new(),
            edge_provenance: BTreeMap::new(),
        };
        projector().project(&input, "t0")
    }

    // ── S-1: Providers lists runs + assurance ──────────────────────────

    #[test]
    fn s1_providers_lists_runs_and_assurance() {
        let p = sample();
        let v = builder().build(&p, CockpitObservabilityKind::Providers, "t0");
        let runs = v
            .sections
            .iter()
            .find(|s| s.heading == "Runs")
            .expect("Runs section");
        assert_eq!(runs.lines.len(), 1);
        assert!(runs.lines[0].contains("r1"));
        let assurance = v
            .sections
            .iter()
            .find(|s| s.heading == "Assurance")
            .expect("Assurance section");
        assert!(assurance.lines.iter().any(|l| l.contains("a1")));
    }

    // ── S-2: Usage counts reference + delegation edges ─────────────────

    #[test]
    fn s2_usage_counts_edges_by_kind() {
        let p = sample();
        let v = builder().build(&p, CockpitObservabilityKind::Usage, "t0");
        let section = v.sections.first().expect("at least one section");
        assert!(section.lines.iter().any(|l| l == "references: 1"));
        assert!(section.lines.iter().any(|l| l == "delegated_to: 1"));
        assert!(section.lines.iter().any(|l| l == "evidence_of: 1"));
        assert!(section.lines.iter().any(|l| l == "parent_of: 1"));
        assert!(section.lines.iter().any(|l| l == "promotes: 1"));
        assert!(section.lines.iter().any(|l| l == "gates: 1"));
    }

    // ── S-3: Assurance lists evidence edges ───────────────────────────

    #[test]
    fn s3_assurance_lists_evidence_edges() {
        let p = sample();
        let v = builder().build(&p, CockpitObservabilityKind::Assurance, "t0");
        let ev = v
            .sections
            .iter()
            .find(|s| s.heading == "Evidence")
            .expect("Evidence section");
        assert!(
            ev.lines
                .iter()
                .any(|l| l.contains("a1") && l.contains("leaf"))
        );
    }

    // ── S-4: Handoff lists delegation edges ────────────────────────────

    #[test]
    fn s4_handoff_lists_delegations() {
        let p = sample();
        let v = builder().build(&p, CockpitObservabilityKind::Handoff, "t0");
        let sec = v
            .sections
            .iter()
            .find(|s| s.heading == "Delegations")
            .expect("Delegations section");
        assert!(
            sec.lines
                .iter()
                .any(|l| l.contains("orch") && l.contains("worker"))
        );
    }

    // ── S-5: Experiments distinguishes promote vs gate ─────────────────

    #[test]
    fn s5_experiments_distinguishes_promote_vs_gate() {
        let p = sample();
        let v = builder().build(&p, CockpitObservabilityKind::Experiments, "t0");
        let promos = v
            .sections
            .iter()
            .find(|s| s.heading == "Promotion Decisions")
            .expect("Promotion Decisions section");
        assert!(promos.lines.iter().any(|l| l.contains("promote")));
        assert!(promos.lines.iter().any(|l| l.contains("gate")));
        let labs = v
            .sections
            .iter()
            .find(|s| s.heading == "Labs")
            .expect("Labs section");
        assert!(labs.lines.iter().any(|l| l.contains("lab1")));
    }

    // ── S-6: Empty projection ⇒ "empty" view ───────────────────────────

    #[test]
    fn s6_empty_projection_yields_empty_placeholder() {
        let p = ActiveGraphProjection::default();
        for kind in [
            CockpitObservabilityKind::Providers,
            CockpitObservabilityKind::Usage,
            CockpitObservabilityKind::Assurance,
            CockpitObservabilityKind::Handoff,
            CockpitObservabilityKind::Experiments,
        ] {
            let v = builder().build(&p, kind, "t0");
            assert_eq!(v.sections.len(), 1);
            assert_eq!(v.sections[0].lines, vec!["empty".to_string()]);
        }
    }

    // ── S-7: Closed-set audit guard ────────────────────────────────────

    #[test]
    fn s7_closed_set_audit_guard() {
        assert_eq!(COCKPIT_OBSERVABILITY_KIND_COUNT, 5);
        let all_kinds = [
            CockpitObservabilityKind::Providers,
            CockpitObservabilityKind::Usage,
            CockpitObservabilityKind::Assurance,
            CockpitObservabilityKind::Handoff,
            CockpitObservabilityKind::Experiments,
        ];
        assert_eq!(all_kinds.len(), COCKPIT_OBSERVABILITY_KIND_COUNT);
    }

    // ── Bonus: deterministic output ────────────────────────────────────

    #[test]
    fn s8_deterministic_output() {
        let p = sample();
        let v1 = builder().build(&p, CockpitObservabilityKind::Usage, "t-A");
        let v2 = builder().build(&p, CockpitObservabilityKind::Usage, "t-B");
        assert_eq!(v1.sections, v2.sections);
        assert_ne!(v1.generated_at, v2.generated_at);
    }
}
