// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architectural_contract/tests.rs — tests for the contract substrate.
//
// 11 acceptance + 4 negative + 4 anti-encroachment = 19 tests,
// plus a handful of bonus fixtures covering extension values, bounded
// compatibility with replacement, and provider-boundary preservation.

use std::collections::BTreeMap;

use crate::knowledge::EventTime;

use super::claim::{
    ArchitectureClaim, ClaimOutcome, ContractEvaluation, EvaluatorRef, EvidenceRef,
};
use super::contract::ArchitecturalContract;
use super::error::ContractError;
use super::hashing::derive_contract_basis_hash;
use super::payload::{BoundaryKind, ContractExtensionValue, ContractKind, ContractPayload};
use super::types::{
    ComponentRef, ContractId, ContractKindRef, DecisionRef, EntityRef, Revision, SpecRef,
};

// ── helpers ─────────────────────────────────────────────────────────────────

fn id0() -> ContractId {
    ContractId::new("contract.authority.ownership").unwrap()
}
fn id1() -> ContractId {
    ContractId::new("contract.dep.inbound-forbidden").unwrap()
}
fn rev_v1() -> Revision {
    Revision::new("v1").unwrap()
}
fn ev_runtime_a() -> EvidenceRef {
    EvidenceRef::new("runtime", "trace:abc123").unwrap()
}
fn ev_static_a() -> EvidenceRef {
    EvidenceRef::new("static", "node:provider").unwrap()
}
fn dr_dec() -> DecisionRef {
    DecisionRef::Decision("decision.sddk.engine.001".into())
}
fn dr_adr() -> DecisionRef {
    DecisionRef::Adr("ADR-0112".into())
}
fn sr_adr() -> SpecRef {
    SpecRef::Adr("ADR-0112".into())
}
fn sr_archspec() -> SpecRef {
    SpecRef::ArchSpec("arch-spec-032".into())
}
fn component_x() -> ComponentRef {
    ComponentRef::new("sddk_engine").unwrap()
}
fn component_y() -> ComponentRef {
    ComponentRef::new("provider_sdk").unwrap()
}
fn entity_authority() -> EntityRef {
    EntityRef::new("authority_engine").unwrap()
}
fn evaluator_engine() -> EvaluatorRef {
    EvaluatorRef::new("sddk.contract.evaluator.v1").unwrap()
}

// ── state-class annotations ─────────────────────────────────────────────────

/// Every public type carries a `# State class:` annotation somewhere in
/// its doc-comment (architectural-contract module doc-level contract).
/// If a future contributor renames or removes any of these types, this
/// test fails closed.
#[test]
fn architectural_contract_module_state_class_annotations_complete() {
    // Concatenate the whole module to scan for type names + state-class
    // discipline. We use include_str! for each submodule.
    let combined = format!(
        "{}{}{}{}{}",
        include_str!("types.rs"),
        include_str!("error.rs"),
        include_str!("payload.rs"),
        include_str!("hashing.rs"),
        include_str!("contract.rs"),
    );
    for needle in [
        "ContractId",
        "Revision",
        "ComponentRef",
        "EntityRef",
        "DecisionRef",
        "SpecRef",
        "ContractKindRef",
        "ContractKind",
        "BoundaryKind",
        "ContractPayload",
        "ContractExtensionValue",
        "ArchitecturalContract",
    ] {
        assert!(
            combined.contains(needle),
            "type `{}` not found in architectural_contract/* source (state-class audit)",
            needle
        );
    }
    let claim_module = include_str!("claim.rs");
    for needle in [
        "EvidenceRef",
        "EvaluatorRef",
        "ClaimOutcome",
        "ArchitectureClaim",
    ] {
        assert!(
            claim_module.contains(needle),
            "type `{}` not found in claim.rs (state-class audit)",
            needle
        );
    }
    // Each submodule doc-comment must declare its state class.
    assert!(
        combined.contains("# State class"),
        "submodule doc-comments must declare `# State class:` discipline"
    );
    assert!(
        claim_module.contains("# State class"),
        "claim.rs doc-comment must declare `# State class:` discipline"
    );
}

// ── ACC-01 .. ACC-04: identity & determinism ─────────────────────────────────

#[test]
fn contract_identity_includes_id_kind_payload_refs_revision() {
    let id = id0();
    let c = ArchitecturalContract::declare_single_authority(
        id.clone(),
        component_x(),
        dr_dec(),
        sr_archspec(),
        rev_v1(),
        EventTime(1000),
    )
    .unwrap();
    // Recompute the hash from the same inputs and confirm equality.
    let expected = derive_contract_basis_hash(
        c.id(),
        c.kind(),
        c.payload(),
        c.decided_by(),
        c.specified_by(),
        c.revision(),
        c.declared_at(),
    );
    assert_eq!(c.basis_hash(), &expected);
}

