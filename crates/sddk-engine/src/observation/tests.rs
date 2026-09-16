// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// observation/tests.rs — A4-0 acceptance pins (arch-spec-042).

use crate::architectural_contract::{ComponentRef, EntityRef};
use crate::architecture_graph::SoftwareUnitRef;
use crate::evidence_ref::{EvidenceKind, EvidenceRef};
use crate::knowledge::{EventTime, KnowledgeBasis};
use crate::semantic_graph::{InMemorySemanticGraph, SemanticGraphProjection};
use crate::semantic_kind::CoreRelationKind;

use super::project::{
    OBSERVATION_NODE_KIND, OBSERVES_RELATION_KIND, RELATION_NODE_KIND, project_into,
};
use super::resolution::{EvidenceResolution, resolve_relation};
use super::types::{
    ObservationBasis, ObservationId, ObservationOrigin, ObservationSet, ObservationStance,
    ObservationSubject, RelationId, SoftwareEntityRef, SoftwareObservation, SoftwareRelation,
};

fn unit(s: &str) -> SoftwareEntityRef {
    SoftwareEntityRef::Unit(SoftwareUnitRef::new(s))
}
fn component(s: &str) -> SoftwareEntityRef {
    SoftwareEntityRef::Component(ComponentRef::new(s).expect("component"))
}
fn entity(s: &str) -> SoftwareEntityRef {
    SoftwareEntityRef::Entity(EntityRef::new(s).expect("entity"))
}
fn relation() -> SoftwareRelation {
    SoftwareRelation::new(
        unit("comp:auth"),
        CoreRelationKind::DependsOn,
        component("comp:log"),
    )
}
fn basis() -> ObservationBasis {
    ObservationBasis::new(
        "rev:1",
        KnowledgeBasis::empty(EventTime(1)).basis_hash().clone(),
        "input:abc",
    )
}
fn evidence(locator: &str) -> EvidenceRef {
    EvidenceRef::new(EvidenceKind::Adhoc, locator)
}
fn observe(
    set: &mut ObservationSet,
    stance: ObservationStance,
    origin: ObservationOrigin,
    locator: &str,
) -> ObservationId {
    let o = SoftwareObservation::declare(
        ObservationSubject::SoftwareRelation(relation()),
        stance,
        evidence(locator),
        origin,
        basis(),
        None,
        "test-producer",
    );
    let id = o.id.clone();
    set.insert(o);
    id
}

// ── Identity ────────────────────────────────────────────────────────────────

#[test]
fn acceptance_relation_is_an_adt_not_a_rendering() {
    // REQ-A4S0-001: endpoints are typed, so a renderer cannot change identity.
    let r = relation();
    assert_eq!(r.from.canonical_tag(), "unit:comp:auth");
    assert_eq!(r.to.canonical_tag(), "component:comp:log");
    assert!(r.render().contains("--depends_on-->"));
    // A rendering is not an identity: the id is a hash, not the string.
    assert_ne!(r.id().as_str(), r.render());
    assert_eq!(r.id().as_str().len(), 64);
}

#[test]
fn acceptance_relation_id_is_stable_and_content_addressed() {
    // REQ-A4S0-002
    let a = relation().id();
    let b = relation().id();
    assert_eq!(a, b);
    // Same endpoints, different kind => different id.
    let other_kind = SoftwareRelation::new(
        unit("comp:auth"),
        CoreRelationKind::Supports,
        component("comp:log"),
    );
    assert_ne!(a, other_kind.id());
    // Different endpoint type for the same text => different id.
    let same_text_other_type = SoftwareRelation::new(
        component("comp:auth"),
        CoreRelationKind::DependsOn,
        component("comp:log"),
    );
    assert_ne!(a, same_text_other_type.id(), "endpoint kind participates");
}

#[test]
fn acceptance_relation_id_is_direction_sensitive() {
    // REQ-A4S0-003: reversing the endpoints is a different relation.
    let forward = SoftwareRelation::new(entity("e:a"), CoreRelationKind::DependsOn, entity("e:b"));
    let reverse = SoftwareRelation::new(entity("e:b"), CoreRelationKind::DependsOn, entity("e:a"));
    assert_ne!(forward.id(), reverse.id());
}

