// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// architecture_why/tests.rs — A3-S15 acceptance tests.
//
// SEE: docs/architecture/specs/arch-spec-A3-S15-why-architecture.md

use std::collections::BTreeMap;

use crate::architectural_contract::{
    ArchitecturalContract, ArchitectureClaim, ClaimOutcome, ComponentRef, ContractEvaluation,
    ContractId, DecisionRef, EvaluatorRef, Revision, SpecRef,
};
use crate::architecture_conformance::contract_set_digest;
use crate::architecture_debverify::{
    DebVerifyAudit, DebVerifyFinding, DebVerifyFindingKind, FindingBasis, FindingId,
    FindingSeverity, audit_digest, run_debverify_audit,
};
use crate::architecture_graph::{
    ArchitectureGraphOverlay, SoftwareUnit, SoftwareUnitRef, UnitKind,
};
use crate::knowledge::EventTime;
use crate::semantic_graph::SemanticGraphProjection;

use super::traverse::{WhyInput, explain};
use super::types::{WhyNotReason, WhyResolvedAs};

const T0: i64 = 1000;

fn comp(s: &str) -> ComponentRef {
    ComponentRef::new(s).expect("valid component")
}

fn cid(s: &str) -> ContractId {
    ContractId::new(s).expect("valid contract id")
}

fn authority(id: &str, component: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_single_authority(
        cid(id),
        comp(component),
        DecisionRef::Adr("ADR-0112".into()),
        SpecRef::ArchSpec("arch-spec-032".into()),
        Revision::new("rev:1").expect("revision"),
        EventTime(T0),
    )
    .expect("declare")
}

/// A fixture mirroring the CLI's seam: contracts, the AC2 overlay with linkage
/// claims, and the AC5 audit — all computed once, exactly as the surfaces do.
struct Fixture {
    contracts: Vec<ArchitecturalContract>,
    claims: BTreeMap<String, ArchitectureClaim>,
    overlay: ArchitectureGraphOverlay,
    audit: DebVerifyAudit,
    basis: FindingBasis,
}

fn build(contracts: Vec<ArchitecturalContract>, units: &[&str]) -> Fixture {
    let now = EventTime(T0 + 2);
    let mut overlay = ArchitectureGraphOverlay::new();
    // Mirror `declare_overlay`: units are nodes before anything references them,
    // so a claim→unit relation terminates at a real unit node.
    for u in units {
        overlay.add_unit(&SoftwareUnit::new(
            SoftwareUnitRef::new((*u).to_string()),
            UnitKind::Module,
            format!("src/{u}.rs"),
        ));
    }
    let mut claims: BTreeMap<String, ArchitectureClaim> = BTreeMap::new();
    let evaluator = EvaluatorRef::new("test.linkage").expect("evaluator");
    for c in &contracts {
        let subject = match c.payload() {
            crate::architectural_contract::ContractPayload::SingleAuthority(s) => {
                s.as_str().to_string()
            }
            crate::architectural_contract::ContractPayload::UniqueOwner(s) => {
                s.as_str().to_string()
            }
            _ => continue,
        };
        // Only declared units are linked, mirroring `declare_overlay`.
        if !units.contains(&subject.as_str()) {
            continue;
        }
        let claim = ContractEvaluation::evaluate(c, Vec::new(), now, evaluator.clone(), None);
        claims.insert(c.id().as_str().to_string(), claim.clone());
        let claim_id = overlay.add_claim(&claim);
        overlay.attach_claim_to_unit(&claim, &claim_id, &SoftwareUnitRef::new(subject));
        overlay.add_contract_metadata(c, c.decided_by(), c.specified_by(), &[]);
    }
    let audit = run_debverify_audit(&overlay, &contracts, now).expect("audit");
    let basis = FindingBasis::new("rev:1", "kb:test", contract_set_digest(&contracts));
    Fixture {
        contracts,
        claims,
        overlay,
        audit,
        basis,
    }
}

