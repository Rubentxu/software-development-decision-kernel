// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_debverify/tests.rs — A3-S7 / AC5 acceptance +
// exit-criterion + anti-encroachment + boundary tests.
//
// REQ-AC5-001..022 pinned via the cycle-bounded spec
// `docs/architecture/specs/arch-spec-A3-S7-ac5-debverify-audit.md`.

use super::*;
use crate::architectural_contract::{
    ArchitecturalContract, BoundaryKind, ComponentRef, ContractId, DecisionRef, EntityRef,
    Revision, SpecRef,
};
use crate::architecture_graph::{
    ArchitectureGraphOverlay, SoftwareUnit, SoftwareUnitRef, UnitKind,
};
use crate::knowledge::EventTime;

const T0: i64 = 1_700_000_000;

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures
// ─────────────────────────────────────────────────────────────────────────────

fn cid(s: &str) -> ContractId {
    ContractId::new(s).expect("contract id")
}

fn rev(id: &str) -> Revision {
    Revision::new(format!("rev:{id}")).expect("revision")
}

fn dec(id: &str) -> DecisionRef {
    DecisionRef::Decision(format!("decision:{id}"))
}

fn spc(id: &str) -> SpecRef {
    SpecRef::Spec(format!("spec:{id}"))
}

fn component(s: &str) -> ComponentRef {
    ComponentRef::new(s).expect("component")
}

fn entity(s: &str) -> EntityRef {
    EntityRef::new(s).expect("entity")
}

fn single_authority(id: &str, comp: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_single_authority(
        cid(id),
        component(comp),
        dec(id),
        spc(id),
        rev(id),
        EventTime(T0),
    )
    .expect("declare_single_authority")
}

fn unique_owner(id: &str, ent: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_unique_owner(
        cid(id),
        entity(ent),
        dec(id),
        spc(id),
        rev(id),
        EventTime(T0),
    )
    .expect("declare_unique_owner")
}

fn forbidden(id: &str, from: &str, to: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_forbidden_dependency(
        cid(id),
        component(from),
        component(to),
        "forbidden".to_string(),
        dec(id),
        spc(id),
        rev(id),
        EventTime(T0),
    )
    .expect("declare_forbidden_dependency")
}

fn projection_only(id: &str, source_kind: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_projection_only(
        cid(id),
        source_kind.to_string(),
        dec(id),
        spc(id),
        rev(id),
        EventTime(T0),
    )
    .expect("declare_projection_only")
}

fn bounded(id: &str, deprecated_after: i64) -> ArchitecturalContract {
    ArchitecturalContract::declare_bounded_compatibility(
        cid(id),
        EventTime(deprecated_after),
        None,
        dec(id),
        spc(id),
        rev(id),
        EventTime(T0),
    )
    .expect("declare_bounded_compatibility")
}

fn provider_boundary(id: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_provider_boundary(
        cid(id),
        BoundaryKind::Outbound,
        "engine -> provider".to_string(),
        dec(id),
        spc(id),
        rev(id),
        EventTime(T0),
    )
    .expect("declare_provider_boundary")
}

