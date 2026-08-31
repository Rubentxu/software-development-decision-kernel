//! Tests for the 13-smell detector.

use sddk_domain::{
    UatFeature, UatFormCheck, UatFormElementKind as FEK, UatFormEvidenceKind as FEVK,
    UatFormInputKind as FIK, UatFormItem, UatFormOracleKind as FOK, UatFormSpec,
    UatFormVisibility as FVIS, UatPlan, UatPlanRelease, UatPriority, UatScenario,
};

use super::detector::detect_13_smells;
use super::report::QualityThreshold;

// ─── Helper constructors ────────────────────────────────────────────────────

fn mk_check(
    id: &str,
    prompt: &str,
    oracle: Option<FOK>,
    ev: Vec<FEVK>,
    expected: Option<&str>,
) -> UatFormItem {
    UatFormItem {
        kind: FEK::Check,
        id: Some(id.into()),
        check: Some(UatFormCheck {
            kind: FIK::Confirm,
            prompt: prompt.into(),
            oracle,
            visibility: FVIS::Visible,
            required: true,
            blocking: true,
            confidence_requirement: None,
            evidence_requirement: ev,
            comment_required_when: None,
            options: vec![],
            expected: expected.map(String::from),
        }),
        text: None,
        flow: None,
        target: None,
        checkpoint: None,
    }
}

fn mk_rating_check(id: &str, prompt: &str) -> UatFormItem {
    UatFormItem {
        kind: FEK::Check,
        id: Some(id.into()),
        check: Some(UatFormCheck {
            kind: FIK::Rating,
            prompt: prompt.into(),
            oracle: None,
            visibility: FVIS::Visible,
            required: true,
            blocking: true,
            confidence_requirement: None,
            evidence_requirement: vec![],
            comment_required_when: None,
            options: vec![],
            expected: None,
        }),
        text: None,
        flow: None,
        target: None,
        checkpoint: None,
    }
}

fn mk_text_item(id: &str, text: &str) -> UatFormItem {
    UatFormItem {
        kind: FEK::Info,
        id: Some(id.into()),
        check: None,
        text: Some(text.into()),
        flow: None,
        target: None,
        checkpoint: None,
    }
}

fn mk_blind_check(id: &str, expected: Option<&str>) -> UatFormItem {
    UatFormItem {
        kind: FEK::Check,
        id: Some(id.into()),
        check: Some(UatFormCheck {
            kind: FIK::Confirm,
            prompt: "Hidden check".into(),
            oracle: None,
            visibility: FVIS::Blind,
            required: true,
            blocking: true,
            confidence_requirement: None,
            evidence_requirement: vec![],
            comment_required_when: None,
            options: vec![],
            expected: expected.map(String::from),
        }),
        text: None,
        flow: None,
        target: None,
        checkpoint: None,
    }
}

fn simple_scenario(id: &str, title: &str, items: Vec<UatFormItem>) -> UatScenario {
    UatScenario {
        id: id.into(),
        title: title.into(),
        priority: UatPriority::P2,
        assignee: sddk_domain::UatAssignee::default(),
        preconditions: vec![],
        plain_steps: vec![],
        technical_steps: vec![],
        rationale: None,
        evidence_prompt: None,
        flags: vec![],
        est_minutes: 0,
        context: None,
        evidence: None,
        risk: None,
        automation: None,
        provenance: None,
        executor: None,
        evidence_bundle: None,
        oracles: vec![],
        review: None,
        acceptance: None,
        form: Some(UatFormSpec {
            dsl_version: 1,
            items,
            completion: None,
        }),
        form_checkpoint: None,
        form_completion: None,
        completion: None,
        staleness: None,
    }
}

// ─── Fixtures ────────────────────────────────────────────────────────────────

fn clean_plan() -> UatPlan {
    UatPlan {
        schema_version: 2,
        release: UatPlanRelease {
            candidate: "v0.0.1".into(),
            project: None,
            last_uat_release: None,
        },
        generated_by: "test".into(),
        generated_at: "2026-08-01T00:00:00Z".into(),
        features: vec![UatFeature {
            id: "F-01".into(),
            name: "Test Feature".into(),
            requirement_ref: None,
            design_ref: None,
            priority: UatPriority::P2,
            scenarios: vec![simple_scenario(
                "S-01",
                "Clean scenario",
                vec![mk_check(
                    "step-1",
                    "Does the button turn blue?",
                    Some(FOK::Http),
                    vec![FEVK::Screenshot],
                    Some("blue"),
                )],
            )],
        }],
        runner_mode: None,
        approval: None,
    }
}