fn explain_finding(
    f: &Fixture,
    kind: DebVerifyFindingKind,
    subject: &str,
) -> super::types::ArchitectureWhy {
    let index = f
        .audit
        .findings
        .iter()
        .position(|x| x.kind == kind && x.subjects.iter().any(|s| s == subject))
        .unwrap_or_else(|| panic!("no {kind:?} finding on {subject} in {:?}", f.audit.findings));
    let finding = &f.audit.findings[index];
    let id = FindingId::derive(
        &f.basis,
        finding.kind,
        &finding.subjects,
        &finding.contract_ids,
    );
    let id = id.as_str().to_string();
    explain(WhyInput {
        query: &id,
        resolved_as: WhyResolvedAs::Finding,
        basis: &f.basis,
        contracts: &f.contracts,
        claims: &f.claims,
        audit: &f.audit,
        overlay: &f.overlay,
        finding: Some((&id, index)),
    })
}

#[test]
fn acceptance_model_keeps_provenance_classes_apart() {
    // REQ-A3S15-014: ASSESSMENT, DECLARED and OBSERVED live in separate fields.
    // A merged narrative blob would make the strength of each statement
    // unreadable, which is the whole point of the answer.
    let f = build(
        vec![authority("c-a", "comp:x"), authority("c-b", "comp:x")],
        &["comp:x"],
    );
    let why = explain_finding(&f, DebVerifyFindingKind::ShadowAuthority, "comp:x");

    assert_eq!(why.contracts.len(), 2, "cardinality preserved");
    let leg = &why.contracts[0];
    // ASSESSMENT
    assert_eq!(leg.assessment.outcome, "unknown");
    assert!(!leg.assessment.evidence_present);
    // DECLARED
    assert_eq!(leg.intent.decisions, vec!["ADR-0112".to_string()]);
    assert_eq!(leg.intent.specs, vec!["arch-spec-032".to_string()]);
    // OBSERVED
    assert!(leg.evidence.is_empty(), "no evidence was supplied");
    // The three are separate fields, so a reader never conflates them.
    assert!(leg.participates_because.contains("comp:x"));
    assert!(
        !leg.participates_because.contains(&leg.assessment.outcome),
        "the participation text must be derived from kind + payload, not from the assessment"
    );
}

#[test]
fn acceptance_evidence_to_software_is_unresolved() {
    // REQ-A3S15-012: the leg does not exist in the substrate. It must be
    // reported, never inferred.
    let f = build(
        vec![authority("c-a", "comp:x"), authority("c-b", "comp:x")],
        &["comp:x"],
    );
    let why = explain_finding(&f, DebVerifyFindingKind::ShadowAuthority, "comp:x");
    assert!(
        why.unresolved_edges
            .iter()
            .any(|u| u.edge == super::types::ArchitectureWhy::EVIDENCE_TO_SOFTWARE_EDGE),
        "the evidence→software leg must always be reported as unresolved"
    );
    let u = why
        .unresolved_edges
        .iter()
        .find(|u| u.edge == super::types::ArchitectureWhy::EVIDENCE_TO_SOFTWARE_EDGE)
        .expect("present");
    assert!(
        u.reason.contains("no evidence node") || u.reason.contains("metadata"),
        "the reason must name the substrate gap: {}",
        u.reason
    );
    // And nothing pretends the leg exists.
    for leg in &why.contracts {
        assert!(leg.evidence.is_empty());
    }
}

#[test]
fn acceptance_why_not_adt_is_prepared() {
    // REQ-A3S15-015: the ADT exists and is populated from the claims' own
    // missing_evidence. A surface is deliberately not created.
    let f = build(
        vec![authority("c-a", "comp:x"), authority("c-b", "comp:x")],
        &["comp:x"],
    );
    let why = explain_finding(&f, DebVerifyFindingKind::ShadowAuthority, "comp:x");
    assert!(
        why.why_not
            .iter()
            .any(|r| matches!(r, WhyNotReason::MissingEvidence { .. })),
        "unknown outcomes carry their missing evidence: {:?}",
        why.why_not
    );
    assert!(
        why.why_not
            .iter()
            .any(|r| matches!(r, WhyNotReason::UnknownRelation { .. })),
        "the unresolved leg is also a why-not reason"
    );
}

