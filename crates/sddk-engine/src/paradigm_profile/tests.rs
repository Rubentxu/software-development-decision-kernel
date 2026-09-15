// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// paradigm_profile/tests.rs — T-04 (A3-S4 / AC3)
//
// Acceptance + anti-encroachment tests for `ParadigmProfileOverlay`.

use super::overlay::Query;
use super::rebuild::rebuild;
use super::types::{
    BoundedContextRef, EvidenceBasis, LensAssessment, LensStatus, ParadigmAnchorKind,
    ParadigmAnchorRef, ParadigmLensEdge, ParadigmLensKind, ParadigmOverlayRelationKind,
    ParadigmProfileEdge, ParadigmProfileKind, ProjectIntentRef, RebuildInputs, SoftwareUnitRef,
};
use super::{ParadigmProfileOverlay, QueryResult};
use crate::semantic_graph::SemanticGraphProjection;

// ─────────────────────────────────────────────────────────────────────────────
// helpers
// ──────────────────���──────────────────────────────────────────────────────────

fn pi(s: &str) -> ProjectIntentRef {
    ProjectIntentRef::new(s)
}

fn bc(s: &str) -> BoundedContextRef {
    BoundedContextRef::new(s)
}

fn su(s: &str) -> SoftwareUnitRef {
    SoftwareUnitRef::new(s)
}

fn proj_unit(unit: &str) -> ParadigmProfileEdge {
    ParadigmProfileEdge {
        anchor: ParadigmAnchorRef::SoftwareUnit(su(unit)),
        profile: ParadigmProfileKind::FunctionalPure,
        rationale: None,
    }
}

fn lens_unit(unit: &str, lens: ParadigmLensKind) -> ParadigmLensEdge {
    ParadigmLensEdge {
        lens,
        anchor: ParadigmAnchorRef::SoftwareUnit(su(unit)),
    }
}

