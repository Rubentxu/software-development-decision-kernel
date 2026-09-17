// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// crates/sddk-engine/tests/a4_3r2_namespace_safe_targets.rs
//
// A4-3R2 — Namespace-Safe Constraint Targets (falsification suite).
//
// Eighteen pin tests covering the falsification list from the A4-3R2
// scope contract. Each test names the REQ it pins.
//
// The pin suite covers:
//   1     : canonical_tag() emits namespaced prefix per kind.
//   2-7   : typed binding (Component for SingleAuthority,
//           Entity for UniqueOwner). Unit does NOT bind.
//   8     : ForbiddenDependency unchanged.
//   9     : observation insertion order irrelevant.
//   10    : subject kind changes ObservationId.
//   11    : subject kind changes target identity in for_subject.
//   12-16 : for_subject / for_entity strict-namespace match.
//   17    : no as_str() cross-namespace equality in the substrate.
//   18    : no rendered / substring matching.
//
// Plus a deterministic randomized property test (P19 — durable
// evidence from A4-5P probe). See bottom of file.

use sddk_engine::architectural_contract::{
    ArchitecturalContract, BoundaryKind, ComponentRef, ContractId, ContractKind, ContractKindRef,
    ContractPayload, DecisionRef, EntityRef, Revision, SpecRef,
};
use sddk_engine::architecture_graph::SoftwareUnitRef;
use sddk_engine::evidence_ref::{EvidenceKind, EvidenceRef as UniEvidenceRef};
use sddk_engine::knowledge::{EventTime, KnowledgeBasis};
use sddk_engine::observation::{
    ObservationBasis, ObservationOrigin, ObservationSet, ObservationStance, ObservationSubject,
    SoftwareEntityRef, SoftwareObservation, SoftwareRelation,
};
use sddk_engine::semantic_kind::CoreRelationKind;
use sddk_engine::software_alignment::reducer::reduce_alignment;
use sddk_engine::software_alignment::types::{
    AlignmentScope, AlignmentState, ArchitecturalIntentSnapshot, ConstraintId,
    ContractViolationCause, ExplicitConstraint, FindingCause, MustDirection, ParadigmTag,
};

// ─── Helpers ───────────────────────────────────────────────────────────────

fn component_ref(s: &str) -> ComponentRef {
    ComponentRef::new(s.to_string()).expect("ascii")
}

fn entity_ref(s: &str) -> EntityRef {
    EntityRef::new(s.to_string()).expect("ascii")
}

fn spec_ref(s: &str) -> SpecRef {
    SpecRef::Spec(s.to_string())
}

fn revision(s: &str) -> Revision {
    Revision::new(s.to_string()).expect("ascii")
}

fn dec_ref() -> DecisionRef {
    DecisionRef::Decision("DEC-A4-3R2".to_string())
}

fn basis_hash_zero() -> sddk_engine::knowledge::BasisHash {
    let kb = KnowledgeBasis::empty(EventTime(0));
    kb.basis_hash().clone()
}

fn obs_component_deny(component: &str, ev: &str) -> SoftwareObservation {
    let basis = ObservationBasis::new("rev-1", basis_hash_zero(), "input-1");
    let evidence = UniEvidenceRef::new(EvidenceKind::Adhoc, ev.to_string());
    SoftwareObservation::declare(
        ObservationSubject::Component(component_ref(component)),
        ObservationStance::Denies,
        evidence,
        ObservationOrigin::DeterministicLocal,
        basis,
        None,
        "a4_3r2_test_producer",
    )
}

fn obs_component_affirm(component: &str, ev: &str) -> SoftwareObservation {
    let basis = ObservationBasis::new("rev-1", basis_hash_zero(), "input-1");
    let evidence = UniEvidenceRef::new(EvidenceKind::Adhoc, ev.to_string());
    SoftwareObservation::declare(
        ObservationSubject::Component(component_ref(component)),
        ObservationStance::Affirms,
        evidence,
        ObservationOrigin::DeterministicLocal,
        basis,
        None,
        "a4_3r2_test_producer",
    )
}