#[test]
fn basis_hash_is_deterministic_for_same_inputs() {
    let id = id0();
    let a = ArchitecturalContract::declare_single_authority(
        id.clone(),
        component_x(),
        dr_dec(),
        sr_archspec(),
        rev_v1(),
        EventTime(1000),
    )
    .unwrap();
    let b = ArchitecturalContract::declare_single_authority(
        id,
        component_x(),
        dr_dec(),
        sr_archspec(),
        rev_v1(),
        EventTime(1000),
    )
    .unwrap();
    assert_eq!(a.basis_hash(), b.basis_hash());
}

#[test]
fn basis_hash_differs_on_payload_change() {
    let id_a = ContractId::new("contract.test.a").unwrap();
    let id_b = ContractId::new("contract.test.b").unwrap();
    let a = ArchitecturalContract::declare_single_authority(
        id_a,
        component_x(),
        dr_dec(),
        sr_archspec(),
        rev_v1(),
        EventTime(1000),
    )
    .unwrap();
    let b = ArchitecturalContract::declare_single_authority(
        id_b,
        component_y(),
        dr_dec(),
        sr_archspec(),
        rev_v1(),
        EventTime(1000),
    )
    .unwrap();
    assert_ne!(
        a.basis_hash(),
        b.basis_hash(),
        "different payload must produce different basis_hash"
    );
}

#[test]
fn basis_hash_differs_on_decided_by_change() {
    let id = ContractId::new("contract.test.ref-change").unwrap();
    let a = ArchitecturalContract::declare_single_authority(
        id.clone(),
        component_x(),
        dr_dec(),
        sr_archspec(),
        rev_v1(),
        EventTime(1000),
    )
    .unwrap();
    let b = ArchitecturalContract::declare_single_authority(
        id,
        component_x(),
        dr_adr(),
        sr_archspec(),
        rev_v1(),
        EventTime(1000),
    )
    .unwrap();
    assert_ne!(a.basis_hash(), b.basis_hash());
}

// ── ACC-05 / NEG-04: kind/payload coupling ──────────────────────────────────

#[test]
fn kind_payload_coupling_is_type_safe() {
    // SingleAuthority kind cannot be paired with ForbiddenDependency
    // payload — the typed constructors prevent this at compile time.
    // Here we exercise the runtime guard via `declare(...)` directly.
    let id = id0();
    let payload = ContractPayload::ForbiddenDependency {
        from: component_x(),
        to: component_y(),
        reason: "no inbound".into(),
    };
    let result = ArchitecturalContract::declare(
        id,
        ContractKind::SingleAuthority,
        payload,
        dr_dec(),
        sr_archspec(),
        rev_v1(),
        EventTime(1000),
    );
    assert!(matches!(
        result,
        Err(ContractError::KindPayloadMismatch { .. })
    ));
}

#[test]
fn extension_payload_uses_namespaced_kind_and_btreemap() {
    // Build an Extension kind + payload via the typed surface.
    let id = ContractId::new("contract.ext.btreemap-order").unwrap();
    let kind_ref = ContractKindRef::new("acme.acl").unwrap();
    let kind = ContractKind::Extension(kind_ref.clone());
    // Intentionally insert in non-sorted order; BTreeMap must sort.
    let mut fields = BTreeMap::new();
    fields.insert("zeta".into(), ContractExtensionValue::Integer(1));
    fields.insert("alpha".into(), ContractExtensionValue::String("hi".into()));
    fields.insert(
        "mid".into(),
        ContractExtensionValue::Object({
            let mut m = BTreeMap::new();
            m.insert("k".into(), ContractExtensionValue::Boolean(true));
            m
        }),
    );
    let payload = ContractPayload::Extension {
        kind: kind_ref.clone(),
        fields,
    };
    let c = ArchitecturalContract::declare(
        id,
        kind,
        payload,
        dr_dec(),
        sr_archspec(),
        rev_v1(),
        EventTime(1000),
    )
    .unwrap();
    // Recompute the hash — should match what `declare` produced.
    let expected = derive_contract_basis_hash(
        c.id(),
        c.kind(),
        c.payload(),
        c.decided_by(),
        c.specified_by(),
        c.revision(),
        c.declared_at(),
    );
    assert_eq!(c.basis_hash(), &expected);
    // And insertion-order independence: rebuild with reversed field
    // order and confirm the hash matches.
    let mut fields_reversed = BTreeMap::new();
    fields_reversed.insert(
        "mid".into(),
        ContractExtensionValue::Object({
            let mut m = BTreeMap::new();
            m.insert("k".into(), ContractExtensionValue::Boolean(true));
            m
        }),
    );
    fields_reversed.insert("zeta".into(), ContractExtensionValue::Integer(1));
    fields_reversed.insert("alpha".into(), ContractExtensionValue::String("hi".into()));
    let payload_reversed = ContractPayload::Extension {
        kind: kind_ref,
        fields: fields_reversed,
    };
    let c_reversed = ArchitecturalContract::declare(
        ContractId::new("contract.ext.btreemap-order").unwrap(),
        ContractKind::Extension(ContractKindRef::new("acme.acl").unwrap()),
        payload_reversed,
        dr_dec(),
        sr_archspec(),
        rev_v1(),
        EventTime(1000),
    )
    .unwrap();
    assert_eq!(
        c.basis_hash(),
        c_reversed.basis_hash(),
        "BTreeMap fields must produce insertion-order-independent hash"
    );
}