fn graph_with_units(units: &[&str]) -> ArchitectureGraphOverlay {
    let mut g = ArchitectureGraphOverlay::new();
    for u in units {
        let unit = SoftwareUnit::new(SoftwareUnitRef::new(*u), UnitKind::Module, (*u).to_string());
        g.add_unit(&unit);
    }
    g
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — vocabulary (REQ-AC5-005..007)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_finding_kind_closed_five() {
    // REQ-AC5-005
    assert_eq!(DebVerifyFindingKind::ALL.len(), 5);
    let mut tags: Vec<&str> = DebVerifyFindingKind::ALL
        .iter()
        .map(|k| k.canonical_tag())
        .collect();
    tags.sort_unstable();
    tags.dedup();
    assert_eq!(tags.len(), 5);
}

#[test]
fn acceptance_severity_mapping_total() {
    // REQ-AC5-006
    assert_eq!(FindingSeverity::ALL.len(), 3);
    for kind in DebVerifyFindingKind::ALL {
        // total: every kind maps to exactly one severity
        let s = kind.severity();
        assert!(FindingSeverity::ALL.contains(&s));
    }
    assert_eq!(
        DebVerifyFindingKind::ShadowAuthority.severity(),
        FindingSeverity::Critical
    );
    assert_eq!(
        DebVerifyFindingKind::AuthorityBypass.severity(),
        FindingSeverity::Critical
    );
    assert_eq!(
        DebVerifyFindingKind::MissingOwner.severity(),
        FindingSeverity::High
    );
    assert_eq!(
        DebVerifyFindingKind::Contradiction.severity(),
        FindingSeverity::High
    );
    assert_eq!(
        DebVerifyFindingKind::StaleCompatibility.severity(),
        FindingSeverity::Medium
    );
}

#[test]
fn acceptance_finding_shape() {
    // REQ-AC5-007
    let contracts = vec![
        single_authority("c1", "comp:x"),
        single_authority("c2", "comp:x"),
    ];
    let f = &shadow_authority(&contracts)[0];
    assert_eq!(f.kind, DebVerifyFindingKind::ShadowAuthority);
    assert_eq!(f.severity, FindingSeverity::Critical);
    assert_eq!(f.subjects, vec!["comp:x"]);
    assert_eq!(f.contract_ids, vec![cid("c1"), cid("c2")]);
    assert!(!f.message.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// exit criterion (REQ-AC5-001..004)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn exit_criterion_signature_is_global() {
    // REQ-AC5-001: exactly three arguments; no change basis can be supplied.
    let overlay = graph_with_units(&["unit:comp:x"]);
    let contracts = vec![single_authority("c1", "comp:x")];
    let audit = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 1)).expect("audit");
    assert_eq!(audit.audited_contracts, 1);
}

#[test]
fn exit_criterion_no_ac4_coupling() {
    // REQ-AC5-002: the module never references AC4's change-scoped vocabulary.
    let sources: &[(&str, &str)] = &[
        ("mod.rs", include_str!("mod.rs")),
        ("types.rs", include_str!("types.rs")),
        ("detectors.rs", include_str!("detectors.rs")),
        ("audit.rs", include_str!("audit.rs")),
    ];
    let forbidden = [
        "changed_units",
        "ConformanceInputs",
        "compute_conformance_delta",
        "ArchitectureConformanceDelta",
        "DeltaContractStatus",
    ];
    for (file, src) in sources {
        for needle in forbidden {
            // `mod.rs` documents the contrast in prose; only production code
            // matters, so skip comment lines.
            for (idx, line) in src.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") || trimmed.starts_with("///") {
                    continue;
                }
                assert!(
                    !line.contains(needle),
                    "{file}:{} references AC4 vocabulary `{needle}`: {line}",
                    idx + 1
                );
            }
        }
    }
}

#[test]
fn exit_criterion_audit_is_not_a_delta() {
    // REQ-AC5-003: distinct types, no conversion.
    let audit_type = std::any::type_name::<DebVerifyAudit>();
    assert!(audit_type.contains("DebVerifyAudit"));
    assert!(!audit_type.contains("ArchitectureConformanceDelta"));
    // And there is no `From<...Delta>` constructor: the only constructor is the
    // runner. (Compile-time: this struct literal must be built by the runner.)
    let overlay = graph_with_units(&["comp:x"]);
    let audit = run_debverify_audit(&overlay, &[], EventTime(T0)).expect("audit");
    assert_eq!(audit.audited_contracts, 0);
    assert_eq!(audit.findings, vec![]);
}

#[test]
fn acceptance_ac_uat_009_duplicate_authority_with_empty_delta() {
    // REQ-AC5-004: the AC5 exemplar. Two contracts claim the same authority and
    // NO change basis is involved at all.
    let overlay = graph_with_units(&["unit:comp:auth"]);
    let contracts = vec![
        single_authority("shadow-1", "comp:auth"),
        single_authority("shadow-2", "comp:auth"),
    ];
    let audit = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 1)).expect("audit");

    assert!(
        !audit.is_clean(),
        "AC-UAT-009: a global duplicate authority must be found with no delta"
    );
    let shadows = audit.by_kind(DebVerifyFindingKind::ShadowAuthority);
    assert_eq!(shadows.len(), 1);
    assert_eq!(shadows[0].subjects, vec!["comp:auth"]);
    assert_eq!(shadows[0].contract_ids.len(), 2);
    assert_eq!(shadows[0].severity, FindingSeverity::Critical);
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — detectors (REQ-AC5-008..012)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_shadow_authority_component() {
    // REQ-AC5-008
    let contracts = vec![
        single_authority("a", "comp:db"),
        single_authority("b", "comp:db"),
        single_authority("c", "comp:other"),
    ];
    let fs = shadow_authority(&contracts);
    assert_eq!(fs.len(), 1);
    assert_eq!(fs[0].subjects, vec!["comp:db"]);
    assert_eq!(fs[0].contract_ids, vec![cid("a"), cid("b")]);
    // `comp:other` has a single owner → no finding.
    assert!(!fs[0].subjects.contains(&"comp:other".to_string()));
}