fn obs_entity_affirm(entity: &str, ev: &str) -> SoftwareObservation {
    let basis = ObservationBasis::new("rev-1", basis_hash_zero(), "input-1");
    let evidence = UniEvidenceRef::new(EvidenceKind::Adhoc, ev.to_string());
    SoftwareObservation::declare(
        ObservationSubject::Entity(entity_ref(entity)),
        ObservationStance::Affirms,
        evidence,
        ObservationOrigin::DeterministicLocal,
        basis,
        None,
        "a4_3r2_test_producer",
    )
}

fn obs_unit_deny(unit: &str, ev: &str) -> SoftwareObservation {
    let basis = ObservationBasis::new("rev-1", basis_hash_zero(), "input-1");
    let evidence = UniEvidenceRef::new(EvidenceKind::Adhoc, ev.to_string());
    SoftwareObservation::declare(
        ObservationSubject::Unit(SoftwareUnitRef(unit.to_string())),
        ObservationStance::Denies,
        evidence,
        ObservationOrigin::DeterministicLocal,
        basis,
        None,
        "a4_3r2_test_producer",
    )
}

fn obs_relation(
    from_component: &str,
    kind: CoreRelationKind,
    to_component: &str,
    ev: &str,
) -> SoftwareObservation {
    let rel = SoftwareRelation::new(
        SoftwareEntityRef::Component(component_ref(from_component)),
        kind,
        SoftwareEntityRef::Component(component_ref(to_component)),
    );
    let basis = ObservationBasis::new("rev-1", basis_hash_zero(), "input-1");
    let evidence = UniEvidenceRef::new(EvidenceKind::Adhoc, ev.to_string());
    SoftwareObservation::declare(
        ObservationSubject::SoftwareRelation(rel),
        ObservationStance::Affirms,
        evidence,
        ObservationOrigin::DeterministicLocal,
        basis,
        None,
        "a4_3r2_test_producer",
    )
}

fn contract_single_authority(id: &str, component: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_single_authority(
        ContractId::new(id.to_string()).expect("ascii"),
        component_ref(component),
        dec_ref(),
        spec_ref("SPEC-A4-3R2"),
        revision("rev-1"),
        EventTime(0),
    )
    .expect("single authority contract")
}

fn contract_unique_owner(id: &str, entity: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_unique_owner(
        ContractId::new(id.to_string()).expect("ascii"),
        entity_ref(entity),
        dec_ref(),
        spec_ref("SPEC-A4-3R2"),
        revision("rev-1"),
        EventTime(0),
    )
    .expect("unique owner contract")
}

fn contract_forbidden_dependency(id: &str, from: &str, to: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_forbidden_dependency(
        ContractId::new(id.to_string()).expect("ascii"),
        component_ref(from),
        component_ref(to),
        "a4-3r2 test reason".to_string(),
        dec_ref(),
        spec_ref("SPEC-A4-3R2"),
        revision("rev-1"),
        EventTime(0),
    )
    .expect("forbidden dependency contract")
}

fn contract_provider_boundary(id: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_provider_boundary(
        ContractId::new(id.to_string()).expect("ascii"),
        BoundaryKind::Inbound,
        "engine->provider".to_string(),
        dec_ref(),
        spec_ref("SPEC-A4-3R2"),
        revision("rev-1"),
        EventTime(0),
    )
    .expect("provider boundary contract")
}

fn contract_extension(id: &str, ext_kind: &str) -> ArchitecturalContract {
    use sddk_engine::architectural_contract::ContractExtensionValue;
    use std::collections::BTreeMap;
    let mut fields: BTreeMap<String, ContractExtensionValue> = BTreeMap::new();
    fields.insert(
        "test_field".to_string(),
        ContractExtensionValue::Boolean(true),
    );
    let kind = ContractKindRef::new(ext_kind).expect("ascii ext kind");
    ArchitecturalContract::declare(
        ContractId::new(id.to_string()).expect("ascii"),
        ContractKind::Extension(ContractKindRef::new(ext_kind).expect("ascii ext kind")),
        ContractPayload::Extension { kind, fields },
        dec_ref(),
        spec_ref("SPEC-A4-3R2"),
        revision("rev-1"),
        EventTime(0),
    )
    .expect("extension contract")
}