#[test]
fn acceptance_observation_id_is_clock_stable() {
    // REQ-A4S0-004: no timestamp enters identity, so an id taken now and later
    // (same basis) agrees.
    let mut a = ObservationSet::new();
    let mut b = ObservationSet::new();
    let ida = observe(
        &mut a,
        ObservationStance::Affirms,
        ObservationOrigin::DeterministicLocal,
        "e1",
    );
    let idb = observe(
        &mut b,
        ObservationStance::Affirms,
        ObservationOrigin::DeterministicLocal,
        "e1",
    );
    assert_eq!(ida, idb);
    assert_ne!(ida.as_str(), "");
}

#[test]
fn acceptance_identities_exclude_volatile_fields() {
    // REQ-A4S0-004: the producer label and the freshness evaluation are not
    // identity. Two observations differing only in `producer` share an id.
    let subject = ObservationSubject::SoftwareRelation(relation());
    let a = SoftwareObservation::declare(
        subject.clone(),
        ObservationStance::Affirms,
        evidence("e1"),
        ObservationOrigin::DeterministicLocal,
        basis(),
        None,
        "producer-one",
    );
    let b = SoftwareObservation::declare(
        subject,
        ObservationStance::Affirms,
        evidence("e1"),
        ObservationOrigin::DeterministicLocal,
        basis(),
        None,
        "producer-two",
    );
    assert_eq!(a.id, b.id, "the producer label is provenance, not identity");
}

#[test]
fn acceptance_relation_kind_reuses_core_vocabulary() {
    // REQ-A4S0-005: no second relation taxonomy.
    assert_eq!(CoreRelationKind::DependsOn.domain_tag(), "depends_on");
    // A4 FU-A3-CO-2: ContractedBy + SpecifiedBy removed from CoreRelationKind (16 -> 14).
    assert_eq!(CoreRelationKind::ALL.len(), 14);
}

#[test]
fn acceptance_origin_is_closed_and_exhaustive() {
    // REQ-A4S0-006
    assert_eq!(ObservationOrigin::ALL.len(), 5);
    let tags: Vec<&str> = ObservationOrigin::ALL
        .iter()
        .map(|o| o.canonical_tag())
        .collect();
    assert_eq!(
        tags,
        vec![
            "deterministic_local",
            "static_provider",
            "runtime_provider",
            "human_declared",
            "inferred"
        ]
    );
}

#[test]
fn acceptance_reuses_the_universal_evidence_ref() {
    // REQ-A4S0-007: the substrate holds the universal EvidenceRef, and no third
    // evidence type was introduced.
    let mut set = ObservationSet::new();
    observe(
        &mut set,
        ObservationStance::Affirms,
        ObservationOrigin::StaticProvider,
        "e1",
    );
    let o = &set.observations()[0];
    assert_eq!(o.evidence.kind, EvidenceKind::Adhoc);
    assert_eq!(o.evidence.locator, "e1");
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/observation/types.rs"
    ))
    .expect("types.rs");
    assert!(
        !src.contains("struct EvidenceRef") && !src.contains("enum EvidenceKind"),
        "the substrate must not mint a third evidence type"
    );
}

#[test]
fn acceptance_basis_is_recorded() {
    // REQ-A4S0-008
    let mut set = ObservationSet::new();
    observe(
        &mut set,
        ObservationStance::Affirms,
        ObservationOrigin::Inferred,
        "e1",
    );
    let o = &set.observations()[0];
    assert_eq!(o.basis.revision, "rev:1");
    assert_eq!(o.basis.knowledge_basis.to_hex().len(), 64);
    assert_eq!(o.basis.input_digest, "input:abc");
}

#[test]
fn acceptance_freshness_is_absent_not_assumed() {
    // REQ-A4S0-009: without an expected basis there is no evaluation, and `None`
    // must not read as fresh.
    let mut set = ObservationSet::new();
    observe(
        &mut set,
        ObservationStance::Affirms,
        ObservationOrigin::RuntimeProvider,
        "e1",
    );
    assert!(set.observations()[0].freshness.is_none());
}

// ── Contradictions ──────────────────────────────────────────────────────────