#[test]
fn acceptance_contract_without_a_claim_is_not_evaluated() {
    // REQ-A3S15-018 (engine half): a contract the overlay could not link — its
    // subject is not a declared unit — is `not_evaluated` with a stated reason.
    // It must never appear as a silent blank leg, which would read as "fine".
    let f = build(vec![authority("c-orphan", "comp:nowhere")], &["comp:x"]);
    assert!(f.claims.is_empty(), "nothing linked for a dangling subject");
    let idx = f
        .audit
        .findings
        .iter()
        .position(|x| x.kind == DebVerifyFindingKind::MissingOwner)
        .expect("missing_owner fires");
    let finding = &f.audit.findings[idx];
    let id = FindingId::derive(
        &f.basis,
        finding.kind,
        &finding.subjects,
        &finding.contract_ids,
    )
    .as_str()
    .to_string();
    let why = explain(WhyInput {
        query: &id,
        resolved_as: WhyResolvedAs::Finding,
        basis: &f.basis,
        contracts: &f.contracts,
        claims: &f.claims,
        audit: &f.audit,
        overlay: &f.overlay,
        finding: Some((&id, idx)),
    });
    let leg = why
        .contracts
        .iter()
        .find(|c| c.contract == "c-orphan")
        .expect("the leg exists");
    assert_eq!(leg.assessment.outcome, "not_evaluated");
    assert!(leg.evidence.is_empty());
    assert!(leg.software_units.is_empty());
    assert!(
        why.why_not
            .iter()
            .any(|r| matches!(r, WhyNotReason::ContractNotEvaluable { .. })),
        "an unevaluable contract must state why: {:?}",
        why.why_not
    );
}

#[test]
fn acceptance_traversal_is_deterministic() {
    // REQ-A3S16-016 (engine half): same inputs, same answer, byte for byte.
    let f = build(
        vec![authority("c-b", "comp:x"), authority("c-a", "comp:x")],
        &["comp:x"],
    );
    let a = explain_finding(&f, DebVerifyFindingKind::ShadowAuthority, "comp:x");
    let b = explain_finding(&f, DebVerifyFindingKind::ShadowAuthority, "comp:x");
    assert_eq!(a, b);
    // Legs are ordered by contract id, independent of declaration order.
    let ids: Vec<&str> = a.contracts.iter().map(|c| c.contract.as_str()).collect();
    assert_eq!(ids, vec!["c-a", "c-b"]);
}

#[test]
fn acceptance_query_resolved_as_contract_explains_one_leg() {
    // REQ-A3S15-005 (engine half): resolving as a contract explains exactly that
    // contract, with no finding attached.
    let f = build(
        vec![authority("c-a", "comp:x"), authority("c-b", "comp:x")],
        &["comp:x"],
    );
    let why = explain(WhyInput {
        query: "c-a",
        resolved_as: WhyResolvedAs::Contract,
        basis: &f.basis,
        contracts: &f.contracts,
        claims: &f.claims,
        audit: &f.audit,
        overlay: &f.overlay,
        finding: None,
    });
    assert!(why.finding.is_none());
    assert_eq!(why.contracts.len(), 1);
    assert_eq!(why.contracts[0].contract, "c-a");
    // The software leg is reachable even without a finding, via the overlay.
    assert_eq!(why.contracts[0].software_units, vec!["comp:x".to_string()]);
    // The unresolved leg is reported regardless of how the query resolved.
    assert_eq!(why.unresolved_edges.len(), 1);
}