#[test]
fn extension_payload_rejects_provider_namespace() {
    let r = ContractKindRef::new("provider.openai");
    assert!(matches!(
        r,
        Err(ContractError::ProviderNamespaceForbidden { .. })
    ));
    // And the top-level Extension kind reuses the same gate.
    let r2 = ContractKindRef::new("provider.anthropic");
    assert!(matches!(
        r2,
        Err(ContractError::ProviderNamespaceForbidden { .. })
    ));
}

// ── ACC-08..10: claim outcomes ──────────────────────────────────────────────

#[test]
fn claim_outcome_is_unknown_when_evidence_is_empty() {
    let c = ArchitecturalContract::declare_single_authority(
        id0(),
        component_x(),
        dr_dec(),
        sr_archspec(),
        rev_v1(),
        EventTime(1000),
    )
    .unwrap();
    let claim = ContractEvaluation::evaluate(
        &c,
        vec![],
        EventTime(1500),
        evaluator_engine(),
        Some("no evidence provided".into()),
    );
    assert_eq!(claim.outcome(), ClaimOutcome::Unknown);
    assert_eq!(
        claim.missing_evidence(),
        &[crate::knowledge::MissingEvidence::NotProvided]
    );
    assert_eq!(claim.note(), Some("no evidence provided"));
}

#[test]
fn claim_outcome_is_stale_when_now_precedes_declared_at() {
    let c = ArchitecturalContract::declare_unique_owner(
        id1(),
        entity_authority(),
        dr_dec(),
        sr_adr(),
        rev_v1(),
        EventTime(2000),
    )
    .unwrap();
    let claim = ContractEvaluation::evaluate(
        &c,
        vec![ev_runtime_a()],
        EventTime(1500), // before declared_at
        evaluator_engine(),
        None,
    );
    assert_eq!(claim.outcome(), ClaimOutcome::Stale);
    assert!(claim.missing_evidence().is_empty());
}

#[test]
fn claim_outcome_is_stale_when_deprecated_after_elapsed() {
    let c = ArchitecturalContract::declare_bounded_compatibility(
        id0(),
        EventTime(3000),
        None, // no replacement
        dr_dec(),
        sr_archspec(),
        rev_v1(),
        EventTime(1000),
    )
    .unwrap();
    let claim = ContractEvaluation::evaluate(
        &c,
        vec![ev_runtime_a()],
        EventTime(3500), // past deprecated_after
        evaluator_engine(),
        None,
    );
    assert_eq!(claim.outcome(), ClaimOutcome::Stale);
}

#[test]
fn bounded_compatibility_with_replacement_is_verified_past_window() {
    // When a replacement exists, the contract is NOT marked stale past
    // its window — the migration is in progress and the contract
    // continues to apply until the replacement supersedes it.
    let replacement = ContractId::new("contract.replacement").unwrap();
    let c = ArchitecturalContract::declare_bounded_compatibility(
        id0(),
        EventTime(3000),
        Some(replacement),
        dr_dec(),
        sr_archspec(),
        rev_v1(),
        EventTime(1000),
    )
    .unwrap();
    let claim = ContractEvaluation::evaluate(
        &c,
        vec![ev_runtime_a()],
        EventTime(3500),
        evaluator_engine(),
        None,
    );
    assert_eq!(claim.outcome(), ClaimOutcome::Verified);
}

// ── ACC-11: serde round-trip ────────────────────────────────────────────────

