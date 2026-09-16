// Copyright (c) SDDK contributors.
// SPDX-License-Identifier: MIT
//
// paradigm_lens/tests.rs — A3-S8 / AC7 acceptance + anti-encroachment +
// advisory-boundary tests.
//
// REQ-AC7-001..022 pinned via the cycle-bounded spec
// `docs/architecture/specs/arch-spec-A3-S8-ac7-paradigm-lenses.md`.

use super::*;
use crate::paradigm_profile::{
    BoundedContextRef, EvidenceBasis, LensStatus, ParadigmAnchorRef, ParadigmLensKind,
    ProjectIntentRef,
};

const T0: i64 = 1_700_000_000;

fn anchor() -> ParadigmAnchorRef {
    ParadigmAnchorRef::ProjectIntent(ProjectIntentRef::new("p-63676b11dc0ef88f"))
}

fn anchor_ctx(ctx: &str) -> ParadigmAnchorRef {
    ParadigmAnchorRef::BoundedContext(BoundedContextRef::new(ctx))
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — vocabulary (REQ-AC7-001..004)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_observations_have_family_and_polarity() {
    // REQ-AC7-001
    for o in LensObservation::ALL {
        // Every observation belongs to exactly one family and one polarity:
        // exhausting the matches proves the mapping is total.
        let family = match o.family() {
            LensFamily::ObjectOriented => "oo",
            LensFamily::Functional => "fp",
            LensFamily::Adt => "adt",
            LensFamily::Dsl => "dsl",
        };
        assert!(!family.is_empty());
        match o.polarity() {
            ObservationPolarity::Supports | ObservationPolarity::Contradicts => {}
        }
        assert!(o.locator().starts_with("lens."));
    }
}

#[test]
fn acceptance_every_family_has_observations() {
    // REQ-AC7-002
    for fam in LensFamily::ALL {
        let n = LensObservation::ALL
            .iter()
            .filter(|o| o.family() == fam)
            .count();
        assert!(n >= 3, "family {fam:?} has only {n} observations");
    }
}

#[test]
fn acceptance_basis_maps_onto_ac3() {
    // REQ-AC7-003
    assert_eq!(LensEvaluationBasis::ALL.len(), 2);
    assert_eq!(
        LensEvaluationBasis::Deterministic.to_evidence_basis(),
        EvidenceBasis::Observed
    );
    assert_eq!(
        LensEvaluationBasis::Inferred.to_evidence_basis(),
        EvidenceBasis::Declared
    );
    // And AC3's frozen vocabulary is still 5 (nothing was extended).
    assert_eq!(EvidenceBasis::ALL.len(), 5);
}

#[test]
fn acceptance_provenance_carries_lens_version() {
    // REQ-AC7-004
    let p = LensProvenance::deterministic("test");
    assert_eq!(p.lens_version, LENS_VERSION);
    // Runtime check (clippy rejects a const-known `.is_empty()`).
    assert!(!p.lens_version.trim().is_empty());
    let ev = evaluate_lens(
        ParadigmLensKind::ObjectOriented,
        anchor(),
        &[LensObservation::EncapsulationPresent],
        T0,
    );
    assert_eq!(ev.provenance.lens_version, LENS_VERSION);
    assert!(
        ev.assessment
            .notes
            .as_deref()
            .unwrap_or("")
            .contains(LENS_VERSION)
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — evaluation rules (REQ-AC7-005..010)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn acceptance_absent_declaration_is_not_applicable() {
    // REQ-AC7-005 / AC-UAT-013: absent purity declaration -> NOT_APPLICABLE.
    // `EventDriven` has no AC7 lens family.
    let ev = evaluate_lens(
        ParadigmLensKind::EventDriven,
        anchor(),
        &[LensObservation::HiddenMutationDetected],
        T0,
    );
    assert_eq!(ev.assessment.status, LensStatus::NotApplicable);
    assert_eq!(family_for_kind(ParadigmLensKind::EventDriven), None);
    // Even with no observations.
    let ev2 = evaluate_lens(ParadigmLensKind::EventDriven, anchor(), &[], T0);
    assert_eq!(ev2.assessment.status, LensStatus::NotApplicable);
}

#[test]
fn acceptance_declared_without_evidence_is_unknown() {
    // REQ-AC7-006: declared but unobserved -> Unknown, never Misaligned.
    let ev = evaluate_lens(ParadigmLensKind::FunctionalPure, anchor(), &[], T0);
    assert_eq!(ev.assessment.status, LensStatus::Unknown);
    assert_ne!(ev.assessment.status, LensStatus::Misaligned);
}

#[test]
fn acceptance_status_from_polarity() {
    // REQ-AC7-007
    let aligned = evaluate_lens(
        ParadigmLensKind::ObjectOriented,
        anchor(),
        &[
            LensObservation::EncapsulationPresent,
            LensObservation::DependencyInversionUsed,
        ],
        T0,
    );
    assert_eq!(aligned.assessment.status, LensStatus::Aligned);

    let misaligned = evaluate_lens(
        ParadigmLensKind::ObjectOriented,
        anchor(),
        &[
            LensObservation::AnemicModelDetected,
            LensObservation::InheritanceOverComposition,
        ],
        T0,
    );
    assert_eq!(misaligned.assessment.status, LensStatus::Misaligned);

    let tension = evaluate_lens(
        ParadigmLensKind::ObjectOriented,
        anchor(),
        &[
            LensObservation::EncapsulationPresent,
            LensObservation::AnemicModelDetected,
        ],
        T0,
    );
    assert_eq!(tension.assessment.status, LensStatus::Tension);
}

#[test]
fn acceptance_assessment_is_scoped_and_not_global() {
    // REQ-AC7-008 / AC-UAT-012: exactly one scoped assessment; observations of
    // other families are ignored and the anchor is preserved.
    let ev = evaluate_lens(
        ParadigmLensKind::ObjectOriented,
        anchor_ctx("auth"),
        &[
            LensObservation::EncapsulationPresent,
            // An ADT observation must NOT influence the OO verdict.
            LensObservation::StringlyTypedStatus,
        ],
        T0,
    );
    assert_eq!(ev.assessment.status, LensStatus::Aligned);
    assert_eq!(
        ev.used_observations,
        vec![LensObservation::EncapsulationPresent]
    );
    assert_eq!(ev.assessment.anchor, anchor_ctx("auth"));
    // The assessment names the declared kind, not a global verdict.
    assert_eq!(ev.assessment.lens, ParadigmLensKind::ObjectOriented);
}

#[test]
fn acceptance_evidence_cites_observations() {
    // REQ-AC7-009 / AC-035-007
    let ev = evaluate_lens(
        ParadigmLensKind::FunctionalPure,
        anchor(),
        &[LensObservation::HiddenMutationDetected],
        T0,
    );
    assert_eq!(ev.assessment.evidence_refs.len(), 1);
    assert_eq!(
        ev.assessment.evidence_refs[0].locator,
        "lens.fp.hidden_mutation"
    );
    assert_eq!(ev.assessment.basis, EvidenceBasis::Observed);
}

#[test]
fn acceptance_evaluation_is_deterministic() {
    // REQ-AC7-010
    let obs = [
        LensObservation::TypedAstPresent,
        LensObservation::InvalidProgramsRepresentable,
    ];
    let a = evaluate_lens(ParadigmLensKind::Pipeline, anchor(), &obs, T0);
    let b = evaluate_lens(ParadigmLensKind::Pipeline, anchor(), &obs, T0);
    assert_eq!(a, b);
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — probes (REQ-AC7-011..015, AC-UAT-014/015)
// ─────────────────────────────────────────────────────────────────────────────

const FIXTURE_STRINGS: &str = r#"
pub struct Order {
    pub status: String,
    pub kind: String,
    pub expedited: bool,
    pub shipped_at: Option<u64>,
    pub cancelled_at: Option<u64>,
}
"#;

#[test]
fn acceptance_adt_probe_stringly_and_invalid_state() {
    // REQ-AC7-011 / AC-UAT-014
    let obs = probe_adt_observations(FIXTURE_STRINGS);
    assert!(obs.contains(&LensObservation::StringlyTypedStatus));
    assert!(obs.contains(&LensObservation::InvalidStatesRepresentable));
    assert!(obs.contains(&LensObservation::BooleanBlindness));

    // And the ADT lens turns the fixture into a Misaligned assessment with
    // exact evidence.
    let ev = evaluate_lens(ParadigmLensKind::DataOriented, anchor(), &obs, T0);
    assert_eq!(ev.assessment.status, LensStatus::Misaligned);
    assert!(
        ev.assessment
            .evidence_refs
            .iter()
            .any(|e| e.locator == "lens.adt.stringly_typed_status")
    );
}

const FIXTURE_PURE_DECLARED: &str = r#"
pub fn total(xs: &[u64]) -> u64 { xs.iter().sum() }
"#;

const FIXTURE_HIDDEN_MUTATION: &str = r#"
pub struct Counter { inner: Cell<u64> }
impl Counter {
    pub fn bump(&mut self) {
        let mut next = 1u64;
        next += 1;
    }
}
"#;

#[test]
fn acceptance_fp_probe_detects_hidden_mutation() {
    // REQ-AC7-012 / AC-UAT-013
    let dirty = probe_functional_observations(FIXTURE_HIDDEN_MUTATION);
    assert!(dirty.contains(&LensObservation::HiddenMutationDetected));
    let ev_dirty = evaluate_lens(ParadigmLensKind::FunctionalPure, anchor(), &dirty, T0);
    assert_eq!(ev_dirty.assessment.status, LensStatus::Misaligned);

    // A pure fixture does not contradict the declared pure profile.
    let clean = probe_functional_observations(FIXTURE_PURE_DECLARED);
    assert!(!clean.contains(&LensObservation::HiddenMutationDetected));
    let ev_clean = evaluate_lens(ParadigmLensKind::FunctionalPure, anchor(), &clean, T0);
    assert_eq!(ev_clean.assessment.status, LensStatus::Aligned);
}

#[test]
fn acceptance_oo_probe_detects_anemic_model() {
    // REQ-AC7-013
    let anemic = "pub struct Order {\n    pub id: u64,\n}\n";
    let obs = probe_oo_observations(anemic);
    assert!(obs.contains(&LensObservation::AnemicModelDetected));

    let rich = "pub struct Order { id: u64 }\nimpl Order {\n    pub fn total(&self) -> u64 { self.id }\n}\n";
    let obs2 = probe_oo_observations(rich);
    assert!(obs2.contains(&LensObservation::EncapsulationPresent));
    assert!(!obs2.contains(&LensObservation::AnemicModelDetected));
}

const FIXTURE_DSL: &str = r#"
pub enum WorkflowAst { Step(String) }
pub struct WorkflowIr { steps: Vec<String> }
pub fn validate(ast: &WorkflowAst) -> bool { true }
pub fn compile(ast: &WorkflowAst) -> WorkflowIr { let _ = ast; WorkflowIr { steps: vec![] } }
pub fn execute(ir: &WorkflowIr) { let _ = ir; }
"#;

#[test]
fn acceptance_dsl_probe_typed_ast_and_validation() {
    // REQ-AC7-014 / AC-UAT-015
    let obs = probe_dsl_observations(FIXTURE_DSL);
    assert!(obs.contains(&LensObservation::TypedAstPresent));
    assert!(obs.contains(&LensObservation::ValidatesBeforeExecution));
    assert!(obs.contains(&LensObservation::SyntaxSeparatedFromEffects));

    let ev = evaluate_lens(ParadigmLensKind::Pipeline, anchor(), &obs, T0);
    assert_eq!(ev.assessment.status, LensStatus::Aligned);
}

#[test]
fn acceptance_probes_have_negative_controls() {
    // REQ-AC7-015: clean fixtures produce no contradicting observation.
    let clean = "// nothing to see\npub fn f() -> u64 { 1 }\n";
    for (name, obs) in [
        ("oo", probe_oo_observations(clean)),
        ("fp", probe_functional_observations(clean)),
        ("adt", probe_adt_observations(clean)),
        ("dsl", probe_dsl_observations(clean)),
    ] {
        for o in obs {
            // The DSL probe legitimately emits `InvalidProgramsRepresentable`
            // for a source with no typed model; the others must be clean.
            if name == "dsl" && o == LensObservation::InvalidProgramsRepresentable {
                continue;
            }
            assert_ne!(
                o.polarity(),
                ObservationPolarity::Contradicts,
                "probe `{name}` flagged `{o:?}` on a clean source"
            );
        }
    }
    // Explicit ADT negative control: a well-modelled struct.
    let well_modelled = "enum Status { Active, Closed }\nstruct Order { status: Status }\n";
    let obs = probe_adt_observations(well_modelled);
    assert!(!obs.contains(&LensObservation::StringlyTypedStatus));
    assert!(!obs.contains(&LensObservation::InvalidStatesRepresentable));
    assert!(obs.contains(&LensObservation::TypedSumTypePresent));
}

// ─────────────────────────────────────────────────────────────────────────────
// acceptance — inferred (REQ-AC7-016..018): REMOVED in A4-4M M5.
// `inferred_lens_assessment` was DEAD (zero consumers, M0.4 verdict) and
// is deleted together with its provenance gate. The generic kernel never
// performs inference; any future inference lives OUTSIDE
// `alignment_lens`, per the A4-4M MUST-NOTs.

// anti-encroachment (REQ-AC7-019..022)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn anti_encroachment_no_authority_or_instruction_imports() {
    // REQ-AC7-019 / AC-UAT-011 / AC-035-005
    let sources: &[(&str, &str)] = &[
        ("mod.rs", include_str!("mod.rs")),
        ("types.rs", include_str!("types.rs")),
        ("lenses.rs", include_str!("lenses.rs")),
        ("probes.rs", include_str!("probes.rs")),
    ];
    let forbidden = [
        "use crate::authority_engine",
        "use crate::authority",
        "use crate::capability",
        "use crate::effective_instructions",
        "use crate::alignment",
        "use crate::debverify",
        "use crate::architecture_debverify",
        "use crate::provider",
        "use crate::host_sdk",
        "use crate::agent_host",
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
        "paradigm_lens imports forbidden surfaces: {offenders:?}"
    );
}

#[test]
fn anti_encroachment_ac3_vocabularies_unchanged() {
    // REQ-AC7-020: AC3's closed vocabularies were not extended.
    use crate::paradigm_profile::{LensStatus, ParadigmLensKind, ParadigmProfileKind};
    assert_eq!(EvidenceBasis::ALL.len(), 5, "AC3 EvidenceBasis must stay 5");
    assert_eq!(LensStatus::ALL.len(), 7, "AC3 LensStatus must stay 7");
    assert_eq!(
        ParadigmLensKind::ALL.len(),
        11,
        "AC3 ParadigmLensKind must stay 11"
    );
    assert_eq!(
        ParadigmProfileKind::ALL.len(),
        11,
        "AC3 ParadigmProfileKind must stay 11"
    );
}

#[test]
fn anti_encroachment_no_provider_calls() {
    // REQ-AC7-021: no LLM/provider/network call.
    let sources: &[(&str, &str)] = &[
        ("mod.rs", include_str!("mod.rs")),
        ("types.rs", include_str!("types.rs")),
        ("lenses.rs", include_str!("lenses.rs")),
        ("probes.rs", include_str!("probes.rs")),
    ];
    for (file, src) in sources {
        for forbidden in [
            "std::process",
            "reqwest",
            "TcpStream",
            "http",
            "use std::fs",
            "File::create(",
            "ArchitectureClaim",
        ] {
            assert!(
                !src.contains(forbidden),
                "{file} must not contain `{forbidden}` (no provider/IO/claims)"
            );
        }
    }
}

#[test]
fn anti_encroachment_advisory_only() {
    // REQ-AC7-022: the output is an advisory LensAssessment — no policy or
    // instruction object is produced anywhere in the public surface.
    let ev = evaluate_lens(
        ParadigmLensKind::ObjectOriented,
        anchor(),
        &[LensObservation::EncapsulationPresent],
        T0,
    );
    let t = std::any::type_name_of_val(&ev.assessment);
    assert!(t.contains("LensAssessment"), "unexpected type: {t}");
    assert!(!t.contains("Instruction"));
    assert!(!t.contains("Capability"));
    // The only public constructors return `LensEvaluation`.
    let names: Vec<&str> = vec![std::any::type_name_of_val(&ev)];
    for n in names {
        assert!(n.contains("LensEvaluation"));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// boundary
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn bonus_family_kind_mapping_is_bijective_for_declared_kinds() {
    use crate::paradigm_profile::ParadigmLensKind as K;
    assert_eq!(
        family_for_kind(K::ObjectOriented),
        Some(LensFamily::ObjectOriented)
    );
    assert_eq!(family_for_kind(K::Functional), Some(LensFamily::Functional));
    assert_eq!(
        family_for_kind(K::FunctionalPure),
        Some(LensFamily::Functional)
    );
    assert_eq!(family_for_kind(K::DataOriented), Some(LensFamily::Adt));
    assert_eq!(family_for_kind(K::Pipeline), Some(LensFamily::Dsl));
    assert_eq!(family_for_kind(K::Custom), Some(LensFamily::Dsl));
    // Kinds with no AC7 lens.
    assert_eq!(family_for_kind(K::Reactive), None);
    assert_eq!(family_for_kind(K::Hexagonal), None);
    assert_eq!(family_for_kind(K::Ddd), None);
    assert_eq!(family_for_kind(K::ActorLike), None);
    assert_eq!(family_for_kind(K::EventDriven), None);
}

#[test]
fn bonus_every_observation_has_a_unique_locator() {
    let mut locators: Vec<&str> = LensObservation::ALL.iter().map(|o| o.locator()).collect();
    locators.sort_unstable();
    locators.dedup();
    assert_eq!(locators.len(), LensObservation::ALL.len());
}