#[test]
fn acceptance_audit_error_is_not_empty_explanation() {
    // REQ-A3S15-018: an audit that could not run must not present as "nothing
    // found". The engine's contribution is that the *audit object is required*:
    // `explain` cannot be called without one, so there is no code path where a
    // missing audit degrades into an empty answer. This asserts the shape that
    // makes that true — the audit digest is over the findings, so an empty audit
    // and an unavailable one are distinguishable values.
    let empty: Vec<DebVerifyFinding> = Vec::new();
    let graph_digest = [1u8; 32];
    let empty_digest = audit_digest(&empty, &graph_digest);

    let one = vec![DebVerifyFinding {
        kind: DebVerifyFindingKind::MissingOwner,
        severity: FindingSeverity::High,
        subjects: vec!["comp:missing".to_string()],
        contract_ids: vec![cid("c-orphan")],
        message: "test".to_string(),
    }];
    assert_ne!(
        empty_digest,
        audit_digest(&one, &graph_digest),
        "an empty audit and a populated one are different values"
    );
    // And `explain` on a contract absent from the contract set reports the gap
    // rather than an empty leg, which is the observable form of the same rule.
    let f = build(vec![authority("c-a", "comp:x")], &["comp:x"]);
    let why = explain(WhyInput {
        query: "c-not-declared",
        resolved_as: WhyResolvedAs::Contract,
        basis: &f.basis,
        contracts: &f.contracts,
        claims: &f.claims,
        audit: &f.audit,
        overlay: &f.overlay,
        finding: None,
    });
    assert_eq!(why.contracts.len(), 1);
    assert_eq!(why.contracts[0].assessment.outcome, "not_evaluated");
    assert!(
        why.contracts[0]
            .participates_because
            .contains("not present"),
        "{}",
        why.contracts[0].participates_because
    );
    assert!(
        why.why_not
            .iter()
            .any(|r| matches!(r, WhyNotReason::ContractNotEvaluable { .. }))
    );
}