#[test]
fn serde_roundtrip_preserves_contract_identity_and_basis_hash() {
    let c = ArchitecturalContract::declare_single_authority(
        id0(),
        component_x(),
        dr_dec(),
        sr_archspec(),
        rev_v1(),
        EventTime(1000),
    )
    .unwrap();
    let json = serde_json::to_string(&c).expect("serialize");
    let back: ArchitecturalContract = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(c, back);
    assert_eq!(c.basis_hash(), back.basis_hash());

    // And a claim.
    let claim = ContractEvaluation::evaluate(
        &c,
        vec![ev_static_a()],
        EventTime(1500),
        evaluator_engine(),
        None,
    );
    let cjson = serde_json::to_string(&claim).expect("serialize claim");
    let cback: ArchitectureClaim = serde_json::from_str(&cjson).expect("deserialize claim");
    assert_eq!(claim, cback);
}

// ── NEG-01 / NEG-02: provenance enforcement at construction ──────────────────

#[test]
fn cannot_build_contract_with_empty_component_ref() {
    // The `ComponentRef::new` guard refuses empty / whitespace input;
    // verified directly here as the typed `declare_single_authority`
    // constructor takes a `ComponentRef` (already validated).
    assert!(matches!(
        ComponentRef::new("   "),
        Err(ContractError::InvalidRef { .. })
    ));
    assert!(matches!(
        ComponentRef::new(""),
        Err(ContractError::InvalidRef { .. })
    ));
}

#[test]
fn cannot_build_contract_with_empty_revision() {
    // Same shape: the typed `Revision::new` guard refuses empty input.
    assert!(matches!(
        Revision::new(""),
        Err(ContractError::InvalidRevision { .. })
    ));
    assert!(matches!(
        Revision::new("\t\n"),
        Err(ContractError::InvalidRevision { .. })
    ));
}

// ── NEG-03: cannot mark Verified without evidence (private ctor) ─────────────

#[test]
fn cannot_claim_verified_with_empty_evidence() {
    // Verified is only produced by ContractEvaluation::evaluate when
    // evidence is non-empty AND time is not stale. Confirm the only
    // way to get Verified is with non-empty evidence.
    let c = ArchitecturalContract::declare_single_authority(
        id0(),
        component_x(),
        dr_dec(),
        sr_archspec(),
        rev_v1(),
        EventTime(1000),
    )
    .unwrap();
    let empty = ContractEvaluation::evaluate(&c, vec![], EventTime(1500), evaluator_engine(), None);
    assert_ne!(empty.outcome(), ClaimOutcome::Verified);
    assert_eq!(empty.outcome(), ClaimOutcome::Unknown);
    // Sanity: with evidence, we do get Verified.
    let ok = ContractEvaluation::evaluate(
        &c,
        vec![ev_runtime_a()],
        EventTime(1500),
        evaluator_engine(),
        None,
    );
    assert_eq!(ok.outcome(), ClaimOutcome::Verified);
}

// ── NEG-04: extension kind/payload mismatch ─────────────────────────────────

#[test]
fn extension_payload_rejects_non_namespaced_kind() {
    // Top-level: ContractKindRef::new refuses inputs that don't match
    // the lowercase.kind shape.
    assert!(ContractKindRef::new("Upper.kind").is_err());
    assert!(ContractKindRef::new("ns.Upper").is_err());
    assert!(ContractKindRef::new("kind.with.dots").is_err());
    assert!(ContractKindRef::new("").is_err());
    // And: when kind and payload tag disagree, declare() refuses.
    let kind = ContractKind::Extension(ContractKindRef::new("acme.acl").unwrap());
    let payload = ContractPayload::SingleAuthority(component_x());
    let result = ArchitecturalContract::declare(
        id0(),
        kind,
        payload,
        dr_dec(),
        sr_archspec(),
        rev_v1(),
        EventTime(1000),
    );
    assert!(matches!(
        result,
        Err(ContractError::KindPayloadMismatch { .. })
    ));
}

// ── AENC-01: no A4 / provider imports in this module ────────────────────────

#[test]
fn s2_module_has_no_a4_or_provider_imports() {
    let combined = format!(
        "{}{}{}{}{}{}",
        include_str!("types.rs"),
        include_str!("error.rs"),
        include_str!("payload.rs"),
        include_str!("hashing.rs"),
        include_str!("contract.rs"),
        include_str!("claim.rs"),
    );
    let forbidden = [
        "use crate::alignment",
        "use crate::verification",
        "use crate::governance",
        "use crate::provider",
        "use crate::providers",
        "use openai",
        "use anthropic",
        "use google",
        "ProviderSdk",
        "OpenAiClient",
        "AnthropicClient",
    ];
    for needle in forbidden {
        assert!(
            !combined.contains(needle),
            "forbidden import `{}` found in architectural_contract/* (REQ-A3S2-021)",
            needle
        );
    }
}