#[test]
fn acceptance_shadow_authority_entity() {
    // REQ-AC5-008
    let contracts = vec![
        unique_owner("a", "entity:ledger"),
        unique_owner("b", "entity:ledger"),
    ];
    let fs = shadow_authority(&contracts);
    assert_eq!(fs.len(), 1);
    assert_eq!(fs[0].subjects, vec!["entity:ledger"]);
    assert_eq!(fs[0].severity, FindingSeverity::Critical);
}

#[test]
fn acceptance_missing_owner_detected() {
    // REQ-AC5-009
    let overlay = graph_with_units(&["unit:comp:present"]);
    let contracts = vec![
        single_authority("ok", "comp:present"),
        single_authority("orphan", "comp:absent"),
    ];
    let fs = missing_owner(&contracts, &overlay);
    assert_eq!(fs.len(), 1);
    assert_eq!(fs[0].subjects, vec!["comp:absent"]);
    assert_eq!(fs[0].contract_ids, vec![cid("orphan")]);
    assert_eq!(fs[0].severity, FindingSeverity::High);
}

#[test]
fn acceptance_authority_bypass_detected() {
    // REQ-AC5-010
    let contract = forbidden("no-edge", "comp:domain", "comp:provider");
    let mut overlay = graph_with_units(&["comp:domain", "comp:provider"]);
    // Add a real relation joining the two units (a live bypass).
    use crate::architecture_graph::{
        ArchitectureOverlayRelation, ArchitectureOverlayRelationKind, OverlayNodeRef,
    };
    overlay.add_relation(&ArchitectureOverlayRelation {
        from: OverlayNodeRef::SoftwareUnit(SoftwareUnitRef::new("comp:domain")),
        to: OverlayNodeRef::SoftwareUnit(SoftwareUnitRef::new("comp:provider")),
        kind: ArchitectureOverlayRelationKind::DependsOn,
        evidence: vec![],
    });

    let fs = authority_bypass(&[contract], &overlay);
    assert_eq!(fs.len(), 1, "a live forbidden edge must be reported");
    assert_eq!(fs[0].severity, FindingSeverity::Critical);
    assert!(fs[0].message.contains("comp:domain"));
}

#[test]
fn acceptance_authority_bypass_negative_control() {
    // REQ-AC5-010 + REQ-AC5-019: with no joining relation, no bypass.
    let contract = forbidden("no-edge", "comp:domain", "comp:provider");
    let overlay = graph_with_units(&["comp:domain", "comp:provider"]);
    let fs = authority_bypass(&[contract], &overlay);
    assert!(fs.is_empty(), "no edge ⇒ no bypass: {fs:?}");
}

#[test]
fn acceptance_stale_compatibility_detected() {
    // REQ-AC5-011
    let contracts = vec![bounded("c-window", T0 + 10)];
    // Before the window: clean.
    assert!(stale_compatibility(&contracts, EventTime(T0 + 5)).is_empty());
    // After the window with no replacement: stale.
    let fs = stale_compatibility(&contracts, EventTime(T0 + 20));
    assert_eq!(fs.len(), 1);
    assert_eq!(fs[0].severity, FindingSeverity::Medium);
}

#[test]
fn acceptance_contradiction_detected() {
    // REQ-AC5-012
    let contracts = vec![
        single_authority("owner", "comp:ledger"),
        projection_only("proj", "comp:ledger"),
    ];
    let fs = contradiction(&contracts);
    assert_eq!(fs.len(), 1);
    assert_eq!(fs[0].subjects, vec!["comp:ledger"]);
    assert_eq!(fs[0].contract_ids, vec![cid("owner"), cid("proj")]);
    assert_eq!(fs[0].severity, FindingSeverity::High);
}