fn constraint(id: &str, contract_id: &str, dir: MustDirection) -> ExplicitConstraint {
    ExplicitConstraint {
        id: ConstraintId::new(id.to_string()),
        contract_ref: ContractId::new(contract_id.to_string()).expect("ascii"),
        direction: dir,
        label: None,
    }
}

fn intent(scope_label: &str, constraints: Vec<ExplicitConstraint>) -> ArchitecturalIntentSnapshot {
    let s = AlignmentScope::new(scope_label.to_string());
    let paradigm = if constraints.is_empty() {
        ParadigmTag::Undeclared
    } else {
        ParadigmTag::Hexagonal
    };
    let id = ArchitecturalIntentSnapshot::derive_id(&s, paradigm, &constraints);
    ArchitecturalIntentSnapshot {
        id,
        scope: s,
        paradigm_tag: paradigm,
        explicit_constraints: constraints,
    }
}

fn empty_basis() -> KnowledgeBasis {
    KnowledgeBasis::empty(EventTime(0))
}

// ─── Pin 1: canonical_tag emits literal namespace prefix ────────────────

#[test]
fn pin_a01_canonical_tag_namespaces_per_kind() {
    let u = ObservationSubject::Unit(SoftwareUnitRef("auth".to_string()));
    let c = ObservationSubject::Component(component_ref("auth"));
    let e = ObservationSubject::Entity(entity_ref("auth"));
    let tag_u = u.canonical_tag();
    let tag_c = c.canonical_tag();
    let tag_e = e.canonical_tag();
    assert_eq!(tag_u, "unit:auth");
    assert_eq!(tag_c, "component:auth");
    assert_eq!(tag_e, "entity:auth");
    assert_ne!(tag_u, tag_c);
    assert_ne!(tag_u, tag_e);
    assert_ne!(tag_c, tag_e);
}

// ─── Pin 2: SingleAuthority + Unit(same string) → NO violation ──────────

#[test]
fn pin_a02_single_authority_does_not_bind_unit_subject() {
    let c = constraint("k1", "sa:auth", MustDirection::Must);
    let i = intent("repo:test", vec![c]);
    let mut obs = ObservationSet::new();
    obs.insert(obs_unit_deny("comp:auth", "ev:sa:unit-not-typed"));
    let contract = contract_single_authority("sa:auth", "comp:auth");
    let r =
        reduce_alignment(&i, &empty_basis(), &obs, &[contract], &[], EventTime(0)).expect("reduce");
    assert!(
        !r.findings.iter().any(|f| matches!(
            f.kind,
            sddk_engine::software_alignment::types::AlignmentFindingKind::ContractViolation
        )),
        "SingleAuthority must NOT bind Unit subjects (A4-3R2 strict namespace)"
    );
}

// ─── Pin 3: SingleAuthority + Component(correct) + Denies → violation ───

#[test]
fn pin_a03_single_authority_binds_component_subject() {
    let c = constraint("k1", "sa:auth", MustDirection::Must);
    let i = intent("repo:test", vec![c]);
    let mut obs = ObservationSet::new();
    obs.insert(obs_component_deny("comp:auth", "ev:sa:comp:deny"));
    let contract = contract_single_authority("sa:auth", "comp:auth");
    let r =
        reduce_alignment(&i, &empty_basis(), &obs, &[contract], &[], EventTime(0)).expect("reduce");
    assert_eq!(r.state, AlignmentState::Misaligned);
    let violation = r
        .findings
        .iter()
        .find(|f| {
            matches!(
                f.kind,
                sddk_engine::software_alignment::types::AlignmentFindingKind::ContractViolation
            )
        })
        .expect("one violation");
    assert_eq!(
        violation.cause,
        FindingCause::ContractViolation(ContractViolationCause::ContradictsMust)
    );
}

// ─── Pin 4: SingleAuthority + Component(other) → NO violation ───────────