#[test]
fn acceptance_contradictory_evidence_coexists() {
    // REQ-A4S0-010: the static affirm and the runtime deny both survive.
    let mut set = ObservationSet::new();
    let affirm = observe(
        &mut set,
        ObservationStance::Affirms,
        ObservationOrigin::StaticProvider,
        "static:e1",
    );
    let deny = observe(
        &mut set,
        ObservationStance::Denies,
        ObservationOrigin::RuntimeProvider,
        "runtime:e1",
    );
    assert_eq!(set.len(), 2, "nothing was overwritten");
    let ids: Vec<&str> = set.observations().iter().map(|o| o.id.as_str()).collect();
    assert!(ids.contains(&affirm.as_str()) && ids.contains(&deny.as_str()));

    match resolve_relation(&set, &relation().id()) {
        EvidenceResolution::Conflicted {
            supporting,
            contradicting,
            ..
        } => {
            assert_eq!(supporting, vec![affirm]);
            assert_eq!(contradicting, vec![deny]);
        }
        other => panic!("expected Conflicted, got {other:?}"),
    }
}

#[test]
fn acceptance_no_latest_wins() {
    // REQ-A4S0-011: insertion order cannot reach the resolution.
    let mut forward = ObservationSet::new();
    observe(
        &mut forward,
        ObservationStance::Affirms,
        ObservationOrigin::StaticProvider,
        "e1",
    );
    observe(
        &mut forward,
        ObservationStance::Denies,
        ObservationOrigin::RuntimeProvider,
        "e2",
    );

    let mut reverse = ObservationSet::new();
    observe(
        &mut reverse,
        ObservationStance::Denies,
        ObservationOrigin::RuntimeProvider,
        "e2",
    );
    observe(
        &mut reverse,
        ObservationStance::Affirms,
        ObservationOrigin::StaticProvider,
        "e1",
    );

    let rel = relation().id();
    assert_eq!(
        resolve_relation(&forward, &rel).canonical_tag(),
        resolve_relation(&reverse, &rel).canonical_tag()
    );
    // And the sets themselves are canonically ordered, so they compare equal.
    assert_eq!(forward.observations(), reverse.observations());
}

#[test]
fn acceptance_resolution_is_closed_and_score_free() {
    // REQ-A4S0-012
    let mut set = ObservationSet::new();
    observe(
        &mut set,
        ObservationStance::Affirms,
        ObservationOrigin::DeterministicLocal,
        "e1",
    );
    let r = resolve_relation(&set, &relation().id());
    assert_eq!(r.canonical_tag(), "supported");
    assert!(r.is_covered());

    // Only denies => contradicted.
    let mut denies = ObservationSet::new();
    observe(
        &mut denies,
        ObservationStance::Denies,
        ObservationOrigin::RuntimeProvider,
        "e1",
    );
    assert_eq!(
        resolve_relation(&denies, &relation().id()).canonical_tag(),
        "contradicted"
    );

    // No confidence number anywhere in the type.
    let json = serde_json::to_string(&r).expect("serde");
    for forbidden in ["confidence", "score", "0."] {
        assert!(!json.contains(forbidden), "{json}");
    }
}

#[test]
fn acceptance_missing_relation_stays_unknown() {
    // REQ-A4S0-013: an uncovered relation is an explicit gap, not an invented one.
    let empty = ObservationSet::new();
    let r = resolve_relation(&empty, &relation().id());
    match &r {
        EvidenceResolution::Insufficient { gap, .. } => {
            assert!(gap.contains("no observation covers"), "{gap}");
        }
        other => panic!("expected Insufficient, got {other:?}"),
    }
    assert!(!r.is_covered());
}

// ── Graph ───────────────────────────────────────────────────────────────────

fn populated() -> ObservationSet {
    let mut set = ObservationSet::new();
    observe(
        &mut set,
        ObservationStance::Affirms,
        ObservationOrigin::StaticProvider,
        "e1",
    );
    observe(
        &mut set,
        ObservationStance::Denies,
        ObservationOrigin::RuntimeProvider,
        "e2",
    );
    set
}