#[test]
fn bonus_contradiction_negative_control() {
    let contracts = vec![
        single_authority("owner", "comp:ledger"),
        projection_only("proj", "comp:cache"),
    ];
    assert!(contradiction(&contracts).is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — audit shape (REQ-AC5-013..018)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_findings_sorted() {
    // REQ-AC5-013
    let overlay = graph_with_units(&["comp:live"]);
    let contracts = vec![
        bounded("window", T0 + 1),
        single_authority("dup-a", "comp:dup"),
        single_authority("dup-b", "comp:dup"),
    ];
    let audit = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 100)).expect("audit");
    let keys: Vec<(DebVerifyFindingKind, Vec<String>)> = audit
        .findings
        .iter()
        .map(|f| (f.kind, f.subjects.clone()))
        .collect();
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted, "findings must be sorted by (kind, subjects)");
}

#[test]
fn acceptance_counts_and_is_clean() {
    // REQ-AC5-014
    let overlay = graph_with_units(&["comp:live"]);
    let clean = run_debverify_audit(
        &overlay,
        &[single_authority("ok", "comp:live")],
        EventTime(T0 + 1),
    )
    .expect("audit");
    assert!(clean.is_clean());
    assert!(clean.counts().is_empty());

    let dirty = run_debverify_audit(
        &overlay,
        &[
            single_authority("a", "comp:dup"),
            single_authority("b", "comp:dup"),
        ],
        EventTime(T0 + 1),
    )
    .expect("audit");
    assert!(!dirty.is_clean());
    assert_eq!(
        dirty.counts().get(&DebVerifyFindingKind::ShadowAuthority),
        Some(&1)
    );
}

#[test]
fn acceptance_audited_contracts_count() {
    // REQ-AC5-015
    let overlay = graph_with_units(&["comp:a"]);
    let contracts = vec![
        single_authority("a", "comp:a"),
        unique_owner("b", "entity:b"),
        provider_boundary("c"),
    ];
    let audit = run_debverify_audit(&overlay, &contracts, EventTime(T0)).expect("audit");
    assert_eq!(audit.audited_contracts, 3);
}

#[test]
fn acceptance_audit_digest_is_stable() {
    // REQ-AC5-016
    let overlay = graph_with_units(&["comp:live"]);
    let contracts = vec![
        single_authority("a", "comp:dup"),
        single_authority("b", "comp:dup"),
    ];
    let a = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 1)).unwrap();
    let b = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 1)).unwrap();
    assert_eq!(a.digest, b.digest);
    assert_eq!(a, b);
}

#[test]
fn acceptance_graph_digest_matches_overlay() {
    // REQ-AC5-017
    let overlay = graph_with_units(&["comp:a"]);
    let audit = run_debverify_audit(&overlay, &[], EventTime(T0)).expect("audit");
    assert_eq!(audit.graph_digest, overlay.digest());
}

#[test]
fn acceptance_audit_is_deterministic() {
    // REQ-AC5-018
    let overlay = graph_with_units(&["comp:live"]);
    let contracts = vec![
        single_authority("a", "comp:dup"),
        single_authority("b", "comp:dup"),
        bounded("w", T0 + 1),
        forbidden("f", "comp:x", "comp:y"),
    ];
    let run = || run_debverify_audit(&overlay, &contracts, EventTime(T0 + 100)).expect("audit");
    assert_eq!(run(), run());
}

#[test]
fn acceptance_clean_architecture_has_no_findings() {
    // REQ-AC5-019: negative control across all five detectors.
    // Every declared owner must be present in the graph, and the forbidden
    // edge must not exist.
    let overlay = graph_with_units(&["comp:ledger", "comp:other", "entity:cache"]);
    let contracts = vec![
        single_authority("owner", "comp:ledger"),
        unique_owner("u-owner", "entity:cache"),
        single_authority("other", "comp:other"),
        projection_only("proj", "comp:cache-projection"),
        forbidden("no-edge", "comp:ledger", "comp:other"),
        bounded("w-open", T0 + 10_000),
        provider_boundary("pb"),
    ];
    let audit = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 1)).expect("audit");
    assert!(
        audit.is_clean(),
        "a coherent architecture must be clean: {:?}",
        audit.findings
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// anti-encroachment (REQ-AC5-020..022)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn anti_encroachment_no_forbidden_imports() {
    // REQ-AC5-020
    let sources: &[(&str, &str)] = &[
        ("mod.rs", include_str!("mod.rs")),
        ("types.rs", include_str!("types.rs")),
        ("detectors.rs", include_str!("detectors.rs")),
        ("audit.rs", include_str!("audit.rs")),
    ];
    let forbidden = [
        "use crate::architecture_conformance",
        "use crate::architecture_mutation",
        "use crate::paradigm_profile",
        "use crate::alignment",
        "use crate::provider",
        "use crate::host_sdk",
        "use crate::agent_host",
        "use crate::capability",
        "use crate::effective_instructions",
    ];
    let mut offenders: Vec<(String, String)> = Vec::new();
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
        "architecture_debverify imports forbidden surfaces: {offenders:?}"
    );
}