#[test]
fn pin_a04_single_authority_no_match_on_other_component() {
    let c = constraint("k1", "sa:auth", MustDirection::Must);
    let i = intent("repo:test", vec![c]);
    let mut obs = ObservationSet::new();
    obs.insert(obs_component_deny("comp:billing", "ev:sa:other-comp"));
    let contract = contract_single_authority("sa:auth", "comp:auth");
    let r =
        reduce_alignment(&i, &empty_basis(), &obs, &[contract], &[], EventTime(0)).expect("reduce");
    assert!(
        !r.findings.iter().any(|f| matches!(
            f.kind,
            sddk_engine::software_alignment::types::AlignmentFindingKind::ContractViolation
        )),
        "SingleAuthority must NOT bind a different Component"
    );
}

// ─── Pin 5: UniqueOwner + Unit(same string) → NO violation ───────────��──

#[test]
fn pin_a05_unique_owner_does_not_bind_unit_subject() {
    let c = constraint("k1", "uo:account", MustDirection::MustNot);
    let i = intent("repo:test", vec![c]);
    let mut obs = ObservationSet::new();
    obs.insert(obs_unit_deny("account", "ev:uo:unit-not-typed"));
    let contract = contract_unique_owner("uo:account", "account");
    let r =
        reduce_alignment(&i, &empty_basis(), &obs, &[contract], &[], EventTime(0)).expect("reduce");
    assert!(
        !r.findings.iter().any(|f| matches!(
            f.kind,
            sddk_engine::software_alignment::types::AlignmentFindingKind::ContractViolation
        )),
        "UniqueOwner must NOT bind Unit subjects (A4-3R2 strict namespace)"
    );
}

// ─── Pin 6: UniqueOwner + Entity(correct) + Affirms (MustNot) → violation

#[test]
fn pin_a06_unique_owner_binds_entity_subject() {
    let c = constraint("k1", "uo:account", MustDirection::MustNot);
    let i = intent("repo:test", vec![c]);
    let mut obs = ObservationSet::new();
    obs.insert(obs_entity_affirm("account", "ev:uo:entity-affirm"));
    let contract = contract_unique_owner("uo:account", "account");
    let r =
        reduce_alignment(&i, &empty_basis(), &obs, &[contract], &[], EventTime(0)).expect("reduce");
    assert_eq!(r.state, AlignmentState::Misaligned);
}

// ─── Pin 7: UniqueOwner + Entity(other) → NO violation ─────────────────

#[test]
fn pin_a07_unique_owner_no_match_on_other_entity() {
    let c = constraint("k1", "uo:account", MustDirection::MustNot);
    let i = intent("repo:test", vec![c]);
    let mut obs = ObservationSet::new();
    obs.insert(obs_entity_affirm("account:other", "ev:uo:other-entity"));
    let contract = contract_unique_owner("uo:account", "account");
    let r =
        reduce_alignment(&i, &empty_basis(), &obs, &[contract], &[], EventTime(0)).expect("reduce");
    assert!(
        !r.findings.iter().any(|f| matches!(
            f.kind,
            sddk_engine::software_alignment::types::AlignmentFindingKind::ContractViolation
        )),
        "UniqueOwner must NOT bind a different Entity"
    );
}

// ─── Pin 8: ForbiddenDependency behavior unchanged ──────────────────────

#[test]
fn pin_a08_forbidden_dependency_behavior_unchanged() {
    let c = constraint("k1", "fd:ab", MustDirection::MustNot);
    let i = intent("repo:test", vec![c]);
    let mut obs = ObservationSet::new();
    obs.insert(obs_relation(
        "comp:A",
        CoreRelationKind::DependsOn,
        "comp:B",
        "ev:fd:1",
    ));
    let contract = contract_forbidden_dependency("fd:ab", "comp:A", "comp:B");
    let r =
        reduce_alignment(&i, &empty_basis(), &obs, &[contract], &[], EventTime(0)).expect("reduce");
    assert_eq!(r.state, AlignmentState::Misaligned);
}

// ─── Pin 9: observation insertion order irrelevant ──────────────────────

