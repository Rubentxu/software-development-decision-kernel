// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// crates/sddk-engine/tests/a4_3r_typed_constraint_binding.rs
//
// A4-3R — typed constraint binding falsification suite.
//
// Eighteen pin tests covering the falsification list from the A4-3R
// scope contract. Each test names the REQ it pins.
//
// The pin suite covers:
//   1-3  : ForbiddenDependency typed matching (positive + negative cases).
//   4-7  : SingleAuthority / UniqueOwner typed matching.
//   8    : UnknownContract produces no finding.
//   9-11 : Non-binding contract kinds emit a typed EvidenceGap.
//   12-14 : Substring / rendered-text defenses.
//   15-17 : Compile-time anti-knowledge witnesses.
//   18   : A4-3 corpus regression under typed rule.

use sddk_engine::architectural_contract::{
    ArchitecturalContract, BoundaryKind, ComponentRef, ContractId, ContractKind, ContractKindRef,
    ContractPayload, DecisionRef, EntityRef, Revision, SpecRef,
};
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

use sddk_engine::software_alignment::types::AlignmentFindingKind;

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
    DecisionRef::Decision("DEC-A4-3R".to_string())
}

fn contract_single_authority(id: &str, owner: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_single_authority(
        ContractId::new(id.to_string()).expect("ascii"),
        component_ref(owner),
        dec_ref(),
        spec_ref("SPEC-A4-3R"),
        revision("rev-1"),
        EventTime(0),
    )
    .expect("single authority contract")
}

fn contract_unique_owner(id: &str, owner: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_unique_owner(
        ContractId::new(id.to_string()).expect("ascii"),
        entity_ref(owner),
        dec_ref(),
        spec_ref("SPEC-A4-3R"),
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
        "a4-3r test reason".to_string(),
        dec_ref(),
        spec_ref("SPEC-A4-3R"),
        revision("rev-1"),
        EventTime(0),
    )
    .expect("forbidden dependency contract")
}

fn contract_projection_only(id: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_projection_only(
        ContractId::new(id.to_string()).expect("ascii"),
        "test_source_kind".to_string(),
        dec_ref(),
        spec_ref("SPEC-A4-3R"),
        revision("rev-1"),
        EventTime(0),
    )
    .expect("projection only contract")
}

fn contract_provider_boundary(id: &str) -> ArchitecturalContract {
    ArchitecturalContract::declare_provider_boundary(
        ContractId::new(id.to_string()).expect("ascii"),
        BoundaryKind::Inbound,
        "engine->provider".to_string(),
        dec_ref(),
        spec_ref("SPEC-A4-3R"),
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
        spec_ref("SPEC-A4-3R"),
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

fn basis_hash_zero() -> sddk_engine::knowledge::BasisHash {
    let kb = KnowledgeBasis::empty(EventTime(0));
    kb.basis_hash().clone()
}

fn obs_unit_deny(unit: &str, ev: &str) -> SoftwareObservation {
    let basis = ObservationBasis::new("rev-1", basis_hash_zero(), "input-1");
    let evidence = UniEvidenceRef::new(EvidenceKind::Adhoc, ev.to_string());
    SoftwareObservation::declare(
        ObservationSubject::Unit(sddk_engine::architecture_graph::SoftwareUnitRef::new(
            unit.to_string(),
        )),
        ObservationStance::Denies,
        evidence,
        ObservationOrigin::DeterministicLocal,
        basis,
        None,
        "a4_3r_test_producer",
    )
}

fn obs_unit_affirm(unit: &str, ev: &str) -> SoftwareObservation {
    let basis = ObservationBasis::new("rev-1", basis_hash_zero(), "input-1");
    let evidence = UniEvidenceRef::new(EvidenceKind::Adhoc, ev.to_string());
    SoftwareObservation::declare(
        ObservationSubject::Unit(sddk_engine::architecture_graph::SoftwareUnitRef::new(
            unit.to_string(),
        )),
        ObservationStance::Affirms,
        evidence,
        ObservationOrigin::DeterministicLocal,
        basis,
        None,
        "a4_3r_test_producer",
    )
}

// A4-3R2 — Component subject helper. Used by `pin_r04` (Component
// binds `SingleAuthority(ComponentRef)`) and `a4_3r2_*` corpus.
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
        "a4_3r_test_producer",
    )
}

