// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_declaration/tests.rs — A3-S10 acceptance + anti-encroachment.
//
// REQ-A3S10-001..008, 015 pinned via the cycle-bounded spec
// `docs/architecture/specs/arch-spec-A3-S10-architecture-cli-surface.md`.

use super::*;
use crate::architectural_contract::{ContractKind, ContractPayload};
use crate::architecture_graph::UnitKind;

fn decl_with(contracts: Vec<ContractDecl>) -> DeclarationFile {
    DeclarationFile {
        revision: "rev-1".to_string(),
        knowledge_basis: None,
        units: vec![],
        contracts,
        waivers: vec![],
    }
}

fn sa(id: &str, component: &str) -> ContractDecl {
    ContractDecl {
        id: id.to_string(),
        kind: "single_authority".to_string(),
        component: Some(component.to_string()),
        entity: None,
        from: None,
        to: None,
        reason: None,
        source_kind: None,
        deprecated_after_ms: None,
        replaced_by: None,
        decided_by: "decision:d".to_string(),
        specified_by: "spec:s".to_string(),
        revision: "rev:r".to_string(),
        declared_at_ms: Some(1_700_000_000),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance (REQ-A3S10-001..008)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_load_declaration_is_pure() {
    // REQ-A3S10-001: `validate` takes an already-parsed value; a malformed
    // declaration cannot reach it, and no IO exists in this module.
    let decl = decl_with(vec![sa("c1", "comp:x")]);
    let a = validate(&decl, "label").expect("valid");
    let b = validate(&decl, "label").expect("valid");
    assert_eq!(
        a, b,
        "validation must be deterministic and side-effect free"
    );
}

#[test]
fn acceptance_valid_declaration_builds_contracts() {
    // REQ-A3S10-002
    let decl = decl_with(vec![sa("c1", "comp:x"), sa("c2", "comp:y")]);
    let out = validate(&decl, "label").expect("valid");
    assert_eq!(out.revision, "rev-1");
    assert_eq!(out.contracts.len(), 2);
    // Sorted by id.
    assert_eq!(out.contracts[0].id().as_str(), "c1");
    assert_eq!(out.contracts[1].id().as_str(), "c2");
    assert_eq!(*out.contracts[0].kind(), ContractKind::SingleAuthority);
    match out.contracts[0].payload() {
        ContractPayload::SingleAuthority(c) => assert_eq!(c.as_str(), "comp:x"),
        other => panic!("unexpected payload: {other:?}"),
    }
}

#[test]
fn acceptance_fail_closed_cases() {
    // REQ-A3S10-003
    // (a) duplicate id
    let dup = decl_with(vec![sa("c1", "comp:x"), sa("c1", "comp:y")]);
    assert!(matches!(
        validate(&dup, "l"),
        Err(DeclarationError::DuplicateId { .. })
    ));

    // (b) unknown kind
    let mut bad_kind = sa("c1", "comp:x");
    bad_kind.kind = "not_a_kind".to_string();
    assert!(matches!(
        validate(&decl_with(vec![bad_kind]), "l"),
        Err(DeclarationError::UnknownKind { .. })
    ));

    // (c) missing kind-specific field
    let mut missing = sa("c1", "comp:x");
    missing.component = None;
    assert!(matches!(
        validate(&decl_with(vec![missing]), "l"),
        Err(DeclarationError::MissingField {
            field: "component",
            ..
        })
    ));

    // (d) blank id
    let mut blank = sa("c1", "comp:x");
    blank.id = "  ".to_string();
    assert!(matches!(
        validate(&decl_with(vec![blank]), "l"),
        Err(DeclarationError::BlankId { .. })
    ));

    // (e) unknown unit kind
    let mut d = decl_with(vec![]);
    d.units = vec![UnitDecl {
        id: "u1".to_string(),
        kind: Some("nonsense".to_string()),
        locator: None,
    }];
    assert!(matches!(
        validate(&d, "l"),
        Err(DeclarationError::UnknownUnitKind { .. })
    ));
}

#[test]
fn acceptance_revision_required() {
    // REQ-A3S10-004
    let mut d = decl_with(vec![]);
    d.revision = "   ".to_string();
    assert_eq!(validate(&d, "l"), Err(DeclarationError::EmptyRevision));
}

#[test]
fn acceptance_knowledge_basis_default() {
    // REQ-A3S10-005
    let mut d = decl_with(vec![]);
    d.knowledge_basis = None;
    assert_eq!(
        validate(&d, "contracts.yaml").unwrap().knowledge_basis,
        "declaration:contracts.yaml"
    );
    d.knowledge_basis = Some("docs/adr".to_string());
    assert_eq!(
        validate(&d, "contracts.yaml").unwrap().knowledge_basis,
        "docs/adr"
    );
}

#[test]
fn acceptance_units_convert() {
    // REQ-A3S10-006
    let mut d = decl_with(vec![]);
    d.units = vec![
        UnitDecl {
            id: "unit:a".to_string(),
            kind: None,
            locator: Some("crates/a.rs".to_string()),
        },
        UnitDecl {
            id: "unit:b".to_string(),
            kind: Some("crate".to_string()),
            locator: None,
        },
    ];
    let out = validate(&d, "l").expect("valid");
    assert_eq!(out.units.len(), 2);
    assert_eq!(
        out.units[0].kind,
        UnitKind::Module,
        "kind defaults to module"
    );
    assert_eq!(out.units[0].locator, "crates/a.rs");
    assert_eq!(out.units[1].kind, UnitKind::Crate);
    assert_eq!(out.units[1].locator, "unit:b", "locator defaults to id");
}

#[test]
fn acceptance_waivers_carried() {
    // REQ-A3S10-007: verbatim, sorted, deduplicated.
    let mut d = decl_with(vec![]);
    d.waivers = vec![
        "waiver:b".to_string(),
        "waiver:a".to_string(),
        "waiver:a".to_string(),
        "   ".to_string(),
    ];
    let out = validate(&d, "l").expect("valid");
    assert_eq!(out.waivers, vec!["waiver:a", "waiver:b"]);
}

#[test]
fn acceptance_all_five_kinds() {
    // REQ-A3S10-008: the five kinds used by the historical classes declare.
    let mk = |id: &str, kind: &str| {
        let mut c = sa(id, "comp:x");
        c.kind = kind.to_string();
        c
    };
    let mut uo = mk("uo", "unique_owner");
    uo.entity = Some("entity:e".to_string());
    let mut fd = mk("fd", "forbidden_dependency");
    fd.from = Some("comp:a".to_string());
    fd.to = Some("comp:b".to_string());
    fd.reason = Some("forbidden".to_string());
    let mut po = mk("po", "projection_only");
    po.source_kind = Some("entity:e".to_string());
    let mut bc = mk("bc", "bounded_compatibility");
    bc.deprecated_after_ms = Some(1_800_000_000);

    let out = validate(&decl_with(vec![sa("sa", "comp:x"), uo, fd, po, bc]), "l").expect("valid");
    assert_eq!(out.contracts.len(), 5);
    let kinds: Vec<ContractKind> = out.contracts.iter().map(|c| c.kind().clone()).collect();
    assert!(kinds.contains(&ContractKind::SingleAuthority));
    assert!(kinds.contains(&ContractKind::UniqueOwner));
    assert!(kinds.contains(&ContractKind::ForbiddenDependency));
    assert!(kinds.contains(&ContractKind::ProjectionOnly));
    assert!(kinds.contains(&ContractKind::BoundedCompatibility));
}

#[test]
fn acceptance_bounded_compatibility_requires_window() {
    let mut c = sa("bc", "comp:x");
    c.kind = "bounded_compatibility".to_string();
    // deprecated_after_ms intentionally absent
    let err = validate(&decl_with(vec![c]), "l").unwrap_err();
    assert!(matches!(
        err,
        DeclarationError::MissingField {
            field: "deprecated_after_ms",
            ..
        }
    ));
}

#[test]
fn acceptance_bounded_compatibility_carries_replacement() {
    let mut c = sa("bc", "comp:x");
    c.kind = "bounded_compatibility".to_string();
    c.deprecated_after_ms = Some(1_800_000_000);
    c.replaced_by = Some("bc-next".to_string());
    let out = validate(&decl_with(vec![c]), "l").expect("valid");
    match out.contracts[0].payload() {
        ContractPayload::BoundedCompatibility { replaced_by, .. } => {
            assert_eq!(replaced_by.as_ref().map(|c| c.as_str()), Some("bc-next"));
        }
        other => panic!("unexpected payload: {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// anti-encroachment (REQ-A3S10-015)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn anti_encroachment_declaration_is_pure() {
    // REQ-A3S10-015: no IO, no authority/capability/provider import.
    let sources: &[(&str, &str)] = &[
        ("mod.rs", include_str!("mod.rs")),
        ("types.rs", include_str!("types.rs")),
        ("validate.rs", include_str!("validate.rs")),
    ];
    for (file, src) in sources {
        for forbidden in [
            "use std::fs",
            "use std::io::Write",
            "std::fs::",
            "File::create(",
            "use crate::authority",
            "use crate::capability",
            "use crate::effective_instructions",
            "use crate::provider",
            "use crate::host_sdk",
            "ArchitectureClaim",
            "serde_yaml",
        ] {
            assert!(
                !src.contains(forbidden),
                "{file} must not contain `{forbidden}`"
            );
        }
    }
}