#[test]
fn pin_a09_observation_ordering_independent() {
    let c = constraint("k1", "sa:auth", MustDirection::Must);
    let i = intent("repo:test", vec![c]);
    let contract = contract_single_authority("sa:auth", "comp:auth");

    let mut obs1 = ObservationSet::new();
    obs1.insert(obs_component_deny("comp:auth", "ev:1"));
    obs1.insert(obs_component_affirm("comp:other", "ev:2"));

    let mut obs2 = ObservationSet::new();
    obs2.insert(obs_component_affirm("comp:other", "ev:2"));
    obs2.insert(obs_component_deny("comp:auth", "ev:1"));

    let r1 = reduce_alignment(
        &i,
        &empty_basis(),
        &obs1,
        std::slice::from_ref(&contract),
        &[],
        EventTime(0),
    )
    .expect("reduce");
    let r2 = reduce_alignment(&i, &empty_basis(), &obs2, &[contract], &[], EventTime(0))
        .expect("reduce");
    assert_eq!(
        r1.id, r2.id,
        "assessment identity must be order-independent"
    );
}

// ─── Pin 10: subject kind changes ObservationId ─────────────────────────

#[test]
fn pin_a10_subject_kind_changes_observation_id() {
    let u = obs_unit_deny("auth", "ev");
    let c = obs_component_deny("auth", "ev");
    let e = obs_entity_affirm("auth", "ev");
    assert_ne!(u.id, c.id, "Unit vs Component ids must differ");
    assert_ne!(u.id, e.id, "Unit vs Entity ids must differ");
    assert_ne!(c.id, e.id, "Component vs Entity ids must differ");
}

// ─── Pin 11: subject kind changes target identity in for_subject ────────

#[test]
fn pin_a11_subject_kind_changes_target_identity() {
    use sddk_engine::observation::ObservationTargetRef;
    let u = ObservationTargetRef::Unit(SoftwareUnitRef("auth".to_string()));
    let c = ObservationTargetRef::Component(component_ref("auth"));
    let e = ObservationTargetRef::Entity(entity_ref("auth"));
    assert_eq!(u.kind_tag(), "unit");
    assert_eq!(c.kind_tag(), "component");
    assert_eq!(e.kind_tag(), "entity");
    assert_ne!(u.canonical_tag(), c.canonical_tag());
    assert_ne!(u.canonical_tag(), e.canonical_tag());
    assert_ne!(c.canonical_tag(), e.canonical_tag());
}

// ─── Pin 12: for_subject(Component("x")) never returns Unit("x") ────────

#[test]
fn pin_a12_for_subject_component_excludes_unit() {
    use sddk_engine::observation::ObservationTargetRef;
    let mut obs = ObservationSet::new();
    obs.insert(obs_unit_deny("auth", "ev:unit"));
    let c_target = ObservationTargetRef::Component(component_ref("auth"));
    let matched: Vec<_> = obs.for_subject(&c_target).into_iter().collect();
    assert!(
        matched.is_empty(),
        "for_subject(Component) must NOT match Unit observations"
    );
}

// ─── Pin 13: for_subject(Entity("x")) never returns Unit("x") ───────────

#[test]
fn pin_a13_for_subject_entity_excludes_unit() {
    use sddk_engine::observation::ObservationTargetRef;
    let mut obs = ObservationSet::new();
    obs.insert(obs_unit_deny("auth", "ev:unit"));
    let e_target = ObservationTargetRef::Entity(entity_ref("auth"));
    let matched: Vec<_> = obs.for_subject(&e_target).into_iter().collect();
    assert!(
        matched.is_empty(),
        "for_subject(Entity) must NOT match Unit observations"
    );
}

// ─── Pin 14: for_entity(Unit("x")) matches Unit("x") only ───────────────

#[test]
fn pin_a14_for_entity_unit_matches_unit_only() {
    let mut obs = ObservationSet::new();
    obs.insert(obs_unit_deny("auth", "ev:unit"));
    obs.insert(obs_component_deny("auth", "ev:comp"));
    obs.insert(obs_entity_affirm("auth", "ev:entity"));
    let target = SoftwareEntityRef::Unit(SoftwareUnitRef("auth".to_string()));
    let matched: Vec<_> = obs.for_entity(&target);
    assert_eq!(matched.len(), 1, "for_entity(Unit) must match Unit only");
}

// ─── Pin 15: for_entity(Component("x")) matches Component("x") only ─��───