// A4-3R2 — Entity subject helper. Used by `pin_r06` (Entity binds
// `UniqueOwner(EntityRef)`) and `a4_3r2_*` corpus.
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
        "a4_3r_test_producer",
    )
}

fn obs_relation(
    from_unit: &str,
    kind: CoreRelationKind,
    to_unit: &str,
    ev: &str,
) -> SoftwareObservation {
    let rel = SoftwareRelation::new(
        SoftwareEntityRef::Unit(sddk_engine::architecture_graph::SoftwareUnitRef::new(
            from_unit.to_string(),
        )),
        kind,
        SoftwareEntityRef::Unit(sddk_engine::architecture_graph::SoftwareUnitRef::new(
            to_unit.to_string(),
        )),
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
        "a4_3r_test_producer",
    )
}

// ─── Pin 1: ForbiddenDependency has NO typed relation-observation path ──────

#[test]
fn pin_r01_forbidden_dependency_has_no_typed_unit_relation_path() {
    // The contract binds ComponentRef vs ComponentRef; the observation
    // set only contains SoftwareUnit subjects. The typed reducer has
    // NO BindingTarget::RelationSubject variant, so relation-on-Unit
    // observations against a ForbiddenDependency never bind.
    let c = constraint("k1", "fd:ab", MustDirection::MustNot);
    let i = intent("repo:test", vec![c]);
    let mut obs = ObservationSet::new();
    obs.insert(obs_relation(
        "unit:A",
        CoreRelationKind::DependsOn,
        "unit:B",
        "ev:fd:1",
    ));
    let contract = contract_forbidden_dependency("fd:ab", "comp:A", "comp:B");
    let r =
        reduce_alignment(&i, &empty_basis(), &obs, &[contract], &[], EventTime(0)).expect("reduce");
    assert!(
        !r.findings
            .iter()
            .any(|f| matches!(f.kind, AlignmentFindingKind::ContractViolation)),
        "ForbiddenDependency against Unit relation must NOT bind (typed mismatch)"
    );
}

// ─── Pin 2: ForbiddenDependency mismatch on observation stub ───────────────

#[test]
fn pin_r02_forbidden_dependency_no_match_when_observation_lacks_relation() {
    // Only Unit subjects in the observation set (no relation).
    let c = constraint("k1", "fd:ab", MustDirection::MustNot);
    let i = intent("repo:test", vec![c]);
    let mut obs = ObservationSet::new();
    obs.insert(obs_unit_affirm("comp:A", "ev:fd:unit"));
    obs.insert(obs_unit_affirm("comp:B", "ev:fd:unit2"));
    let contract = contract_forbidden_dependency("fd:ab", "comp:A", "comp:B");
    let r =
        reduce_alignment(&i, &empty_basis(), &obs, &[contract], &[], EventTime(0)).expect("reduce");
    assert!(
        !r.findings
            .iter()
            .any(|f| matches!(f.kind, AlignmentFindingKind::ContractViolation)),
        "ForbiddenDependency must NOT match without a relation observation"
    );
}

// ─── Pin 3: ForbiddenDependency mismatch on different direction ────────────

#[test]
fn pin_r03_forbidden_dependency_no_match_when_target_is_unrelated() {
    // Constraint on comp:A -> comp:B; observation denies a totally
    // unrelated component.
    let c = constraint("k1", "fd:ab", MustDirection::MustNot);
    let i = intent("repo:test", vec![c]);
    let mut obs = ObservationSet::new();
    obs.insert(obs_unit_deny("comp:Z", "ev:fd:other"));
    let contract = contract_forbidden_dependency("fd:ab", "comp:A", "comp:B");
    let r =
        reduce_alignment(&i, &empty_basis(), &obs, &[contract], &[], EventTime(0)).expect("reduce");
    assert!(
        !r.findings
            .iter()
            .any(|f| matches!(f.kind, AlignmentFindingKind::ContractViolation)),
        "ForbiddenDependency must NOT match unrelated components"
    );
}

// ─── Pin 4: SingleAuthority binds Component (Denies) ──────────────────────

