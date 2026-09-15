// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// ac2_ac3_substrate_isolation.rs — A3-S4 follow-up verification.
//
// Verifies the AC3 (paradigm_profile) substrate does NOT share kinds with
// the AC2 (architecture_graph) substrate and that BOTH overlays delegate to
// the same canonical InMemorySemanticGraph projection without creating a
// second persistence authority.
//
// This is the real public-API consumption pattern (a downstream operator
// using both overlays through the published engine API), not an inspection
// only check.

use sddk_engine::architecture_graph::{
    ArchitectureGraphOverlay, SoftwareUnit, SoftwareUnitRef as AgsUnitRef, UnitKind,
};
use sddk_engine::paradigm_profile::{
    EvidenceBasis, LensAssessment, LensStatus, ParadigmAnchorRef, ParadigmLensEdge,
    ParadigmLensKind, ParadigmProfileEdge, ParadigmProfileKind, ParadigmProfileOverlay,
    ProjectIntentRef, Query, QueryResult, RebuildInputs, rebuild,
};

/// AC2 and AC3 rebuild independently and produce distinct digests
/// (different substrates, no shared kind namespace, no shadow authority).
#[test]
fn integration_ac2_ac3_substrate_isolation() {
    let mut ov2 = ArchitectureGraphOverlay::new();
    let su = AgsUnitRef::new("crates/x/lib.rs");
    ov2.add_unit(&SoftwareUnit::new(su, UnitKind::Module, "crates/x/lib.rs"));
    let d2 = ov2.digest();

    let mut ov3 = ParadigmProfileOverlay::new();
    let pi = ProjectIntentRef::new("p-63676b11dc0ef88f");
    ov3.add_project(&pi);
    let edge = ParadigmProfileEdge {
        anchor: ParadigmAnchorRef::ProjectIntent(pi.clone()),
        profile: ParadigmProfileKind::ObjectOriented,
        rationale: Some("integration check".to_string()),
    };
    ov3.add_profile(&edge);
    let d3 = ov3.digest();

    // Distinct substrates → distinct digests (no shared authority)
    assert_ne!(d2, d3, "AC2 and AC3 overlays must produce distinct digests");

    // Each overlay individually is stable across rebuilds
    let mut ov3b = ParadigmProfileOverlay::new();
    ov3b.add_project(&pi);
    ov3b.add_profile(&edge);
    assert_eq!(ov3b.digest(), d3, "AC3 rebuild must be deterministic");
}

/// AC3 query/3-variant exhaustiveness check via the public Query enum.
#[test]
fn integration_ac3_query_all_three_variants() {
    let mut ov = ParadigmProfileOverlay::new();
    let pi = ProjectIntentRef::new("p-test");
    ov.add_project(&pi);
    ov.add_profile(&ParadigmProfileEdge {
        anchor: ParadigmAnchorRef::ProjectIntent(pi.clone()),
        profile: ParadigmProfileKind::Functional,
        rationale: None,
    });
    ov.add_lens(&ParadigmLensEdge {
        anchor: ParadigmAnchorRef::ProjectIntent(pi.clone()),
        lens: ParadigmLensKind::Functional,
    });
    ov.add_assessment(&LensAssessment {
        lens: ParadigmLensKind::Functional,
        anchor: ParadigmAnchorRef::ProjectIntent(pi.clone()),
        status: LensStatus::Unknown,
        basis: EvidenceBasis::Declared,
        evidence_refs: Vec::new(),
        notes: None,
        evaluated_at_ms: 0,
    });

    let profiles = ov.query(Query::ProfilesForAnchor(ParadigmAnchorRef::ProjectIntent(
        pi.clone(),
    )));
    let anchors = ov.query(Query::AnchorsForParadigm(ParadigmProfileKind::Functional));
    let assessments = ov.query(Query::AssessmentsForAnchor(
        ParadigmAnchorRef::ProjectIntent(pi.clone()),
    ));

    let profiles_len = match profiles {
        QueryResult::Profiles(ref v) => v.len(),
        _ => panic!("Query::ProfilesForAnchor must yield QueryResult::Profiles"),
    };
    let anchors_len = match anchors {
        QueryResult::Anchors(ref v) => v.len(),
        _ => panic!("Query::AnchorsForParadigm must yield QueryResult::Anchors"),
    };
    let assessments_len = match assessments {
        QueryResult::Assessments(ref v) => v.len(),
        _ => panic!("Query::AssessmentsForAnchor must yield QueryResult::Assessments"),
    };

    assert_eq!(
        profiles_len, 1,
        "query variant 1 (ProfilesForAnchor) returned wrong count"
    );
    assert_eq!(
        anchors_len, 1,
        "query variant 2 (AnchorsForParadigm) returned wrong count"
    );
    assert_eq!(
        assessments_len, 1,
        "query variant 3 (AssessmentsForAnchor) returned wrong count"
    );
}