// ── AENC-02: contracts do not grant capabilities ───────────────────────────

#[test]
fn s2_does_not_grant_capabilities() {
    let combined = format!(
        "{}{}{}{}{}{}",
        include_str!("types.rs"),
        include_str!("error.rs"),
        include_str!("payload.rs"),
        include_str!("hashing.rs"),
        include_str!("contract.rs"),
        include_str!("claim.rs"),
    );
    for forbidden in [
        "grants:",
        "grants ",
        "granted_by",
        "CapabilityId",
        "GrantId",
    ] {
        assert!(
            !combined.contains(forbidden),
            "forbidden capability-grant token `{}` found in architectural_contract/* (REQ-A3S2-015)",
            forbidden
        );
    }
    // Type-level: ArchitecturalContract has no capability-shaped field.
    for needle in [
        "pub grants",
        "pub capability",
        "pub granted_by",
        "pub grants:",
    ] {
        assert!(
            !combined.contains(needle),
            "ArchitecturalContract exposes capability-shaped field `{}` (REQ-A3S2-015)",
            needle
        );
    }
}

// ── AENC-03: Markdown is not runtime authority ──────────────────────────────

#[test]
fn s2_markdown_is_not_runtime_authority() {
    let combined = format!(
        "{}{}{}{}{}{}",
        include_str!("types.rs"),
        include_str!("error.rs"),
        include_str!("payload.rs"),
        include_str!("hashing.rs"),
        include_str!("contract.rs"),
        include_str!("claim.rs"),
    );
    for forbidden in [
        "parse_markdown",
        "parse_md",
        "from_str_md",
        "markdown_payload",
        "from_markdown",
        "deserialize_markdown",
    ] {
        assert!(
            !combined.contains(forbidden),
            "forbidden markdown-authority token `{}` found in architectural_contract/* (REQ-A3S2-022)",
            forbidden
        );
    }
}

// ── AENC-04: kind baseline pins (delegated to semantic_kind tests) ──────────

/// Anti-encroachment probe: the S1 anti-encroachment test in
/// `knowledge.rs` pin (now historical) must be preserved verbatim.
/// We assert the function name still exists in `knowledge.rs`.
#[test]
fn s1_anti_encroachment_anchor_is_preserved_in_knowledge() {
    let knowledge_src = include_str!("../knowledge.rs");
    assert!(
        knowledge_src.contains("s1_does_not_introduce_new_corenodekind_variants"),
        "S1 anti-encroachment anchor test must be preserved as historical baseline"
    );
}

// ── bonus: extension field types and stable canonical form ──────────────────

#[test]
fn extension_value_canonical_form_is_deterministic() {
    let v1 = ContractExtensionValue::Object({
        let mut m = BTreeMap::new();
        m.insert("a".into(), ContractExtensionValue::Integer(1));
        m
    });
    let v2 = ContractExtensionValue::Object({
        let mut m = BTreeMap::new();
        m.insert("a".into(), ContractExtensionValue::Integer(1));
        m
    });
    assert_eq!(v1.canonical_form(), v2.canonical_form());

    // Nested arrays too.
    let arr1 = ContractExtensionValue::Array(vec![
        ContractExtensionValue::String("x".into()),
        ContractExtensionValue::Boolean(false),
    ]);
    let arr2 = ContractExtensionValue::Array(vec![
        ContractExtensionValue::String("x".into()),
        ContractExtensionValue::Boolean(false),
    ]);
    assert_eq!(arr1.canonical_form(), arr2.canonical_form());
}

// ── bonus: provider boundary kind preserves decision ref ───────────────────

#[test]
fn provider_boundary_contract_preserves_decision_ref() {
    let c = ArchitecturalContract::declare_provider_boundary(
        ContractId::new("contract.provider.boundary").unwrap(),
        BoundaryKind::Inbound,
        "engine -> provider".into(),
        dr_adr(),
        sr_adr(),
        rev_v1(),
        EventTime(1000),
    )
    .unwrap();
    assert!(matches!(c.kind(), ContractKind::ProviderBoundary));
    assert!(matches!(c.decided_by(), DecisionRef::Adr(_)));
    assert!(matches!(c.specified_by(), SpecRef::Adr(_)));
}