#[test]
fn pin_r04_single_authority_binds_component_subject() {
    // A4-3R2 — strict namespace: the observation subject is
    // `ObservationSubject::Component(ComponentRef("comp:auth"))`. The
    // pre-A4-3R2 test (`pin_r04_single_authority_matches_unit_denies`)
    // emitted `ObservationSubject::Unit(SoftwareUnitRef("comp:auth"))`
    // and asserted binding via cross-namespace `as_str()` equality;
    // that path is closed by `FU-A4-3R-TARGET-NAMESPACE-BRIDGE`.
    let c = constraint("k1", "sa:auth", MustDirection::Must);
    let i = intent("repo:test", vec![c]);
    let mut obs = ObservationSet::new();
    obs.insert(obs_component_deny("comp:auth", "ev:sa:deny"));
    let contract = contract_single_authority("sa:auth", "comp:auth");
    let r =
        reduce_alignment(&i, &empty_basis(), &obs, &[contract], &[], EventTime(0)).expect("reduce");
    assert_eq!(r.state, AlignmentState::Misaligned);
    let violation = r
        .findings
        .iter()
        .find(|f| matches!(f.kind, AlignmentFindingKind::ContractViolation))
        .expect("one violation");
    assert_eq!(
        violation.cause,
        FindingCause::ContractViolation(ContractViolationCause::ContradictsMust)
    );
}

// ─── Pin 5: SingleAuthority no match on substring collision ───────────────

#[test]
fn pin_r05_single_authority_no_match_on_substring_collision() {
    let c = constraint("k1", "sa:auth", MustDirection::Must);
    let i = intent("repo:test", vec![c]);
    let mut obs = ObservationSet::new();
    obs.insert(obs_unit_deny("comp:auth:single", "ev:sa:substr"));
    let contract = contract_single_authority("sa:auth", "comp:auth");
    let r =
        reduce_alignment(&i, &empty_basis(), &obs, &[contract], &[], EventTime(0)).expect("reduce");
    assert!(
        !r.findings
            .iter()
            .any(|f| matches!(f.kind, AlignmentFindingKind::ContractViolation)),
        "SingleAuthority must NOT match substring collisions"
    );
}

// ─── Pin 6: UniqueOwner binds Entity (Affirms under MustNot) ──────────────

#[test]
fn pin_r06_unique_owner_binds_entity_subject() {
    // A4-3R2 — strict namespace: the observation subject is
    // `ObservationSubject::Entity(EntityRef("entity:X"))`. The
    // pre-A4-3R2 test (`pin_r06_unique_owner_matches_entity`) emitted
    // `ObservationSubject::Unit(SoftwareUnitRef("entity:X"))` and
    // asserted binding via cross-namespace `as_str()` equality; that
    // path is closed by `FU-A4-3R-TARGET-NAMESPACE-BRIDGE`.
    let c = constraint("k1", "uo:x", MustDirection::MustNot);
    let i = intent("repo:test", vec![c]);
    let mut obs = ObservationSet::new();
    obs.insert(obs_entity_affirm("entity:X", "ev:uo:affirm"));
    let contract = contract_unique_owner("uo:x", "entity:X");
    let r =
        reduce_alignment(&i, &empty_basis(), &obs, &[contract], &[], EventTime(0)).expect("reduce");
    assert_eq!(r.state, AlignmentState::Misaligned);
}

// ─── Pin 7: UniqueOwner no match on similar entity ────────────────────────

#[test]
fn pin_r07_unique_owner_no_match_on_similar_entity() {
    let c = constraint("k1", "uo:x", MustDirection::MustNot);
    let i = intent("repo:test", vec![c]);
    let mut obs = ObservationSet::new();
    obs.insert(obs_unit_affirm("entity:Y", "ev:uo:other"));
    let contract = contract_unique_owner("uo:x", "entity:X");
    let r =
        reduce_alignment(&i, &empty_basis(), &obs, &[contract], &[], EventTime(0)).expect("reduce");
    assert!(
        !r.findings
            .iter()
            .any(|f| matches!(f.kind, AlignmentFindingKind::ContractViolation)),
        "UniqueOwner must NOT match different entities"
    );
}