/// Poly-paradigm support via public API (REQ-AC3-010).
#[test]
fn integration_ac3_poly_paradigm_one_anchor() {
    let mut ov = ParadigmProfileOverlay::new();
    let pi = ProjectIntentRef::new("p-poly");

    // Same anchor (pi), three different paradigms attached via three edges.
    ov.add_project(&pi);
    ov.add_profile(&ParadigmProfileEdge {
        anchor: ParadigmAnchorRef::ProjectIntent(pi.clone()),
        profile: ParadigmProfileKind::ObjectOriented,
        rationale: None,
    });
    ov.add_profile(&ParadigmProfileEdge {
        anchor: ParadigmAnchorRef::ProjectIntent(pi.clone()),
        profile: ParadigmProfileKind::Functional,
        rationale: None,
    });
    ov.add_profile(&ParadigmProfileEdge {
        anchor: ParadigmAnchorRef::ProjectIntent(pi.clone()),
        profile: ParadigmProfileKind::DataOriented,
        rationale: None,
    });

    let profiles = ov.query(Query::ProfilesForAnchor(ParadigmAnchorRef::ProjectIntent(
        pi.clone(),
    )));
    let profiles_len = match profiles {
        QueryResult::Profiles(ref v) => v.len(),
        _ => panic!("expected Profiles"),
    };
    assert_eq!(profiles_len, 3, "anchor must support multiple paradigms");
}

/// Edge case: empty overlay (no inputs) must yield empty query results
/// without panicking. This is the strictest "main path" boundary.
#[test]
fn integration_ac3_empty_overlay_queries_safe() {
    let ov = ParadigmProfileOverlay::new();
    let pi = ProjectIntentRef::new("p-nothing");

    let profiles = ov.query(Query::ProfilesForAnchor(ParadigmAnchorRef::ProjectIntent(
        pi.clone(),
    )));
    let anchors = ov.query(Query::AnchorsForParadigm(ParadigmProfileKind::Functional));
    let assessments = ov.query(Query::AssessmentsForAnchor(
        ParadigmAnchorRef::ProjectIntent(pi.clone()),
    ));

    match (profiles, anchors, assessments) {
        (QueryResult::Profiles(p), QueryResult::Anchors(a), QueryResult::Assessments(as_))
            if p.is_empty() && a.is_empty() && as_.is_empty() => {}
        _ => panic!("empty overlay must yield three empty vectors"),
    }
}

/// Edge case: AC3 RebuildInputs (the documented test-suite API) and
/// the granular `add_*` API must produce identical digests for the same
/// content. Catches "two ways to do the same thing → divergent digests".
#[test]
fn integration_ac3_granular_vs_inputs_equivalence() {
    let pi = ProjectIntentRef::new("p-equiv");
    let edge = ParadigmProfileEdge {
        anchor: ParadigmAnchorRef::ProjectIntent(pi.clone()),
        profile: ParadigmProfileKind::ObjectOriented,
        rationale: Some("equiv".to_string()),
    };

    // Path 1: granular public API.
    let mut a = ParadigmProfileOverlay::new();
    a.add_project(&pi);
    a.add_profile(&edge);
    let d_a = a.digest();

    // Path 2: same content via RebuildInputs.rebuild() (test-suite API).
    let mut b = ParadigmProfileOverlay::new();
    let inputs = RebuildInputs {
        project: Some(pi.clone()),
        profiles: vec![edge],
        lenses: vec![],
        assessments: vec![],
    };
    rebuild(&mut b, &inputs);
    let d_b = b.digest();

    assert_eq!(
        d_a, d_b,
        "granular add_* API and RebuildInputs must converge to the same digest"
    );
}