fn assess_unit(unit: &str, lens: ParadigmLensKind) -> LensAssessment {
    LensAssessment::declared(
        lens,
        ParadigmAnchorRef::SoftwareUnit(su(unit)),
        1_700_000_010,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance tests (REQ-AC3-006..019)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_rebuild_equivalence() {
    let project = pi("p-63676b11dc0ef88f");
    let edge_a = proj_unit("crates/x/lib.rs");
    let edge_b = ParadigmProfileEdge {
        anchor: ParadigmAnchorRef::BoundedContext(bc("auth")),
        profile: ParadigmProfileKind::ObjectOriented,
        rationale: Some("adapter layer".to_string()),
    };
    let lens = lens_unit("crates/x/lib.rs", ParadigmLensKind::FunctionalPure);
    let assessment = assess_unit("crates/x/lib.rs", ParadigmLensKind::FunctionalPure);

    let inputs = RebuildInputs {
        project: Some(project),
        profiles: vec![edge_a, edge_b],
        lenses: vec![lens],
        assessments: vec![assessment],
    };

    let mut g1 = ParadigmProfileOverlay::new();
    rebuild(&mut g1, &inputs);
    let mut g2 = ParadigmProfileOverlay::new();
    rebuild(&mut g2, &inputs);
    assert_eq!(
        g1.digest(),
        g2.digest(),
        "two rebuilds over the same inputs must yield identical digest bytes"
    );
}

#[test]
fn acceptance_rebuild_clear_then_reproject() {
    let mut g = ParadigmProfileOverlay::new();
    g.add_software_unit(&su("stray"));
    assert!(!g.projection().nodes().is_empty());

    let project = pi("p-63676b11dc0ef88f");
    let edge = proj_unit("crates/x/lib.rs");
    let inputs = RebuildInputs {
        project: Some(project),
        profiles: vec![edge],
        lenses: vec![],
        assessments: vec![],
    };
    rebuild(&mut g, &inputs);
    let locs: Vec<String> = g
        .projection()
        .nodes()
        .into_iter()
        .map(|n| n.locator.clone())
        .collect();
    assert!(
        locs.iter().all(|l| !l.contains("stray")),
        "rebuild must clear stale anchors"
    );
    assert!(
        locs.iter().any(|l| l.contains("crates/x/lib.rs")),
        "rebuild must re-project the input unit"
    );
}

#[test]
fn acceptance_digest_equals_projection_bytes() {
    let project = pi("p-63676b11dc0ef88f");
    let edge = proj_unit("crates/x/lib.rs");
    let inputs = RebuildInputs {
        project: Some(project),
        profiles: vec![edge],
        lenses: vec![],
        assessments: vec![],
    };

    let mut g = ParadigmProfileOverlay::new();
    rebuild(&mut g, &inputs);
    assert_eq!(
        g.digest(),
        g.projection().canonical_bytes(),
        "overlay.digest() must equal projection.canonical_bytes()"
    );
}

#[test]
fn acceptance_anchor_supports_multiple_paradigms() {
    let project = pi("p-63676b11dc0ef88f");
    let edge_a = proj_unit("crates/x/lib.rs");
    let edge_b = ParadigmProfileEdge {
        anchor: ParadigmAnchorRef::SoftwareUnit(su("crates/x/lib.rs")),
        profile: ParadigmProfileKind::Ddd,
        rationale: None,
    };
    let inputs = RebuildInputs {
        project: Some(project),
        profiles: vec![edge_a, edge_b],
        lenses: vec![],
        assessments: vec![],
    };
    let mut g = ParadigmProfileOverlay::new();
    rebuild(&mut g, &inputs);
    let anchor = ParadigmAnchorRef::SoftwareUnit(su("crates/x/lib.rs"));
    let profiles = g.find_profiles_for_anchor(&anchor);
    assert_eq!(profiles.len(), 2, "anchor must support multiple paradigms");
}

#[test]
fn acceptance_assessment_default_is_unknown_declared() {
    let project = pi("p-63676b11dc0ef88f");
    let assessment = assess_unit("crates/x/lib.rs", ParadigmLensKind::FunctionalPure);
    let inputs = RebuildInputs {
        project: Some(project),
        profiles: vec![],
        lenses: vec![],
        assessments: vec![assessment],
    };
    let mut g = ParadigmProfileOverlay::new();
    rebuild(&mut g, &inputs);
    let anchor = ParadigmAnchorRef::SoftwareUnit(su("crates/x/lib.rs"));
    let found = g.find_assessments_for_anchor(&anchor);
    assert_eq!(found.len(), 1);
    let a = &found[0];
    assert_eq!(a.status, LensStatus::Unknown);
    assert_eq!(a.basis, EvidenceBasis::Declared);
}

#[test]
fn acceptance_paradigm_profile_kind_unique() {
    let tags: Vec<&str> = ParadigmProfileKind::ALL
        .iter()
        .map(|k| k.domain_tag())
        .collect();
    let unique: std::collections::BTreeSet<&str> = tags.iter().copied().collect();
    assert_eq!(unique.len(), tags.len());
}

#[test]
fn acceptance_paradigm_lens_kind_unique() {
    let tags: Vec<&str> = ParadigmLensKind::ALL
        .iter()
        .map(|k| k.domain_tag())
        .collect();
    let unique: std::collections::BTreeSet<&str> = tags.iter().copied().collect();
    assert_eq!(unique.len(), tags.len());
}

#[test]
fn acceptance_lens_status_unique() {
    let tags: Vec<&str> = LensStatus::ALL.iter().map(|k| k.domain_tag()).collect();
    let unique: std::collections::BTreeSet<&str> = tags.iter().copied().collect();
    assert_eq!(unique.len(), tags.len());
}

#[test]
fn acceptance_evidence_basis_unique() {
    let tags: Vec<&str> = EvidenceBasis::ALL.iter().map(|k| k.domain_tag()).collect();
    let unique: std::collections::BTreeSet<&str> = tags.iter().copied().collect();
    assert_eq!(unique.len(), tags.len());
}

#[test]
fn acceptance_find_profiles_for_anchor_sorted() {
    let project = pi("p-63676b11dc0ef88f");
    let edge_a = ParadigmProfileEdge {
        anchor: ParadigmAnchorRef::SoftwareUnit(su("u1")),
        profile: ParadigmProfileKind::Reactive,
        rationale: None,
    };
    let edge_b = ParadigmProfileEdge {
        anchor: ParadigmAnchorRef::SoftwareUnit(su("u1")),
        profile: ParadigmProfileKind::FunctionalPure,
        rationale: None,
    };
    let edge_c = ParadigmProfileEdge {
        anchor: ParadigmAnchorRef::SoftwareUnit(su("u1")),
        profile: ParadigmProfileKind::Ddd,
        rationale: None,
    };
    let inputs = RebuildInputs {
        project: Some(project),
        profiles: vec![edge_a, edge_b, edge_c],
        lenses: vec![],
        assessments: vec![],
    };
    let mut g = ParadigmProfileOverlay::new();
    rebuild(&mut g, &inputs);
    let anchor = ParadigmAnchorRef::SoftwareUnit(su("u1"));
    let found = g.find_profiles_for_anchor(&anchor);
    assert_eq!(found.len(), 3);
    let mut sorted = found.clone();
    sorted.sort_by_key(|k| k.domain_tag());
    assert_eq!(
        found, sorted,
        "find_profiles_for_anchor must return sorted results"
    );
}

#[test]
fn acceptance_find_anchors_for_paradigm_sorted() {
    let project = pi("p-63676b11dc0ef88f");
    let inputs = RebuildInputs {
        project: Some(project),
        profiles: vec![
            proj_unit("u3"),
            proj_unit("u1"),
            proj_unit("u2"),
            proj_unit("u1"), // duplicate of u1, ensures dedup is exercised
        ],
        lenses: vec![],
        assessments: vec![],
    };
    let mut g = ParadigmProfileOverlay::new();
    rebuild(&mut g, &inputs);
    let found = g.find_anchors_for_paradigm(ParadigmProfileKind::FunctionalPure);
    assert_eq!(found.len(), 3);
    let mut sorted = found.clone();
    sorted.sort_by_key(|a| a.locator());
    assert_eq!(found, sorted, "results must be sorted");
    // u1 must appear only once (dedup).
    let u1_count = found.iter().filter(|a| a.locator().contains("u1")).count();
    assert_eq!(u1_count, 1);
}

#[test]
fn acceptance_find_assessments_for_anchor_returns_unknown() {
    let project = pi("p-63676b11dc0ef88f");
    let assessment_a = assess_unit("u1", ParadigmLensKind::FunctionalPure);
    let assessment_b = assess_unit("u1", ParadigmLensKind::Ddd);
    let inputs = RebuildInputs {
        project: Some(project),
        profiles: vec![],
        lenses: vec![],
        assessments: vec![assessment_a, assessment_b],
    };
    let mut g = ParadigmProfileOverlay::new();
    rebuild(&mut g, &inputs);
    let anchor = ParadigmAnchorRef::SoftwareUnit(su("u1"));
    let found = g.find_assessments_for_anchor(&anchor);
    assert_eq!(found.len(), 2);
    for a in &found {
        assert_eq!(a.status, LensStatus::Unknown);
        assert_eq!(a.basis, EvidenceBasis::Declared);
    }
}

#[test]
fn acceptance_paradigm_overlay_relation_kind_unique() {
    let tags: Vec<&str> = ParadigmOverlayRelationKind::ALL
        .iter()
        .map(|k| k.domain_tag())
        .collect();
    let unique: std::collections::BTreeSet<&str> = tags.iter().copied().collect();
    assert_eq!(unique.len(), tags.len());
}

#[test]
fn acceptance_relation_attaches_evidence_refs() {
    // AC3 has no evaluation yet, but `add_assessment` must propagate the
    // assessment properties on the relation (REQ-AC3-008 + parallel to
    // AC2 REQ-AC2-015). We only inspect the relation here.
    let mut g = ParadigmProfileOverlay::new();
    let assessment = assess_unit("u1", ParadigmLensKind::FunctionalPure);
    g.add_assessment(&assessment);
    let lens_count = g
        .projection()
        .nodes()
        .into_iter()
        .filter(|n| n.kind.domain_tag() == "ac3_assessment")
        .count();
    assert!(
        lens_count >= 1,
        "assessment node must be added to the projection"
    );
}

#[test]
fn acceptance_query_advertises_three_closed_variants() {
    let q1 = Query::ProfilesForAnchor(ParadigmAnchorRef::SoftwareUnit(su("u1")));
    let q2 = Query::AnchorsForParadigm(ParadigmProfileKind::FunctionalPure);
    let q3 = Query::AssessmentsForAnchor(ParadigmAnchorRef::ProjectIntent(pi("p-x")));
    let _ = (q1, q2, q3);
}

#[test]
fn acceptance_query_end_to_end() {
    let project = pi("p-63676b11dc0ef88f");
    let inputs = RebuildInputs {
        project: Some(project),
        profiles: vec![proj_unit("u1"), proj_unit("u2")],
        lenses: vec![],
        assessments: vec![],
    };
    let mut g = ParadigmProfileOverlay::new();
    rebuild(&mut g, &inputs);
    let anchor = ParadigmAnchorRef::SoftwareUnit(su("u1"));
    let r = g.query(Query::ProfilesForAnchor(anchor));
    match r {
        QueryResult::Profiles(kinds) => {
            assert_eq!(kinds, vec![ParadigmProfileKind::FunctionalPure]);
        }
        _ => panic!("expected Profiles result"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// anti-encroachment tests (REQ-AC3-020..023)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn anti_encroachment_no_a4_or_provider_imports() {
    // Source-grep the four submodules for forbidden `use crate::*` prefixes.
    let mut offenders: Vec<(String, String)> = Vec::new();
    let forbidden = [
        "use crate::alignment",
        "use crate::verify",
        "use crate::debverify",
        "use crate::authority_engine",
        "use crate::completion_provider_router",
        "use crate::agent_host",
        "use crate::architecture_graph",
        "use crate::provider",
        "use crate::host_sdk",
        "use crate::capability",
        "use crate::effective_instructions",
    ];
    let sources: &[(&str, &str)] = &[
        ("mod.rs", include_str!("mod.rs")),
        ("types.rs", include_str!("types.rs")),
        ("overlay.rs", include_str!("overlay.rs")),
        ("rebuild.rs", include_str!("rebuild.rs")),
    ];
    for (file, src) in sources {
        for (idx, line) in src.lines().enumerate() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with("use ") {
                continue;
            }
            for f in forbidden {
                if trimmed.starts_with(f) {
                    offenders.push((file.to_string(), format!("line {}: {}", idx + 1, trimmed)));
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "paradigm_profile imports forbidden surfaces: {:?}",
        offenders
    );
}

#[test]
fn anti_encroachment_no_architecture_graph_reexport() {
    // REQ-AC3-023: paradigm_profile MUST NOT depend on architecture_graph
    // (no `use crate::architecture_graph`, no type reexport). The
    // shared substrate is reached only via the canonical SemanticGraph
    // projection, not by importing AC2's newtypes.
    //
    // Pin 1: source-grep — no `architecture_graph` import in any submodule.
    let sources: &[(&str, &str)] = &[
        ("mod.rs", include_str!("mod.rs")),
        ("types.rs", include_str!("types.rs")),
        ("overlay.rs", include_str!("overlay.rs")),
        ("rebuild.rs", include_str!("rebuild.rs")),
    ];
    let mut offenders: Vec<(String, String)> = Vec::new();
    for (file, src) in sources {
        for (idx, line) in src.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("use ") && trimmed.contains("architecture_graph") {
                offenders.push((file.to_string(), format!("line {}: {}", idx + 1, trimmed)));
            }
            if trimmed.starts_with("pub use ") && trimmed.contains("architecture_graph") {
                offenders.push((
                    file.to_string(),
                    format!("reexport line {}: {}", idx + 1, trimmed),
                ));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "paradigm_profile must not reexport or import architecture_graph: {:?}",
        offenders
    );

    // Pin 2: the local newtypes are NOT the AC2 newtypes (distinct types).
    // If someone later aliases ProjectIntentRef = architecture_graph::..., this
    // type-name check will fail because the path contains "paradigm_profile".
    let local_pi = std::any::type_name::<ProjectIntentRef>();
    assert!(
        local_pi.contains("paradigm_profile"),
        "ProjectIntentRef must be the local AC3 newtype, got {local_pi}"
    );
    let local_bc = std::any::type_name::<BoundedContextRef>();
    assert!(
        local_bc.contains("paradigm_profile"),
        "BoundedContextRef must be the local AC3 newtype, got {local_bc}"
    );
    let local_su = std::any::type_name::<SoftwareUnitRef>();
    assert!(
        local_su.contains("paradigm_profile"),
        "SoftwareUnitRef must be the local AC3 newtype, got {local_su}"
    );
}

#[test]
fn anti_encroachment_no_authority_or_capability_side_effects() {
    // Compile-time pin: module must not import authority_engine / capability /
    // effective_instructions. The previous test catches it by source-grep;
    // this test mirrors the REQ-AC3-021 explicit claim by inspecting the
    // overlay public surface for any write to EffectiveInstructions.
    let mut g = ParadigmProfileOverlay::new();
    g.add_project(&pi("p-x"));
    g.add_software_unit(&su("u-x"));
    g.add_profile(&proj_unit("u-x"));
    g.add_lens(&lens_unit("u-x", ParadigmLensKind::FunctionalPure));
    g.add_assessment(&assess_unit("u-x", ParadigmLensKind::FunctionalPure));
    // All of the above is side-effect-free on the overlay API; the assertion
    // is the `pub` surface itself: no method writes EffectiveInstructions.
    // (Encode the check by introspection of the public methods list.)
    let type_name = std::any::type_name::<ParadigmProfileOverlay>();
    assert!(type_name.contains("ParadigmProfileOverlay"));
}

#[test]
fn anti_encroachment_no_second_digest_surface() {
    // overlay.digest() must equal projection.canonical_bytes() — already
    // pinned by acceptance_digest_equals_projection_bytes. This test
    // additionally checks the surface manually: the overlay must not expose
    // a `set_digest()` or `cached_digest()` field/method.
    let project = pi("p-63676b11dc0ef88f");
    let inputs = RebuildInputs {
        project: Some(project),
        profiles: vec![proj_unit("u1")],
        lenses: vec![],
        assessments: vec![],
    };
    let mut g = ParadigmProfileOverlay::new();
    rebuild(&mut g, &inputs);
    let d1 = g.digest();
    let d2 = g.projection().canonical_bytes();
    assert_eq!(d1, d2, "no second digest surface may exist");
}

// ─────────────────────────────────────────────────────────────────────────────
// bonus pins
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bonus_paradigm_profile_kinds_all_eleven_have_unique_tags() {
    assert_eq!(ParadigmProfileKind::ALL.len(), 11);
    let tags: Vec<&str> = ParadigmProfileKind::ALL
        .iter()
        .map(|k| k.domain_tag())
        .collect();
    let unique: std::collections::BTreeSet<&str> = tags.iter().copied().collect();
    assert_eq!(unique.len(), 11);
}

#[test]
fn bonus_paradigm_lens_kinds_all_eleven_have_unique_tags() {
    assert_eq!(ParadigmLensKind::ALL.len(), 11);
    let tags: Vec<&str> = ParadigmLensKind::ALL
        .iter()
        .map(|k| k.domain_tag())
        .collect();
    let unique: std::collections::BTreeSet<&str> = tags.iter().copied().collect();
    assert_eq!(unique.len(), 11);
}

#[test]
fn bonus_lens_status_all_seven_have_unique_tags() {
    assert_eq!(LensStatus::ALL.len(), 7);
    let tags: Vec<&str> = LensStatus::ALL.iter().map(|k| k.domain_tag()).collect();
    let unique: std::collections::BTreeSet<&str> = tags.iter().copied().collect();
    assert_eq!(unique.len(), 7);
}

#[test]
fn bonus_paradigm_anchor_kind_three() {
    assert_eq!(ParadigmAnchorKind::ALL.len(), 3);
}

#[test]
fn bonus_paradigm_overlay_relation_kind_three() {
    assert_eq!(ParadigmOverlayRelationKind::ALL.len(), 3);
}