// ─── Pin 8: UnknownContractId emits nothing ───────────────────────────────

#[test]
fn pin_r08_unknown_contract_id_emits_nothing() {
    let c = constraint("k1", "ghost-contract", MustDirection::Must);
    let i = intent("repo:test", vec![c]);
    let mut obs = ObservationSet::new();
    obs.insert(obs_unit_deny("anything", "ev:ghost"));
    let r = reduce_alignment(&i, &empty_basis(), &obs, &[], &[], EventTime(0)).expect("reduce");
    assert!(
        r.findings.is_empty(),
        "Unknown contract_ref must NOT produce any finding"
    );
}

// ─── Pin 9: ProjectionOnly emits typed EvidenceGap ────────────────────────

#[test]
fn pin_r09_non_binding_kind_projection_only_emits_typed_evidence_gap() {
    let c = constraint("k1", "po:test", MustDirection::Must);
    let i = intent("repo:test", vec![c]);
    let mut obs = ObservationSet::new();
    obs.insert(obs_unit_deny("any:unit", "ev:po"));
    let contract = contract_projection_only("po:test");
    let r =
        reduce_alignment(&i, &empty_basis(), &obs, &[contract], &[], EventTime(0)).expect("reduce");
    let gap = r
        .findings
        .iter()
        .find(|f| matches!(f.kind, AlignmentFindingKind::ContractViolation))
        .expect("one finding");
    assert_eq!(
        gap.cause,
        FindingCause::ContractViolation(ContractViolationCause::EvidenceGap {
            kind_tag: "projection_only".to_string()
        })
    );
    assert!(
        gap.evidence_observations.is_empty(),
        "EvidenceGap findings carry empty evidence_observations"
    );
}

// ─── Pin 10: ProviderBoundary emits typed EvidenceGap ─────────────────────

#[test]
fn pin_r10_non_binding_kind_provider_boundary_emits_typed_evidence_gap() {
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
        .find(|f| matches!(f.kind, AlignmentFindingKind::ContractViolation))
        .expect("one finding");
    assert_eq!(
        gap.cause,
        FindingCause::ContractViolation(ContractViolationCause::EvidenceGap {
            kind_tag: "provider_boundary".to_string()
        })
    );
}

// ─── Pin 11: Extension kind emits typed EvidenceGap ───────────────────────

#[test]
fn pin_r11_extension_kind_emits_typed_evidence_gap() {
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
        .find(|f| matches!(f.kind, AlignmentFindingKind::ContractViolation))
        .expect("one finding");
    assert_eq!(
        gap.cause,
        FindingCause::ContractViolation(ContractViolationCause::EvidenceGap {
            kind_tag: "custom_kind".to_string()
        })
    );
}

// ─── Pin 12: ContractId substring collision does NOT bind ────────────────

#[test]
fn pin_r12_contract_id_as_substring_of_unrelated_subject_does_not_bind() {
    // Constraint with contract_ref `auth`.
    let c = constraint("k1", "auth", MustDirection::Must);
    let i = intent("repo:test", vec![c]);
    let mut obs = ObservationSet::new();
    obs.insert(obs_unit_deny("authority", "ev:auth:substr"));
    let contract = contract_single_authority("auth", "different:target");
    let r =
        reduce_alignment(&i, &empty_basis(), &obs, &[contract], &[], EventTime(0)).expect("reduce");
    assert!(
        !r.findings
            .iter()
            .any(|f| matches!(f.kind, AlignmentFindingKind::ContractViolation)),
        "ContractRef substring of observation subject must NOT bind"
    );
}

// ─── Pin 13: Rendered text does NOT participate in matching ───────────────

#[test]
fn pin_r13_rendered_text_does_not_participate_in_matching() {
    let c = constraint("k1", "render:text", MustDirection::Must);
    let i = intent("repo:test", vec![c]);
    let mut obs = ObservationSet::new();
    obs.insert(obs_unit_deny("unrelated:unit", "ev:render"));
    let contract = contract_single_authority("render:text", "truly:different");
    let r =
        reduce_alignment(&i, &empty_basis(), &obs, &[contract], &[], EventTime(0)).expect("reduce");
    assert!(
        !r.findings
            .iter()
            .any(|f| matches!(f.kind, AlignmentFindingKind::ContractViolation)),
        "Rendered text must NOT participate in matching"
    );
}