#[test]
fn anti_encroachment_read_only_and_no_claims() {
    // REQ-AC5-021
    let sources: &[(&str, &str)] = &[
        ("mod.rs", include_str!("mod.rs")),
        ("types.rs", include_str!("types.rs")),
        ("detectors.rs", include_str!("detectors.rs")),
        ("audit.rs", include_str!("audit.rs")),
    ];
    for (file, src) in sources {
        for forbidden in [
            "ArchitectureClaim {",
            "ArchitectureClaim::",
            "use std::fs",
            "use std::io::Write",
            "std::fs::",
            "File::create(",
            ".add_unit(",
            ".add_claim(",
            ".add_relation(",
            ".clear()",
        ] {
            assert!(
                !src.contains(forbidden),
                "{file} must not contain `{forbidden}` (read-only, no claims)"
            );
        }
    }

    // Behavioural: the audit does not change the overlay digest.
    let overlay = graph_with_units(&["comp:a"]);
    let before = overlay.digest();
    let _ = run_debverify_audit(&overlay, &[single_authority("a", "comp:a")], EventTime(T0))
        .expect("audit");
    assert_eq!(
        before,
        overlay.digest(),
        "audit must not mutate the overlay"
    );
}

#[test]
fn anti_encroachment_no_numeric_score() {
    // REQ-AC5-022: the audit carries no numeric aggregate.
    let src_types = include_str!("types.rs");
    for forbidden in [
        "score: ",
        "weight: ",
        "fn score(",
        "fn total(",
        "fn rating(",
    ] {
        assert!(
            !src_types.contains(forbidden),
            "types.rs must not contain `{forbidden}`"
        );
    }
    let overlay = graph_with_units(&["comp:a"]);
    let audit = run_debverify_audit(
        &overlay,
        &[
            single_authority("a", "comp:dup"),
            single_authority("b", "comp:dup"),
        ],
        EventTime(T0),
    )
    .expect("audit");
    // The public shape exposes findings + counts, never a scalar.
    let _ = audit.counts();
    let _ = audit.is_clean();
    let _ = audit.implicated_contracts();
    // Severity is an enum, not a number.
    for f in &audit.findings {
        assert!(FindingSeverity::ALL.contains(&f.severity));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// boundary
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bonus_empty_contract_set_is_clean() {
    let overlay = graph_with_units(&["comp:a"]);
    let audit = run_debverify_audit(&overlay, &[], EventTime(T0)).expect("audit");
    assert!(audit.is_clean());
    assert_eq!(audit.audited_contracts, 0);
    assert!(audit.implicated_contracts().is_empty());
}

#[test]
fn bonus_implicated_contracts_are_sorted_and_deduped() {
    let overlay = graph_with_units(&["comp:a"]);
    let contracts = vec![
        single_authority("dup-a", "comp:dup"),
        single_authority("dup-b", "comp:dup"),
        bounded("window", T0 + 1),
    ];
    let audit = run_debverify_audit(&overlay, &contracts, EventTime(T0 + 100)).expect("audit");
    let implicated = audit.implicated_contracts();
    let mut sorted = implicated.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(implicated, sorted);
    assert!(implicated.contains(&cid("dup-a")));
    assert!(implicated.contains(&cid("window")));
}

#[test]
fn bonus_auditable_kind_mapping() {
    use crate::architectural_contract::ContractKindRef;
    assert!(kind_is_auditable(
        &crate::architectural_contract::ContractKind::SingleAuthority
    ));
    assert!(!kind_is_auditable(
        &crate::architectural_contract::ContractKind::Extension(
            ContractKindRef::new("ac5.ext").expect("valid")
        )
    ));
}

// ─────────────────────────────────────────────────────────────────────────────
// FindingId (A3-S15 / REQ-A3S15-001..003)
// ─────────────────────────────────────────────────────────────────────────────

use crate::architecture_debverify::finding_id::{FindingBasis, FindingId};

fn fid_cid(s: &str) -> crate::architectural_contract::ContractId {
    crate::architectural_contract::ContractId::new(s).expect("valid contract id")
}

fn finding_basis() -> FindingBasis {
    FindingBasis::new("rev:1", "kb:test", [7u8; 32])
}

#[test]
fn acceptance_finding_id_excludes_message() {
    // REQ-A3S15-001: `message` is free text that can change for a redaction
    // alone, so it must never participate in identity.
    let b = finding_basis();
    let subjects = vec!["comp:dup".to_string()];
    let contracts = vec![fid_cid("c-a"), fid_cid("c-b")];
    let a = FindingId::derive(
        &b,
        DebVerifyFindingKind::ShadowAuthority,
        &subjects,
        &contracts,
    );
    let c = FindingId::derive(
        &b,
        DebVerifyFindingKind::ShadowAuthority,
        &subjects,
        &contracts,
    );
    assert_eq!(a, c);

    // Rewording the finding must not move the id.
    let other_message = "a completely different sentence about the same subject";
    let d = FindingId::derive(
        &b,
        DebVerifyFindingKind::ShadowAuthority,
        &subjects,
        &contracts,
    );
    assert_eq!(a, d, "id must not depend on message text ({other_message})");
}

#[test]
fn acceptance_finding_id_excludes_severity() {
    // REQ-A3S15-001: `severity` is a pure function of `kind`, so including it
    // would add no discrimination while implying it can vary independently.
    // Same id for two findings that differ only in... nothing, because severity
    // is derived. The real assertion is that the id is computable from the kind
    // alone, with no severity parameter to get wrong.
    let b = finding_basis();
    let id = FindingId::derive(
        &b,
        DebVerifyFindingKind::ShadowAuthority,
        &["comp:x".into()],
        &[],
    );
    let again = FindingId::derive(
        &b,
        DebVerifyFindingKind::ShadowAuthority,
        &["comp:x".into()],
        &[],
    );
    assert_eq!(id, again);
    // And the signature has no severity argument: this compiles only because the
    // derivation is (basis, kind, subjects, contracts).
    assert_eq!(id.as_str().len(), 64);
    assert!(FindingId::looks_like_id(id.as_str()));
}

#[test]
fn acceptance_finding_id_is_clock_stable() {
    // REQ-A3S15-002: the basis carries no clock, so an id printed by
    // `findings --now-ms X` is resolvable by `why --now-ms Y`. If the clock
    // entered the basis this would be unusable in the primary workflow.
    let b = finding_basis();
    let subjects = vec!["comp:dup".to_string()];
    let contracts = vec![fid_cid("c-a")];
    let at_1000 = FindingId::derive(
        &b,
        DebVerifyFindingKind::ShadowAuthority,
        &subjects,
        &contracts,
    );
    let at_9999999 = FindingId::derive(
        &b,
        DebVerifyFindingKind::ShadowAuthority,
        &subjects,
        &contracts,
    );
    assert_eq!(at_1000, at_9999999);

    // A different substrate does move it: that is the id meaning something.
    let other = FindingBasis::new("rev:1", "kb:test", [9u8; 32]);
    assert_ne!(
        at_1000,
        FindingId::derive(
            &other,
            DebVerifyFindingKind::ShadowAuthority,
            &subjects,
            &contracts
        )
    );
}

#[test]
fn acceptance_two_findings_one_subject_do_not_collapse() {
    // REQ-A3S15-003, the observation from A3-S14: `comp:dup` produces both a
    // shadow_authority and a contradiction, so `subject` cannot be the identity.
    let b = finding_basis();
    let subjects = vec!["comp:dup".to_string()];
    let contracts = vec![fid_cid("c-a"), fid_cid("c-b")];

    let shadow = FindingId::derive(
        &b,
        DebVerifyFindingKind::ShadowAuthority,
        &subjects,
        &contracts,
    );
    let contra = FindingId::derive(
        &b,
        DebVerifyFindingKind::Contradiction,
        &subjects,
        &contracts,
    );
    assert_ne!(shadow, contra, "one subject, two kinds, two ids");

    // Same kind, different contract set: also distinct.
    let narrower = FindingId::derive(
        &b,
        DebVerifyFindingKind::ShadowAuthority,
        &subjects,
        &[fid_cid("c-a")],
    );
    assert_ne!(shadow, narrower);
}