#[test]
fn acceptance_observations_project_into_the_one_projection() {
    // REQ-A4S0-014: the one canonical projection type, the OBSERVES edge, no
    // second graph.
    let set = populated();
    let mut graph = InMemorySemanticGraph::new();
    let ids = project_into(&set, &mut graph).expect("project");
    assert_eq!(ids.len(), 2);

    let kinds: Vec<String> = graph
        .nodes()
        .iter()
        .map(|n| match &n.kind {
            crate::semantic_kind::NodeKind::Extension(k) => k.as_str().to_string(),
            other => format!("{other:?}"),
        })
        .collect();
    assert!(
        kinds
            .iter()
            .all(|k| k == OBSERVATION_NODE_KIND || k == RELATION_NODE_KIND)
    );
    // One relation node (both observations observe the same relation).
    assert_eq!(kinds.iter().filter(|k| *k == RELATION_NODE_KIND).count(), 1);

    let observes = crate::semantic_kind::RelationKind::parse(OBSERVES_RELATION_KIND).expect("tag");
    let edges: Vec<_> = graph
        .relations()
        .into_iter()
        .filter(|r| r.kind == observes)
        .collect();
    assert_eq!(edges.len(), 2, "each observation OBSERVES the relation");
}

#[test]
fn acceptance_rebuild_preserves_observations() {
    // REQ-A4S0-015: two projections over an equal set are byte-identical.
    let set = populated();
    let mut a = InMemorySemanticGraph::new();
    let mut b = InMemorySemanticGraph::new();
    project_into(&set, &mut a).expect("project");
    project_into(&set, &mut b).expect("project");
    assert_eq!(a.canonical_bytes(), b.canonical_bytes());
}

#[test]
fn acceptance_read_paths_do_not_write() {
    // REQ-A4S0-017: resolving and projecting mutate neither the set nor anything
    // canonical beyond the projection the caller passed in.
    let set = populated();
    let before = set.clone();
    let _ = resolve_relation(&set, &relation().id());
    let mut graph = InMemorySemanticGraph::new();
    let _ = project_into(&set, &mut graph).expect("project");
    assert_eq!(set, before, "resolution is pure over an immutable set");
}

// ── Derivation ──────────────────────────────────────────────────────────────

#[test]
fn acceptance_derivation_is_deterministic() {
    // REQ-A4S0-016
    let a = populated();
    let b = populated();
    assert_eq!(a, b);
    assert_eq!(
        serde_json::to_string(&a).expect("serde"),
        serde_json::to_string(&b).expect("serde")
    );
}

#[test]
fn acceptance_serde_is_deterministic() {
    // REQ-A4S0-016: serialisation is a function of the value, not of insertion
    // order, so two equal sets serialise byte-identically.
    let a = populated();
    let b = populated();
    assert_eq!(
        serde_json::to_string(&a).expect("serialise"),
        serde_json::to_string(&b).expect("serialise")
    );
    // The envelope carries the pieces a consumer needs to reconstruct provenance.
    let json = serde_json::to_string(&a).expect("serialise");
    for key in [
        "id", "subject", "stance", "evidence", "origin", "basis", "producer",
    ] {
        assert!(
            json.contains(key),
            "serialised observation missing `{key}`: {json}"
        );
    }
}

// ── Boundary ────────────────────────────────────────────────────────────────

#[test]
fn acceptance_no_provider_or_alignment_dependency() {
    // REQ-A4S0-018: the substrate is neutral to its future consumers.
    let mut sources = String::new();
    for f in ["mod.rs", "types.rs", "resolution.rs", "project.rs"] {
        sources.push_str(
            &std::fs::read_to_string(format!(
                "{}/src/observation/{f}",
                env!("CARGO_MANIFEST_DIR")
            ))
            .expect("source"),
        );
    }
    for forbidden in [
        "software_alignment",
        "AlignmentAssessment",
        "cognicode",
        "chronos",
        "authority_engine",
        "AlignmentLens",
    ] {
        assert!(
            !sources.contains(forbidden),
            "the observation substrate must not depend on `{forbidden}`"
        );
    }
    // And it evaluates nothing.
    for forbidden in [
        "ContractEvaluation",
        "compute_conformance_delta",
        "run_debverify_audit",
    ] {
        assert!(
            !sources.contains(forbidden),
            "the substrate must not evaluate: {forbidden}"
        );
    }
}

#[test]
fn acceptance_relation_id_domain_is_pinned() {
    // The domain prefix is part of the identity contract.
    assert_eq!(RelationId::DOMAIN, "sddk.software_relation.id.v1|");
    assert_eq!(ObservationId::DOMAIN, "sddk.software_observation.id.v1|");
}