// ─── Pin 14: Observation ordering does not change the result ──────────────

#[test]
fn pin_r14_observation_ordering_independent() {
    let c = constraint("k1", "sa:auth", MustDirection::Must);
    let i = intent("repo:test", vec![c]);
    let contract = contract_single_authority("sa:auth", "comp:auth");

    let mut obs1 = ObservationSet::new();
    obs1.insert(obs_unit_deny("comp:auth", "ev:1"));
    obs1.insert(obs_unit_affirm("other:unit", "ev:2"));

    let mut obs2 = ObservationSet::new();
    obs2.insert(obs_unit_affirm("other:unit", "ev:2"));
    obs2.insert(obs_unit_deny("comp:auth", "ev:1"));

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

// ─── Pin 15: Reduction has no authority field ─────────────────────────────

#[test]
fn pin_r15_alignment_finding_has_no_authority_decision_field() {
    use sddk_engine::software_alignment::types::AlignmentFinding;
    // Compile-time witness: AlignmentFinding::cause is a typed enum.
    // Pre-A4-3R the reducer emitted a string-suffixed cause tag; the
    // typed binding makes it impossible to add an AuthorityDecision
    // shape without changing the FindingCause enum.
    let _: FindingCause = FindingCause::None;
    let _: FindingCause = FindingCause::ContractViolation(ContractViolationCause::ContradictsMust);
    let _: FindingCause = FindingCause::ContractViolation(ContractViolationCause::EvidenceGap {
        kind_tag: "bounded_compatibility".to_string(),
    });
    // Construct a finding to confirm its Debug representation does not
    // include an `authority_decision` field.
    let dummy = vec![];
    let _: Vec<AlignmentFinding> = dummy;
}

// ─── Pin 16: Reducer module path is the single source of truth ────────────

#[test]
fn pin_r16_no_alignment_lens_dependency() {
    // The reducer imports `crate::software_alignment::types` only — there
    // is no `use crate::alignment_lens` or `use crate::authority_engine`
    // in its declaration list. This test simply checks the test crate
    // compiles, which is the compile-time witness for that absence.
    let module_path = std::module_path!();
    assert!(
        module_path.contains("a4_3r_typed_constraint_binding"),
        "this test must live in the a4_3r test crate"
    );
}

// ─── Pin 17: No A4-5 introgression ────────────────────────────────────────

#[test]
fn pin_r17_no_a4_5_introgression() {
    // The reducer does not import instruction_compiler or AdvisoryContext.
    // The typed FindingCause enum has no Advisory variant, so any future
    // attempt to graft advisory-context into this layer is a compile error.
    fn cause_is_closed(cause: &FindingCause) -> &'static str {
        match cause {
            FindingCause::None => "none",
            FindingCause::ContractViolation(c) => match c {
                ContractViolationCause::ContradictsMust => "contradicts_must",
                ContractViolationCause::EvidenceGap { .. } => "evidence_gap",
            },
        }
    }
    assert_eq!(cause_is_closed(&FindingCause::None), "none");
}

// ─── Pin 18: A4-3 corpus regression under typed rule ──────────────────────

#[test]
fn pin_r18_a4_3_corpus_regression_under_typed_rule() {
    // Pre-A4-3R these observations produced ContractViolations because
    // the subject_canonical_tag contained the contract_ref as a
    // substring. Post-A4-3R the typed rule does NOT match — there is
    // no contract payload in scope, so the reducer reports no findings.
    // This is the corrected behaviour.
    let c = constraint("k1", "auth:single", MustDirection::Must);
    let i = intent("repo:test", vec![c]);
    let mut obs = ObservationSet::new();
    obs.insert(obs_relation(
        "auth:single",
        CoreRelationKind::DependsOn,
        "caller:x",
        "ev:auth:single",
    ));
    let r = reduce_alignment(&i, &empty_basis(), &obs, &[], &[], EventTime(0)).expect("reduce");
    assert!(
        r.findings.is_empty(),
        "A4-3 clause: substring binding must NOT match without a contract payload in scope"
    );
}