fn plan_with_expected_absent() -> UatPlan {
    UatPlan {
        schema_version: 2,
        release: UatPlanRelease {
            candidate: "v0.0.1".into(),
            project: None,
            last_uat_release: None,
        },
        generated_by: "test".into(),
        generated_at: "2026-08-01T00:00:00Z".into(),
        features: vec![UatFeature {
            id: "F-01".into(),
            name: "Test Feature".into(),
            requirement_ref: None,
            design_ref: None,
            priority: UatPriority::P2,
            scenarios: vec![simple_scenario(
                "S-01",
                "Missing expected",
                vec![mk_check("step-1", "Is it correcto?", None, vec![], None)],
            )],
        }],
        runner_mode: None,
        approval: None,
    }
}

/// Plan that triggers all 13 smell categories.
fn smelly_13_plan() -> UatPlan {
    UatPlan {
        schema_version: 2,
        release: UatPlanRelease {
            candidate: "v0.0.1".into(),
            project: None,
            last_uat_release: None,
        },
        generated_by: "test".into(),
        generated_at: "2026-08-01T00:00:00Z".into(),
        features: vec![UatFeature {
            id: "F-01".into(),
            name: "Smelly Feature".into(),
            requirement_ref: None,
            design_ref: None,
            priority: UatPriority::P2,
            scenarios: vec![UatScenario {
                id: "S-all".into(),
                title: "All 13 smells".into(),
                priority: UatPriority::P2,
                assignee: sddk_domain::UatAssignee::default(),
                preconditions: vec!["CONFIG_DB is set".into()],
                plain_steps: vec![],
                technical_steps: vec![],
                rationale: None,
                evidence_prompt: None,
                flags: vec![],
                est_minutes: 0,
                context: None,
                evidence: None,
                risk: None,
                automation: None,
                provenance: None,
                executor: None,
                evidence_bundle: None,
                oracles: vec![],
                review: None,
                acceptance: None,
                form: Some(UatFormSpec {
                    dsl_version: 1,
                    items: vec![
                        // 1. AMBIGUOUS_INSTRUCTION
                        mk_check(
                            "step-1",
                            "Is it correcto?",
                            None,
                            vec![FEVK::Screenshot],
                            Some("ok"),
                        ),
                        // 2. EXPECTED_ABSENT (no expected, no oracle)
                        mk_check("step-2", "Check something", None, vec![], None),
                        // 3. MACHINE_OBSERVABLE (has oracle)
                        mk_check(
                            "step-3",
                            "Does HTTP return 200?",
                            Some(FOK::Http),
                            vec![FEVK::Screenshot],
                            Some("200"),
                        ),
                        // 4. DUPLICATED_CHECK
                        mk_check(
                            "step-4",
                            "HTTP 200 again?",
                            Some(FOK::Http),
                            vec![FEVK::Screenshot],
                            Some("200"),
                        ),
                        // 5. NO_RECOVERY_PATH (blocking without Retry/Block/Repeat/Branch flow)
                        mk_check(
                            "step-5",
                            "Critical check",
                            None,
                            vec![FEVK::Screenshot],
                            Some("ok"),
                        ),
                        // 6. LEADING_QUESTION
                        mk_check(
                            "step-6",
                            "¿Es la respuesta correcta?",
                            None,
                            vec![FEVK::Screenshot],
                            Some("yes"),
                        ),
                        // 7. SUBJECTIVE_NO_SCALE (Rating without options)
                        mk_rating_check("step-7", "Is the UX good?"),
                        // 8. FAIL_NO_EVIDENCE (blocking without evidence)
                        mk_check("step-8", "Check without evidence", None, vec![], Some("ok")),
                        // 9. STEP_TOO_LARGE (many separators in text)
                        mk_text_item(
                            "step-9",
                            "Click the button, fill the form, submit, check email, verify database",
                        ),
                        // 10. EXCESSIVE_STEPS (13 items, no checkpoint)
                        mk_check("step-10", "Check 10", None, vec![], None),
                        mk_check("step-11", "Check 11", None, vec![], None),
                        mk_check("step-12", "Check 12", None, vec![], None),
                        mk_check("step-13", "Check 13", None, vec![], None),
                        mk_check("step-14", "Check 14", None, vec![], None),
                        mk_check("step-15", "Check 15", None, vec![], None),
                        mk_check("step-16", "Check 16", None, vec![], None),
                        mk_check("step-17", "Check 17", None, vec![], None),
                        mk_check("step-18", "Check 18", None, vec![], None),
                        mk_check("step-19", "Check 19", None, vec![], None),
                        mk_check("step-20", "Check 20", None, vec![], None),
                        mk_check("step-21", "Check 21", None, vec![], None),
                        // 11. HIDDEN_PREREQUISITE ($TOKEN not in preconditions)
                        mk_text_item("step-22", "Using $TOKEN in action"),
                        // 12. NO_BRANCHING (4 checks, no flow)
                        mk_check("step-23", "Check 23", None, vec![], None),
                        mk_check("step-24", "Check 24", None, vec![], None),
                        mk_check("step-25", "Check 25", None, vec![], None),
                        mk_check("step-26", "Check 26", None, vec![], None),
                        // 13. BLIND_CHECK_WITHOUT_HIDDEN
                        mk_blind_check("step-27", None),
                    ],
                    completion: None,
                }),
                form_checkpoint: None,
                form_completion: None,
                completion: None,
                staleness: None,
            }],
        }],
        runner_mode: None,
        approval: None,
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[test]
fn test_clean_plan_passes() {
    let plan = clean_plan();
    let report = detect_13_smells(&plan, QualityThreshold::Blocker);
    for smell in &report.smells {
        eprintln!(
            "CLEAN PLAN SMELL: {} @ {:?}",
            smell.smell_id, smell.location
        );
    }
    assert_eq!(report.summary.total, 0, "clean plan has no smells");
    assert_eq!(report.verdict, "PASS");
}

#[test]
fn test_expected_absent_detected() {
    let plan = plan_with_expected_absent();
    let report = detect_13_smells(&plan, QualityThreshold::Blocker);
    assert!(
        report
            .smells
            .iter()
            .any(|s| s.smell_id == "EXPECTED_ABSENT"),
        "should detect EXPECTED_ABSENT"
    );
}

#[test]
fn test_blocker_threshold_stops_on_blockers() {
    let plan = plan_with_expected_absent();
    let report = detect_13_smells(&plan, QualityThreshold::Blocker);
    assert_eq!(report.verdict, "NEEDS_REVISION");
    assert!(report.summary.blockers >= 1);
}

#[test]
fn test_smelly_13_has_all_13_smell_ids() {
    let plan = smelly_13_plan();
    let report = detect_13_smells(&plan, QualityThreshold::Blocker);
    let found_ids: std::collections::HashSet<_> =
        report.smells.iter().map(|s| s.smell_id.as_str()).collect();

    let expected = [
        "EXPECTED_ABSENT",
        "AMBIGUOUS_INSTRUCTION",
        "MACHINE_OBSERVABLE",
        "DUPLICATED_CHECK",
        "NO_RECOVERY_PATH",
        "LEADING_QUESTION",
        "SUBJECTIVE_NO_SCALE",
        "FAIL_NO_EVIDENCE",
        "STEP_TOO_LARGE",
        "EXCESSIVE_STEPS",
        "HIDDEN_PREREQUISITE",
        "NO_BRANCHING",
        "BLIND_CHECK_WITHOUT_HIDDEN",
    ];

    for smell_id in expected {
        assert!(found_ids.contains(smell_id), "missing smell: {smell_id}");
    }
}

#[test]
fn test_threshold_warning_escalation() {
    let plan = plan_with_expected_absent();
    let report = detect_13_smells(&plan, QualityThreshold::Warning);
    assert_eq!(report.threshold_applied, "WARNING");
}

#[test]
fn test_produces_quality_report_yaml() {
    let plan = clean_plan();
    let report = detect_13_smells(&plan, QualityThreshold::Blocker);
    let yaml = serde_saphyr::to_string(&report).unwrap();
    assert!(yaml.contains("smells"));
    assert!(yaml.contains("verdict"));
    assert!(yaml.contains("summary"));
}

#[test]
fn test_report_has_correct_schema_version() {
    let plan = clean_plan();
    let report = detect_13_smells(&plan, QualityThreshold::Blocker);
    assert_eq!(report.schema_version, 1);
    assert_eq!(report.analyzer, "uat-form-quality");
    assert_eq!(report.model, "heuristic-v1");
}

#[test]
fn test_feature_id_preserved_in_findings() {
    let plan = smelly_13_plan();
    let report = detect_13_smells(&plan, QualityThreshold::Blocker);
    assert!(
        report
            .smells
            .iter()
            .any(|s| s.location.feature_id == "F-01"),
        "feature_id should be preserved in smell location"
    );
}