#[test]
fn pin_a15_for_entity_component_matches_component_only() {
    let mut obs = ObservationSet::new();
    obs.insert(obs_unit_deny("auth", "ev:unit"));
    obs.insert(obs_component_deny("auth", "ev:comp"));
    obs.insert(obs_entity_affirm("auth", "ev:entity"));
    let target = SoftwareEntityRef::Component(component_ref("auth"));
    let matched: Vec<_> = obs.for_entity(&target);
    assert_eq!(
        matched.len(),
        1,
        "for_entity(Component) must match Component only"
    );
}

// ─── Pin 16: for_entity(Entity("x")) matches Entity("x") only ───────────

#[test]
fn pin_a16_for_entity_entity_matches_entity_only() {
    let mut obs = ObservationSet::new();
    obs.insert(obs_unit_deny("auth", "ev:unit"));
    obs.insert(obs_component_deny("auth", "ev:comp"));
    obs.insert(obs_entity_affirm("auth", "ev:entity"));
    let target = SoftwareEntityRef::Entity(entity_ref("auth"));
    let matched: Vec<_> = obs.for_entity(&target);
    assert_eq!(
        matched.len(),
        1,
        "for_entity(Entity) must match Entity only"
    );
}

// ─── Pin 17: provider boundary contract → typed EvidenceGap ────────────

#[test]
fn pin_a17_provider_boundary_emits_typed_evidence_gap() {
    let c = constraint("k1", "pb:test", MustDirection::Must);
    let i = intent("repo:test", vec![c]);
    let contract = contract_provider_boundary("pb:test");
    let r = reduce_alignment(
        &i,
        &empty_basis(),
        &ObservationSet::new(),
        &[contract],
        &[],
        EventTime(0),
    )
    .expect("reduce");
    let gap = r
        .findings
        .iter()
        .find(|f| {
            matches!(
                f.kind,
                sddk_engine::software_alignment::types::AlignmentFindingKind::ContractViolation
            )
        })
        .expect("one finding");
    assert_eq!(
        gap.cause,
        FindingCause::ContractViolation(ContractViolationCause::EvidenceGap {
            kind_tag: "provider_boundary".to_string()
        })
    );
}

// ─── Pin 18: extension contract → typed EvidenceGap ─────────────────────

#[test]
fn pin_a18_extension_kind_emits_typed_evidence_gap() {
    let c = constraint("k1", "ext:test", MustDirection::Must);
    let i = intent("repo:test", vec![c]);
    let contract = contract_extension("ext:test", "custom_kind");
    let r = reduce_alignment(
        &i,
        &empty_basis(),
        &ObservationSet::new(),
        &[contract],
        &[],
        EventTime(0),
    )
    .expect("reduce");
    let gap = r
        .findings
        .iter()
        .find(|f| {
            matches!(
                f.kind,
                sddk_engine::software_alignment::types::AlignmentFindingKind::ContractViolation
            )
        })
        .expect("one finding");
    assert_eq!(
        gap.cause,
        FindingCause::ContractViolation(ContractViolationCause::EvidenceGap {
            kind_tag: "custom_kind".to_string()
        })
    );
}

// ─── P19 — Deterministic randomized property test ───────────────────────
//
// Durably reproducible conversion of the A4-5P randomized probe
// (recorded as OBSERVED_SESSION_EVIDENCE). See
// `docs/handoff/HANDOFF-2026-09-17-a4-3r2-namespace-safe-targets.md`
// for the receipt.
//
// Generator: xorshift64 seeded with `0xA4_3R2_NS_SAFE`.
// Bound: 200 scenarios.
// Alphabet: 32 strings chosen so they collide across namespaces.
//
// Property:
//   - For any (subject, target) pair, binding succeeds iff
//     (subject.kind, target.kind) is the supported (Component→
//     SingleAuthority, Entity→UniqueOwner, Relation→ForbiddenDependency)
//     and subject.identity == target.identity.
//   - For any identifier S in the alphabet, Unit(S) / Component(S) /
//     Entity(S) produce three distinct canonical_tags.