#[test]
fn acceptance_claim_outcome_maps_through() {
    // The assessment reuses AC4's vocabulary rather than minting a second one.
    let _ = ClaimOutcome::Unknown;
    let f = build(vec![authority("c-a", "comp:x")], &["comp:x"]);
    let why = explain(WhyInput {
        query: "c-a",
        resolved_as: WhyResolvedAs::Contract,
        basis: &f.basis,
        contracts: &f.contracts,
        claims: &f.claims,
        audit: &f.audit,
        overlay: &f.overlay,
        finding: None,
    });
    let outcome = &why.contracts[0].assessment.outcome;
    assert!(
        [
            "verified",
            "contradicted",
            "unknown",
            "stale",
            "not_evaluated"
        ]
        .contains(&outcome.as_str()),
        "assessment outcome must be AC4's closed vocabulary, got {outcome}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Verification probes (A3-S15)
// ─────────────────────────────────────────────────────────────────────────────

/// Rebuild the projection from the same inputs, the way every CLI invocation
/// does, and assert the `SpecifiedBy` edge survives and the WHY is unchanged.
///
/// NOTE on scope: `SemanticGraphProjection::rebuild_from_canonical` is the
/// intended rebuild path but has no adapter for the AC2 overlay (no caller
/// anywhere in the workspace), so the rebuild exercised here is the one the
/// system actually performs: a fresh overlay built from the same declaration.
/// The missing adapter is recorded as a follow-up, not papered over.
#[test]
fn acceptance_specified_by_survives_a_projection_rebuild() {
    let contracts = vec![authority("c-a", "comp:x"), authority("c-b", "comp:x")];

    let a = build(contracts.clone(), &["comp:x"]);
    let b = build(contracts, &["comp:x"]);

    // Same inputs => same projection bytes.
    assert_eq!(
        a.overlay.projection().canonical_bytes(),
        b.overlay.projection().canonical_bytes(),
        "a rebuild from identical inputs must produce identical projection bytes"
    );

    // The edge is present in both, with the same endpoints.
    let specified = |f: &Fixture| -> Vec<(String, String)> {
        let kind = crate::architecture_graph::ArchitectureOverlayRelationKind::SpecifiedBy
            .as_relation_kind()
            .expect("tag");
        let mut v: Vec<(String, String)> = f
            .overlay
            .projection()
            .relations()
            .into_iter()
            .filter(|r| r.kind == kind)
            .map(|r| (r.from.as_str().to_string(), r.to.as_str().to_string()))
            .collect();
        v.sort();
        v
    };
    let sa = specified(&a);
    assert_eq!(sa.len(), 2, "one SpecifiedBy edge per contract");
    assert_eq!(sa, specified(&b), "same endpoints after rebuild");

    // And the answer is the same, which is what "semantically equivalent" means.
    let wa = explain_finding(&a, DebVerifyFindingKind::ShadowAuthority, "comp:x");
    let wb = explain_finding(&b, DebVerifyFindingKind::ShadowAuthority, "comp:x");
    assert_eq!(wa, wb);
    for leg in &wa.contracts {
        assert_eq!(leg.intent.specs, vec!["arch-spec-032".to_string()]);
    }
}

/// A finding is ASSESSMENT. Explaining it must not promote it into the graph.
#[test]
fn acceptance_finding_stays_an_assessment() {
    let f = build(
        vec![authority("c-a", "comp:x"), authority("c-b", "comp:x")],
        &["comp:x"],
    );
    let before = f.overlay.projection().canonical_bytes();
    let why = explain_finding(&f, DebVerifyFindingKind::ShadowAuthority, "comp:x");
    let after = f.overlay.projection().canonical_bytes();

    // `explain` takes `&ArchitectureGraphOverlay`, so mutation is impossible by
    // construction; this asserts the consequence the reader depends on.
    assert_eq!(
        before, after,
        "explaining a finding must not change the graph"
    );

    // The finding is not a node kind, and no finding kind is projected as one.
    use crate::architecture_graph::ArchitectureOverlayNodeKind;
    use crate::semantic_kind::NodeKind;
    let node_tags: Vec<String> = f
        .overlay
        .projection()
        .nodes()
        .into_iter()
        .map(|n| match n.kind {
            NodeKind::Extension(k) => k.as_str().to_string(),
            other => format!("{other:?}"),
        })
        .collect();
    assert!(
        !node_tags
            .iter()
            .any(|t| t.contains("shadow_authority") || t.contains("contradiction")),
        "an AC5 finding kind must never appear as a graph node kind: {node_tags:?}"
    );
    assert!(
        node_tags
            .iter()
            .any(|t| t == ArchitectureOverlayNodeKind::ArchitectureClaim.domain_tag()),
        "the AC1 claim is the projected object, not the AC5 finding"
    );

    // The finding carries no observation of its own: OBSERVED lives on the legs.
    let finding = why.finding.as_ref().expect("finding");
    assert!(finding.contract_ids.len() == 2);
    assert!(why.contracts.iter().all(|c| c.evidence.is_empty()));
}

/// The audit is total today, so the failure path is pinned as a structural
/// invariant rather than a behavioural fixture — and this records *why*.
#[test]
fn acceptance_audit_error_surface_is_reserved_not_reachable() {
    // `DebVerifyError` has exactly one variant and it is documented as reserved:
    // current detectors never fail because subject identifiers are always
    // readable. That is the honest reason no fixture can force an audit error.
    let variants = crate::architecture_debverify::DebVerifyError::UnreadableSubject {
        contract: cid("c-x"),
    };
    assert!(variants.to_string().contains("c-x"));

    // What must hold regardless: an unevaluable contract is `not_evaluated` with
    // a stated reason, never a blank leg and never an implicit "fine".
    let f = build(vec![authority("c-orphan", "comp:nowhere")], &["comp:x"]);
    let idx = f
        .audit
        .findings
        .iter()
        .position(|x| x.kind == DebVerifyFindingKind::MissingOwner)
        .expect("missing_owner");
    let finding = &f.audit.findings[idx];
    let id = FindingId::derive(
        &f.basis,
        finding.kind,
        &finding.subjects,
        &finding.contract_ids,
    )
    .as_str()
    .to_string();
    let why = explain(WhyInput {
        query: &id,
        resolved_as: WhyResolvedAs::Finding,
        basis: &f.basis,
        contracts: &f.contracts,
        claims: &f.claims,
        audit: &f.audit,
        overlay: &f.overlay,
        finding: Some((&id, idx)),
    });
    assert!(!why.contracts.is_empty(), "never an empty explanation");
    assert_eq!(why.contracts[0].assessment.outcome, "not_evaluated");
    assert!(!why.why_not.is_empty(), "the gap is stated, not implied");
}