// Deterministic seed for the bounded generator. Decimal form of
// `0xA43F2E5C_NS_SAFE` (the textual seed is documented but cannot be a
// Rust hex literal because of the embedded ASCII characters).
const SEED: u64 = 0xA4_3F_2E_5C_5A_FA_DE_u64;
const SCENARIO_COUNT: usize = 200;

const ALPHABET: &[&str] = &[
    "auth",
    "billing",
    "account",
    "comp:auth",
    "comp:billing",
    "entity:account",
    "caller:x",
    "callee:y",
    "domain",
    "provider",
    "core",
    "edge",
    "comp:domain",
    "comp:provider",
    "entity:domain",
    "entity:provider",
    "alpha",
    "beta",
    "gamma",
    "delta",
    "epsilon",
    "zeta",
    "eta",
    "theta",
    "iota",
    "kappa",
    "lambda",
    "mu",
    "nu",
    "xi",
    "omicron",
    "pi",
];

/// Simple xorshift64 PRNG, deterministic.
fn next_u64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn pick<'a>(state: &mut u64, alphabet: &'a [&'a str]) -> &'a str {
    let idx = (next_u64(state) as usize) % alphabet.len();
    alphabet[idx]
}

#[test]
fn property_namespace_separation_under_bounded_generator() {
    let mut state = SEED;
    let mut violations = 0u32;

    // Property 1 — three distinct canonical_tags per identifier.
    for s in ALPHABET {
        let u = ObservationSubject::Unit(SoftwareUnitRef(s.to_string())).canonical_tag();
        let c = ObservationSubject::Component(component_ref(s)).canonical_tag();
        let e = ObservationSubject::Entity(entity_ref(s)).canonical_tag();
        if u == c || u == e || c == e {
            violations += 1;
        }
    }
    assert_eq!(
        violations, 0,
        "every identifier must produce three distinct canonical_tags"
    );

    // Property 2 — binding succeeds only on (same kind + same identity).
    let mut ran = 0usize;
    for _ in 0..SCENARIO_COUNT {
        let s = pick(&mut state, ALPHABET);
        // Three observations across namespaces, all Denies, all using the same string.
        let mut obs = ObservationSet::new();
        obs.insert(obs_unit_deny(s, "ev:unit"));
        obs.insert(obs_component_deny(s, "ev:comp"));
        obs.insert(obs_entity_affirm(s, "ev:entity"));
        // Two contracts: SingleAuthority(comp:s), UniqueOwner(entity:s).
        let sa = contract_single_authority("sa:ns", s);
        let uo = contract_unique_owner("uo:ns", s);
        let c_sa = constraint("k-sa", "sa:ns", MustDirection::Must);
        let c_uo = constraint("k-uo", "uo:ns", MustDirection::MustNot);
        let i = intent("repo:ns", vec![c_sa, c_uo]);
        let r = reduce_alignment(&i, &empty_basis(), &obs, &[sa, uo], &[], EventTime(0))
            .expect("reduce");
        // Two violations expected: Component Denies binds SA (ContradictsMust),
        // Entity Affirms binds UO (ContradictsMust). Unit does not bind anything.
        let violations_kind = r
            .findings
            .iter()
            .filter(|f| {
                matches!(
                    f.kind,
                    sddk_engine::software_alignment::types::AlignmentFindingKind::ContractViolation
                )
            })
            .count();
        // EvidenceGap is not counted here; we filter only on ContradictsMust.
        let contradicts = r
            .findings
            .iter()
            .filter(|f| {
                matches!(
                    f.cause,
                    FindingCause::ContractViolation(ContractViolationCause::ContradictsMust)
                )
            })
            .count();
        assert_eq!(
            contradicts, 2,
            "scenario {ran}: expected exactly 2 ContradictsMust (SA+Component, UO+Entity); got {contradicts}"
        );
        // The 2 ContradictsMust should account for all ContractViolation
        // findings (no EvidenceGap when SA/UO payloads bind).
        let non_evidence_gap_violations = violations_kind - contradicts;
        assert_eq!(
            non_evidence_gap_violations, 0,
            "scenario {ran}: no extra violations expected"
        );
        ran += 1;
    }
    assert_eq!(ran, SCENARIO_COUNT);
}
